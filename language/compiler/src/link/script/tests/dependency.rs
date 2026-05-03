use std::path::PathBuf;

use destack_artifact::{ArtifactKey, ModuleOutput, ScriptDependencyTarget};
use destack_source::{DiagnosticSeverity, TargetId};
use destack_workspace::TargetDiscovery;

use super::{TestProgram, js};

/// Collect one resolved static dependency for Destack source imports.
#[test]
fn test_collects_static_script_dependency_for_destack_source_import() {
    // preserve one resolved static import for one Destack source module
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module(
        "dep.ds",
        &js(r#"
export const dependencyValue = 1;
"#),
    );
    let main = test.add_module(
        "main.ds",
        &js(r#"
import { dependencyValue } from './dep.ds';

export const value = dependencyValue;
"#),
    );

    // build one script output with resolved dependencies
    test.configure_target(main, "js", |target| {
        // generated artifacts should be enough here
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("main.ds")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
    });

    let package_id = test.program.module_descriptor(main).package_id;
    let target_id = TargetId::new(package_id, "js");
    test.run(ArtifactKey::module_output(main, target_id));
    test.check_no_diagnostic(DiagnosticSeverity::Error);

    // keep the resolved module id and the original specifier text
    let artifact = test.module_output(main, "js");
    let ModuleOutput::Script(script) = artifact else {
        panic!("expected script output");
    };

    assert_eq!(
        script.dependencies.static_dependencies.len(),
        1,
        "expected one static dependency"
    );

    let dependency = &script.dependencies.static_dependencies[0];
    match &dependency.target {
        ScriptDependencyTarget::Module { module, specifier } => {
            assert_eq!(*module, dep, "expected dependency to resolve to dep.ds");
            assert_eq!(
                specifier, "./dep.ds",
                "expected dependency specifier to preserve source text"
            );
        }
        ScriptDependencyTarget::External { specifier } => {
            panic!("expected internal dependency, found external '{specifier}'");
        }
    }
}

/// Collect one resolved static dependency for JavaScript source imports.
#[test]
fn test_collects_static_script_dependency_for_javascript_import() {
    // preserve one resolved static import for one JavaScript source module
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dependency_module = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
    let entry_module = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // build one script output with resolved dependencies
    test.configure_target(entry_module, "js", |target| {
        // generated artifacts should preserve resolved js import metadata
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("app.ts")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
    });

    let package_id = test.program.module_descriptor(entry_module).package_id;
    let target_id = TargetId::new(package_id, "js");
    test.run(ArtifactKey::module_output(entry_module, target_id));
    test.check_no_diagnostic(DiagnosticSeverity::Error);

    // keep the resolved module id and the original specifier text
    let artifact = test.module_output(entry_module, "js");
    let ModuleOutput::Script(script) = artifact else {
        panic!("expected script output");
    };

    assert_eq!(
        script.dependencies.static_dependencies.len(),
        1,
        "expected one static dependency"
    );

    let dependency = &script.dependencies.static_dependencies[0];
    match &dependency.target {
        ScriptDependencyTarget::Module { module, specifier } => {
            assert_eq!(
                *module, dependency_module,
                "expected dependency to resolve to common.ts"
            );
            assert_eq!(
                specifier, "./common.ts",
                "expected dependency specifier to preserve source text"
            );
        }
        ScriptDependencyTarget::External { specifier } => {
            panic!("expected internal dependency, found external '{specifier}'");
        }
    }
}
