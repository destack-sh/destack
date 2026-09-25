use crate::tests::{DirRows, TestSession};

#[test]
fn test_import_records_static_if_true_import_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(true)
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
@if(true)
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_omits_static_if_false_import_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(false)
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
@if(false)
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_omits_static_if_false_reexport_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(true && false)
export { Foo } from "./dep.tspp";
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
@if(true && false)
export { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_records_static_if_true_reexport_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(true)
export { Foo } from "./dep.tspp";
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
@if(true)
export { Foo } from "./dep.tspp";
/// @module.edge relation=re_export specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_static_if_global_import_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
global {
    @if(true)
    import { Foo } from "./dep.tspp";
}
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
global {
    @if(true)
    import { Foo } from "./dep.tspp";
    /// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

}

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_omits_static_if_false_import_item_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { @if(false) Foo } from "./missing.tspp";
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { @if(false) Foo } from "./missing.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_records_static_if_true_import_item_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
import { @if(false) Foo, @if(true) Bar } from "./dep.tspp";
"#,
        )
        .module(
            "dep.tspp",
            r#"
export type Foo = string;
export type Bar = string;
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
import { @if(false) Foo, @if(true) Bar } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_omits_static_if_false_reexport_item_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
export { @if(false) Foo } from "./missing.tspp";
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
export { @if(false) Foo } from "./missing.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_records_static_if_active_mode_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.modes.includes("preview"))
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
@if(import.meta.modes.includes("preview"))
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_omits_static_if_inactive_mode_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.modes.includes("dev"))
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
@if(import.meta.modes.includes("dev"))
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_records_static_if_active_role_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "compiler": {
        "roles": ["server"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.roles.includes("server"))
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
@if(import.meta.roles.includes("server"))
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_static_if_active_label_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "conditions": {
        "modes": {
            "preview": {
                "labels": {
                    "release": "preview"
                }
            }
        }
    },
    "compiler": {
        "modes": ["preview"]
    }
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.labels["release"].includes("preview"))
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
@if(import.meta.labels["release"].includes("preview"))
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_static_if_runtime_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "targets": {
        "default": {
            "emit": "js",
            "runtime": "js"
        }
    },
    "defaultTarget": "default"
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.runtime == "js")
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
@if(import.meta.runtime == "js")
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_records_static_if_output_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "targets": {
        "default": {
            "emit": "js"
        }
    },
    "defaultTarget": "default"
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.output == "bundle")
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
@if(import.meta.output == "bundle")
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_omits_static_if_platform_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "targets": {
        "default": {
            "emit": "js",
            "platform": "linux"
        }
    },
    "defaultTarget": "default"
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.platform == "windows")
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
@if(import.meta.platform == "windows")
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_omits_static_if_undefined_product_edge() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(import.meta.product == "app")
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
@if(import.meta.product == "app")
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_omits_static_if_target_family_edge() {
    let compiler = TestSession::builder()
        .data(
            "destack.json",
            r#"
{
    "name": "test",
    "targets": {
        "default": {
            "emit": "js",
            "platform": "linux"
        }
    },
    "defaultTarget": "default"
}
"#,
        )
        .module(
            "main.tspp",
            r#"
@if(import.meta.target.family == "windows")
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
@if(import.meta.target.family == "windows")
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_static_if_false_suppresses_unresolved_module() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(false)
import { Foo } from "./missing.tspp";
"#,
        )
        .build();

    compiler.assert_dir_imported(
        "main.tspp",
        DirRows::modules().with_summaries(),
        r#"
@if(false)
import { Foo } from "./missing.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_static_if_short_circuits_false_and_runtime_condition() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const enabled = true;

@if(false && enabled)
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
const enabled = true;

@if(false && enabled)
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_static_if_short_circuits_true_or_runtime_condition() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const enabled = false;

@if(true || enabled)
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
const enabled = false;

@if(true || enabled)
import { Foo } from "./dep.tspp";
/// @module.edge relation=import specifier=./dep.tspp module=dep.tspp

/// @module.summary edges=1
"#,
    );
}

#[test]
fn test_import_static_if_multiple_guards_are_conjunctive() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(true)
@if(false)
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
@if(true)
@if(false)
import { Foo } from "./dep.tspp";

/// @module.summary edges=0
"#,
    );
}

#[test]
fn test_import_reports_static_if_missing_condition() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if
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

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=missing-static-import-condition message="`@if` import guard requires a condition"
/// @diagnostic.label line=2 column=1 span="@if" line_source="@if"
"#,
    );
}

#[test]
fn test_import_reports_static_if_extra_condition() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(true, false)
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

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=multiple-static-import-conditions message="`@if` import guard requires exactly one condition"
/// @diagnostic.label line=2 column=1 span="@if(true, false)" line_source="@if(true, false)"
"#,
    );
}

#[test]
fn test_import_reports_static_if_non_boolean_condition() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if(1)
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

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=non-boolean-static-import-condition message="`@if` import guard condition must be boolean"
/// @diagnostic.label line=2 column=5 span="1" line_source="@if(1)"
"#,
    );
}

#[test]
fn test_import_reports_static_if_runtime_condition() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
const enabled = true;

@if(enabled)
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

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=non-static-import-condition message="`@if` import guard condition is not static"
/// @diagnostic.label line=4 column=5 span="enabled" line_source="@if(enabled)"
"#,
    );
}

#[test]
fn test_import_reports_generic_static_if_invocation() {
    let compiler = TestSession::builder()
        .module(
            "main.tspp",
            r#"
@if<boolean>(true)
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

    compiler.assert_dir_imported_diagnostics(
        "main.tspp",
        r#"
/// @diagnostic.error id=invalid-static-import-condition message="`@if` import guard must be invoked as `@if(condition)`"
/// @diagnostic.label line=2 column=1 span="@if<boolean>(true)" line_source="@if<boolean>(true)"
"#,
    );
}
