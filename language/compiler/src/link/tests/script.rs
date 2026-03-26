use std::path::PathBuf;

use super::{ExpectedDiagnostic, LinkedJsonFile, LinkedScriptTarget, LinkedTextFile, TestProgram};
use destack_artifact::{
    ArtifactKey, BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader,
    ModuleArtifact, PackageAssembly, ScriptDependencyTarget, SourceMapArtifact, TargetOutputName,
};
use destack_source::FileType;
use destack_workspace::{TargetDiscovery, TargetId};
use indexmap::indexmap;

const LINKED_ENTRY_PATH: &str = "dist/js.js";
const LINKED_MAP_PATH: &str = "dist/js.js.map";
const LINKED_MANIFEST_PATH: &str = "dist/js.manifest.json";
const MANIFEST_ENTRY_PATH: &str = "js.js";
const MANIFEST_MAP_PATH: &str = "js.js.map";

/// Build one expected linked entry file.
fn expected_linked_entry(text: &str) -> LinkedTextFile {
    LinkedTextFile {
        path: LINKED_ENTRY_PATH.to_string(),
        file_type: FileType::JavaScript,
        text: text.to_string(),
    }
}

/// Build one expected linked manifest file.
fn expected_linked_manifest(value: BuildManifest) -> LinkedJsonFile<BuildManifest> {
    let text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|error| panic!("failed to serialize expected manifest: {error}"));

    LinkedJsonFile {
        path: LINKED_MANIFEST_PATH.to_string(),
        file_type: FileType::Json,
        text,
        value,
    }
}

/// Build one expected linked manifest file record.
fn expected_manifest_file(
    path: &str,
    file_type: BuildManifestFileType,
    loader: BuildManifestLoader,
) -> BuildManifestFile {
    BuildManifestFile {
        path: path.to_string(),
        r#type: file_type,
        loader,
        name: None,
        input: None,
        is_entry: None,
        is_dynamic_entry: None,
        imports: Vec::new(),
        dynamic_imports: Vec::new(),
    }
}

/// Build one expected linked source map file.
fn expected_linked_map(value: SourceMapArtifact) -> LinkedJsonFile<SourceMapArtifact> {
    let text = serde_json::to_string(&value)
        .unwrap_or_else(|error| panic!("failed to serialize expected source map: {error}"));

    LinkedJsonFile {
        path: LINKED_MAP_PATH.to_string(),
        file_type: FileType::SourceMap,
        text,
        value,
    }
}

/// Link a single-file script target over the full reachable graph.
#[test]
fn test_links_single_file_js_target_over_reachable_modules() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("dep.ts", "export const dep = 1;");
    let main = test.add_module(
        "main.ts",
        "import { dep } from './dep';\n\nexport const value = dep;",
    );

    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let manifest = BuildManifest {
        index: None,
        files: vec![
            BuildManifestFile {
                path: MANIFEST_ENTRY_PATH.to_string(),
                r#type: BuildManifestFileType::Chunk,
                loader: BuildManifestLoader::Js,
                name: Some("js".to_string()),
                input: Some(main_path.clone()),
                is_entry: Some(true),
                is_dynamic_entry: Some(false),
                imports: Vec::new(),
                dynamic_imports: Vec::new(),
            },
            expected_manifest_file(
                MANIFEST_MAP_PATH,
                BuildManifestFileType::Asset,
                BuildManifestLoader::Map,
            ),
        ],
    };
    let map = SourceMapArtifact {
        version: 3,
        file: None,
        source_root: None,
        sources: vec![dep_path, main_path],
        sources_content: None,
        names: Vec::new(),
        mappings: String::new(),
        debug_id: None,
    };

    let linked = test.link_single_file_js_target(main, "js");
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec![LINKED_ENTRY_PATH.to_string()],
            TargetOutputName::Maps => vec![LINKED_MAP_PATH.to_string()],
            TargetOutputName::Manifest => vec![LINKED_MANIFEST_PATH.to_string()],
        },
        entry: expected_linked_entry("export const dep = 1;\n\n\nexport const value = dep;\n"),
        manifest: expected_linked_manifest(manifest),
        map: expected_linked_map(map),
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Record the script link plan in the target manifest.
#[test]
fn test_records_script_link_plan_in_manifest() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let _dep = test.add_module("dep.ts", "export const dep = 1;");
    let main = test.add_module(
        "main.ts",
        "import { dep } from './dep';\n\nexport const value = dep;",
    );

    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target(main, "js");
    let expected = expected_linked_manifest(BuildManifest {
        index: None,
        files: vec![
            BuildManifestFile {
                path: MANIFEST_ENTRY_PATH.to_string(),
                r#type: BuildManifestFileType::Chunk,
                loader: BuildManifestLoader::Js,
                name: Some("js".to_string()),
                input: Some(main_path),
                is_entry: Some(true),
                is_dynamic_entry: Some(false),
                imports: Vec::new(),
                dynamic_imports: Vec::new(),
            },
            expected_manifest_file(
                MANIFEST_MAP_PATH,
                BuildManifestFileType::Asset,
                BuildManifestLoader::Map,
            ),
        ],
    });

    test.assert_linked_script_manifest(&linked, &expected);
}

