use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_indirect_binding() {
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
/// @export.indirect key=Bar imported=Foo module=dep.ds

/// @module.summary edges=1
/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:1
"#,
    );
}

#[test]
fn test_export_records_default_indirect_aliases() {
    let compiler = TestSession::new()
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
/// @export.indirect key=value imported=<default> module=dep.ds
/// @export.indirect key=<default> imported=named module=dep.ds

/// @module.summary edges=1
/// @export.summary exports=2
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:1
"#,
    );
}
