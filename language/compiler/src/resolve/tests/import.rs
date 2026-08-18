use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_records_named_import_references() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value as local } from "./dep.ds";

local;
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import { value as local } from "./dep.ds";
/// @import.resolved symbol=local declarations=[dep.value] targets=[dep.value]
/// @reference.target source=value kind=bound targets=[dep.value]

local;
/// @reference.target source=local kind=bound targets=[dep.value]
/// @reference.declaration source=local kind=bound targets=[local]

/// @import.summary symbols=1
/// @resolve.stats roots=2 expressions=2 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=2 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_imported_overload_group() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { parse } from "./dep.ds";

parse;
"#,
        )
        .module(
            "dep.ds",
            r#"
export declare function parse(value: int32): int32;
export declare function parse(value: string): string;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { parse } from "./dep.ds";
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
            "main.ds",
            r#"
import * as dep from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import * as dep from "./dep.ds";
/// @import.resolved symbol=dep declarations=[dep.ds] targets=[dep.ds]
/// @reference.target source=<namespace> kind=namespace module=dep.ds

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_namespace_import_path() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as dep from "./dep.ds";

dep.value;
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import * as dep from "./dep.ds";
/// @import.resolved symbol=dep declarations=[dep.ds] targets=[dep.ds]
/// @reference.target source=<namespace> kind=namespace module=dep.ds

dep.value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.ds
/// @reference.target source=dep.value kind=bound targets=[dep.value]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary symbols=1 language=6
/// @resolve.stats roots=2 expressions=3 types=0 clauses=import:1,reexport:0 language=uses:6 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=3 declarations=1
"#,
    );
}

#[test]
fn test_collect_namespace_path_from_call_callee() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as dep from "./dep.ds";

dep.make().value;
"#,
        )
        .module(
            "dep.ds",
            r#"
export declare function make(): { value: number };
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import * as dep from "./dep.ds";
/// @import.resolved symbol=dep declarations=[dep.ds] targets=[dep.ds]
/// @reference.target source=<namespace> kind=namespace module=dep.ds

dep.make().value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.ds
/// @reference.target source=dep.make kind=bound targets=[dep.make]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary symbols=1 language=6
/// @resolve.stats roots=2 expressions=5 types=0 clauses=import:1,reexport:0 language=uses:6 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=3 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_nested_namespace_import_path() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as dep from "./dep.ds";

dep.api.value;
"#,
        )
        .module(
            "dep.ds",
            r#"
export * as api from "./api.ds";
"#,
        )
        .module(
            "api.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import * as dep from "./dep.ds";
/// @import.resolved symbol=dep declarations=[dep.ds] targets=[dep.ds]
/// @reference.target source=<namespace> kind=namespace module=dep.ds

dep.api.value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.ds
/// @reference.declaration source=dep.api kind=bound targets=[dep.api]
/// @reference.target source=dep.api kind=namespace module=api.ds
/// @reference.target source=dep.api.value kind=bound targets=[api.value]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary symbols=1 language=6
/// @resolve.stats roots=2 expressions=4 types=0 clauses=import:1,reexport:0 language=uses:6 exports=miss:2,hit:0,cycle:0
/// @reference.summary references=4 declarations=2
"#,
    );
}

#[test]
fn test_resolve_records_exported_namespace_import_alias_path() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import * as dep from "./dep.ds";

dep.api.value;
"#,
        )
        .module(
            "dep.ds",
            r#"
import * as api from "./api.ds";
export { api };
"#,
        )
        .module(
            "api.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import * as dep from "./dep.ds";
/// @import.resolved symbol=dep declarations=[dep.ds] targets=[dep.ds]
/// @reference.target source=<namespace> kind=namespace module=dep.ds

dep.api.value;
/// @reference.declaration source=dep kind=bound targets=[dep]
/// @reference.target source=dep kind=namespace module=dep.ds
/// @reference.declaration source=dep.api kind=bound targets=[dep.api]
/// @reference.target source=dep.api kind=namespace module=api.ds
/// @reference.target source=dep.api.value kind=bound targets=[api.value]

/// @import.language item=collections.Array symbol=collections.array.Array
/// @import.language item=collections.FixedArray symbol=collections.fixed-array.FixedArray
/// @import.language item=collections.Slice symbol=collections.slice.Slice
/// @import.language item=math.BigInt symbol=math.bigint.BigInt
/// @import.language item=math.Number symbol=math.number.Number
/// @import.language item=string.String symbol=string.string.String

/// @import.summary symbols=1 language=6
/// @resolve.stats roots=2 expressions=4 types=0 clauses=import:1,reexport:0 language=uses:6 exports=miss:2,hit:0,cycle:0
/// @reference.summary references=4 declarations=2
"#,
    );
}

