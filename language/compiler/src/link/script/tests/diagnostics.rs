use std::path::PathBuf;

use destack_artifact::ArtifactKey;
use destack_workspace::{BundleFormat, TargetDiscovery};

use super::{TestProgram, js};
use crate::tests::ExpectedDiagnostic;

/// Reject bundled package dependencies that are not in `onlyBundle`.
#[test]
fn test_rejects_unlisted_only_bundle_dependency() {
    // reject bundled package imports that are outside onlyBundle
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_file(
        "node_modules/react/package.json",
        r#"{"name":"react","type":"module","exports":"./index.js"}"#,
    );
    test.add_module(
        "node_modules/react/index.js",
        r#"export const version = "18.0.0";"#,
    );
    let main = test.add_module("main.ts", &js(r#"export * from 'react';"#));

    // configure one invalid single-file target
    test.configure_target(main, "js", |target| {
        // single-file linking forces assembly and therefore dependency policy checks
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("main.ts")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
        target.bundle_dependencies.only_bundle = vec!["lodash".to_string()];
    });

    let package_id = test.program.module_descriptor(main).package_id;
    let target_id = test.target_id(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // report one exact linker diagnostic
    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message: "invalid target: js: dependencies.onlyBundle does not allow bundled dependency 'react'"
            .to_string(),
    }]);
}

/// Reject bundled dynamic imports outside chunked assembly.
#[test]
fn test_rejects_bundled_dynamic_import() {
    // reject one bundled dynamic import in single-file mode
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module("dependency.ts", &js(r#"export const value = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"export const dependencyPromise = import('./dependency.ts');"#),
    );

    // configure one invalid single-file target
    test.configure_target(main, "js", |target| {
        // single-file linking forces target-level dynamic import handling
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("app.ts")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
    });

    let package_id = test.program.module_descriptor(main).package_id;
    let target_id = test.target_id(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // report one exact linker diagnostic
    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message:
            "invalid target: js: bundled dynamic import './dependency.ts' is not implemented yet"
                .to_string(),
    }]);
}

/// Reject bundle output formats that the linker does not implement yet.
#[test]
fn test_rejects_unimplemented_bundle_output_format() {
    // reject one bundle output format the linker does not support yet
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module("app.ts", &js(r#"export const value = 1;"#));

    // configure one invalid single-file target
    test.configure_target(main, "js", |target| {
        // single-file linking forces target assembly through the linker
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("app.ts")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
        target.bundle_output.format = Some(BundleFormat::Iife);
    });

    let package_id = test.program.module_descriptor(main).package_id;
    let target_id = test.target_id(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // report one exact linker diagnostic
    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message: "invalid target: js: output.format 'iife' is not implemented yet".to_string(),
    }]);
}

/// Reject bundle minification until the linker implements it.
#[test]
fn test_rejects_unimplemented_bundle_minify() {
    // reject one minify setting the linker does not support yet
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module("app.ts", &js(r#"export const value = 1;"#));

    // configure one invalid single-file target
    test.configure_target(main, "js", |target| {
        // single-file linking forces target assembly through the linker
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("app.ts")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
        target.minify.enabled = true;
    });

    let package_id = test.program.module_descriptor(main).package_id;
    let target_id = test.target_id(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // report one exact linker diagnostic
    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message: "invalid target: js: bundle.minify is not implemented yet".to_string(),
    }]);
}
