use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_resolves_builtin_package_export() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { Math } from "tspp:math";
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { Math } from "tspp:math";
/// @module.edge relation=import specifier=tspp:math module=tspp://math/index.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_authored_builtin_modules() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "tspp"
}
"#,
        )
        .module(
            "src/main.tspp",
            r#"
import { absolute } from "tspp:absolute";
import { relative } from "./relative.tspp";
"#,
        )
        .module(
            "src/absolute.tspp",
            r#"
export const absolute = 1;
"#,
        )
        .module(
            "src/relative.tspp",
            r#"
export const relative = 2;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "src/main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { absolute } from "tspp:absolute";
/// @module.edge relation=import specifier=tspp:absolute module=tspp://absolute.tspp

import { relative } from "./relative.tspp";
/// @module.edge relation=import specifier=./relative.tspp module=tspp://relative.tspp

/// @module.summary edges=2
"#,
    );
}

#[test]
fn test_import_reports_builtin_internal_subpath() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { HostError } from "tspp:error/host";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unsupported-module-specifier message="unsupported module specifier 'tspp:error/host'"
/// @diagnostic.label line=2 column=1 span="import { HostError } from \"tspp:error/host\"" line_source="import { HostError } from \"tspp:error/host\";"
"#,
    );
}

#[test]
fn test_import_reports_private_specifier() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r##"
import { value } from "#internal";
"##,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r##"
/// @diagnostic.error id=unsupported-module-specifier message="unsupported module specifier '#internal'"
/// @diagnostic.label line=2 column=1 span="import { value } from \"#internal\"" line_source="import { value } from \"#internal\";"
"##,
    );
}

#[test]
fn test_import_reports_absolute_specifier() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "/dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unsupported-module-specifier message="unsupported module specifier '/dep.tspp'"
/// @diagnostic.label line=2 column=1 span="import { value } from \"/dep.tspp\"" line_source="import { value } from \"/dep.tspp\";"
"#,
    );
}

#[test]
fn test_import_reports_scheme_specifier() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "host:runtime";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unsupported-module-specifier message="unsupported module specifier 'host:runtime'"
/// @diagnostic.label line=2 column=1 span="import { value } from \"host:runtime\"" line_source="import { value } from \"host:runtime\";"
"#,
    );
}

#[test]
fn test_import_reports_local_query_specifier() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { value } from "./dep.tspp?raw";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unsupported-module-specifier message="unsupported module specifier './dep.tspp?raw'"
/// @diagnostic.label line=2 column=1 span="import { value } from \"./dep.tspp?raw\"" line_source="import { value } from \"./dep.tspp?raw\";"
"#,
    );
}

#[test]
fn test_import_reports_conditional_file_specifier() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "conditions": {
        "modes": {
            "preview": {}
        }
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
import { value } from "./user.preview.tspp";
"#,
        )
        .module(
            "user.tspp",
            r#"
export let value = 1;
"#,
        )
        .module(
            "user.preview.tspp",
            r#"
export let preview = true;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=unsupported-module-specifier message="unsupported module specifier './user.preview.tspp'"
/// @diagnostic.label line=2 column=1 span="import { value } from \"./user.preview.tspp\"" line_source="import { value } from \"./user.preview.tspp\";"
"#,
    );
}
