use crate::tests::{DirRows, TestSession, assert_snapshot};

#[test]
fn test_import_records_static_edge() {
    let compiler = TestSession::new()
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

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::dependencies().with_summaries(),
        r#"
import { Foo } from "./dep.ds";
/// @dependency.edge relation=import specifier=./dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_extensionless_source_path() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { Foo } from "./dep";
"#,
        )
        .module(
            "dep.ds",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::dependencies().with_summaries(),
        r#"
import { Foo } from "./dep";
/// @dependency.edge relation=import specifier=./dep module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_side_effect_edge() {
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
let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::dependencies().with_summaries(),
        r#"
import "./dep.ds";
/// @dependency.edge relation=import specifier=./dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_protocol_specifier() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "host:runtime";
"#,
        )
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier 'host:runtime'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"host:runtime\";"
"#,
    );
}

#[test]
fn test_import_renders_multi_module_snapshot() {
    let compiler = TestSession::new()
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

    compiler.assert_dir_imported_many(
        &["main.ds", "dep.ds"],
        DirRows::dependencies().with_summaries(),
        r#"
=== main.ds ===
import { Foo } from "./dep.ds";
/// @dependency.edge relation=import specifier=./dep.ds module=dep.ds

/// @dependency.summary edges=1

=== dep.ds ===
export type Foo = string;

/// @dependency.summary edges=0
"#,
    );
}

#[test]
fn test_import_resolves_relative_parent_path() {
    let compiler = TestSession::new()
        .module(
            "src/main.ds",
            r#"
import { Foo } from "../dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "src/main.ds",
        DirRows::dependencies().with_summaries(),
        r#"
import { Foo } from "../dep.ds";
/// @dependency.edge relation=import specifier=../dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}
