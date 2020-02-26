use std::collections::HashMap;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

use csv::ReaderBuilder;
use semver::Version;
use serde::de::DeserializeOwned;
use serde_derive::Deserialize;

use rust_search_extension::minify::Minifier;

const MAX_CRATE_SIZE: usize = 20 * 1000;
const CRATES_INDEX_PATH: &str = "../extension/index/crates.js";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize, Debug)]
struct Crate {
    id: u64,
    name: String,
    downloads: u64,
    description: Option<String>,
    #[serde(skip_deserializing, default = "default_version")]
    version: Version,
}

#[derive(Deserialize, Debug)]
struct CrateVersion {
    crate_id: u64,
    num: Version,
}

#[derive(Debug)]
struct WordCollector {
    words: Vec<String>,
}

impl WordCollector {
    fn new() -> Self {
        WordCollector { words: vec![] }
    }

    #[inline]
    fn collect_crate_id(&mut self, value: &str) {
        let id = value.replace("-", "_");
        for word in id
            .to_lowercase()
            .split(|c| c == '_')
            .filter(|c| c.len() >= 3)
            .collect::<Vec<_>>()
        {
            self.words.push(word.to_string());
        }
    }

    #[inline]
    fn collect_crate_description(&mut self, value: &str) {
        let mut description = value.trim().to_string();
        // Check char boundary to prevent panic
        if description.is_char_boundary(100) {
            description.truncate(100);
        }
        self.words.push(description);
    }
}

fn default_version() -> Version {
    Version::parse("0.0.0").unwrap()
}

fn read_csv<D: DeserializeOwned>(path: &str) -> Result<Vec<D>> {
    let mut records: Vec<D> = vec![];
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .from_path(Path::new(path))?;
    for record in reader.deserialize() {
        records.push(record?);
    }
    Ok(records)
}

fn generate_javascript_crates_index(
    crates: Vec<Crate>,
    minifier: &Minifier,
) -> std::io::Result<String> {
    let mut contents = String::from("var N=null;");
    let crates_map: HashMap<String, (Option<String>, Version)> = crates
        .into_iter()
        .map(|item| {
            (
                minifier.mapping_minify_crate_id(item.name),
                (
                    item.description.map(|value| minifier.mapping_minify(value)),
                    item.version,
                ),
            )
        })
        .collect();
    let crate_index = format!(
        "var crateIndex={};",
        serde_json::to_string(&crates_map).unwrap()
    );
    contents.push_str(&Minifier::minify_json(crate_index));
    Ok(contents)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let csv_path = args.get(1).expect("Path is required...");

    let mut crates: Vec<Crate> = read_csv(&format!("{}{}", csv_path, "crates.csv"))?;
    crates.sort_unstable_by(|a, b| b.downloads.cmp(&a.downloads));
    crates = crates.drain(0..=MAX_CRATE_SIZE).collect();
    let mut versions: Vec<CrateVersion> = read_csv(&format!("{}{}", csv_path, "versions.csv"))?;
    versions.sort_unstable_by(|a, b| b.num.cmp(&a.num));

    // Filter out duplicated version to speed up find in the later.
    let mut unique_crate_ids: HashSet<u64> = HashSet::with_capacity(2 * MAX_CRATE_SIZE);
    versions = versions
        .into_iter()
        .filter(|v| {
            if unique_crate_ids.contains(&v.crate_id) {
                return false;
            }
            unique_crate_ids.insert(v.crate_id);
            false
        })
        .collect();
    let mut collector = WordCollector::new();
    crates.iter_mut().for_each(|item: &mut Crate| {
        if let Some(version) = versions.iter().find(|&v| v.crate_id == item.id) {
            item.version = version.num.to_owned();
        }

        if let Some(description) = &item.description {
            collector.collect_crate_description(description);
        }
        collector.collect_crate_id(&item.name);
    });

    // Extract frequency word mapping
    let minifier = Minifier::new(&collector.words);
    let mapping = minifier.get_mapping();
    let mut contents = format!("var mapping={};", serde_json::to_string(&mapping)?);
    contents.push_str(&generate_javascript_crates_index(crates, &minifier)?);
    let path = Path::new(
        args.get(2)
            .map(|path| path.as_str())
            .unwrap_or(CRATES_INDEX_PATH),
    );
    fs::write(path, &contents)?;
    println!("\nGenerate javascript crates index successful!");
    Ok(())
}
