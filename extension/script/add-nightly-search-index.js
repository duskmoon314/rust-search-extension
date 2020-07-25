(function() {
    // Remove needless crate's search index, such as core, alloc, etc
    function cleanSearchIndex() {
        let searchIndex = {};
        searchIndex['std'] = window.searchIndex['std'];
        searchIndex['test'] = window.searchIndex['test'];
        searchIndex['proc_macro'] = window.searchIndex['proc_macro'];
        return searchIndex;
    }
    
    let p = document.querySelector('nav.sidebar>div.version>p');
    let nightlyVersion = p.textContent;
    window.postMessage({
        direction: "rust-search-extension:nightly",
        message: {
            nightlyVersion,
            searchIndex: cleanSearchIndex(window.searchIndex),
        },
    }, "*");
})();