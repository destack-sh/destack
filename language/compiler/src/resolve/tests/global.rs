use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_ignores_unreferenced_profile_global_symbols() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
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
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
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
/// @reference.target source=answer kind=bound targets=[globals.answer]

/// @import.global key=answer declarations=[globals.answer] targets=[globals.answer]

/// @import.summary globals=1
/// @resolve.stats roots=1 expressions=2 types=0 globals=required:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_references() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
let value = Function;
let projected: Function.Member;
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
let value = Function;
/// @reference.target source=Function kind=ambiguous targets=[types.function.Function, types.Function]

let projected: Function.Member;
/// @reference.target source=Function.Member kind=ambiguous targets=[types.function.Function, types.Function]

/// @import.global key=Function declarations=[types.function.Function, types.Function] targets=[types.function.Function, types.Function]

/// @import.summary globals=1
/// @resolve.stats roots=2 expressions=3 types=1 globals=required:1
/// @reference.summary references=2
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_namespace_reexports() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
let local = api;
"#,
        )
        .module(
            "globals.ds",
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

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let local = api;
/// @reference.declaration source=api kind=bound targets=[globals.api]
/// @reference.target source=api kind=namespace module=api.ds

/// @import.global key=api declarations=[globals.api] targets=[api.ds]

/// @import.summary globals=1
/// @resolve.stats roots=1 expressions=2 types=0 globals=required:1
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_namespace_paths() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
let local = api.value;
"#,
        )
        .module(
            "globals.ds",
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

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
let local = api.value;
/// @reference.declaration source=api kind=bound targets=[globals.api]
/// @reference.target source=api kind=namespace module=api.ds
/// @reference.target source=api.value kind=bound targets=[api.value]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String
/// @import.global key=api declarations=[globals.api] targets=[api.ds]

/// @import.summary globals=1 language=6
/// @resolve.stats roots=1 expressions=3 types=0 globals=required:1 language=uses:6 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=2 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_referenced_type_profile_globals() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
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
/// @reference.target source=Promise kind=ambiguous targets=[async.promise.Promise, async.Promise]

/// @import.language item=string.String symbol=string.string.String
/// @import.global key=Promise declarations=[async.promise.Promise, async.Promise] targets=[async.promise.Promise, async.Promise]

/// @import.summary globals=1 language=1
/// @resolve.stats roots=1 expressions=1 types=2 globals=required:1 language=uses:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_referenced_builtin_language_symbol_names() {
    let compiler = TestSession::builder()
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
/// @reference.target source=Promise kind=bound targets=[async.promise.Promise]

/// @import.language item=string.String symbol=string.string.String
/// @import.global key=Promise declarations=[async.promise.Promise] targets=[async.promise.Promise]

/// @import.summary globals=1 language=1
/// @resolve.stats roots=1 expressions=1 types=2 globals=required:1 language=uses:1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_async_function_language_item() {
    let compiler = TestSession::builder()
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
/// @resolve.stats roots=1 expressions=3 types=0 language=uses:1
"#,
    );
}

#[test]
fn test_resolve_keeps_async_language_item_separate_from_local_promise() {
    let compiler = TestSession::builder()
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
/// @resolve.stats roots=2 expressions=4 types=0 language=uses:1
"#,
    );
}

#[test]
fn test_resolve_records_import_meta_language_item() {
    let compiler = TestSession::builder()
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

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=module.ImportMeta symbol=module.meta.ImportMeta
/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=7
/// @resolve.stats roots=1 expressions=3 types=0 language=uses:7
"#,
    );
}

#[test]
fn test_resolve_records_operator_language_item() {
    let compiler = TestSession::builder()
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
/// @reference.target source=left kind=bound targets=[left]
/// @reference.target source=right kind=bound targets=[right]

/// @import.language item=ops.Add symbol=ops.plus.Add

/// @import.summary language=1
/// @resolve.stats roots=3 expressions=8 types=0 language=uses:1
/// @reference.summary references=2
"#,
    );
}

#[test]
fn test_resolve_records_strict_equality_language_item() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
declare const left: int32;
declare const right: int32;
const same = left === right;
const different = left !== right;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
declare const left: int32;
declare const right: int32;
const same = left === right;
/// @reference.target source=left kind=bound targets=[left]
/// @reference.target source=right kind=bound targets=[right]

const different = left !== right;
/// @reference.target source=left kind=bound targets=[left]
/// @reference.target source=right kind=bound targets=[right]

/// @import.language item=ops.StrictEqual symbol=ops.equality.StrictEqual

/// @import.summary language=1
/// @resolve.stats roots=4 expressions=10 types=2 language=uses:1
/// @reference.summary references=4
"#,
    );
}

#[test]
fn test_resolve_does_not_import_shadowed_profile_globals() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
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
/// @reference.target source=answer kind=bound targets=[answer]

/// @import.summary
/// @resolve.stats roots=2 expressions=4 types=0
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_does_not_import_nested_shadowed_profile_globals() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
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
    /// @reference.target source=answer kind=bound targets=[read.answer]

}

/// @import.summary
/// @resolve.stats roots=1 expressions=6 types=0
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_does_not_resolve_shadowed_profile_global_namespace_paths() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "globals": ["globals.ds"]
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
const api = {};
const value = api.value;
"#,
        )
        .module(
            "globals.ds",
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

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
const api = {};
const value = api.value;
/// @reference.target source=api kind=bound targets=[api]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary language=6
/// @resolve.stats roots=2 expressions=5 types=0 language=uses:6
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_side_effect_import_does_not_import_globals() {
    let compiler = TestSession::builder()
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
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0
"#,
    );
}

#[test]
fn test_resolve_named_import_does_not_import_globals() {
    let compiler = TestSession::builder()
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
/// @import.resolved symbol=value declarations=[dep.value] targets=[dep.value]
/// @reference.target source=value kind=bound targets=[dep.value]

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=1
"#,
    );
}
