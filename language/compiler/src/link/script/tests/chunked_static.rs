use destack_artifact::ArtifactKey;
use destack_source::TargetId;
use destack_workspace::{BundleMode, TargetDiscovery};

use super::super::OutputKind;
use super::{
    TestProgram, expected_chunked_script_target, expected_manifest, expected_manifest_chunk,
    expected_manifest_map, expected_planned_output, expected_script_output, js, js_output,
};

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
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .imports(&["./common-a9ad2188.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("common-a9ad2188.js", "common")
                .input(&common_path)
                .shared()
                .into(),
            expected_manifest_map("common-a9ad2188.js.map"),
            expected_manifest_chunk("dashboard.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["./common-a9ad2188.js"])
                .into(),
            expected_manifest_map("dashboard.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/common-a9ad2188.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./common-a9ad2188.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { commonValue } from "./common-a9ad2188.js";
export const appValue = commonValue;
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
            expected_script_output(
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
            expected_planned_output("common", OutputKind::Shared, &["common.ts"],),
            expected_planned_output("app", OutputKind::Entry, &["app.ts"])
                .static_dependencies(&["common"]),
            expected_planned_output("dashboard", OutputKind::Entry, &["dashboard.ts"])
                .static_dependencies(&["common"]),
        ],
    );
}

/// Reject chunk file-name templates that collapse distinct shared outputs.
#[test]
fn test_rejects_chunked_shared_output_path_collisions() {
    // keep three distinct shared chunks so a fixed file name collides
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [_dep1, _dep2, _dep3, main1, _main2, _main3] = test.add_modules([
        (
            "dep1.ts",
            js(r#"
export const dep1 = "dep1";
"#),
        ),
        (
            "dep2.ts",
            js(r#"
export const dep2 = "dep2";
"#),
        ),
        (
            "dep3.ts",
            js(r#"
export const dep3 = "dep3";
"#),
        ),
        (
            "main1.ts",
            js(r#"
import { dep1 } from "./dep1.ts";
import { dep2 } from "./dep2.ts";

export const main1 = [dep1, dep2];
"#),
        ),
        (
            "main2.ts",
            js(r#"
import { dep2 } from "./dep2.ts";
import { dep3 } from "./dep3.ts";

export const main2 = [dep2, dep3];
"#),
        ),
        (
            "main3.ts",
            js(r#"
import { dep1 } from "./dep1.ts";
import { dep3 } from "./dep3.ts";

export const main3 = [dep1, dep3];
"#),
        ),
    ]);

    // force all shared chunks onto the same emitted path
    test.configure_target(main1, "js", |target| {
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec!["main1.ts".into(), "main2.ts".into(), "main3.ts".into()];
        target.out_dir = "dist".into();
        target.assembly = BundleMode::Chunked;
        target.bundle_output.chunk_file_names = Some("chunks/chunk.js".to_string());
    });

    let package_id = test.program.modules.get(main1).package_id;
    let target_id = TargetId::new(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // report one exact linker failure shape
    let diagnostics = test.program.diagnostics.collect();
    let diagnostics = diagnostics.iter();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "EK101");
    assert!(
        diagnostics[0]
            .message
            .contains("multiple script outputs resolve to the same emitted path"),
    );
    assert!(diagnostics[0].message.contains("chunks/chunk.js"));
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
            expected_planned_output("shared-leaf", OutputKind::Shared, &["shared-leaf.ts"]),
            expected_planned_output("app", OutputKind::Entry, &["feature-a.ts", "app.ts"],)
                .static_dependencies(&["shared-leaf"]),
            expected_planned_output(
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
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .imports(&["./shared-leaf-29dc937d.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("dashboard.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["./shared-leaf-29dc937d.js"])
                .into(),
            expected_manifest_map("dashboard.js.map"),
            expected_manifest_chunk("shared-leaf-29dc937d.js", "shared-leaf")
                .input(&shared_path)
                .shared()
                .into(),
            expected_manifest_map("shared-leaf-29dc937d.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/shared-leaf-29dc937d.js",
                &js_output(
                    r#"
export const sharedLeaf = 1;
//# sourceMappingURL=./shared-leaf-29dc937d.js.map
"#,
                ),
            ),
            expected_script_output(
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
            expected_script_output(
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
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&test.module_relative_path(app))
                .entry()
                .imports(&["./vendor-187cb3ea.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("dashboard.js", "dashboard")
                .input(&test.module_relative_path(dashboard))
                .entry()
                .imports(&["./vendor-187cb3ea.js"])
                .into(),
            expected_manifest_map("dashboard.js.map"),
            expected_manifest_chunk("vendor-187cb3ea.js", "vendor")
                .input(&common_path)
                .shared()
                .into(),
            expected_manifest_map("vendor-187cb3ea.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/vendor-187cb3ea.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./vendor-187cb3ea.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { commonValue } from "./vendor-187cb3ea.js";
export const appValue = commonValue;
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
            expected_script_output(
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
            expected_planned_output("vendor", OutputKind::Shared, &["common.ts", "helper.ts"],),
            expected_planned_output("app", OutputKind::Entry, &["app.ts"])
                .static_dependencies(&["vendor"]),
        ],
    );

    // emit the vendor chunk without any internal same-chunk import
    let package_id = test.program.modules.get(app).package_id;
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
