use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_local_import_edge() {
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

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
import { Foo } from "./dep.ds";
/// @module.edge relation=import specifier=./dep.ds module=dep.ds

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:1,reexport:0
"#,
    );
}

#[test]
fn test_import_records_side_effect_edge() {
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
let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
import "./dep.ds";
/// @module.edge relation=import specifier=./dep.ds module=dep.ds

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:1,reexport:0
"#,
    );
}

#[test]
fn test_import_resolves_extensionless_source_path() {
    let compiler = TestSession::builder()
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
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
import { Foo } from "./dep";
/// @module.edge relation=import specifier=./dep module=dep.ds

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:1,reexport:0
"#,
    );
}

#[test]
fn test_import_resolves_relative_parent_path() {
    let compiler = TestSession::builder()
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
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
import { Foo } from "../dep.ds";
/// @module.edge relation=import specifier=../dep.ds module=dep.ds

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:1,reexport:0
"#,
    );
}

#[test]
fn test_import_renders_multi_module_snapshot() {
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

    compiler.assert_dir_imported_many(
        &["main.ds", "dep.ds"],
        DirRows::modules().with_summaries().with_import_stats(),
        r#"
=== main.ds ===
import { Foo } from "./dep.ds";
/// @module.edge relation=import specifier=./dep.ds module=dep.ds

/// @module.summary edges=1
/// @import.stats roots=1 expressions=1 clauses=import:1,reexport:0

=== dep.ds ===
export type Foo = string;

/// @module.summary edges=0
/// @import.stats roots=1 expressions=1 clauses=import:0,reexport:0
"#,
    );
}

#[test]
fn test_import_reports_missing_local_module() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { Missing } from "./missing.ds";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI200 message="unresolved module './missing.ds'"
/// @diagnostic.label line=2 column=1 source="import { Missing } from \"./missing.ds\";"
"#,
    );
}

#[test]
fn test_import_reports_ambiguous_extensionless_specifier() {
    let compiler = TestSession::builder()
        .module(
            "main.ds",
            r#"
import { value } from "./dep";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .module(
            "dep.ts",
            r#"
export let value = 2;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI205 message="ambiguous module specifier './dep': dep.ds, dep.ts"
/// @diagnostic.label line=2 column=1 source="import { value } from \"./dep\";"
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
            "packages/app/main.ds",
            r#"
import { value } from "../lib/dep.ds";
"#,
        )
        .module(
            "packages/lib/dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "packages/app/main.ds",
        r#"
/// @diagnostic.error code=EI206 message="relative module specifier '../lib/dep.ds' crosses package boundaries"
/// @diagnostic.label line=2 column=1 source="import { value } from \"../lib/dep.ds\";"
"#,
    );
}
