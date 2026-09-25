use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_records_namespace_reexport_reference() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export * as api from "./dep.tspp";
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
export * as api from "./dep.tspp";
/// @reference.declaration source=<namespace> kind=bound targets=[api]
/// @reference.target source=<namespace> kind=namespace module=dep.tspp

/// @import.summary
/// @reference.summary references=1 declarations=1
"#,
    );
}

#[test]
fn test_resolve_follows_indirect_reexport_target() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { renamed } from "./mid.tspp";
"#,
        )
        .module(
            "mid.tspp",
            r#"
import { value as imported } from "./dep.tspp";
export { imported as renamed };
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
import { renamed } from "./mid.tspp";
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
            "main.tspp",
            r#"
import { renamed } from "./mid.tspp";
"#,
        )
        .module(
            "mid.tspp",
            r#"
export { default as renamed } from "./dep.tspp";
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
import { renamed } from "./mid.tspp";
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
            "main.tspp",
            r#"
import { value } from "./mid.tspp";
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
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./mid.tspp";
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
            "main.tspp",
            r#"
import { value } from "./mid.tspp";
"#,
        )
        .module(
            "mid.tspp",
            r#"
export * from "./star.tspp";
export { value } from "./explicit.tspp";
"#,
        )
        .module(
            "star.tspp",
            r#"
export let value = 1;
"#,
        )
        .module(
            "explicit.tspp",
            r#"
export let value = 2;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./mid.tspp";
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
            "main.tspp",
            r#"
import { value } from "./a.tspp";
"#,
        )
        .module(
            "a.tspp",
            r#"
export * from "./b.tspp";
"#,
        )
        .module(
            "b.tspp",
            r#"
export * from "./a.tspp";
export * from "./c.tspp";
"#,
        )
        .module(
            "c.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_resolved(
        "main.tspp",
        DirRows::imports().with_summaries(),
        r#"
import { value } from "./a.tspp";
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
            "main.tspp",
            r#"
import { missing } from "./a.tspp";
"#,
        )
        .module(
            "a.tspp",
            r#"
export * from "./b.tspp";
"#,
        )
        .module(
            "b.tspp",
            r#"
export * from "./a.tspp";
"#,
        )
        .build();
    compiler.assert_dir_resolved_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-export message="missing export 'missing' from './a.tspp'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="import { missing } from \"./a.tspp\";"
"#,
    );
}

#[test]
fn test_resolve_reports_missing_reexport_target() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export { missing } from "./dep.tspp";
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
export { missing } from "./dep.tspp";
/// @reference.target source=missing kind=missing

/// @import.summary
/// @reference.summary references=1
"#,
        r#"
/// @diagnostic.error id=missing-export message="missing export 'missing' from './dep.tspp'"
/// @diagnostic.label line=2 column=10 span="missing" line_source="export { missing } from \"./dep.tspp\";"
"#,
    );
}
