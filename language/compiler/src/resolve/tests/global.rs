use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_ignores_unreferenced_profile_global_symbols() {
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let local = 1;

/// @import.summary
/// @resolve.stats roots=1 expressions=2 types=0
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const value = answer;

/// @import.global key=answer symbols=[globals.answer]

/// @import.summary globals=1
/// @resolve.stats roots=1 expressions=2 types=0 imports=dep:1,symbol:0 globals=required:1,module:1,loaded:1
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let local = Function;

/// @import.global key=Function symbols=[types.Function]

/// @import.summary globals=1
/// @resolve.stats roots=1 expressions=2 types=0 imports=dep:1,symbol:0 globals=required:1,module:1,loaded:1 exports=miss:1,hit:0,cycle:0
"#,
    );
}

#[test]
fn test_resolve_records_referenced_type_profile_globals() {
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
let promise: Promise<string>;
"#,
        )
        .module(
            "globals.ds",
            r#"
global {
    export { Promise } from "./async.ds";
}
"#,
        )
        .module(
            "async.ds",
            r#"
export class Promise<T> {}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let promise: Promise<string>;

/// @import.global key=Promise symbols=[async.Promise]

/// @import.summary globals=1
/// @resolve.stats roots=1 expressions=1 types=2 imports=dep:1,symbol:0 globals=required:1,module:1,loaded:1 exports=miss:1,hit:0,cycle:0
"#,
    );
}

#[test]
fn test_resolve_records_referenced_builtin_language_symbol_names() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
let promise: Promise<string>;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let promise: Promise<string>;

/// @import.global key=Promise symbols=[async.promise.Promise]

/// @import.summary globals=1
/// @resolve.stats roots=1 expressions=1 types=2 imports=dep:1,symbol:0 globals=required:1,module:0,loaded:1
"#,
    );
}

#[test]
fn test_resolve_records_async_function_language_item() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
const load = async () => 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const load = async () => 1;

/// @import.language item=async.Promise symbol=async.promise.Promise

/// @import.summary language=1
/// @resolve.stats roots=1 expressions=3 types=0 imports=dep:1,symbol:0 language=required:1,loaded:1
"#,
    );
}

#[test]
fn test_resolve_keeps_async_language_item_separate_from_local_promise() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
class Promise {}
const load = async () => 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
class Promise {}
const load = async () => 1;

/// @import.language item=async.Promise symbol=async.promise.Promise

/// @import.summary language=1
/// @resolve.stats roots=2 expressions=4 types=0 imports=dep:1,symbol:0 language=required:1,loaded:1
"#,
    );
}

#[test]
fn test_resolve_records_import_meta_language_item() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
const runtime = import.meta.runtime;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const runtime = import.meta.runtime;

/// @import.language item=module.ImportMeta symbol=module.meta.ImportMeta

/// @import.summary language=1
/// @resolve.stats roots=1 expressions=3 types=0 imports=dep:1,symbol:0 language=required:1,loaded:1
"#,
    );
}

#[test]
fn test_resolve_records_operator_language_item() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
const left = 1;
const right = 2;
const value = left + right;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const left = 1;
const right = 2;
const value = left + right;

/// @import.language item=ops.Add symbol=ops.plus.Add

/// @import.summary language=1
/// @resolve.stats roots=3 expressions=8 types=0 imports=dep:1,symbol:0 language=required:1,loaded:1
"#,
    );
}

#[test]
fn test_resolve_does_not_import_shadowed_profile_globals() {
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
const answer = 1;
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const answer = 1;
const value = answer;

/// @import.summary
/// @resolve.stats roots=2 expressions=4 types=0
"#,
    );
}

#[test]
fn test_resolve_does_not_import_nested_shadowed_profile_globals() {
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
function read() {
    const answer = 1;
    return answer;
}
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
function read() {
    const answer = 1;
    return answer;
}

/// @import.summary
/// @resolve.stats roots=1 expressions=6 types=0
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import "./dep.ds";

/// @import.summary
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 imports=dep:1,symbol:0
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
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import { value } from "./dep.ds";
/// @import.symbol symbol=value target=dep.value

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 imports=dep:1,symbol:1 exports=miss:1,hit:0,cycle:0
"#,
    );
}
