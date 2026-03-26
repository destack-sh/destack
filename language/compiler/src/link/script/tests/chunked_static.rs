use crate::link::ScriptOutputKind;

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
        ("common.ts", js(r#"export const commonValue = 1;"#)),
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
                .imports(&["./common.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("common.js", "common")
                .input(&common_path)
                .shared()
                .into(),
            expected_manifest_map("common.js.map"),
            expected_manifest_chunk("dashboard.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["./common.js"])
                .into(),
            expected_manifest_map("dashboard.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/common.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./common.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/dashboard.js",
                &js_output(
                    r#"
import { commonValue } from "./common.js";
export const dashboardValue = commonValue;
//# sourceMappingURL=./dashboard.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { commonValue } from "./common.js";
export const appValue = commonValue;
//# sourceMappingURL=./app.js.map
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
        ("common.ts", js(r#"export const commonValue = 1;"#)),
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
            expected_planned_output("common", ScriptOutputKind::Shared, &["common.ts"],),
            expected_planned_output("dashboard", ScriptOutputKind::Entry, &["dashboard.ts"])
                .static_dependencies(&["common"]),
            expected_planned_output("app", ScriptOutputKind::Entry, &["app.ts"])
                .static_dependencies(&["common"]),
        ],
    );
}

/// Group entry-private static chains while extracting one shared static leaf chunk.
#[test]
fn test_plans_chunked_js_target_with_shared_static_subgraph() {
    // keep one shared leaf separate while leaving private chains with their entries
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [_shared, _feature_a, _feature_b, app, dashboard] = test.add_modules([
        ("shared-leaf.ts", js(r#"export const sharedLeaf = 1;"#)),
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
            expected_planned_output("shared-leaf", ScriptOutputKind::Shared, &["shared-leaf.ts"]),
            expected_planned_output(
                "dashboard",
                ScriptOutputKind::Entry,
                &["feature-b.ts", "dashboard.ts"],
            )
            .static_dependencies(&["shared-leaf"]),
            expected_planned_output("app", ScriptOutputKind::Entry, &["feature-a.ts", "app.ts"],)
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
        ("shared-leaf.ts", js(r#"export const sharedLeaf = 1;"#)),
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
                .imports(&["./shared-leaf.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("dashboard.js", "dashboard")
                .input(&dashboard_path)
                .entry()
                .imports(&["./shared-leaf.js"])
                .into(),
            expected_manifest_map("dashboard.js.map"),
            expected_manifest_chunk("shared-leaf.js", "shared-leaf")
                .input(&shared_path)
                .shared()
                .into(),
            expected_manifest_map("shared-leaf.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/shared-leaf.js",
                &js_output(
                    r#"
export const sharedLeaf = 1;
//# sourceMappingURL=./shared-leaf.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/dashboard.js",
                &js_output(
                    r#"
import { sharedLeaf } from "./shared-leaf.js";
export const featureB = sharedLeaf;

export const dashboardValue = featureB;
//# sourceMappingURL=./dashboard.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { sharedLeaf } from "./shared-leaf.js";
export const featureA = sharedLeaf;

export const appValue = featureA;
//# sourceMappingURL=./app.js.map
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
        ("common.ts", js(r#"export const commonValue = 1;"#)),
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
            .bundle
            .manual_chunks
            .insert("vendor".to_string(), vec![common_path.clone()]);
    });
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&test.module_relative_path(app))
                .entry()
                .imports(&["./vendor.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("dashboard.js", "dashboard")
                .input(&test.module_relative_path(dashboard))
                .entry()
                .imports(&["./vendor.js"])
                .into(),
            expected_manifest_map("dashboard.js.map"),
            expected_manifest_chunk("vendor.js", "vendor")
                .input(&common_path)
                .shared()
                .into(),
            expected_manifest_map("vendor.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/vendor.js",
                &js_output(
                    r#"
export const commonValue = 1;
//# sourceMappingURL=./vendor.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/dashboard.js",
                &js_output(
                    r#"
import { commonValue } from "./vendor.js";
export const dashboardValue = commonValue;
//# sourceMappingURL=./dashboard.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
import { commonValue } from "./vendor.js";
export const appValue = commonValue;
//# sourceMappingURL=./app.js.map
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
        ("common.ts", js(r#"export const commonValue = 1;"#)),
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
        target.bundle.manual_chunks.insert(
            "vendor".to_string(),
            vec![common_path.clone(), helper_path.clone()],
        );
    });

    assert_eq!(
        chunks,
        vec![
            expected_planned_output(
                "vendor",
                ScriptOutputKind::Shared,
                &["common.ts", "helper.ts"],
            ),
            expected_planned_output("app", ScriptOutputKind::Entry, &["app.ts"])
                .static_dependencies(&["vendor"]),
        ],
    );

    // emit the vendor chunk without any internal same-chunk import
    let package_id = test.program.modules.get(app).package_id;
    let output = test.link_chunked_js_target_with(&[app], "js", |target| {
        target
            .bundle
            .manual_chunks
            .insert("vendor".to_string(), vec![common_path, helper_path]);
    });

    test.assert_script_module_output(
        package_id,
        &output,
        "dist/vendor.js",
        &js_output(
            r#"
export const commonValue = 1;

export const helperValue = commonValue;
//# sourceMappingURL=./vendor.js.map
"#,
        ),
        "multi module manual chunk output",
    );
}