/// Retain unresolved external package dependencies in the linked entry and manifest.
#[test]
fn test_retains_unresolved_external_dependency_in_single_file_manifest_and_entry() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module("main.ts", "export * from 'react';");

    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle.dependency.never_bundle = vec!["react".to_string()];
    });
    let expected_manifest = expected_linked_manifest(BuildManifest {
        index: None,
        files: vec![
            BuildManifestFile {
                path: MANIFEST_ENTRY_PATH.to_string(),
                r#type: BuildManifestFileType::Chunk,
                loader: BuildManifestLoader::Js,
                name: Some("js".to_string()),
                input: Some(main_path),
                is_entry: Some(true),
                is_dynamic_entry: Some(false),
                imports: vec!["react".to_string()],
                dynamic_imports: Vec::new(),
            },
            expected_manifest_file(
                MANIFEST_MAP_PATH,
                BuildManifestFileType::Asset,
                BuildManifestLoader::Map,
            ),
        ],
    });
    let expected_entry = expected_linked_entry("export * from \"react\";\n");

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
}

/// Reject bundled package dependencies that are not in `onlyBundle`.
#[test]
fn test_rejects_unlisted_only_bundle_dependency() {
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
    let main = test.add_module("main.ts", "export * from 'react';");

    test.configure_target(main, "js", |target| {
        // single-file linking forces assembly and therefore dependency policy checks
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("main.ts")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
        target.bundle.dependency.only_bundle = vec!["lodash".to_string()];
    });

    let package_id = test.program.modules.get(main).package_id;
    let target_id = TargetId::new(package_id, "js");

    test.run(ArtifactKey::package_output(package_id, target_id));

    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message: "invalid target: js: bundle.dependency.onlyBundle does not allow bundled dependency 'react'".to_string(),
    }]);
}

/// Reject bundled dynamic imports until chunked or inline-dynamic assembly exists.
#[test]
fn test_rejects_bundled_dynamic_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module("dep.js", "export const value = 1;");
    let main = test.add_module("main.js", "export const dep = import('./dep.js');");

    test.configure_target(main, "js", |target| {
        // single-file linking forces target-level dynamic import handling
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("main.js")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
    });

    let package_id = test.program.modules.get(main).package_id;
    let target_id = TargetId::new(package_id, "js");

    test.run(ArtifactKey::package_output(package_id, target_id));

    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message: "invalid target: js: bundled dynamic import './dep.js' is not implemented yet"
            .to_string(),
    }]);
}

/// Collect one resolved static dependency for Destack source imports.
#[test]
fn test_collects_static_script_dependency_for_destack_source_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("dep.ds", "export const dep = 1;");
    let main = test.add_module(
        "main.ds",
        "import { dep } from './dep.ds';\n\nexport const value = dep;",
    );

    test.configure_target(main, "js", |target| {
        // generated artifacts should be enough here
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("main.ds")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
    });

    let package_id = test.program.modules.get(main).package_id;
    let target_id = TargetId::new(package_id, "js");

    test.run(ArtifactKey::module_artifact(main, target_id));
    test.check_no_diagnostic(destack_source::DiagnosticSeverity::Error);

    let artifact = test.module_artifact(main, "js");
    let ModuleArtifact::Script(script) = artifact else {
        panic!("expected script artifact");
    };

    assert_eq!(
        script.linkage.static_dependencies.len(),
        1,
        "expected one static dependency"
    );

    let dependency = &script.linkage.static_dependencies[0];
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
