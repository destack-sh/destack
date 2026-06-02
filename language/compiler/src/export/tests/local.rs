use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_local_value() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
export let value: number = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
export let value: number = 1;
/// @export.local key=value source=value

/// @export.summary exports=1
/// @export.stats roots=1 expressions=visibility:1,export:1 symbols=scanned:2
"#,
    );
}

#[test]
fn test_export_records_local_alias() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let value = 1;
export { value as renamed };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
let value = 1;
export { value as renamed };
/// @export.local key=renamed source=value

/// @export.summary exports=1
/// @export.stats roots=2 expressions=visibility:2,export:2 symbols=scanned:2
"#,
    );
}

#[test]
fn test_export_uses_latest_local_binding() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let value = 1;
let value = 2;
export { value };
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
let value = 1;
let value = 2;
export { value };
/// @export.local key=value source=value#2

/// @export.summary exports=1
/// @export.stats roots=3 expressions=visibility:3,export:3 symbols=scanned:3
"#,
    );
}

#[test]
fn test_export_records_global_symbols() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
global {
    let process: string;
    const answer: int32 = 42;
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
global {
    let process: string;
    /// @global.local key=process source=process

    const answer: int32 = 42;
    /// @global.local key=answer source=answer

}

/// @export.summary
/// @export.stats roots=1 expressions=visibility:3,export:3 symbols=scanned:3
/// @global.summary keys=2 entries=2
"#,
    );
}

#[test]
fn test_export_records_global_reexports() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
global {
    export { Function, Option as Maybe } from "./types.ds";
}
"#,
        )
        .module(
            "types.ds",
            r#"
export type Function = () => void;
export type Option = string;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
global {
    export { Function, Option as Maybe } from "./types.ds";
    /// @global.indirect key=Function imported=Function module=types.ds
    /// @global.indirect key=Maybe imported=Option module=types.ds

}

/// @export.summary
/// @export.stats roots=1 expressions=visibility:2,export:2 symbols=scanned:1
/// @global.summary keys=2 entries=2
"#,
    );
}

#[test]
fn test_export_records_global_namespace_reexport() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
global {
    export * as api from "./api.ds";
}
"#,
        )
        .module(
            "api.ds",
            r#"
export const value = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
global {
    export * as api from "./api.ds";
    /// @global.indirect key=api imported=<namespace> module=api.ds

}

/// @export.summary
/// @export.stats roots=1 expressions=visibility:2,export:2 symbols=scanned:1
/// @global.summary keys=1 entries=1
"#,
    );
}

#[test]
fn test_export_records_global_local_export() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
const value: int32 = 1;

global {
    export { value as globalValue };
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries().with_export_stats(),
        r#"
const value: int32 = 1;
/// @global.local key=globalValue source=value

global {
    export { value as globalValue };
}

/// @export.summary
/// @export.stats roots=2 expressions=visibility:3,export:3 symbols=scanned:2
/// @global.summary keys=1 entries=1
"#,
    );
}
