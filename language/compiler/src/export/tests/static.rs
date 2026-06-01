use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_omits_static_if_false_declaration() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
@if(false)
export let value: number = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
@if(false)
export let value: number = 1;

/// @export.summary
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:2 guards=evaluated:1,skipped:1
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_clause() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let value = 1;

@if(false)
export { value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
let value = 1;

@if(false)
export { value };

/// @export.summary
/// @export.stats roots=2 expressions=visibility:2,export:2 symbols=scanned:2 guards=evaluated:1,skipped:1
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_clause_item() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let value = 1;
export { @if(false) value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
let value = 1;
export { @if(false) value };

/// @export.summary
/// @export.stats roots=2 expressions=visibility:2,export:2 symbols=scanned:2 guards=evaluated:1,skipped:0
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_global_declaration() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
@if(false)
global {
    export let value: number = 1;
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
@if(false)
global {
    export let value: number = 1;
}

/// @export.summary
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:2 guards=evaluated:1,skipped:1
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_global_reexport() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
global {
    @if(false)
    export { Foo } from "./missing.ds";
}
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
global {
    @if(false)
    export { Foo } from "./missing.ds";
}

/// @module.summary edges=0
/// @export.summary
/// @export.stats roots=1 expressions=visibility:2,export:2 symbols=scanned:1 guards=evaluated:1,skipped:1
"#,
    );
}

#[test]
fn test_export_records_static_if_true_reexport_item() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export { @if(false) Foo, @if(true) Bar } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let Foo = 1;
export let Bar = 2;
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
export { @if(false) Foo, @if(true) Bar } from "./dep.ds";
/// @module.edge relation=re_export specifier=./dep.ds module=dep.ds
/// @export.indirect key=Bar imported=Bar module=dep.ds

/// @module.summary edges=1
/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:1 guards=evaluated:2,skipped:0
"#,
    );
}
