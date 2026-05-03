use destack_artifact::{
    ArtifactKey, BuildManifestFile, BuildManifestFileType, BuildManifestLoader, TargetOutputName,
};
use destack_source::TargetId;
use destack_workspace::{BundleMode, TargetDiscovery};
use indexmap::indexmap;

use super::super::plan::OutputKind;
use super::{
    TestProgram, chunked_script_target, js, js_output, manifest, manifest_chunk, manifest_map,
    planned_output, script_output,
};
use crate::LinkError;
use crate::link::ScriptLinker;

/// Emit one real chunked assembly over multiple static entry roots.
#[test]
fn test_links_chunked_js_target_over_static_entry_roots() {
    // split two entry roots through one shared static module
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [common, app, dashboard] = test.add_modules([
        (
            "common.ts",
            js(r#"
export const commonValue = 1;
"#),
        ),
        (
            "app.ts",
            js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { commonValue } from './common.ts';

export const dashboardValue = commonValue;
"#),
        ),
    ]);

    // emit one shared chunk plus one chunk per entry
    let linked = test.linked_chunked_js_target_with(&[app, dashboard], "js", |_| {});
    let app_path = test.module_relative_path(app);
    let dashboard_path = test.module_relative_path(dashboard);
    let common_path = test.module_relative_path(common);
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/common-a9ad2188.js".to_string(),
                "dist/app.js".to_string(),
                "dist/dashboard.js".to_string(),
            ],
            TargetOutputName::Maps => vec![
                "dist/common-a9ad2188.js.map".to_string(),
                "dist/app.js.map".to_string(),
                "dist/dashboard.js.map".to_string(),
            ],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .imports(&["./common-a9ad2188.js"])
                .into(),
            manifest_map("app.js.map"),
            manifest_chunk("common-a9ad2188.js", "common")
                .input(&common_path)
                .shared()
                .into(),
            manifest_map("common-a9ad2188.js.map"),
            manifest_chunk("dashboard.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["./common-a9ad2188.js"])
                .into(),
            manifest_map("dashboard.js.map"),
        ]),
        vec![
            script_output(
                "dist/common-a9ad2188.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./common-a9ad2188.js.map
"#,
                ),
            ),
            script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { commonValue } from "./common-a9ad2188.js";
