use super::super::plan::OutputKind;
use super::{
    TestProgram, chunked_script_target, js, js_output, manifest, manifest_chunk, manifest_map,
    map_output, planned_output, script_output, source_map,
};
use destack_artifact::TargetOutputName;
use indexmap::indexmap;

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
            planned_output("feature", OutputKind::DynamicEntry, &["feature.ts"]),
            planned_output("app", OutputKind::Entry, &["app.ts"])
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
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/feature-9cbf4dc4.js".to_string(),
                "dist/app.js".to_string(),
            ],
            TargetOutputName::Maps => vec![
                "dist/feature-9cbf4dc4.js.map".to_string(),
                "dist/app.js.map".to_string(),
            ],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .dynamic_imports(&["./feature-9cbf4dc4.js"])
                .into(),
            manifest_map("app.js.map"),
            manifest_chunk("feature-9cbf4dc4.js", "feature")
                .input(&feature_path)
                .dynamic_entry()
                .into(),
            manifest_map("feature-9cbf4dc4.js.map"),
        ]),
        vec![
            script_output(
                "dist/feature-9cbf4dc4.js",
                &js_output(
                    r#"
export const featureValue = 1;
//# sourceMappingURL=./feature-9cbf4dc4.js.map
"#,
                ),
            ),
            script_output(
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

#[test]
fn test_emits_exact_chunked_source_maps_for_dynamic_import() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let _feature = test.add_module("feature.ts", &js(r#"export const featureValue = 1;"#));
    let app = test.add_module(
        "app.ts",
        &js(r#"export const featurePromise = import("./feature.ts");"#),
    );

    let package_id = test.program.module_descriptor(app).package_id;
    let output = test.link_chunked_js_target_with(&[app], "js", |_| {});

    test.assert_json_output_at_path(
        package_id,
        &output,
        TargetOutputName::Maps,
        &map_output(
            "dist/app.js.map",
            source_map(&["../app.ts"], "AAAA,aAAa,cAAc,GAAG,OAAO,uBAAc,CAAC;"),
        ),
        "linked app chunk source map",
    );
    test.assert_json_output_at_path(
        package_id,
        &output,
        TargetOutputName::Maps,
        &map_output(
            "dist/feature-9cbf4dc4.js.map",
            source_map(&["../feature.ts"], "AAAA,aAAa,YAAY,GAAG,CAAC;"),
        ),
        "linked feature chunk source map",
    );
}

/// Preserve relative chunk imports under one configured public path.
#[test]
fn test_links_chunked_js_target_with_public_path() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let [feature, app] = test.add_modules([
        ("feature.ts", js(r#"export const featureValue = 1;"#)),
        (
            "app.ts",
            js(r#"export const featurePromise = import("./feature.ts");"#),
        ),
    ]);

    let linked = test.linked_chunked_js_target_with(&[app], "js", |target| {
        target.bundle_output.public_path = Some("/assets".to_string());
        target.bundle_output.sourcemap = None;
    });
    let app_path = test.module_relative_path(app);
    let feature_path = test.module_relative_path(feature);
    let expected = chunked_script_target(
        indexmap! {
            TargetOutputName::Module => vec![
                "dist/feature-9cbf4dc4.js".to_string(),
                "dist/app.js".to_string(),
            ],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        manifest(vec![
            manifest_chunk("app.js", "app")
                .input(&app_path)
                .entry()
                .dynamic_imports(&["./feature-9cbf4dc4.js"])
                .into(),
            manifest_chunk("feature-9cbf4dc4.js", "feature")
                .input(&feature_path)
                .dynamic_entry()
                .into(),
        ]),
        vec![
            script_output(
                "dist/feature-9cbf4dc4.js",
                &js_output(
                    r#"
export const featureValue = 1;
"#,
                ),
            ),
            script_output(
                "dist/app.js",
                &js_output(
                    r#"
export const featurePromise = import("./feature-9cbf4dc4.js");
"#,
                ),
            ),
        ],
    );

    test.assert_linked_chunked_script_target(&linked, &expected);
}
