use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_reexport_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export { Foo as Bar } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
export { Foo as Bar } from "./dep.ds";
/// @module.edge relation=re_export specifier=./dep.ds module=dep.ds

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:0,reexport:1
"#,
    );
}

#[test]
fn test_import_records_reexport_loader_attribute() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export { schema } from "./schema" with { type: "json" };
"#,
        )
        .data(
            "schema.json",
            r#"
{
    "type": "object"
}
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
export { schema } from "./schema" with { type: "json" };
/// @module.edge relation=re_export specifier=./schema loader=json module=schema.json

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:0,reexport:1
"#,
    );
}
