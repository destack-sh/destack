use std::path::PathBuf;

use super::{ExpectedDiagnostic, LinkedJsonFile, LinkedScriptTarget, LinkedTextFile, TestProgram};
use base64::Engine as _;
use destack_artifact::{
    ArtifactKey, BuildManifest, BuildManifestFile, BuildManifestFileType, BuildManifestLoader,
    ModuleArtifact, PackageAssembly, ScriptDependencyTarget, SourceMapArtifact, TargetOutputName,
};
use destack_source::FileType;
use destack_workspace::{SourceMapMode, TargetDiscovery, TargetId};
use indexmap::indexmap;

const LINKED_ENTRY_PATH: &str = "dist/js.js";
const LINKED_MAP_PATH: &str = "dist/js.js.map";
const LINKED_MANIFEST_PATH: &str = "dist/js.manifest.json";
const MANIFEST_ENTRY_PATH: &str = "dist/js.js";
const MANIFEST_MAP_PATH: &str = "dist/js.js.map";

/// Build one expected linked entry file.
fn expected_linked_entry(text: &str) -> LinkedTextFile {
    LinkedTextFile {
        path: LINKED_ENTRY_PATH.to_string(),
        file_type: FileType::JavaScript,
        text: text.to_string(),
    }
}

/// Build one expected linked entry file with one appended source map reference.
fn expected_linked_entry_with_source_map_reference(
    text: &str,
    source_map_reference: &str,
) -> LinkedTextFile {
    let mut text = text.to_string();

    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }

    text.push_str(&format!("//# sourceMappingURL={source_map_reference}\n"));

    expected_linked_entry(&text)
}

/// Build one expected linked manifest file.
fn expected_linked_manifest(value: BuildManifest) -> LinkedJsonFile<BuildManifest> {
    let text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|error| panic!("failed to serialize expected manifest: {error}"));
    let text = format!("{text}\n");

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
    let text = format!("{text}\n");

    LinkedJsonFile {
        path: LINKED_MAP_PATH.to_string(),
        file_type: FileType::SourceMap,
        text,
        value,
    }
}

/// Build one expected inline source map reference for one map payload.
fn expected_inline_source_map_reference(value: &SourceMapArtifact) -> String {
    let source_map = serde_json::to_string(value)
        .unwrap_or_else(|error| panic!("failed to serialize expected inline source map: {error}"));
    let encoded = base64::engine::general_purpose::STANDARD.encode(source_map.as_bytes());

    format!("data:application/json;charset=utf-8;base64,{encoded}")
}

/// Link a single-file script target over the full reachable graph.
#[test]
fn test_links_single_file_js_target_over_reachable_modules() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("shared-value.js", "export const shared_value = 1;");
    let main = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
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
        entry: expected_linked_entry_with_source_map_reference(
            "export const shared_value = 1;\n\nexport const application_value = shared_value;\n",
            "./js.js.map",
        ),
        manifest: expected_linked_manifest(manifest),
        map: Some(expected_linked_map(map)),
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Record the script link plan in the target manifest.
#[test]
fn test_records_script_link_plan_in_manifest() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module("shared-value.js", "export const shared_value = 1;");
    let main = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
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
    let expected_entry = expected_linked_entry_with_source_map_reference(
        "export const shared_value = 1;\n\nexport const application_value = shared_value;\n",
        "./js.js.map",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected);
}

