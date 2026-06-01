use std::collections::BTreeMap;

use crate::tests::TestSession;

#[test]
fn test_import_stats_ignore_function_body_size() {
    const BODY_ITEMS: usize = 50_000;

    let body = (0..BODY_ITEMS)
        .map(|index| format!("    let value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("import {{ dep }} from \"./dep\";\n\nfunction heavy() {{\n{body}\n}}");
    let compiler = TestSession::new()
        .module("main.ds", &source)
        .module("dep.ds", "export const dep = 1;")
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_imported_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "import".to_string())]),
    );
    let expected = "import.stats.roots=2\n\
import.stats.expressions=2\n\
import.stats.clauses=import:1,reexport:0\n\
import.stats.resolve.specifiers=1\n\
import.stats.resolve.package_exports=0\n\
import.stats.resolve.candidates=6\n\
import.stats.resolve.probes=6";

    assert_eq!(metadata, expected);
}

#[test]
fn test_import_stats_scale_with_many_module_clauses() {
    const ITEMS: usize = 50_000;

    let source = (0..ITEMS)
        .map(|_| "import { dep } from \"./dep\";".to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let compiler = TestSession::new()
        .module("main.ds", &source)
        .module("dep.ds", "export const dep = 1;")
        .build();
    let metadata = compiler.artifact_text_sidecar(
        compiler.dir_imported_key("main.ds"),
        "metadata",
        &BTreeMap::from([("phase".to_string(), "import".to_string())]),
    );
    let expected = format!(
        "import.stats.roots={ITEMS}\n\
import.stats.expressions={ITEMS}\n\
import.stats.clauses=import:{ITEMS},reexport:0\n\
import.stats.resolve.specifiers={ITEMS}\n\
import.stats.resolve.package_exports=0\n\
import.stats.resolve.candidates={}\n\
import.stats.resolve.probes={}",
        ITEMS * 6,
        ITEMS * 6,
    );

    assert_eq!(metadata, expected);
}
