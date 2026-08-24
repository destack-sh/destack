use crate::tests::{DirRows, TestSession};

#[test]
fn test_export_records_local_value() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export let value: number = 1;
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries(),
        r#"
export let value: number = 1;
/// @export.local key=value symbols=[value]

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_local_overload_group() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export function parse(value: int32): int32 {
    return value;
}

export function parse(value: string): string {
    return value;
}
"#,
        )
        .build();

    compiler.assert_dir_exported(
        "main.ds",
        DirRows::exports().with_summaries(),
        r#"
export function parse(value: int32): int32 {
/// @export.local key=parse symbols=[parse#1, parse#2]

    return value;
}

export function parse(value: string): string {
    return value;
}

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_local_alias() {
    let compiler = TestSession::builder()
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
        DirRows::exports().with_summaries(),
        r#"
let value = 1;
export { value as renamed };
/// @export.local key=renamed symbols=[value] declaration=renamed

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_local_namespace_alias() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as api from "./api.ds";
export { api };
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
        DirRows::modules().with_export().with_summaries(),
        r#"
import * as api from "./api.ds";
/// @module.edge relation=import specifier=./api.ds module=api.ds

export { api };
/// @export.import key=api imported=<namespace> local=api declaration=api module=api.ds

/// @module.summary edges=1
/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_uses_latest_local_binding() {
    let compiler = TestSession::builder()
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
        DirRows::exports().with_summaries(),
        r#"
let value = 1;
let value = 2;
export { value };
/// @export.local key=value symbols=[value#2]

/// @export.summary exports=1
"#,
    );
}

#[test]
fn test_export_records_global_symbols() {
    let compiler = TestSession::builder()
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
        DirRows::exports().with_summaries(),
        r#"
global {
    let process: string;
    /// @global.local key=process symbols=[process]

    const answer: int32 = 42;
    /// @global.local key=answer symbols=[answer]

}

/// @export.summary
/// @global.summary keys=2 entries=2
"#,
    );
}

#[test]
fn test_export_records_global_reexports() {
    let compiler = TestSession::builder()
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
        DirRows::exports().with_summaries(),
        r#"
global {
    export { Function, Option as Maybe } from "./types.ds";
    /// @global.reexport key=Function imported=Function module=types.ds
    /// @global.reexport key=Maybe imported=Option declaration=Maybe module=types.ds

}

/// @export.summary
/// @global.summary keys=2 entries=2
"#,
    );
}

#[test]
fn test_export_records_global_namespace_reexport() {
    let compiler = TestSession::builder()
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
        DirRows::exports().with_summaries(),
        r#"
global {
    export * as api from "./api.ds";
    /// @global.reexport key=api imported=<namespace> declaration=api module=api.ds

}

/// @export.summary
/// @global.summary keys=1 entries=1
"#,
    );
}

#[test]
fn test_export_records_global_local_export() {
    let compiler = TestSession::builder()
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
        DirRows::exports().with_summaries(),
        r#"
const value: int32 = 1;

global {
    export { value as globalValue };
    /// @global.local key=globalValue symbols=[value] declaration=globalValue

}

/// @export.summary
/// @global.summary keys=1 entries=1
"#,
    );
}

#[test]
fn test_export_records_global_local_namespace_alias() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as api from "./api.ds";

global {
    export { api };
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
        DirRows::modules().with_export().with_summaries(),
        r#"
import * as api from "./api.ds";
/// @module.edge relation=import specifier=./api.ds module=api.ds

global {
    export { api };
    /// @global.import key=api imported=<namespace> local=api declaration=api module=api.ds

}

/// @module.summary edges=1
/// @export.summary
/// @global.summary keys=1 entries=1
"#,
    );
}
