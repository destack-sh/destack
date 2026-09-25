use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_indirect_binding() {
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
export let Foo = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::modules().with_export().with_summaries(),
        r#"
export { Foo as Bar } from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp
/// @export.reexport key=Bar imported=Foo declaration=Bar module=dep.tspp

/// @module.summary edges=1
/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_default_indirect_aliases() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export { default as value, named as default } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export default 1;
export let named = 2;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::modules().with_export().with_summaries(),
        r#"
export { default as value, named as default } from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp
/// @export.reexport key=value imported=<default> declaration=value module=dep.tspp
/// @export.reexport key=<default> imported=named declaration=default module=dep.tspp

/// @module.summary edges=1
/// @export.summary exports=2
"#,
    );
}
