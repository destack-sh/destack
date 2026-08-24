use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_records_namespace_reexport_reference() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export * as api from "./dep.ds";
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
        DirRows::imports().with_summaries(),
        r#"
export * as api from "./dep.ds";
/// @reference.declaration source=<namespace> kind=bound targets=[api]
/// @reference.target source=<namespace> kind=namespace module=dep.ds

/// @import.summary
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_follows_indirect_reexport_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { renamed } from "./mid.ds";
"#,
        )
        .module(
            "mid.ds",
            r#"
import { value as imported } from "./dep.ds";
export { imported as renamed };
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
        DirRows::imports().with_summaries(),
        r#"
import { renamed } from "./mid.ds";
/// @import.resolved symbol=renamed declarations=[mid.renamed] targets=[dep.value]
/// @reference.target source=renamed kind=bound targets=[dep.value]
/// @reference.declaration source=renamed kind=bound targets=[mid.renamed]

/// @import.summary symbols=1
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_follows_default_reexport_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { renamed } from "./mid.ds";
"#,
        )
        .module(
            "mid.ds",
            r#"
export { default as renamed } from "./dep.ds";
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
        DirRows::imports().with_summaries(),
        r#"
import { renamed } from "./mid.ds";
/// @import.resolved symbol=renamed declarations=[mid.renamed] targets=[dep.value]
/// @reference.target source=renamed kind=bound targets=[dep.value]
/// @reference.declaration source=renamed kind=bound targets=[mid.renamed]

/// @import.summary symbols=1
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_follows_star_reexport_target() {
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
export * from "./dep.ds";
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
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./mid.ds";
/// @import.resolved symbol=value declarations=[dep.value] targets=[dep.value]
/// @reference.target source=value kind=bound targets=[dep.value]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_prefers_explicit_export_over_star_export() {
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
export * from "./star.ds";
export { value } from "./explicit.ds";
"#,
        )
        .module(
            "star.ds",
            r#"
export let value = 1;
"#,
        )
        .module(
            "explicit.ds",
            r#"
export let value = 2;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./mid.ds";
/// @import.resolved symbol=value declarations=[explicit.value] targets=[explicit.value]
/// @reference.target source=value kind=bound targets=[explicit.value]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_follows_star_reexport_cycle_when_target_is_found() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value } from "./a.ds";
"#,
        )
        .module(
            "a.ds",
            r#"
export * from "./b.ds";
"#,
        )
        .module(
            "b.ds",
            r#"
export * from "./a.ds";
export * from "./c.ds";
"#,
        )
        .module(
            "c.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.ds",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./a.ds";
/// @import.resolved symbol=value declarations=[c.value] targets=[c.value]
/// @reference.target source=value kind=bound targets=[c.value]

/// @import.summary symbols=1
/// @reference.summary references=1
"#,
    );
}

#[test]
fn test_resolve_reports_missing_star_reexport_cycle_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { missing } from "./a.ds";
"#,
        )
        .module(
            "a.ds",
            r#"
export * from "./b.ds";
"#,
        )
        .module(
            "b.ds",
            r#"
export * from "./a.ds";
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error id=missing-export message="missing export 'missing' from './a.ds'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="import { missing } from \"./a.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_missing_reexport_target() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
export { missing } from "./dep.ds";
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
export { missing } from "./dep.ds";
/// @reference.target source=missing kind=missing

/// @import.summary
/// @reference.summary references=1
"#,
        r#"
/// @diagnostic.error id=missing-export message="missing export 'missing' from './dep.ds'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="export { missing } from \"./dep.ds\";"
"#,
    );
}
