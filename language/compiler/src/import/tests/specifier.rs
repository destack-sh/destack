use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_resolves_builtin_package_export() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { Math } from "destack:math";
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.ds",
        DirRows::modules().with_summaries(),
        r#"
import { Math } from "destack:math";
/// @module.edge relation=import specifier=destack:math module=destack://math

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_builtin_internal_subpath() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { HostError } from "destack:error/host";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier 'destack:error/host'"
/// @diagnostic.label line=2 column=1 source="import { HostError } from \"destack:error/host\";"
"#,
    );
}

#[test]
fn test_import_reports_private_specifier() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r##"
import { value } from "#internal";
"##,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r##"
/// @diagnostic.error code=EI204 message="unsupported module specifier '#internal'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"#internal\";"
"##,
    );
}

#[test]
fn test_import_reports_absolute_specifier() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "/dep.ds";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier '/dep.ds'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"/dep.ds\";"
"#,
    );
}

#[test]
fn test_import_reports_scheme_specifier() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "host:runtime";
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier 'host:runtime'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"host:runtime\";"
"#,
    );
}

#[test]
fn test_import_reports_local_query_specifier() {
    let compiler = TestSession::new()
        .module(
            "main.ds",
            r#"
import { value } from "./dep.ds?raw";
"#,
        )
        .module(
            "dep.ds",
            r#"
export let value = 1;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier './dep.ds?raw'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"./dep.ds?raw\";"
"#,
    );
}

#[test]
fn test_import_reports_conditional_file_specifier() {
    let compiler = TestSession::new()
        .data(
            "destack.json",
            r#"
{
    "conditions": {
        "modes": {
            "preview": {}
        }
    }
}
"#,
        )
        .module(
            "main.ds",
            r#"
import { value } from "./user.preview.ds";
"#,
        )
        .module(
            "user.ds",
            r#"
export let value = 1;
"#,
        )
        .module(
            "user.preview.ds",
            r#"
export let preview = true;
"#,
        )
        .build();

    compiler.assert_dir_imported_diagnostics(
        "main.ds",
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier './user.preview.ds'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"./user.preview.ds\";"
"#,
    );
}
