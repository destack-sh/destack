use crate::tests::TestSession;

#[test]
fn test_export_counters_show_symbol_scan_and_static_cache_work() {
    const ITEMS: usize = 50_000;

    let locals = (0..ITEMS)
        .map(|index| format!("let value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{locals}\n\n@if(true)\nexport {{ value0 }};");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_exported_key("main.tspp"), "export.");
    let expected = format!(
        "export.roots={}\n\
export.visibility_expressions={}\n\
export.export_expressions={}\n\
export.scanned_symbols={}\n\
export.guards=1\n\
export.skipped=0\n\
export.static_checks={}\n\
export.static_cache_hits={}",
        ITEMS + 1,
        ITEMS + 1,
        ITEMS + 1,
        ITEMS + 1,
        ITEMS * 5 + 3,
        ITEMS * 2 + 1,
    );

    assert_eq!(counters, expected);
}

#[test]
fn test_export_counters_scale_with_many_static_guards() {
    const ITEMS: usize = 50_000;

    let source = (0..ITEMS)
        .map(|index| format!("@if(false)\nexport let value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_exported_key("main.tspp"), "export.");
    let expected = format!(
        "export.roots={ITEMS}\n\
export.visibility_expressions={ITEMS}\n\
export.export_expressions={ITEMS}\n\
export.scanned_symbols={}\n\
export.guards={ITEMS}\n\
export.skipped={ITEMS}\n\
export.static_checks={}\n\
export.static_cache_hits={ITEMS}",
        ITEMS + 1,
        ITEMS * 2,
    );

    assert_eq!(counters, expected);
}
