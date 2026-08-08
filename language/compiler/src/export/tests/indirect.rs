use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_indirect_binding() {
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
export let Foo = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::modules()
            .with_export()
            .with_summaries()
            .with_export_stats(),
        r#"
export { Foo as Bar } from "./dep.ds";
/// @module.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.reexport key=Bar imported=Foo declaration=Bar module=dep.ds

/// @module.summary edges=1
/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:2
"#,
    );
}

#[test]
fn test_export_records_default_indirect_aliases() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export { default as value, named as default } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export default 1;
export let named = 2;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::modules()
            .with_export()
            .with_summaries()
            .with_export_stats(),
        r#"
export { default as value, named as default } from "./dep.ds";
/// @module.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.reexport key=value imported=<default> declaration=value module=dep.ds
/// @export.reexport key=<default> imported=named declaration=default module=dep.ds

/// @module.summary edges=1
/// @export.summary exports=2
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:3
"#,
    );
}
