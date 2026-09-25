use crate::tests::{DirRows, TestSession};

#[test]
fn test_warn_duplicate_import() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { left } from "./dep.tspp";
import { right } from "././dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export const left = 1;
export const right = 2;
"#,
        )
        .build();

    compiler.assert_dir_resolved_and_diagnostics(
        "main.tspp",
        DirRows::imports(),
        r#"
import { left } from "./dep.tspp";
/// @import.resolved symbol=left declarations=[dep.left] targets=[dep.left]
/// @reference.target source=left kind=bound targets=[dep.left]

import { right } from "././dep.tspp";
/// @import.resolved symbol=right declarations=[dep.right] targets=[dep.right]
/// @reference.target source=right kind=bound targets=[dep.right]
"#,
        r#"
/// @diagnostic.warning id=duplicate-import message="module '././dep.tspp' is imported more than once"
/// @diagnostic.label line=3 column=1 span="import { right } from \"././dep.tspp\"" line_source="import { right } from \"././dep.tspp\";"
/// @diagnostic.related line=2 column=1 span="import { left } from \"./dep.tspp\"" line_source="import { left } from \"./dep.tspp\";" message="first imported here"
"#,
    );
}

#[test]
fn test_resolve_records_named_import_references() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value as local } from "./dep.tspp";

local;
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { value as local } from "./dep.tspp";
/// @import.resolved symbol=local declarations=[dep.value] targets=[dep.value]
/// @reference.target source=value kind=bound targets=[dep.value]

local;
/// @reference.target source=local kind=bound targets=[dep.value]
/// @reference.declaration source=local kind=bound targets=[local]

/// @import.summary symbols=1
/// @reference.summary references=2 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_imported_overload_group() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { parse } from "./dep.tspp";

parse;
"#,
        )
        .module(
            "dep.tspp",
            r#"
export declare function parse(value: int32): int32;
export declare function parse(value: string): string;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { parse } from "./dep.tspp";
/// @import.resolved symbol=parse declarations=[dep.parse#1, dep.parse#2] targets=[dep.parse#1, dep.parse#2]
/// @reference.target source=parse kind=bound targets=[dep.parse#1, dep.parse#2]

parse;
/// @reference.target source=parse kind=bound targets=[dep.parse#1, dep.parse#2]
/// @reference.declaration source=parse kind=bound targets=[parse]

/// @import.summary symbols=1
/// @reference.summary references=2 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_namespace_import_references() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import * as dep from "./dep.tspp";
/// @import.resolved symbol=dep declarations=[dep.tspp] targets=[dep.tspp]
/// @reference.target source=<namespace> kind=namespace module=dep.tspp

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_namespace_import_path() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";

dep.value;
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import * as dep from "./dep.tspp";
/// @import.resolved symbol=dep declarations=[dep.tspp] targets=[dep.tspp]
/// @reference.target source=<namespace> kind=namespace module=dep.tspp

dep.value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.tspp
/// @reference.target source=dep.value kind=bound targets=[dep.value]

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=string.String symbol=String

/// @import.summary symbols=1 language=6
/// @reference.summary references=3 declarations=1
"#,
    );
}

#[test]
fn test_collect_namespace_path_from_call_callee() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";

dep.make().value;
"#,
        )
        .module(
            "dep.tspp",
            r#"
export declare function make(): { value: number };
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import * as dep from "./dep.tspp";
/// @import.resolved symbol=dep declarations=[dep.tspp] targets=[dep.tspp]
/// @reference.target source=<namespace> kind=namespace module=dep.tspp

dep.make().value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.tspp
/// @reference.target source=dep.make kind=bound targets=[dep.make]

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=string.String symbol=String

/// @import.summary symbols=1 language=6
/// @reference.summary references=3 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_nested_namespace_import_path() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";

dep.api.value;
"#,
        )
        .module(
            "dep.tspp",
            r#"
export * as api from "./api.tspp";
"#,
        )
        .module(
            "api.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import * as dep from "./dep.tspp";
/// @import.resolved symbol=dep declarations=[dep.tspp] targets=[dep.tspp]
/// @reference.target source=<namespace> kind=namespace module=dep.tspp

dep.api.value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.tspp
/// @reference.declaration source=dep.api kind=bound targets=[dep.api]
/// @reference.target source=dep.api kind=namespace module=api.tspp
/// @reference.target source=dep.api.value kind=bound targets=[api.value]

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=string.String symbol=String

/// @import.summary symbols=1 language=6
/// @reference.summary references=4 declarations=2
"#,
    );
}

#[test]
fn test_resolve_records_exported_namespace_import_alias_path() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import * as dep from "./dep.tspp";