export const appValue = commonValue;
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
            script_output(
                "dist/dashboard.js",
                &js_output(
                    r#"
import { commonValue } from "./common-a9ad2188.js";
export const dashboardValue = commonValue;
//# sourceMappingURL=./dashboard.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Emit one exact chunked assembly with nested entry and shared file-name templates.
#[test]
fn test_links_chunked_js_target_with_nested_file_name_templates() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [common, app, dashboard] = test.add_modules([
        ("common.ts", js(r#"export const commonValue = 1;"#)),
        (
            "app.ts",
            js(r#"
import { commonValue } from "./common.ts";

export const appValue = commonValue;
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { commonValue } from "./common.ts";

export const dashboardValue = commonValue;
"#),
        ),
    ]);

    let linked = test.linked_chunked_js_target_with(&[app, dashboard], "js", |target| {
        target.bundle_output.entry_file_names = Some("entries/[name]-entry.[ext]".to_string());
        target.bundle_output.chunk_file_names = Some("chunks/[name]-shared.[ext]".to_string());
    });
    let app_path = test.module_relative_path(app);
    let dashboard_path = test.module_relative_path(dashboard);
    let common_path = test.module_relative_path(common);
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/chunks/common-shared.js".to_string(),
                "dist/entries/app-entry.js".to_string(),
                "dist/entries/dashboard-entry.js".to_string(),
            ],
            TargetOutputName::Maps => vec![
                "dist/chunks/common-shared.js.map".to_string(),
                "dist/entries/app-entry.js.map".to_string(),
                "dist/entries/dashboard-entry.js.map".to_string(),
            ],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("chunks/common-shared.js", "common")
                .input(&common_path)
                .shared()
                .into(),
            manifest_map("chunks/common-shared.js.map"),
            manifest_chunk("entries/app-entry.js", "app")
                .input(&app_path)
                .entry()
                .imports(&["../chunks/common-shared.js"])
                .into(),
            manifest_map("entries/app-entry.js.map"),
            manifest_chunk("entries/dashboard-entry.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["../chunks/common-shared.js"])
                .into(),
            manifest_map("entries/dashboard-entry.js.map"),
        ]),
        vec![
            script_output(
                "dist/chunks/common-shared.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./common-shared.js.map
"#,
                ),
            ),
            script_output(
                "dist/entries/app-entry.js",
                &js_output(
                    r#"
import { commonValue } from "../chunks/common-shared.js";
export const appValue = commonValue;
//# sourceMappingURL=./app-entry.js.map
"#,
                ),
            ),
            script_output(
                "dist/entries/dashboard-entry.js",
                &js_output(
                    r#"
import { commonValue } from "../chunks/common-shared.js";
export const dashboardValue = commonValue;
//# sourceMappingURL=./dashboard-entry.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Plan stable chunk graph facts for shared static chunking.
#[test]
fn test_plans_chunked_js_target_over_static_entry_roots() {
    // keep one shared module separate from both entry chunks
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [_common, app, dashboard] = test.add_modules([
        (
            "common.ts",
            js(r#"
export const commonValue = 1;
"#),
        ),
        (
            "app.ts",
            js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { commonValue } from './common.ts';

export const dashboardValue = commonValue;
"#),
        ),
    ]);

    // plan one shared output and two dependent entry outputs
    let chunks = test.plan_chunked_js_target_with(&[app, dashboard], "js", |_| {});

    assert_eq!(
        chunks,
        vec![
            planned_output("common", OutputKind::Shared, &["common.ts"]),
            planned_output("app", OutputKind::Entry, &["app.ts"]).static_dependencies(&["common"]),
            planned_output("dashboard", OutputKind::Entry, &["dashboard.ts"])
                .static_dependencies(&["common"]),
        ],
    );
}

/// Reject chunk file-name templates that collapse distinct shared outputs.
#[test]
fn test_rejects_chunked_shared_output_path_collisions() {
    // keep two distinct shared chunks so a fixed file name collides
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [vendor_a, vendor_b, main1, main2] = test.add_modules([
        (
            "vendor-a.ts",
            js(r#"
export const vendorA = "vendor-a";
"#),
        ),
        (
            "vendor-b.ts",
            js(r#"
export const vendorB = "vendor-b";
"#),
        ),
        (
            "main1.ts",
            js(r#"
import { vendorA } from "./vendor-a.ts";
import { vendorB } from "./vendor-b.ts";

export const main1 = [vendorA, vendorB];
"#),
        ),
        (
            "main2.ts",
            js(r#"
import { vendorA } from "./vendor-a.ts";
import { vendorB } from "./vendor-b.ts";

export const main2 = [vendorA, vendorB];
"#),
        ),
    ]);

    // force all shared chunks onto the same emitted path
    let vendor_a_path = test.module_relative_path(vendor_a);
    let vendor_b_path = test.module_relative_path(vendor_b);
    test.configure_target(main1, "js", |target| {
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec!["main1.ts".into(), "main2.ts".into()];
        target.out_dir = "dist".into();
        target.assembly = BundleMode::Chunked;
        target.bundle_output.chunk_file_names = Some("chunks/chunk.js".to_string());
        target
            .manual_chunks
            .insert("vendor-a".to_string(), vec![vendor_a_path.clone()]);
        target
            .manual_chunks
            .insert("vendor-b".to_string(), vec![vendor_b_path.clone()]);
    });

    let package_id = test.program.module_descriptor(main1).package_id;
    let target_id = TargetId::new(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // rebuild the linker plan directly so the layout failure is observable here
    let package = test.program.package_descriptor(package_id);
    let package_dir = package
        .path
        .clone()
        .unwrap_or_else(|| test.program.root_directory().clone());
    let target = package
        .targets
        .get(&target_id)
        .cloned()
        .unwrap_or_else(|| panic!("missing target 'js'"));
    let context = crate::tests::test_provider_context(
        test.compiler.as_ref(),
        test.current_revision(),
        ArtifactKey::WorkspaceLinted,
    );
    let linker = ScriptLinker::new(
        test.compiler.as_ref(),
        context.as_ref(),
        &package_dir,
        None,
        &target,
        &target_id,
        package_id,
    );
    let linked_modules = linker
        .require_module_artifacts(&[main1, main2])
        .unwrap_or_else(|error| panic!("failed to require script target artifacts: {error:?}"));
    let module_set = linker
        .build_script_module_set(&[main1, main2], &linked_modules)
        .unwrap_or_else(|error| panic!("failed to build script module set: {error:?}"));
    let output_graph = linker
        .build_script_output_graph(&module_set)
        .unwrap_or_else(|error| panic!("failed to build script output graph: {error:?}"));
    let error = linker
        .build_output_layout(&output_graph)
        .expect_err("expected chunk output path collision");

    // report one exact linker failure shape
    assert_eq!(
        error,
        LinkError::InvalidTarget {
            anchor: package_id.into(),
            package: package_id,
            target: target_id,
            message:
                "multiple script outputs resolve to the same emitted path 'dist/chunks/chunk.js'"
                    .to_string(),
        },
    );
}

/// Retain one unresolved external dependency in one chunked entry output and manifest.
#[test]
fn test_retains_unresolved_external_dependency_in_chunked_manifest_and_entry() {
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
    let app = test.add_module("app.ts", &js(r#"export * from "react";"#));

    let linked = test.linked_chunked_js_target_with(&[app], "js", |target| {
        target.bundle_dependencies.never_bundle = vec!["react".to_string()];
        target.bundle_output.sourcemap = None;
    });
    let app_path = test.module_relative_path(app);
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec!["dist/app.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .imports(&["react"])
                .into(),
        ]),
        vec![script_output(
            "dist/app.js",
            &js_output(
                r#"
export * from "react";
"#,
            ),
        )],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Keep chunked stylesheet assets and manifest stylesheet links exact.
#[test]
fn test_links_chunked_js_target_with_css_assets() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let _styles = test.add_module(
        "styles.css",
        r#"
body {
  color: red;
}
"#,
    );
    let app = test.add_module(
        "app.ts",
        &js(r#"
import "./styles.css";

export const panelState = "ready";
"#),
    );

    let package_id = test.program.module_descriptor(app).package_id;
    let output = test.link_chunked_js_target_with(&[app], "js", |target| {
        target.bundle_output.sourcemap = None;
        target.bundle_output.asset_file_names = Some("[name].[ext]".to_string());
    });
    let linked = test.linked_chunked_script_target(package_id, &output);
    let app_path = test.module_relative_path(app);
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec!["dist/app.js".to_string()],
            TargetOutputName::Assets => vec!["dist/styles.css".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .stylesheets(&["./styles.css"])
                .into(),
            BuildManifestFile {
                path: "styles.css".to_string(),
                r#type: BuildManifestFileType::Asset,
                loader: BuildManifestLoader::Asset,
                name: None,
                input: None,
                is_entry: None,
                is_dynamic_entry: None,
                imports: Vec::new(),
                dynamic_imports: Vec::new(),
                stylesheets: Vec::new(),
            },
        ]),
        vec![script_output(
            "dist/app.js",
            &js_output(
                r#"
export const panelState = "ready";
"#,
            ),
        )],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
    test.assert_text_output_at_path(
        package_id,
        &output,
        TargetOutputName::Assets,
        &super::LinkedTextFile {
            path: "dist/styles.css".to_string(),
            file_type: destack_source::FileType::Css,
            text: "body{color:red}\n".to_string(),
        },
        "linked chunked stylesheet asset",
    );
}

/// Group entry-private static chains while extracting one shared static leaf chunk.
#[test]
fn test_plans_chunked_js_target_with_shared_static_subgraph() {
    // keep one shared leaf separate while leaving private chains with their entries
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [_shared, _feature_a, _feature_b, app, dashboard] = test.add_modules([
        (
            "shared-leaf.ts",
            js(r#"
export const sharedLeaf = 1;
"#),
        ),
        (
            "feature-a.ts",
            js(r#"
import { sharedLeaf } from './shared-leaf.ts';

export const featureA = sharedLeaf;
"#),
        ),
        (
            "feature-b.ts",
            js(r#"
import { sharedLeaf } from './shared-leaf.ts';

export const featureB = sharedLeaf;
"#),
        ),
        (
            "app.ts",
            js(r#"
import { featureA } from './feature-a.ts';

export const appValue = featureA;
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { featureB } from './feature-b.ts';

export const dashboardValue = featureB;
"#),
        ),
    ]);

    // plan one shared leaf chunk and two grouped entry chunks
    let chunks = test.plan_chunked_js_target_with(&[app, dashboard], "js", |_| {});

    assert_eq!(
        chunks,
        vec![
            planned_output("shared-leaf", OutputKind::Shared, &["shared-leaf.ts"]),
            planned_output("app", OutputKind::Entry, &["feature-a.ts", "app.ts"])
                .static_dependencies(&["shared-leaf"]),
            planned_output(
                "dashboard",
                OutputKind::Entry,
                &["feature-b.ts", "dashboard.ts"],
            )
            .static_dependencies(&["shared-leaf"]),
        ],
    );
}

/// Emit grouped entry chunks for one shared static subgraph.
#[test]
fn test_links_chunked_js_target_with_shared_static_subgraph() {
    // split out the shared leaf while keeping each private chain inside its entry chunk
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [shared, _feature_a, _feature_b, app, dashboard] = test.add_modules([
        (
            "shared-leaf.ts",
            js(r#"
export const sharedLeaf = 1;
"#),
        ),
        (
            "feature-a.ts",
            js(r#"
import { sharedLeaf } from './shared-leaf.ts';

export const featureA = sharedLeaf;
"#),
        ),
        (
            "feature-b.ts",
            js(r#"
import { sharedLeaf } from './shared-leaf.ts';

export const featureB = sharedLeaf;
"#),
        ),
        (
            "app.ts",
            js(r#"
import { featureA } from './feature-a.ts';

export const appValue = featureA;
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { featureB } from './feature-b.ts';

export const dashboardValue = featureB;
"#),
        ),
    ]);

    // emit one shared leaf chunk and two grouped entry chunks
    let linked = test.linked_chunked_js_target_with(&[app, dashboard], "js", |_| {});
    let app_path = test.module_relative_path(app);
    let dashboard_path = test.module_relative_path(dashboard);
    let shared_path = test.module_relative_path(shared);
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/shared-leaf-29dc937d.js".to_string(),
                "dist/app.js".to_string(),
                "dist/dashboard.js".to_string(),
            ],
            TargetOutputName::Maps => vec![
                "dist/shared-leaf-29dc937d.js.map".to_string(),
                "dist/app.js.map".to_string(),
                "dist/dashboard.js.map".to_string(),
            ],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .imports(&["./shared-leaf-29dc937d.js"])
                .into(),
            manifest_map("app.js.map"),
            manifest_chunk("dashboard.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["./shared-leaf-29dc937d.js"])
                .into(),
            manifest_map("dashboard.js.map"),
            manifest_chunk("shared-leaf-29dc937d.js", "shared-leaf")
                .input(&shared_path)
                .shared()
                .into(),
            manifest_map("shared-leaf-29dc937d.js.map"),
        ]),
        vec![
            script_output(
                "dist/shared-leaf-29dc937d.js",
                &js_output(
                    r#"
export const sharedLeaf = 1;
//# sourceMappingURL=./shared-leaf-29dc937d.js.map
"#,
                ),
            ),
            script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { sharedLeaf } from "./shared-leaf-29dc937d.js";
export const featureA = sharedLeaf;

export const appValue = featureA;
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
            script_output(
                "dist/dashboard.js",
                &js_output(
                    r#"
import { sharedLeaf } from "./shared-leaf-29dc937d.js";
export const featureB = sharedLeaf;

export const dashboardValue = featureB;
//# sourceMappingURL=./dashboard.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Honor one explicit manual chunk name for one linked module in chunked mode.
#[test]
fn test_links_chunked_js_target_with_manual_chunk_name() {
    // place one shared module into a manually named vendor chunk
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [common, app, dashboard] = test.add_modules([
        (
            "common.ts",
            js(r#"
export const commonValue = 1;
"#),
        ),
        (
            "app.ts",
            js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { commonValue } from './common.ts';

export const dashboardValue = commonValue;
"#),
        ),
    ]);

    // rewrite both entries through the manual vendor chunk
    let common_path = test.module_relative_path(common);
    let linked = test.linked_chunked_js_target_with(&[app, dashboard], "js", |target| {
        target
            .manual_chunks
            .insert("vendor".to_string(), vec![common_path.clone()]);
    });
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/vendor-187cb3ea.js".to_string(),
                "dist/app.js".to_string(),
                "dist/dashboard.js".to_string(),
            ],
            TargetOutputName::Maps => vec![
                "dist/vendor-187cb3ea.js.map".to_string(),
                "dist/app.js.map".to_string(),
                "dist/dashboard.js.map".to_string(),
            ],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&test.module_relative_path(app))
                .entry()
                .imports(&["./vendor-187cb3ea.js"])
                .into(),
            manifest_map("app.js.map"),
            manifest_chunk("dashboard.js", "dashboard")
                .input(&test.module_relative_path(dashboard))
                .entry()
                .imports(&["./vendor-187cb3ea.js"])
                .into(),
            manifest_map("dashboard.js.map"),
            manifest_chunk("vendor-187cb3ea.js", "vendor")
                .input(&common_path)
                .shared()
                .into(),
            manifest_map("vendor-187cb3ea.js.map"),
        ]),
        vec![
            script_output(
                "dist/vendor-187cb3ea.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./vendor-187cb3ea.js.map
"#,
                ),
            ),
            script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { commonValue } from "./vendor-187cb3ea.js";
export const appValue = commonValue;
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
            script_output(
                "dist/dashboard.js",
                &js_output(
                    r#"
import { commonValue } from "./vendor-187cb3ea.js";
export const dashboardValue = commonValue;
//# sourceMappingURL=./dashboard.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Group multiple modules into one explicit manual chunk and strip same-chunk imports.
#[test]
fn test_links_chunked_js_target_with_multi_module_manual_chunk() {
    // merge two shared modules into one vendor chunk and collapse their internal import
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [common, helper, app] = test.add_modules([
        (
            "common.ts",
            js(r#"
export const commonValue = 1;
"#),
        ),
        (
            "helper.ts",
            js(r#"
import { commonValue } from './common.ts';

export const helperValue = commonValue;
"#),
        ),
        (
            "app.ts",
            js(r#"
import { helperValue } from './helper.ts';

export const appValue = helperValue;
"#),
        ),
    ]);

    // keep both shared modules inside the manual vendor chunk plan
    let common_path = test.module_relative_path(common);
    let helper_path = test.module_relative_path(helper);
    let chunks = test.plan_chunked_js_target_with(&[app], "js", |target| {
        target.manual_chunks.insert(
            "vendor".to_string(),
            vec![common_path.clone(), helper_path.clone()],
        );
    });

    assert_eq!(
        chunks,
        vec![
            planned_output("vendor", OutputKind::Shared, &["common.ts", "helper.ts"]),
            planned_output("app", OutputKind::Entry, &["app.ts"]).static_dependencies(&["vendor"]),
        ],
    );

    // emit the vendor chunk without any internal same-chunk import
    let package_id = test.program.module_descriptor(app).package_id;
    let output = test.link_chunked_js_target_with(&[app], "js", |target| {
        target
            .manual_chunks
            .insert("vendor".to_string(), vec![common_path, helper_path]);
    });

    // find the emitted hashed vendor chunk
    let linked = test.linked_chunked_script_target(package_id, &output);
    let vendor_path = linked
        .modules
        .keys()
        .find(|path| path.starts_with("dist/vendor-") && path.ends_with(".js"))
        .cloned()
        .unwrap_or_else(|| panic!("missing hashed vendor chunk"));
    let vendor_output = linked
        .modules
        .get(&vendor_path)
        .unwrap_or_else(|| panic!("missing linked vendor chunk '{vendor_path}'"));
    let vendor_map_path = vendor_path.strip_prefix("dist/").unwrap_or(&vendor_path);
    let vendor_map_path = vendor_map_path.replace(".js", ".js.map");

    // keep the same chunk content regardless of the content-derived hash
    assert_eq!(
        vendor_output.text,
        js_output(&format!(
            r#"
export const commonValue = 1;

export const helperValue = commonValue;
//# sourceMappingURL=./{vendor_map_path}
"#,
        )),
    );
}
