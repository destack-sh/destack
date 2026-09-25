use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_ignores_unreferenced_profile_global_symbols() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
let local = 1;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    let process: string;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
let local = 1;

/// @import.summary
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_symbol_names() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
const value = answer;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
const value = answer;
/// @reference.target source=answer kind=bound targets=[globals.answer]

/// @import.global key=answer declarations=[globals.answer] targets=[globals.answer]

/// @import.summary globals=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_references() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
let value = Function;
let projected: Function.Member;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    export { Function, Option as Maybe } from "./types.tspp";
}
"#,
        )
        .module(
            "types.tspp",
            r#"
export type Function = () => void;
export type Option = string;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
let value = Function;
/// @reference.target source=Function kind=ambiguous targets=[types.Function, Function]

let projected: Function.Member;
/// @reference.target source=Function.Member kind=ambiguous targets=[types.Function, Function]
/// @reference.target source=Function.Member segment=0 kind=ambiguous targets=[types.Function, Function]

/// @import.global key=Function declarations=[types.Function, Function] targets=[types.Function, Function]

/// @import.summary globals=1
/// @reference.summary references=3
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_namespace_reexports() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
let local = api;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    export * as api from "./api.tspp";
}
"#,
        )
        .module(
            "api.tspp",
            r#"
export const value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
let local = api;
/// @reference.declaration source=api kind=bound targets=[globals.api]
/// @reference.target source=api kind=namespace module=api.tspp

/// @import.global key=api declarations=[globals.api] targets=[api.tspp]

/// @import.summary globals=1
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_profile_global_namespace_paths() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
let local = api.value;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    export * as api from "./api.tspp";
}
"#,
        )
        .module(
            "api.tspp",
            r#"
export const value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
let local = api.value;
/// @reference.declaration source=api kind=bound targets=[globals.api]
/// @reference.target source=api kind=namespace module=api.tspp
/// @reference.target source=api.value kind=bound targets=[api.value]

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=string.String symbol=String
/// @import.global key=api declarations=[globals.api] targets=[api.tspp]

/// @import.summary globals=1 language=6
/// @reference.summary references=2 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_referenced_type_profile_globals() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
let promise: Promise<string>;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    export { Promise } from "./async.tspp";
}
"#,
        )
        .module(
            "async.tspp",
            r#"
export class Promise<T> {}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
let promise: Promise<string>;
/// @reference.target source=Promise kind=ambiguous targets=[async.Promise, Promise]

/// @import.language item=string.String symbol=String
/// @import.global key=Promise declarations=[async.Promise, Promise] targets=[async.Promise, Promise]

/// @import.summary globals=1 language=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_referenced_prelude_symbols() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
let promise: Promise<string>;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
let promise: Promise<string>;
/// @reference.target source=Promise kind=bound targets=[Promise]

/// @import.language item=string.String symbol=String
/// @import.global key=Promise declarations=[Promise] targets=[Promise]

/// @import.summary globals=1 language=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_async_function_language_item() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const load = async () => 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
const load = async () => 1;

/// @import.language item=async.Promise symbol=Promise

/// @import.summary language=1
"#,
    );
}

#[test]
fn test_resolve_keeps_async_language_item_separate_from_local_promise() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
class Promise {}
const load = async () => 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
class Promise {}
const load = async () => 1;

/// @import.language item=async.Promise symbol=Promise

/// @import.summary language=1
"#,
    );
}

#[test]
fn test_resolve_records_import_meta_language_item() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const runtime = import.meta.runtime;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
const runtime = import.meta.runtime;

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=module.ImportMeta symbol=ImportMeta
/// @import.language item=string.String symbol=String

/// @import.summary language=7
"#,
    );
}

#[test]
fn test_resolve_records_operator_language_item() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const left = 1;
const right = 2;
const value = left + right;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
const left = 1;
const right = 2;
const value = left + right;
/// @reference.target source=left kind=bound targets=[left]
/// @reference.target source=right kind=bound targets=[right]

/// @import.language item=ops.Add symbol=Add

/// @import.summary language=1
/// @reference.summary references=2
"#,
    );
}

#[test]
fn test_resolve_records_strict_equality_language_item() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
declare const left: int32;
declare const right: int32;
const same = left === right;
const different = left !== right;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
declare const left: int32;
declare const right: int32;
const same = left === right;
/// @reference.target source=left kind=bound targets=[left]
/// @reference.target source=right kind=bound targets=[right]

const different = left !== right;
/// @reference.target source=left kind=bound targets=[left]
/// @reference.target source=right kind=bound targets=[right]

/// @import.language item=ops.StrictEqual symbol=StrictEqual

/// @import.summary language=1
/// @reference.summary references=4
"#,
    );
}

#[test]
fn test_resolve_does_not_import_shadowed_profile_globals() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
const answer = 1;
const value = answer;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
const answer = 1;
const value = answer;
/// @reference.target source=answer kind=bound targets=[answer]

/// @import.summary
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_does_not_import_nested_shadowed_profile_globals() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
function read() {
    const answer = 1;
    return answer;
}
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    const answer: int32 = 42;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
function read() {
    const answer = 1;
    return answer;
    /// @reference.target source=answer kind=bound targets=[read.answer]

}

/// @import.summary
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_does_not_resolve_shadowed_profile_global_namespace_paths() {
    let compiler = TestSession::builder()
        .data(
            "package.json",
            r#"
{
    "packageManager": "tspp@2026.9.0",
    "name": "test",
    "compiler": {
        "globals": ["globals.tspp"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
const api = {};
const value = api.value;
"#,
        )
        .module(
            "globals.tspp",
            r#"
global {
    export * as api from "./api.tspp";
}
"#,
        )
        .module(
            "api.tspp",
            r#"
export const value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
const api = {};
const value = api.value;
/// @reference.target source=api kind=bound targets=[api]

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=string.String symbol=String

/// @import.summary language=6
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_side_effect_import_does_not_import_globals() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
global {
    let process: string;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import "./dep.tspp";

/// @import.summary
"#,
    );
}

#[test]
fn test_resolve_named_import_does_not_import_globals() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;

global {
    let process: string;
}
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./dep.tspp";
/// @import.resolved symbol=value declarations=[dep.value] targets=[dep.value]
/// @reference.target source=value kind=bound targets=[dep.value]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}
