use crate::tests::TestCompiler;
use crate::tests::snapshot::{DirSnapshotSet, assert_snapshot};

#[test]
fn test_import_records_static_edge() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
import { Foo } from "./dep.ds";
/// @dependency.edge relation=import specifier=./dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_resolves_extensionless_source_path() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
import { Foo } from "./dep";
/// @dependency.edge relation=import specifier=./dep module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_reports_side_effect_import() {
    let compiler = TestCompiler::new()
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

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI201 message="side-effect import './dep.ds' is not supported"
/// @diagnostic.label line=2 column=1 source="import \"./dep.ds\";"
"#,
    );
}

#[test]
fn test_import_records_external_edge() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import { value } from "host:runtime";
"#,
        )
        .build();

    assert_snapshot(
        compiler.dir_snapshot("main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
import { value } from "host:runtime";
/// @dependency.edge relation=import specifier=host:runtime external=host:runtime

/// @dependency.summary edges=1
"#,
    );
}

#[test]
fn test_import_renders_multi_module_snapshot() {
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshots(
            &["main.ds", "dep.ds"],
            DirSnapshotSet::none().with_dependency(),
        ),
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
    let compiler = TestCompiler::new()
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

    assert_snapshot(
        compiler.dir_snapshot("src/main.ds", DirSnapshotSet::none().with_dependency()),
        r#"
import { Foo } from "../dep.ds";
/// @dependency.edge relation=import specifier=../dep.ds module=dep.ds

/// @dependency.summary edges=1
"#,
    );
}
