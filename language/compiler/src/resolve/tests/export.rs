use crate::tests::snapshot::assert_snapshot;
use crate::tests::{DirRows, TestSession};

#[test]
fn test_resolve_follows_indirect_reexport_target() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { renamed } from "./mid.ds";
"#,
        )
        .module(
            "mid.ds",
            r#"
export { value as renamed } from "./dep.ds";
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
/// @import.symbol symbol=renamed target=dep.value

/// @import.summary symbols=1
"#,
    );
}

#[test]
fn test_resolve_follows_default_reexport_target() {
    let compiler = TestSession::new()
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
/// @import.symbol symbol=renamed target=dep.value

/// @import.summary symbols=1
"#,
    );
}

#[test]
fn test_resolve_follows_star_reexport_target() {
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
/// @import.symbol symbol=value target=dep.value

/// @import.summary symbols=1
"#,
    );
}

#[test]
fn test_resolve_prefers_explicit_export_over_star_export() {
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
/// @import.symbol symbol=value target=explicit.value

/// @import.summary symbols=1
"#,
    );
}

#[test]
fn test_resolve_follows_star_reexport_cycle_when_target_is_found() {
    let compiler = TestSession::new()
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
/// @import.symbol symbol=value target=c.value

/// @import.summary symbols=1
"#,
    );
}

#[test]
fn test_resolve_reports_missing_star_reexport_cycle_target() {
    let compiler = TestSession::new()
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

    compiler
        .provide_dir_resolved("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_resolved_key("main.ds")),
        r#"
/// @diagnostic.error code=ER200 message="missing export 'missing' from './a.ds'"
/// @diagnostic.label line=2 column=10 source="import { missing } from \"./a.ds\";"
"#,
    );
}

#[test]
fn test_resolve_reports_missing_reexport_target() {
    let compiler = TestSession::new()
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

    compiler
        .provide_dir_resolved("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_resolved_key("main.ds")),
        r#"
/// @diagnostic.error code=ER200 message="missing export 'missing' from './dep.ds'"
/// @diagnostic.label line=2 column=10 source="export { missing } from \"./dep.ds\";"
"#,
    );
}
