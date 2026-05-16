use crate::tests::TestCompiler;
use crate::tests::snapshot::assert_snapshot;

#[test]
fn test_import_reports_missing_local_module() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import { Missing } from "./missing.ds";
"#,
        )
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI200 message="unresolved module './missing.ds'"
/// @diagnostic.label line=2 column=1 source="import { Missing } from \"./missing.ds\";"
"#,
    );
}

#[test]
fn test_import_reports_bare_specifier() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import { value } from "pkg";
"#,
        )
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier 'pkg'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"pkg\";"
"#,
    );
}

#[test]
fn test_import_reports_scoped_package_specifier() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r#"
import { value } from "@scope/pkg";
"#,
        )
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier '@scope/pkg'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"@scope/pkg\";"
"#,
    );
}

#[test]
fn test_import_reports_hash_specifier() {
    let compiler = TestCompiler::new()
        .module(
            "main.ds",
            r##"
import { value } from "#internal";
"##,
        )
        .build();

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r##"
/// @diagnostic.error code=EI204 message="unsupported module specifier '#internal'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"#internal\";"
"##,
    );
}

#[test]
fn test_import_reports_absolute_specifier() {
    let compiler = TestCompiler::new()
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

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier '/dep.ds'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"/dep.ds\";"
"#,
    );
}

#[test]
fn test_import_reports_local_query_specifier() {
    let compiler = TestCompiler::new()
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

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI204 message="unsupported module specifier './dep.ds?raw'"
/// @diagnostic.label line=2 column=1 source="import { value } from \"./dep.ds?raw\";"
"#,
    );
}

#[test]
fn test_import_reports_ambiguous_extensionless_specifier() {
    let compiler = TestCompiler::new()
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

    compiler
        .provide_dir_imported("main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("main.ds")),
        r#"
/// @diagnostic.error code=EI205 message="ambiguous module specifier './dep': dep.ds, dep.ts"
/// @diagnostic.label line=2 column=1 source="import { value } from \"./dep\";"
"#,
    );
}

#[test]
fn test_import_reports_cross_package_relative_specifier() {
    let compiler = TestCompiler::new()
        .data(
            "destack.json",
            r#"{ "compiler": {}, "policy": {}, "runtime": {}, "formatter": {}, "linter": {}, "workspace": { "packages": ["packages/*"] } }"#,
        )
        .data(
            "packages/app/destack.json",
            r#"{ "name": "app", "compiler": {}, "policy": {}, "runtime": {}, "formatter": {}, "linter": {} }"#,
        )
        .data(
            "packages/lib/destack.json",
            r#"{ "name": "lib", "compiler": {}, "policy": {}, "runtime": {}, "formatter": {}, "linter": {} }"#,
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

    compiler
        .provide_dir_imported("packages/app/main.ds")
        .expect("artifact should be provided with diagnostics");
    assert_snapshot(
        compiler.diagnostic_snapshot(compiler.dir_imported_key("packages/app/main.ds")),
        r#"
/// @diagnostic.error code=EI206 message="local module specifier '../lib/dep.ds' crosses package boundaries"
/// @diagnostic.label line=2 column=1 source="import { value } from \"../lib/dep.ds\";"
"#,
    );
}