#[test]
fn test_resolve_records_default_import_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import value from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
let value = 1;
export { value as default };
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import value from "./dep.ds";
/// @import.resolved symbol=value declarations=[dep.default] targets=[dep.value]
/// @reference.declaration source=<default> kind=bound targets=[dep.default]
/// @reference.target source=<default> kind=bound targets=[dep.value]

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_records_type_alias_import_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { Foo } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import { Foo } from "./dep.ds";
/// @import.resolved symbol=Foo declarations=[dep.Foo] targets=[dep.Foo]
/// @reference.target source=Foo kind=bound targets=[dep.Foo]

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_records_builtin_star_import_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { todo } from "destack:error";
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries().with_resolve_stats(),
        r#"
import { todo } from "destack:error";
/// @import.resolved symbol=todo declarations=[error.panic.todo] targets=[error.panic.todo]
/// @reference.target source=todo kind=bound targets=[error.panic.todo]

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:9,hit:0,cycle:0
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_reports_missing_named_import() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { missing } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();
    compiler.assert_dir_resolved_and_diagnostics(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { missing } from "./dep.ds";
/// @import.missing symbol=missing
/// @reference.target source=missing kind=missing

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
        r#"
/// @diagnostic.error id=missing-export message="missing export 'missing' from './dep.ds'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="import { missing } from \"./dep.ds\";"
"#,
    );
}

#[test]
fn test_resolve_records_unresolved_module_import() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { Missing } from "./missing.ds";
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { Missing } from "./missing.ds";
/// @import.missing symbol=Missing
/// @reference.target source=Missing kind=missing

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=unresolved-module message="unresolved module './missing.ds'"
/// @diagnostic.label line=2 column=1 span="import { Missing } from \"./missing.ds\"" line_source="import { Missing } from \"./missing.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_unexported_local_binding() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { hidden } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
let hidden = 1;
export let value = 2;
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=not-exported message="'hidden' exists in './dep.ds' but is not exported"
/// @diagnostic.label line=2 column=10 span="hidden" line_source="import { hidden } from \"./dep.ds\";"
/// @diagnostic.help message="export 'hidden' from './dep.ds'"
"#,
    );
}

#[test]
fn test_resolve_suggests_closest_export_across_case() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { json } from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let JSON = 1;
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=missing-export message="missing export 'json' from './dep.ds'; did you mean 'JSON'?"
/// @diagnostic.label line=2 column=10 span="json" line_source="import { json } from \"./dep.ds\";"
/// @diagnostic.suggestion message="rename to 'JSON'" applicability=automatic patched="import { JSON } from \"./dep.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_default_import_hidden_by_star_export() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { default as value } from "./mid.ds";
"#,
        )
        .module(
            "mid.ds",
            r#"
export * from "./dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
let value = 1;
export { value as default };
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=missing-export message="missing export 'default' from './mid.ds'"
/// @diagnostic.label line=2 column=10 span="default as value" line_source="import { default as value } from \"./mid.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_ambiguous_star_import() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value } from "./mid.ds";
"#,
        )
        .module(
            "mid.ds",
            r#"
export * from "./a.ds";
export * from "./b.ds";
"#,
        )
        .module(
            "a.ds",
            r#"
export let value = 1;
"#,
        )
        .module(
            "b.ds",
            r#"
export let value = 2;
"#,
        )
        .build();
    compiler.assert_dir_resolved_and_diagnostics(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./mid.ds";
/// @import.ambiguous symbol=value declarations=[a.value, b.value] targets=[a.value, b.value]
/// @reference.target source=value kind=ambiguous targets=[a.value, b.value]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
        r#"
/// @diagnostic.error id=ambiguous-export message="ambiguous export 'value' from './mid.ds'"
/// @diagnostic.label line=2 column=10 span="value" line_source="import { value } from \"./mid.ds\";"
/// @diagnostic.related file="a.ds" message="one 'value' comes from this module"
/// @diagnostic.related file="b.ds" message="one 'value' comes from this module"
/// @diagnostic.help message="import 'value' directly from one origin module"
"#,
    );
}
