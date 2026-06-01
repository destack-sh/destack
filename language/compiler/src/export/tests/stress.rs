use std::collections::BTreeMap;

use crate::tests::TestSession;

#[test]
fn test_export_stats_show_symbol_scan_and_static_cache_work() {
    const ITEMS: usize = 50_000;

    let locals = (0..ITEMS)
        .map(|index| format!("let value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{locals}\n\n@if(true)\nexport {{ value0 }};");
    let compiler = TestSession::single(&source);
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_exported_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "export".to_string())]),
    );
    let expected = format!(
        "export.stats.roots={}\n\
export.stats.expressions=visibility:{},export:{}\n\
export.stats.symbols=scanned:{}\n\
export.stats.guards=evaluated:1,skipped:0\n\
export.stats.static.checks={}\n\
export.stats.static.cache_hits={}",
        ITEMS + 1,
        ITEMS + 1,
        ITEMS + 1,
        ITEMS + 1,
        (ITEMS + 1) * 3,
        ITEMS + 1,
    );

    assert_eq!(metadata, expected);
}