/// Emit one real chunked assembly over multiple static entry roots.
#[test]
fn test_links_chunked_js_target_over_static_entry_roots() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let shared = test.add_module("shared-value.js", "export const shared_value = 1;");
    let application = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
    );
    let dashboard = test.add_module(
        "dashboard.js",
        "import { shared_value } from './shared-value.js';\n\nexport const dashboard_value = shared_value;",
    );

    let package_id = test.program.modules.get(application).package_id;
    let output = test.link_chunked_js_target_with(&[application, dashboard], "js", |_| {});
    let output_groups = output
        .outputs
        .iter()
        .map(|(name, files)| {
            let files = files
                .iter()
                .map(|file| {
                    let path = file
                        .uri
                        .to_path()
                        .unwrap_or_else(|| panic!("uri '{}' is not a path", file.uri));
                    let package = test.program.packages.get(package_id);
                    let package = package.read();
                    let package_dir = package
                        .path
                        .clone()
                        .unwrap_or_else(|| test.program.cwd.clone());
                    let relative = path.strip_prefix(&package_dir).unwrap_or(path);

                    relative.to_string_lossy().replace('\\', "/")
                })
                .collect::<Vec<_>>();
            (*name, files)
        })
        .collect::<indexmap::IndexMap<_, _>>();
    let manifest =
        test.single_json_output::<BuildManifest>(package_id, &output, TargetOutputName::Manifest);
    let application_path = test.module_relative_path(application);
    let dashboard_path = test.module_relative_path(dashboard);
    let shared_path = test.module_relative_path(shared);

    assert_eq!(output.assembly, PackageAssembly::Chunked);
    assert_eq!(
        output_groups,
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/shared-value.js".to_string(),
                "dist/dashboard.js".to_string(),
                "dist/application.js".to_string(),
            ],
            TargetOutputName::Maps => vec![
                "dist/shared-value.js.map".to_string(),
                "dist/dashboard.js.map".to_string(),
                "dist/application.js.map".to_string(),
            ],
            TargetOutputName::Manifest => vec![
                "dist/js.manifest.json".to_string(),
            ],
        }
    );
    test.assert_linked_json_file(
        &manifest,
        &expected_linked_manifest(BuildManifest {
            index: None,
            files: vec![
                BuildManifestFile {
                    path: "application.js".to_string(),
                    r#type: BuildManifestFileType::Chunk,
                    loader: BuildManifestLoader::Js,
                    name: Some("application".to_string()),
                    input: Some(application_path),
                    is_entry: Some(true),
                    is_dynamic_entry: Some(false),
                    imports: vec!["./shared-value.js".to_string()],
                    dynamic_imports: Vec::new(),
                },
                expected_manifest_file(
                    "application.js.map",
                    BuildManifestFileType::Asset,
                    BuildManifestLoader::Map,
                ),
                BuildManifestFile {
                    path: "dashboard.js".to_string(),
                    r#type: BuildManifestFileType::Chunk,
                    loader: BuildManifestLoader::Js,
                    name: Some("dashboard".to_string()),
                    input: Some(dashboard_path),
                    is_entry: Some(true),
                    is_dynamic_entry: Some(false),
                    imports: vec!["./shared-value.js".to_string()],
                    dynamic_imports: Vec::new(),
                },
                expected_manifest_file(
                    "dashboard.js.map",
                    BuildManifestFileType::Asset,
                    BuildManifestLoader::Map,
                ),
                BuildManifestFile {
                    path: "shared-value.js".to_string(),
                    r#type: BuildManifestFileType::Chunk,
                    loader: BuildManifestLoader::Js,
                    name: Some("shared-value".to_string()),
                    input: Some(shared_path),
                    is_entry: Some(false),
                    is_dynamic_entry: Some(false),
                    imports: Vec::new(),
                    dynamic_imports: Vec::new(),
                },
                expected_manifest_file(
                    "shared-value.js.map",
                    BuildManifestFileType::Asset,
                    BuildManifestLoader::Map,
                ),
            ],
        }),
        "chunked manifest output",
    );
    test.assert_linked_text_file(
        &test.linked_text_output_at_path(
            package_id,
            &output,
            TargetOutputName::Module,
            "dist/application.js",
        ),
        &LinkedTextFile {
            path: "dist/application.js".to_string(),
            file_type: FileType::JavaScript,
            text: "import { shared_value } from \"./shared-value.js\";\nexport const application_value = shared_value;\n//# sourceMappingURL=./application.js.map\n".to_string(),
        },
        "application chunk output",
    );
    test.assert_linked_text_file(
        &test.linked_text_output_at_path(
            package_id,
            &output,
            TargetOutputName::Module,
            "dist/dashboard.js",
        ),
        &LinkedTextFile {
            path: "dist/dashboard.js".to_string(),
            file_type: FileType::JavaScript,
            text: "import { shared_value } from \"./shared-value.js\";\nexport const dashboard_value = shared_value;\n//# sourceMappingURL=./dashboard.js.map\n".to_string(),
        },
        "dashboard chunk output",
    );
    test.assert_linked_text_file(
        &test.linked_text_output_at_path(
            package_id,
            &output,
            TargetOutputName::Module,
            "dist/shared-value.js",
        ),
        &LinkedTextFile {
            path: "dist/shared-value.js".to_string(),
            file_type: FileType::JavaScript,
            text: "export const shared_value = 1;\n//# sourceMappingURL=./shared-value.js.map\n"
                .to_string(),
        },
        "shared chunk output",
    );
}

/// Retain unresolved external package dependencies in the linked entry and manifest.
#[test]
fn test_retains_unresolved_external_dependency_in_single_file_manifest_and_entry() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module("main.ts", "export * from 'react';");

    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle.dependencies.never_bundle = vec!["react".to_string()];
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

