use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_reexport_edge() {
    let compiler = TestSession::new()
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
        DirRows::imports().with_summaries(),
        r#"
export { Foo as Bar } from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_reexport_loader_attribute() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export { schema } from "./schema" with { type: "json" };
"#,
        )
        .data("schema.json", r#"{ "type": "object" }"#)
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
export { schema } from "./schema" with { type: "json" };
/// @dependency.edge relation=re_export specifier=./schema loader=json module=schema.json

/// @dependency.summary edges=1
"#,
    );
}
