use std::path::PathBuf;

use destack_artifact::ArtifactKey;
use destack_workspace::{BundleMode, TargetDiscovery, TargetId};

use crate::link::ScriptOutputKind;

use super::{
    ExpectedDiagnostic, TestProgram, expected_chunked_script_target, expected_manifest,
    expected_manifest_chunk, expected_manifest_map, expected_planned_output,
    expected_script_output, js, js_output,
};

/// Rewrite bundled local dynamic imports to chunk output paths in chunked mode.
#[test]
fn test_links_chunked_js_target_with_bundled_dynamic_import() {
    // split one dynamic import into an entry chunk and one lazy chunk
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [feature, app] = test.add_modules([
        ("feature.ts", js(r#"export const featureValue = 1;"#)),
        (
            "app.ts",
            js(r#"export const dependencyPromise = import('./feature.ts');"#),
        ),
    ]);

    // emit one entry chunk that dynamically imports one lazy chunk
    let linked = test.linked_chunked_js_target_with(&[app], "js", |_| {});
    let app_path = test.module_relative_path(app);
    let feature_path = test.module_relative_path(feature);
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .dynamic_imports(&["./feature.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("feature.js", "feature")
                .input(&feature_path)
                .dynamic_entry()
                .into(),
            expected_manifest_map("feature.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/feature.js",
                &js_output(
                    r#"
export const featureValue = 1;
//# sourceMappingURL=./feature.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
export const dependencyPromise = import("./feature.js");
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Plan stable chunk graph facts for one bundled dynamic import boundary.
#[test]
fn test_plans_chunked_js_target_with_bundled_dynamic_import() {
    // treat the imported module as one dynamic entry output
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [app, _feature] = test.add_modules([
        (
            "app.ts",
            js(r#"export const dependencyPromise = import('./feature.ts');"#),
        ),
        ("feature.ts", js(r#"export const featureValue = 1;"#)),
    ]);

    // plan one entry output that points at one dynamic entry output
    let chunks = test.plan_chunked_js_target_with(&[app], "js", |_| {});

    assert_eq!(
        chunks,
        vec![
            expected_planned_output("feature", ScriptOutputKind::DynamicEntry, &["feature.ts"],),
            expected_planned_output("app", ScriptOutputKind::Entry, &["app.ts"])
                .dynamic_dependencies(&["feature"]),
        ],
    );
}

/// Group lazy-only static dependencies into one shared chunk across dynamic entries.
#[test]
fn test_plans_chunked_js_target_with_shared_dynamic_subgraph() {
    // keep lazy shared code separate from two lazy entry chunks
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [app, _shared, _feature_a, _feature_b] = test.add_modules([
        (
            "app.ts",
            js(r#"
export const featureA = import('./feature-a.ts');
export const featureB = import('./feature-b.ts');
"#),
        ),
        ("shared-lazy.ts", js(r#"export const sharedLazy = 1;"#)),
        (
            "feature-a.ts",
            js(r#"
import { sharedLazy } from './shared-lazy.ts';

export const featureA = sharedLazy;
"#),
        ),
        (
            "feature-b.ts",
            js(r#"
import { sharedLazy } from './shared-lazy.ts';

export const featureB = sharedLazy;
"#),
        ),
    ]);

    // plan one shared lazy chunk and two dynamic entry chunks
    let chunks = test.plan_chunked_js_target_with(&[app], "js", |_| {});

    assert_eq!(
        chunks,
        vec![
            expected_planned_output("shared-lazy", ScriptOutputKind::Shared, &["shared-lazy.ts"]),
            expected_planned_output(
                "feature-a",
                ScriptOutputKind::DynamicEntry,
                &["feature-a.ts"],
            )
            .static_dependencies(&["shared-lazy"]),
            expected_planned_output(
                "feature-b",
                ScriptOutputKind::DynamicEntry,
                &["feature-b.ts"],
            )
            .static_dependencies(&["shared-lazy"]),
            expected_planned_output("app", ScriptOutputKind::Entry, &["app.ts"])
                .dynamic_dependencies(&["feature-a", "feature-b"]),
        ],
    );
}

/// Emit one shared lazy chunk across multiple dynamic entries.
#[test]
fn test_links_chunked_js_target_with_shared_dynamic_subgraph() {
    // reuse one lazy shared chunk across two lazy entry chunks
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [app, shared, feature_a, feature_b] = test.add_modules([
        (
            "app.ts",
            js(r#"
export const featureA = import('./feature-a.ts');
export const featureB = import('./feature-b.ts');
"#),
        ),
        ("shared-lazy.ts", js(r#"export const sharedLazy = 1;"#)),
        (
            "feature-a.ts",
            js(r#"
import { sharedLazy } from './shared-lazy.ts';

export const featureA = sharedLazy;
"#),
        ),
        (
            "feature-b.ts",
            js(r#"
import { sharedLazy } from './shared-lazy.ts';

export const featureB = sharedLazy;
"#),
        ),
    ]);

    // emit one root entry, two lazy entry chunks, and one shared lazy chunk
    let linked = test.linked_chunked_js_target_with(&[app], "js", |_| {});
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&test.module_relative_path(app))
                .entry()
                .dynamic_imports(&["./feature-a.js", "./feature-b.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("feature-a.js", "feature-a")
                .input(&test.module_relative_path(feature_a))
                .dynamic_entry()
                .imports(&["./shared-lazy.js"])
                .into(),
            expected_manifest_map("feature-a.js.map"),
            expected_manifest_chunk("feature-b.js", "feature-b")
                .input(&test.module_relative_path(feature_b))
                .dynamic_entry()
                .imports(&["./shared-lazy.js"])
                .into(),
            expected_manifest_map("feature-b.js.map"),
            expected_manifest_chunk("shared-lazy.js", "shared-lazy")
                .input(&test.module_relative_path(shared))
                .shared()
                .into(),
            expected_manifest_map("shared-lazy.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/shared-lazy.js",
                &js_output(
                    r#"
export const sharedLazy = 1;
//# sourceMappingURL=./shared-lazy.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/feature-a.js",
                &js_output(
                    r#"
import { sharedLazy } from "./shared-lazy.js";
export const featureA = sharedLazy;
//# sourceMappingURL=./feature-a.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/feature-b.js",
                &js_output(
                    r#"
import { sharedLazy } from "./shared-lazy.js";
export const featureB = sharedLazy;
//# sourceMappingURL=./feature-b.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
export const featureA = import("./feature-a.js");
export const featureB = import("./feature-b.js");
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Keep statically reachable modules out of the dynamic-entry role.
#[test]
fn test_plans_chunked_js_target_with_static_and_dynamic_access_to_same_module() {
    // keep one eagerly imported module shared even if it is also dynamically imported
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let app = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const eagerValue = commonValue;
export const lazyValue = import('./common.ts');
"#),
    );

    // plan one shared output instead of promoting it to a dynamic entry
    let chunks = test.plan_chunked_js_target_with(&[app], "js", |_| {});

    assert_eq!(
        chunks,
        vec![
            expected_planned_output("common", ScriptOutputKind::Shared, &["common.ts"],),
            expected_planned_output("app", ScriptOutputKind::Entry, &["app.ts"])
                .static_dependencies(&["common"])
                .dynamic_dependencies(&["common"]),
        ],
    );
}

/// Keep transitive eager helpers out of lazy entry outputs.
#[test]
fn test_plans_chunked_js_target_with_static_shared_helper_and_lazy_consumer() {
    // keep one helper shared across eager entries even when one lazy chunk also uses it
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [app, dashboard, _feature, _helper] = test.add_modules([
        (
            "app.ts",
            js(r#"
import { helperValue } from './helper.ts';

export const appValue = helperValue;
export const featurePromise = import('./feature.ts');
"#),
        ),
        (
            "dashboard.ts",
            js(r#"
import { helperValue } from './helper.ts';

export const dashboardValue = helperValue;
"#),
        ),
        (
            "feature.ts",
            js(r#"
import { helperValue } from './helper.ts';

export const featureValue = helperValue;
"#),
        ),
        ("helper.ts", js(r#"export const helperValue = 1;"#)),
    ]);

    // keep the helper in one eager shared chunk and the feature in one lazy chunk
    let chunks = test.plan_chunked_js_target_with(&[app, dashboard], "js", |_| {});

    assert_eq!(
        chunks,
        vec![
            expected_planned_output("helper", ScriptOutputKind::Shared, &["helper.ts"]),
            expected_planned_output("feature", ScriptOutputKind::DynamicEntry, &["feature.ts"])
                .static_dependencies(&["helper"]),
            expected_planned_output("dashboard", ScriptOutputKind::Entry, &["dashboard.ts"])
                .static_dependencies(&["helper"]),
            expected_planned_output("app", ScriptOutputKind::Entry, &["app.ts"])
                .static_dependencies(&["helper"])
                .dynamic_dependencies(&["feature"]),
        ],
    );
}

/// Reject same-output dynamic imports until output-local namespace bridging exists.
#[test]
fn test_rejects_chunked_same_chunk_dynamic_import() {
    // reject one manual chunk that would collapse a dynamic import into the same output
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("dependency.ts", &js(r#"export const value = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"export const dependencyPromise = import('./dependency.ts');"#),
    );
    let main_path = test.module_relative_path(main);
    let dep_path = test.module_relative_path(dep);

    // configure one invalid chunked target
    test.configure_target(main, "js", |target| {
        // chunked linking should reject same-output dynamic imports loudly
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("app.ts")];
        target.out_dir = PathBuf::from("dist");
        target.bundle.mode = BundleMode::Chunked;
        target.bundle.manual_chunks.insert(
            "vendor".to_string(),
            vec![main_path.clone(), dep_path.clone()],
        );
    });

    let package_id = test.program.modules.get(main).package_id;
    let target_id = TargetId::new(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    // report one exact linker diagnostic
    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message:
            "invalid target: js: bundled same-output dynamic imports are not supported yet in 'js'"
                .to_string(),
    }]);
}