/// Emit one external source map reference for bundled single-file output.
#[test]
fn test_emits_external_source_map_reference_for_single_file_target() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("shared-value.js", "export const shared_value = 1;");
    let main = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
    );

    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = SourceMapArtifact {
        version: 3,
        file: None,
        source_root: None,
        sources: vec![dep_path, main_path.clone()],
        sources_content: None,
        names: Vec::new(),
        mappings: String::new(),
        debug_id: None,
    };
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("application.js")];
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
    let expected_entry = expected_linked_entry_with_source_map_reference(
        "export const shared_value = 1;\n\nexport const application_value = shared_value;\n",
        "./js.js.map",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_linked_script_map(&linked, Some(&expected_linked_map(map)));
}

/// Emit one hidden source map sidecar without annotating the bundled entry text.
#[test]
fn test_emits_hidden_source_map_for_single_file_target() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("shared-value.js", "export const shared_value = 1;");
    let main = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
    );

    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = SourceMapArtifact {
        version: 3,
        file: None,
        source_root: None,
        sources: vec![dep_path, main_path.clone()],
        sources_content: None,
        names: Vec::new(),
        mappings: String::new(),
        debug_id: None,
    };
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("application.js")];
        target.bundle.output.sourcemap = Some(SourceMapMode::Hidden);
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
    let expected_entry = expected_linked_entry(
        "export const shared_value = 1;\n\nexport const application_value = shared_value;\n",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_linked_script_map(&linked, Some(&expected_linked_map(map)));
}

/// Emit one inline source map for bundled single-file output without a sidecar map file.
#[test]
fn test_emits_inline_source_map_for_single_file_target() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("shared-value.js", "export const shared_value = 1;");
    let main = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
    );

    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = SourceMapArtifact {
        version: 3,
        file: None,
        source_root: None,
        sources: vec![dep_path, main_path.clone()],
        sources_content: None,
        names: Vec::new(),
        mappings: String::new(),
        debug_id: None,
    };
    let inline_reference = expected_inline_source_map_reference(&map);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("application.js")];
        target.bundle.output.sourcemap = Some(SourceMapMode::Inline);
    });
    let expected_manifest = expected_linked_manifest(BuildManifest {
        index: None,
        files: vec![BuildManifestFile {
            path: MANIFEST_ENTRY_PATH.to_string(),
            r#type: BuildManifestFileType::Chunk,
            loader: BuildManifestLoader::Js,
            name: Some("js".to_string()),
            input: Some(main_path),
            is_entry: Some(true),
            is_dynamic_entry: Some(false),
            imports: Vec::new(),
            dynamic_imports: Vec::new(),
        }],
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec![LINKED_ENTRY_PATH.to_string()],
            TargetOutputName::Manifest => vec![LINKED_MANIFEST_PATH.to_string()],
        },
        entry: expected_linked_entry_with_source_map_reference(
            "export const shared_value = 1;\n\nexport const application_value = shared_value;\n",
            &inline_reference,
        ),
        manifest: expected_manifest,
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
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
        target.bundle.dependencies.only_bundle = vec!["lodash".to_string()];
    });

    let package_id = test.program.modules.get(main).package_id;
    let target_id = TargetId::new(package_id, "js");

    test.run(ArtifactKey::package_output(package_id, target_id));

    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message:
            "invalid target: js: bundle.dependencies.onlyBundle does not allow bundled dependency 'react'"
                .to_string(),
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

/// Collect one resolved static dependency for JavaScript source imports.
#[test]
fn test_collects_static_script_dependency_for_javascript_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dependency_module = test.add_module("shared-value.js", "export const shared_value = 1;");
    let entry_module = test.add_module(
        "application.js",
        "import { shared_value } from './shared-value.js';\n\nexport const application_value = shared_value;",
    );

    test.configure_target(entry_module, "js", |target| {
        // generated artifacts should preserve resolved js import metadata
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("application.js")];
        target.out_file = Some(PathBuf::from("dist/js.js"));
    });

    let package_id = test.program.modules.get(entry_module).package_id;
    let target_id = TargetId::new(package_id, "js");

    test.run(ArtifactKey::module_artifact(entry_module, target_id));
    test.check_no_diagnostic(destack_source::DiagnosticSeverity::Error);

    let artifact = test.module_artifact(entry_module, "js");
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
            assert_eq!(
                *module, dependency_module,
                "expected dependency to resolve to shared-value.js"
            );
            assert_eq!(
                specifier, "./shared-value.js",
                "expected dependency specifier to preserve source text"
            );
        }
        ScriptDependencyTarget::External { specifier } => {
            panic!("expected internal dependency, found external '{specifier}'");
        }
    }
}
