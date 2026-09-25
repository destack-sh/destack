use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_reexport_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export { Foo as Bar } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
export { Foo as Bar } from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_reexport_loader_attribute() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
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
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
export { schema } from "./schema" with { type: "json" };
/// @module.edge relation=re_export specifier=./schema loader=json module=schema.json

/// @module.summary edges=1
"#,
    );
}
