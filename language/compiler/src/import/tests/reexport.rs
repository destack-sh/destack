use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_import_records_reexport_edge() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
export { Foo as Bar } from "./dep.ds";
/// @dependency.edge relation=re_export specifier=./dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_reexport_loader_attribute() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
export { schema } from "./schema" with { type: "json" };
"#,
        )
        .data("schema.json", r#"{ "type": "object" }"#)
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
export { schema } from "./schema" with { type: "json" };
/// @dependency.edge relation=re_export specifier=./schema loader=json module=schema.json

/// @dependency.summary edges=1
"#,
    );
}
