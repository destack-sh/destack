use crate::tests::TestSession;

#[test]
fn test_resolve_counters_dedupe_repeated_global_references() {
    const ITEMS: usize = 50_000;

    let source = (0..ITEMS)
        .map(|index| format!("const value{index} = answer;"))
        .collect::<Vec<_>>()
        .join("\n");
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module("main.tspp", &source)
        .module(
            "globals.tspp",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_resolved_key("main.tspp"), "resolve.");
    let expected = format!(
        "resolve.roots={ITEMS}\n\
resolve.expressions={}\n\
resolve.type_expressions=0\n\
resolve.import_clauses=0\n\
resolve.reexport_clauses=0\n\
resolve.required_globals=1\n\
resolve.language_item_uses=0\n\
resolve.export_cache_misses=0\n\
resolve.export_cache_hits=0\n\
resolve.export_cycle_hits=0\n\
resolve.local_binding_lookups={ITEMS}\n\
resolve.import_items=0\n\
resolve.reexport_items=0\n\
resolve.export_table_loads=0",
        ITEMS * 2
    );

    assert_eq!(counters, expected);
}

#[test]
fn test_resolve_counters_cache_repeated_export_lookups() {
    const ITEMS: usize = 50_000;

    let imports = (0..ITEMS)
        .map(|index| format!("target as local{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("import {{ {imports} }} from \"./dep\";");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .module("dep.tspp", "export const target = 1;")
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_resolved_key("main.tspp"), "resolve.");
    let expected = format!(
        "resolve.roots=1\n\
resolve.expressions=1\n\
resolve.type_expressions=0\n\
resolve.import_clauses=1\n\
resolve.reexport_clauses=0\n\
resolve.required_globals=0\n\
resolve.language_item_uses=0\n\
resolve.export_cache_misses=1\n\
resolve.export_cache_hits={}\n\
resolve.export_cycle_hits=0\n\
resolve.local_binding_lookups=0\n\
resolve.import_items={ITEMS}\n\
resolve.reexport_items=0\n\
resolve.export_table_loads=1",
        ITEMS - 1
    );

    assert_eq!(counters, expected);
}

#[test]
fn test_resolve_counters_load_export_table_once_for_distinct_imports() {
    const ITEMS: usize = 50_000;

    let imports = (0..ITEMS)
        .map(|index| format!("value{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let exports = (0..ITEMS)
        .map(|index| format!("export const value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("import {{ {imports} }} from \"./dep\";");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .module("dep.tspp", &exports)
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_resolved_key("main.tspp"), "resolve.");
    let expected = format!(
        "resolve.roots=1\n\
resolve.expressions=1\n\
resolve.type_expressions=0\n\
resolve.import_clauses=1\n\
resolve.reexport_clauses=0\n\
resolve.required_globals=0\n\
resolve.language_item_uses=0\n\
resolve.export_cache_misses={ITEMS}\n\
resolve.export_cache_hits=0\n\
resolve.export_cycle_hits=0\n\
resolve.local_binding_lookups=0\n\
resolve.import_items={ITEMS}\n\
resolve.reexport_items=0\n\
resolve.export_table_loads=1",
    );

    assert_eq!(counters, expected);
}

#[test]
fn test_resolve_counters_cache_repeated_namespace_paths() {
    const ITEMS: usize = 50_000;

    let references = (0..ITEMS)
        .map(|_| "dep.target;".to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "import * as dep from \"./dep\";\n\
{references}"
    );
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .module("dep.tspp", "export const target = 1;")
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_resolved_key("main.tspp"), "resolve.");
    let expected = format!(
        "resolve.roots={}\n\
resolve.expressions={}\n\
resolve.type_expressions=0\n\
resolve.import_clauses=1\n\
resolve.reexport_clauses=0\n\
resolve.required_globals=0\n\
resolve.language_item_uses=6\n\
resolve.export_cache_misses=1\n\
resolve.export_cache_hits={}\n\
resolve.export_cycle_hits=0\n\
resolve.local_binding_lookups={ITEMS}\n\
resolve.import_items=1\n\
resolve.reexport_items=0\n\
resolve.export_table_loads=1",
        ITEMS + 1,
        ITEMS * 2 + 1,
        ITEMS - 1,
    );

    assert_eq!(counters, expected);
}

#[test]
fn test_resolve_counters_cache_repeated_nested_namespace_paths() {
    const ITEMS: usize = 50_000;

    let references = (0..ITEMS)
        .map(|_| "dep.api.target;".to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        "import * as dep from \"./dep\";\n\
{references}"
    );
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .module("dep.tspp", "export * as api from \"./api\";")
        .module("api.tspp", "export const target = 1;")
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_resolved_key("main.tspp"), "resolve.");
    let expected = format!(
        "resolve.roots={}\n\
resolve.expressions={}\n\
resolve.type_expressions=0\n\
resolve.import_clauses=1\n\
resolve.reexport_clauses=0\n\
resolve.required_globals=0\n\
resolve.language_item_uses=6\n\
resolve.export_cache_misses=2\n\
resolve.export_cache_hits={}\n\
resolve.export_cycle_hits=0\n\
resolve.local_binding_lookups={ITEMS}\n\
resolve.import_items=1\n\
resolve.reexport_items=0\n\
resolve.export_table_loads=2",
        ITEMS + 1,
        ITEMS * 3 + 1,
        (ITEMS - 1) * 2,
    );

    assert_eq!(counters, expected);
}
