use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_records_profile_global_symbols() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
let local = 1;
"#,
        )
        .module(
            "globals.ds",
            r#"
global {
    let process: string;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
let local = 1;
/// @import.global key=process symbols=[globals.process]

/// @import.summary globals=1
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_symbol_names() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
const value = answer;
"#,
        )
        .module(
            "globals.ds",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
const value = answer;
/// @import.global key=answer symbols=[globals.answer]

/// @import.summary globals=1
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_reexports() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
let local = Function;
"#,
        )
        .module(
            "globals.ds",
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

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
let local = Function;
/// @import.global key=Function symbols=[types.Function]
/// @import.global key=Maybe symbols=[types.Option]

/// @import.summary globals=2
"#,
    );
}

#[test]
fn test_resolve_side_effect_import_does_not_import_globals() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
global {
    let process: string;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import "./dep.ds";

/// @import.summary
"#,
    );
}

#[test]
fn test_resolve_named_import_does_not_import_globals() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;

global {
    let process: string;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./dep.ds";
/// @import.symbol symbol=value target=dep.value

/// @import.summary symbols=1
"#,
    );
}
