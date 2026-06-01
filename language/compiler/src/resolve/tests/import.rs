use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_records_named_import_target() {
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
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
"#,
    );
}

#[test]
fn test_resolve_records_namespace_import_target() {
    let compiler = TestSession::new()
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
/// @import.namespace symbol=dep module=dep.ds

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0
"#,
    );
}

#[test]
fn test_resolve_records_default_import_target() {
    let compiler = TestSession::new()
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
/// @import.symbol symbol=value target=dep.value

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
"#,
    );
}

#[test]
fn test_resolve_records_type_only_import_target() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import type { Foo } from "./dep.ds";
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
import type { Foo } from "./dep.ds";
/// @import.symbol symbol=Foo target=dep.Foo

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:1,hit:0,cycle:0
"#,
    );
}

#[test]
fn test_resolve_records_builtin_star_import_target() {
    let compiler = TestSession::new()
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
/// @import.symbol symbol=todo target=error.panic.todo

/// @import.summary symbols=1
/// @resolve.stats roots=1 expressions=1 types=0 clauses=import:1,reexport:0 exports=miss:10,hit:0,cycle:0
"#,
    );
}

#[test]
fn test_resolve_reports_missing_named_import() {
    let compiler = TestSession::new()
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
    compiler.assert_dir_resolved_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=ER200 message="missing export 'missing' from './dep.ds'"
/// @diagnostic.label line=2 column=10 source="import { missing } from \"./dep.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_default_import_hidden_by_star_export() {
    let compiler = TestSession::new()
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
/// @diagnostic.error code=ER200 message="missing export 'default' from './mid.ds'"
/// @diagnostic.label line=2 column=10 source="import { default as value } from \"./mid.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_ambiguous_star_import() {
    let compiler = TestSession::new()
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
    compiler.assert_dir_resolved_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=ER201 message="ambiguous export 'value' from './mid.ds'"
/// @diagnostic.label line=2 column=10 source="import { value } from \"./mid.ds\";"
"#,
    );
}
