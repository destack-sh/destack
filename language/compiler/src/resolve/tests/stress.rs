use std::collections::BTreeMap;

use crate::tests::TestSession;

#[test]
fn test_resolve_stats_dedupe_repeated_global_references() {
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
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module("main.ds", &source)
        .module(
            "globals.ds",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_resolved_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "resolve".to_string())]),
    );
    let expected = format!(
        "resolve.stats.roots={ITEMS}\n\
resolve.stats.expressions={}\n\
resolve.stats.types=0\n\
resolve.stats.globals=required:1,modules:1\n\
resolve.stats.lookups.local={ITEMS}\n\
resolve.stats.lookups.import_items=0\n\
resolve.stats.lookups.reexport_items=0\n\
resolve.stats.loads.exports=0\n\
resolve.stats.loads.globals=1",
        ITEMS * 2
    );

    assert_eq!(metadata, expected);
}

#[test]
fn test_resolve_stats_cache_repeated_export_lookups() {
    const ITEMS: usize = 50_000;

    let imports = (0..ITEMS)
        .map(|index| format!("target as local{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("import {{ {imports} }} from \"./dep\";");
    let compiler = TestSession::builder()
        .module("main.ds", &source)
        .module("dep.ds", "export const target = 1;")
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_resolved_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "resolve".to_string())]),
    );
    let expected = format!(
        "resolve.stats.roots=1\n\
resolve.stats.expressions=1\n\
resolve.stats.types=0\n\
resolve.stats.clauses=import:1,reexport:0\n\
resolve.stats.exports=miss:1,hit:{},cycle:0\n\
resolve.stats.lookups.local=0\n\
resolve.stats.lookups.import_items={ITEMS}\n\
resolve.stats.lookups.reexport_items=0\n\
resolve.stats.loads.exports=1\n\
resolve.stats.loads.globals=0",
        ITEMS - 1
    );

    assert_eq!(metadata, expected);
}

#[test]
fn test_resolve_stats_load_export_table_once_for_distinct_imports() {
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
        .module("main.ds", &source)
        .module("dep.ds", &exports)
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_resolved_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "resolve".to_string())]),
    );
    let expected = format!(
        "resolve.stats.roots=1\n\
resolve.stats.expressions=1\n\
resolve.stats.types=0\n\
resolve.stats.clauses=import:1,reexport:0\n\
resolve.stats.exports=miss:{ITEMS},hit:0,cycle:0\n\
resolve.stats.lookups.local=0\n\
resolve.stats.lookups.import_items={ITEMS}\n\
resolve.stats.lookups.reexport_items=0\n\
resolve.stats.loads.exports=1\n\
resolve.stats.loads.globals=0",
    );

    assert_eq!(metadata, expected);
}

#[test]
fn test_resolve_stats_cache_repeated_namespace_paths() {
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
        .module("main.ds", &source)
        .module("dep.ds", "export const target = 1;")
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_resolved_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "resolve".to_string())]),
    );
    let expected = format!(
        "resolve.stats.roots={}\n\
resolve.stats.expressions={}\n\
resolve.stats.types=0\n\
resolve.stats.clauses=import:1,reexport:0\n\
resolve.stats.exports=miss:1,hit:{},cycle:0\n\
resolve.stats.lookups.local={ITEMS}\n\
resolve.stats.lookups.import_items=1\n\
resolve.stats.lookups.reexport_items=0\n\
resolve.stats.loads.exports=1\n\
resolve.stats.loads.globals=0",
        ITEMS + 1,
        ITEMS * 2 + 1,
        ITEMS - 1
    );

    assert_eq!(metadata, expected);
}

#[test]
fn test_resolve_stats_cache_repeated_nested_namespace_paths() {
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
        .module("main.ds", &source)
        .module("dep.ds", "export * as api from \"./api\";")
        .module("api.ds", "export const target = 1;")
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_resolved_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "resolve".to_string())]),
    );
    let expected = format!(
        "resolve.stats.roots={}\n\
resolve.stats.expressions={}\n\
resolve.stats.types=0\n\
resolve.stats.clauses=import:1,reexport:0\n\
resolve.stats.exports=miss:2,hit:{},cycle:0\n\
resolve.stats.lookups.local={ITEMS}\n\
resolve.stats.lookups.import_items=1\n\
resolve.stats.lookups.reexport_items=0\n\
resolve.stats.loads.exports=2\n\
resolve.stats.loads.globals=0",
        ITEMS + 1,
        ITEMS * 3 + 1,
        (ITEMS - 1) * 2
    );

    assert_eq!(metadata, expected);
}
