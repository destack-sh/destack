use crate::tests::TestSession;

#[test]
fn test_import_counters_ignore_function_body_size() {
    const BODY_ITEMS: usize = 50_000;

    let body = (0..BODY_ITEMS)
        .map(|index| format!("    let value{index} = {index};"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("import {{ dep }} from \"./dep\";\n\nfunction heavy() {{\n{body}\n}}");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .module("dep.tspp", "export const dep = 1;")
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_imported_key("main.tspp"), "import.");
    let expected = "import.roots=2\n\
import.expressions=2\n\
import.import_clauses=1\n\
import.reexport_clauses=0\n\
import.guards=0\n\
import.skipped=0\n\
import.specifiers=1\n\
import.package_exports=0\n\
import.candidates=2\n\
import.probes=2";

    assert_eq!(counters, expected);
}

#[test]
fn test_import_counters_scale_with_many_module_clauses() {
    const ITEMS: usize = 50_000;

    let source = (0..ITEMS)
        .map(|_| "import { dep } from \"./dep\";".to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let compiler = TestSession::builder()
        .module("main.tspp", &source)
        .module("dep.tspp", "export const dep = 1;")
        .cold()
        .build();
    let counters = compiler.artifact_counters(compiler.dir_imported_key("main.tspp"), "import.");
    let expected = format!(
        "import.roots={ITEMS}\n\
import.expressions={ITEMS}\n\
import.import_clauses={ITEMS}\n\
import.reexport_clauses=0\n\
import.guards=0\n\
import.skipped=0\n\
import.specifiers={ITEMS}\n\
import.package_exports=0\n\
import.candidates={}\n\
import.probes={}",
        ITEMS * 2,
        ITEMS * 2,
    );

    assert_eq!(counters, expected);
}
