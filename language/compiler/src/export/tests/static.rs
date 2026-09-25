use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_omits_static_if_false_declaration() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(false)
export let value: number = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
@if(false)
export let value: number = 1;

/// @export.summary
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_clause() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
let value = 1;

@if(false)
export { value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
let value = 1;

@if(false)
export { value };

/// @export.summary
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_clause_item() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
let value = 1;
export { @if(false) value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
let value = 1;
export { @if(false) value };

/// @export.summary
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_global_declaration() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(false)
global {
    export let value: number = 1;
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
@if(false)
global {
    export let value: number = 1;
}

/// @export.summary
"#,
    );
}

#[test]
fn test_export_omits_static_if_false_global_reexport() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
global {
    @if(false)
    export { Foo } from "./missing.tspp";
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::modules().with_export().with_summaries(),
        r#"
global {
    @if(false)
    export { Foo } from "./missing.tspp";
}

/// @module.summary edges=0
/// @export.summary
"#,
    );
}

#[test]
fn test_export_records_static_if_true_reexport_item() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export { @if(false) Foo, @if(true) Bar } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let Foo = 1;
export let Bar = 2;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::modules().with_export().with_summaries(),
        r#"
export { @if(false) Foo, @if(true) Bar } from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp
/// @export.reexport key=Bar imported=Bar module=dep.tspp

/// @module.summary edges=1
/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_drops_generic_static_if_invocation() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const value = 1;

@if<boolean>(true)
export { value };
"#,
        )
        .build();

    // checking owns the guard diagnostics; export drops the gated root silently
    compiler.assert_dir_exported_diagnostics(
        "main.tspp",
        r#"
"#,
    );
}

/// Reject generic arguments on `@if` export guards.
#[test]
fn test_export_reports_generic_static_if_invocation() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const value = 1;

@if<boolean>(true)
export { value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.tspp",
        DirRows::exports().with_summaries(),
        r#"
const value = 1;

@if<boolean>(true)
export { value };

/// @export.summary
"#,
    );
}
