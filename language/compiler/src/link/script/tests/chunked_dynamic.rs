use std::path::PathBuf;

use destack_artifact::ArtifactKey;
use destack_source::TargetId;
use destack_workspace::{BundleMode, TargetDiscovery};

use super::super::OutputKind;
use super::{
    ExpectedDiagnostic, TestProgram, expected_chunked_script_target, expected_manifest,
    expected_manifest_chunk, expected_manifest_map, expected_planned_output,
    expected_script_output, js, js_output,
};

/// Plan one lazy entry output for one bundled dynamic import.
#[test]
fn test_plans_chunked_js_target_with_dynamic_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [_feature, app] = test.add_modules([
        (
            "feature.ts",
            js(r#"
export const featureValue = 1;
"#),
        ),
        (
            "app.ts",
            js(r#"
export const featurePromise = import("./feature.ts");
"#),
        ),
    ]);

    let outputs = test.plan_chunked_js_target_with(&[app], "js", |_| {});

    assert_eq!(
        outputs,
        vec![
            expected_planned_output("feature", OutputKind::DynamicEntry, &["feature.ts"]),
            expected_planned_output("app", OutputKind::Entry, &["app.ts"])
                .dynamic_dependencies(&["feature"]),
        ],
    );
}

/// Emit one lazy entry chunk for one bundled dynamic import.
#[test]
fn test_links_chunked_js_target_with_dynamic_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [feature, app] = test.add_modules([
        (
            "feature.ts",
            js(r#"
export const featureValue = 1;
"#),
        ),
        (
            "app.ts",
            js(r#"
export const featurePromise = import("./feature.ts");
"#),
        ),
    ]);

    let linked = test.linked_chunked_js_target_with(&[app], "js", |_| {});
    let app_path = test.module_relative_path(app);
    let feature_path = test.module_relative_path(feature);
    let expected = expected_chunked_script_target(
        expected_manifest(vec![
            expected_manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .dynamic_imports(&["./feature-9cbf4dc4.js"])
                .into(),
            expected_manifest_map("app.js.map"),
            expected_manifest_chunk("feature-9cbf4dc4.js", "feature")
                .input(&feature_path)
                .dynamic_entry()
                .into(),
            expected_manifest_map("feature-9cbf4dc4.js.map"),
        ]),
        vec![
            expected_script_output(
                "dist/feature-9cbf4dc4.js",
                &js_output(
                    r#"
export const featureValue = 1;
//# sourceMappingURL=./feature-9cbf4dc4.js.map
"#,
                ),
            ),
            expected_script_output(
                "dist/app.js",
                &js_output(
                    r#"
export const featurePromise = import("./feature-9cbf4dc4.js");
//# sourceMappingURL=./app.js.map
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}

/// Reject one dynamic import that collapses into the same output.
#[test]
fn test_rejects_chunked_same_output_dynamic_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dependency = test.add_module(
        "dependency.ts",
        &js(r#"
export const value = 1;
"#),
    );
    let app = test.add_module(
        "app.ts",
        &js(r#"
export const dependencyPromise = import("./dependency.ts");
"#),
    );
    let app_path = test.module_relative_path(app);
    let dependency_path = test.module_relative_path(dependency);

    test.configure_target(app, "js", |target| {
        target.discovery = TargetDiscovery::Entry;
        target.entry = vec![PathBuf::from("app.ts")];
        target.out_dir = PathBuf::from("dist");
        target.assembly = BundleMode::Chunked;
        target.manual_chunks.insert(
            "vendor".to_string(),
            vec![app_path.clone(), dependency_path.clone()],
        );
    });

    let package_id = test.program.modules.get(app).package_id;
    let target_id = TargetId::new(package_id, "js");
    test.run(ArtifactKey::package_output(package_id, target_id));

    test.check_exact_diagnostics(&[ExpectedDiagnostic {
        code: "EK101".to_string(),
        message:
            "invalid target: js: bundled same-output dynamic imports are not supported yet in 'js'"
                .to_string(),
    }]);
}
