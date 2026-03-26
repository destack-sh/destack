use std::path::PathBuf;

use destack_artifact::{PackageAssembly, TargetOutputName};
use indexmap::indexmap;

use super::{
    LinkedScriptTarget, TestProgram, expected_inline_source_map_reference, expected_linked_entry,
    expected_linked_entry_with_source_map_reference, expected_linked_map, expected_manifest,
    expected_manifest_chunk, expected_manifest_map, expected_source_map, js, js_output,
};

/// The exact precise mappings for one plain single-file bundle.
const SINGLE_FILE_SOURCE_MAP_MAPPINGS: &str =
    "AAAA,aAAa,WAAW,GAAG,CAAC;;ACE5B,aAAa,QAAQ,GAAG,WAAW;";

/// The exact precise mappings for one banner and footer wrapped single-file bundle.
const SINGLE_FILE_BANNER_FOOTER_SOURCE_MAP_MAPPINGS: &str =
    ";AAAA,aAAa,WAAW,GAAG,CAAC;;ACE5B,aAAa,QAAQ,GAAG,WAAW;;";

/// Link a single-file script target over the full reachable graph.
#[test]
fn test_links_single_file_js_target_over_reachable_modules() {
    // bundle one entry through one shared local module
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // preserve both input modules in the linked source map
    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = expected_source_map(&[&dep_path, &main_path], SINGLE_FILE_SOURCE_MAP_MAPPINGS);

    // emit one exact bundled target
    let linked = test.link_single_file_js_target(main, "js");
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Maps => vec!["dist/js.js.map".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: expected_linked_entry_with_source_map_reference(
            &js_output(
                r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
            ),
            "./js.js.map",
        ),
        manifest: expected_manifest(vec![
            expected_manifest_chunk("dist/js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
            expected_manifest_map("dist/js.js.map"),
        ]),
        map: Some(expected_linked_map(map)),
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Record the script assembly in the target manifest.
#[test]
fn test_records_script_assembly_in_manifest() {
    // keep the emitted manifest aligned with the bundled entry surface
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // describe the same single bundled entry in both places
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target(main, "js");
    let expected = expected_manifest(vec![
        expected_manifest_chunk("dist/js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        expected_manifest_map("dist/js.js.map"),
    ]);
    let expected_entry = expected_linked_entry_with_source_map_reference(
        &js_output(
            r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
        ),
        "./js.js.map",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected);
}

/// Retain unresolved external package dependencies in the linked entry and manifest.
#[test]
fn test_retains_unresolved_external_dependency_in_single_file_manifest_and_entry() {
    // leave never-bundled package imports untouched
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module("main.ts", &js(r#"export * from 'react';"#));

    // report the external import in both the entry text and the manifest
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle.dependencies.never_bundle = vec!["react".to_string()];
    });
    let expected_manifest = expected_manifest(vec![
        expected_manifest_chunk("dist/js.js", "js")
            .input(&main_path)
            .entry()
            .imports(&["react"])
            .into(),
        expected_manifest_map("dist/js.js.map"),
    ]);
    let expected_entry = expected_linked_entry_with_source_map_reference(
        "export * from \"react\";\n",
        "./js.js.map",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
}

/// Emit one external source map reference for bundled single-file output.
#[test]
fn test_emits_external_source_map_reference_for_single_file_target() {
    // annotate the bundled entry with one external source map reference
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // preserve both input modules in the source map sidecar
    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = expected_source_map(&[&dep_path, &main_path], SINGLE_FILE_SOURCE_MAP_MAPPINGS);

    // emit one bundled entry plus one external map file
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
    });
    let expected_manifest = expected_manifest(vec![
        expected_manifest_chunk("dist/js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        expected_manifest_map("dist/js.js.map"),
    ]);
    let expected_entry = expected_linked_entry_with_source_map_reference(
        &js_output(
            r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
        ),
        "./js.js.map",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_linked_script_map(&linked, Some(&expected_linked_map(map)));
}

/// Emit one hidden source map sidecar without annotating the bundled entry text.
#[test]
fn test_emits_hidden_source_map_for_single_file_target() {
    // keep the hidden source map out of the bundled entry text
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // preserve both input modules in the hidden source map sidecar
    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = expected_source_map(&[&dep_path, &main_path], SINGLE_FILE_SOURCE_MAP_MAPPINGS);

    // emit the bundled entry without any source map comment
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
        target.bundle.output.sourcemap = Some(destack_workspace::SourceMapMode::Hidden);
    });
    let expected_manifest = expected_manifest(vec![
        expected_manifest_chunk("dist/js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        expected_manifest_map("dist/js.js.map"),
    ]);
    let expected_entry = expected_linked_entry(&js_output(
        r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
    ));

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_linked_script_map(&linked, Some(&expected_linked_map(map)));
}

/// Emit one inline source map for bundled single-file output without a sidecar map file.
#[test]
fn test_emits_inline_source_map_for_single_file_target() {
    // inline the linked source map directly into the bundled entry
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // build the inline source map payload from both input modules
    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = expected_source_map(&[&dep_path, &main_path], SINGLE_FILE_SOURCE_MAP_MAPPINGS);
    let inline_reference = expected_inline_source_map_reference(&map);

    // emit one entry file with one inline source map reference
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
        target.bundle.output.sourcemap = Some(destack_workspace::SourceMapMode::Inline);
    });
    let expected_manifest = expected_manifest(vec![
        expected_manifest_chunk("dist/js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
    ]);
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: expected_linked_entry_with_source_map_reference(
            &js_output(
                r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
            ),
            &inline_reference,
        ),
        manifest: expected_manifest,
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Apply banner and footer text to bundled single-file output before the source map reference.
#[test]
fn test_emits_banner_and_footer_for_single_file_target() {
    // wrap the bundled entry with banner and footer text
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module("common.ts", &js(r#"export const commonValue = 1;"#));
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // preserve both input modules in the external source map
    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let map = expected_source_map(
        &[&dep_path, &main_path],
        SINGLE_FILE_BANNER_FOOTER_SOURCE_MAP_MAPPINGS,
    );

    // emit the wrapped entry before the source map annotation
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
        target.bundle.output.banner = Some("/* banner */".to_string());
        target.bundle.output.footer = Some("/* footer */".to_string());
    });
    let expected_manifest = expected_manifest(vec![
        expected_manifest_chunk("dist/js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        expected_manifest_map("dist/js.js.map"),
    ]);
    let expected_entry = expected_linked_entry_with_source_map_reference(
        &js_output(
            r#"
/* banner */
export const commonValue = 1;

export const appValue = commonValue;
/* footer */
"#,
        ),
        "./js.js.map",
    );

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_linked_script_map(&linked, Some(&expected_linked_map(map)));
}
