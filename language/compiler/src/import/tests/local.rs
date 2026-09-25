use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_local_import_edge() {
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

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_side_effect_edge() {
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
let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_extensionless_source_path() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { Foo } from "./dep";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Foo } from "./dep";
/// @module.edge relation=import specifier=./dep module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_relative_parent_path() {
    let compiler = TestSession::builder()
        .module(
            "src/main.tspp",
            r#"
import { Foo } from "../dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export type Foo = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "src/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Foo } from "../dep.tspp";
/// @module.edge relation=import specifier=../dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_renders_multi_module_snapshot() {
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

    compiler.assert_dir_imported_many(
        &["main.tspp", "dep.tspp"],
        DirRows::modules().with_summaries(),
        r#"
=== main.tspp ===

import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1

=== dep.tspp ===

export type Foo = string;

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_reports_missing_local_module() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { Missing } from "./missing.tspp";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unresolved-module message="unresolved module './missing.tspp'"
/// @diagnostic.label line=2 column=1 span="import { Missing } from \"./missing.tspp\"" line_source="import { Missing } from \"./missing.tspp\";"
"#,
    );
}

#[test]
fn test_import_reports_ambiguous_extensionless_specifier() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "./dep";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .module(
            "dep.d.tspp",
            r#"
export declare let value: int32;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=ambiguous-module-specifier message="ambiguous module specifier './dep': dep.tspp, dep.d.tspp"
/// @diagnostic.label line=2 column=1 span="import { value } from \"./dep\"" line_source="import { value } from \"./dep\";"
"#,
    );
}

#[test]
fn test_import_reports_cross_package_relative_specifier() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "workspace": {
        "packages": ["packages/*"]
    }
}
"#,
        )
        .data(
            "packages/app/destack.json",
            r#"
{
    "name": "app"
}
"#,
        )
        .data(
            "packages/lib/destack.json",
            r#"
{
    "name": "lib"
}
"#,
        )
        .module(
            "packages/app/main.tspp",
            r#"
import { value } from "../lib/dep.tspp";
"#,
        )
        .module(
            "packages/lib/dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.tspp",
        r#"
/// @diagnostic.error id=cross-package-relative-import message="relative module specifier '../lib/dep.tspp' crosses package boundaries"
/// @diagnostic.label line=2 column=1 span="import { value } from \"../lib/dep.tspp\"" line_source="import { value } from \"../lib/dep.tspp\";"
"#,
    );
}