dep.api.value;
"#,
        )
        .module(
            "dep.tspp",
            r#"
import * as api from "./api.tspp";
export { api };
"#,
        )
        .module(
            "api.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import * as dep from "./dep.tspp";
/// @import.resolved symbol=dep declarations=[dep.tspp] targets=[dep.tspp]
/// @reference.target source=<namespace> kind=namespace module=dep.tspp

dep.api.value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.tspp
/// @reference.declaration source=dep.api kind=bound targets=[dep.api]
/// @reference.target source=dep.api kind=namespace module=api.tspp
/// @reference.target source=dep.api.value kind=bound targets=[api.value]

/// @import.language item=collections.Array symbol=Array
/// @import.language item=collections.FixedArray symbol=FixedArray
/// @import.language item=collections.Slice symbol=Slice
/// @import.language item=math.BigInt symbol=BigInt
/// @import.language item=math.Number symbol=Number
/// @import.language item=string.String symbol=String

/// @import.summary symbols=1 language=6
/// @reference.summary references=4 declarations=2
"#,
    );
}

#[test]
fn test_resolve_records_default_import_target() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import value from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
let value = 1;
export { value as default };
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import value from "./dep.tspp";
/// @import.resolved symbol=value declarations=[dep.default] targets=[dep.value]
/// @reference.declaration source=<default> kind=bound targets=[dep.default]
/// @reference.target source=<default> kind=bound targets=[dep.value]

/// @import.summary symbols=1
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_type_alias_import_target() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { Foo } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { Foo } from "./dep.tspp";
/// @import.resolved symbol=Foo declarations=[dep.Foo] targets=[dep.Foo]
/// @reference.target source=Foo kind=bound targets=[dep.Foo]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_builtin_star_import_target() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { todo } from "tspp:error";
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { todo } from "tspp:error";
/// @import.resolved symbol=todo declarations=[todo] targets=[todo]
/// @reference.target source=todo kind=bound targets=[todo]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_reports_missing_named_import() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { missing } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();
    compiler.assert_dir_resolved_and_diagnostics(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { missing } from "./dep.tspp";
/// @import.missing symbol=missing
/// @reference.target source=missing kind=missing

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
        r#"
/// @diagnostic.error id=missing-export message="missing export 'missing' from './dep.tspp'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="import { missing } from \"./dep.tspp\";"
"#,
    );
}

#[test]
fn test_resolve_records_unresolved_module_import() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { Missing } from "./missing.tspp";
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { Missing } from "./missing.tspp";
/// @import.missing symbol=Missing
/// @reference.target source=Missing kind=missing

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unresolved-module message="unresolved module './missing.tspp'"
/// @diagnostic.label line=2 column=1 span="import { Missing } from \"./missing.tspp\"" line_source="import { Missing } from \"./missing.tspp\";"
"#,
    );
}

#[test]
fn test_resolve_reports_unexported_local_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { hidden } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
let hidden = 1;
export let value = 2;
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=not-exported message="'hidden' exists in './dep.tspp' but is not exported"
/// @diagnostic.label line=2 column=10 span="hidden" line_source="import { hidden } from \"./dep.tspp\";"
/// @diagnostic.help message="export 'hidden' from './dep.tspp'"
"#,
    );
}

#[test]
fn test_resolve_suggests_closest_export_across_case() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { json } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let JSON = 1;
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-export message="missing export 'json' from './dep.tspp'; did you mean 'JSON'?"
/// @diagnostic.label line=2 column=10 span="json" line_source="import { json } from \"./dep.tspp\";"
/// @diagnostic.suggestion message="rename to 'JSON'" applicability=automatic patched="import { JSON } from \"./dep.tspp\";"
"#,
    );
}

#[test]
fn test_resolve_reports_default_import_hidden_by_star_export() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { default as value } from "./mid.tspp";
"#,
        )
        .module(
            "mid.tspp",
            r#"
export * from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
let value = 1;
export { value as default };
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-export message="missing export 'default' from './mid.tspp'"
/// @diagnostic.label line=2 column=10 span="default as value" line_source="import { default as value } from \"./mid.tspp\";"
"#,
    );
}

#[test]
fn test_resolve_reports_ambiguous_star_import() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "./mid.tspp";
"#,
        )
        .module(
            "mid.tspp",
            r#"
export * from "./a.tspp";
export * from "./b.tspp";
"#,
        )
        .module(
            "a.tspp",
            r#"
export let value = 1;
"#,
        )
        .module(
            "b.tspp",
            r#"
export let value = 2;
"#,
        )
        .build();
    compiler.assert_dir_resolved_and_diagnostics(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./mid.tspp";
/// @import.ambiguous symbol=value declarations=[a.value, b.value] targets=[a.value, b.value]
/// @reference.target source=value kind=ambiguous targets=[a.value, b.value]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
        r#"
/// @diagnostic.error id=ambiguous-export message="ambiguous export 'value' from './mid.tspp'"
/// @diagnostic.label line=2 column=10 span="value" line_source="import { value } from \"./mid.tspp\";"
/// @diagnostic.related file="a.tspp" line=2 column=12 span="value" line_source="export let value = 1;" message="one 'value' is declared here"
/// @diagnostic.related file="b.tspp" line=2 column=12 span="value" line_source="export let value = 2;" message="one 'value' is declared here"
/// @diagnostic.help message="import 'value' directly from one origin module"
"#,
    );
}
