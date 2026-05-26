use std::path::PathBuf;

use destack_artifact::{
    BuildManifestFile, BuildManifestFileType, BuildManifestLoader, EmitFormat, PackageAssembly,
    Runtime, TargetOutputName,
};
use indexmap::indexmap;

use super::{
    LinkedScriptTarget, LinkedTextFile, TestProgram, inline_source_map_reference, js, js_output,
    linked_entry, linked_entry_with_source_map, linked_map, manifest, manifest_chunk, manifest_map,
    map_output, source_map,
};
use destack_workspace::EsTarget;

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
    let dep = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
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
    let dep_map_path = format!("../{dep_path}");
    let main_map_path = format!("../{main_path}");
    let map = source_map(
        &[dep_map_path.as_str(), main_map_path.as_str()],
        SINGLE_FILE_SOURCE_MAP_MAPPINGS,
    );

    // emit one exact bundled target
    let linked = test.link_single_file_js_target(main, "js");
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Maps => vec!["dist/js.js.map".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry_with_source_map(
            &js_output(
                r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
            ),
            "./js.js.map",
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
            manifest_map("js.js.map"),
        ]),
        map: Some(linked_map(map)),
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Record the script assembly in the target manifest.
#[test]
fn test_records_script_assembly_in_manifest() {
    // keep the emitted manifest aligned with the bundled entry surface
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
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
    let expected = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        manifest_map("js.js.map"),
    ]);
    let expected_entry = linked_entry_with_source_map(
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

/// Erase structural type aliases and parameter annotations during js emission.
#[test]
fn test_erases_type_alias_object_annotations_in_js_output() {
    // type only annotations should stay entirely on the type lane
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
type PageState = {
    slug: string;
    navigation: { slug: string; href: string; isActive: boolean }[];
    section: { heading: string; cards: string[] };
};

export function renderShell(page: PageState) {
    const navigation = page.navigation
        .map((item) => `${item.slug}:${item.href}:${item.isActive}`)
        .join("|");

    return `${page.slug}::${page.section.heading}::${page.section.cards.length}::${navigation}`;
}
"#),
    );

    // keep the emitted js surface exact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(&js_output(
            r#"
export function renderShell(page) {
    const navigation = page.navigation.map(
        (item) => `${item.slug}:${item.href}:${item.isActive}`
    ).join("|");
    return `${page.slug}::${page.section.heading}::${page.section.cards.length}::${navigation}`;
}
"#,
        )),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Rewrite one bundled same-output default import through one local alias binding.
#[test]
fn test_rewrites_single_file_same_output_default_import_alias() {
    // bridge one default import alias to the bundled local binding
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module(
        "dep.ts",
        &js(r#"
const bundledValue = 1;

export default bundledValue;
"#),
    );
    let main = test.add_module(
        "app.ts",
        &js(r#"
import value from "./dep.ts";

export const appValue = value;
"#),
    );

    // keep the rewritten local alias exact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(&js_output(
            r#"
const bundledValue = 1;

const value = bundledValue;
export const appValue = value;
"#,
        )),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Rewrite one bundled same-output namespace import through one getter bridge object.
#[test]
fn test_rewrites_single_file_same_output_namespace_import() {
    // bridge one namespace import to bundled local exports
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module(
        "dep.ts",
        &js(r#"
export const first = 1;
export const second = 2;
"#),
    );
    let main = test.add_module(
        "app.ts",
        &js(r#"
import * as values from "./dep.ts";

export const appValue = values.first + values.second;
"#),
    );

    // keep the emitted namespace bridge exact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(&js_output(
            r#"
export const first = 1;
export const second = 2;

const values = { get first() {
    return first;
}, get second() {
    return second;
} };
export const appValue = values.first + values.second;
"#,
        )),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Erase advanced object type members during js emission.
#[test]
fn test_erases_advanced_object_type_members_in_js_output() {
    // keep call, construct, index, and symbol members on the type lane
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
declare const renderKey: unique symbol;

type Registry = {
    readonly [name: string]: string;
    [renderKey](this: Registry, value: string): Registry;
    (value: string): string;
    new (value: string): Registry;
};

export function renderValue(registry: Registry, key: string) {
    return registry[key];
}
"#),
    );

    // keep the emitted js surface exact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(&js_output(
            r#"
export function renderValue(registry, key) {
    return registry[key];
}
"#,
        )),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Erase modern type-level syntax cleanly from plain js output.
#[test]
fn test_erases_modern_typescript_type_syntax_in_js_output() {
    // keep supported modern type syntax entirely on the type lane in plain js output
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export interface Builder {
    clone(this: this): this;
}

export type BoxLabel<T extends string> = `box:${T}`;

export function isBoxValue(value: unknown): value is BoxLabel<"alpha"> {
    return typeof value === "string";
}
"#),
    );

    // keep the runtime surface exact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(&js_output(
            r#"
export function isBoxValue(value) {
    return typeof value === "string";
}
"#,
        )),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Preserve modern type-level syntax when the selected output is typescript.
#[test]
fn test_preserves_modern_typescript_type_syntax_in_ts_output() {
    // keep supported modern type syntax on the ts output lane
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export interface Builder {
    clone(this: this): this;
}

export type BoxLabel<T extends string> = `box:${T}`;

export function isBoxValue(value: unknown): value is BoxLabel<"alpha"> {
    return typeof value === "string";
}
"#),
    );

    // keep the emitted ts surface exact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.emit = EmitFormat::Ts;
        target.out_file = Some(PathBuf::from("dist/types.ts"));
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/types.ts".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: LinkedTextFile {
            path: "dist/types.ts".to_string(),
            file_type: destack_source::FileType::TypeScript,
            text: js_output(
                r#"
export interface Builder{
    clone(this: this): this
}
export type BoxLabel<T extends string> = `box:${T}`
export function isBoxValue(value: unknown): value is BoxLabel<"alpha"> {
    return typeof value === "string";
}
"#,
            ),
        },
        manifest: manifest(vec![BuildManifestFile {
            path: "types.ts".to_string(),
            r#type: BuildManifestFileType::Chunk,
            loader: BuildManifestLoader::Ts,
            name: Some("js".to_string()),
            input: Some(main_path.clone()),
            is_entry: Some(true),
            is_dynamic_entry: Some(false),
            imports: Vec::new(),
            dynamic_imports: Vec::new(),
            stylesheets: Vec::new(),
        }]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Retain unresolved external package dependencies in the linked entry and manifest.
#[test]
fn test_retains_unresolved_external_dependency_in_single_file_manifest_and_entry() {
    // leave never-bundled package imports untouched
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "main.ts",
        &js(r#"
export * from 'react';
"#),
    );

    // report the external import in both the entry text and the manifest
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle_dependencies.never_bundle = vec!["react".to_string()];
    });
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
            .entry()
            .imports(&["react"])
            .into(),
        manifest_map("js.js.map"),
    ]);
    let expected_entry = linked_entry_with_source_map("export * from \"react\";\n", "./js.js.map");

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
}

/// Minify bundled single-file identifiers without changing the export surface.
#[test]
fn test_minifies_single_file_identifiers() {
    // minify local bundled bindings while keeping export names stable
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    test.add_module(
        "common.ts",
        &js(r#"
const sharedLocal = 1;

export const commonValue = sharedLocal;
"#),
    );
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

const localValue = commonValue;

export const appValue = localValue;
"#),
    );

    // emit one exact symbol minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.identifiers = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const b = 1;
export const commonValue = b;

const a = commonValue;
export const appValue = a;
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify bundled single-file syntax forms without renaming bindings.
#[test]
fn test_minifies_single_file_syntax() {
    // rewrite real syntax forms while preserving the binding surface
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const flag = true;
const config = { flag: flag, empty: undefined, off: false };
let placeholder = undefined;
const selected = false || config;
const branch = true ? config : placeholder;
const kind = typeof null;

function maybeValue() {
    return undefined;
}

export const appValue = config;
export const maybe = maybeValue;
export const rest = placeholder;
export const chosen = selected;
export const picked = branch;
export const type = kind;
"#),
    );

    // emit one exact syntax minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const flag=!0,config={flag,empty:void 0,off:!1};let placeholder;const selected=config,branch=config,kind="object";function maybeValue(){return;};export const appValue=config,maybe=maybeValue,rest=placeholder,chosen=selected,picked=branch,type=kind;
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify local syntax rewrites such as boolean ternaries, sequences, and literal comparisons.
#[test]
fn test_minifies_single_file_local_syntax_rewrites() {
    // fold local syntax forms without changing the binding surface
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const flag = true;
const source = "value";
const booleanValue = flag ? true : false;
const sequenceValue = (0, "prefix", source);
const comparisonValue = "a" < "b";

export const appValue = { booleanValue, sequenceValue, comparisonValue };
"#),
    );

    // emit one exact syntax minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const flag=!0,source="value",booleanValue=!!flag,sequenceValue=source,comparisonValue=!0;export const appValue={booleanValue,sequenceValue,comparisonValue};
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify `typeof` and `void 0` syntax using the serious output-side comparison rules.
#[test]
fn test_minifies_single_file_typeof_and_void_syntax() {
    // keep the source program intact and prove the exact shorter output spellings
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const value = "text";
const voidCheck = value == void 0;
const typeCheck = typeof value === "string";
const nullType = typeof null === "object";

export const appValue = { voidCheck, typeCheck, nullType };
"#),
    );

    // emit one exact syntax minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const value="text",voidCheck=value==null,typeCheck=typeof value=="string",nullType=!0;export const appValue={voidCheck,typeCheck,nullType};
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Link authored bare JavaScript globals when the selected profile includes builtin ES libs.
#[test]
fn test_links_single_file_bare_javascript_globals_from_builtin_libraries() {
    // keep the real authored JS global surface available in this linker case
    let test =
        TestProgram::memory_sequential_with_prelude();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const infinityValue = Infinity;
const nanValue = NaN;
const rootValue = globalThis;

export const appValue = [infinityValue, nanValue, rootValue];
"#),
    );

    // emit one exact bundled entry over the ambient ES library surface
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const infinityValue = Infinity;
const nanValue = NaN;
const rootValue = globalThis;
export const appValue = [infinityValue, nanValue, rootValue];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Link bundled array spread literals through the lowered JS backend.
#[test]
fn test_links_single_file_array_spread_literal() {
    // preserve array spread syntax in one plain linked bundle
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const base = [1, 2];

export const values = [...value, 3];
"#),
    );

    // emit one exact bundled entry with the spread literal intact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const base = [1, 2];
export const values = [...value, 3];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Link bundled array destructuring patterns through the lowered JS backend.
#[test]
fn test_links_single_file_array_rest_pattern() {
    // preserve array rest destructuring in one plain linked bundle
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const values = [1, 2, 3];
const [head, ...tail] = values;

export const appValue = [head, tail];
"#),
    );

    // emit one exact bundled entry with the destructuring pattern intact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const values = [1, 2, 3];
const [head, ...tail] = values;
export const appValue = [head, tail];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Link bundled for in bindings that use array patterns.
#[test]
fn test_links_single_file_for_in_array_pattern() {
    // preserve array destructuring in for in loop bindings
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const target = { a: 1 };
let first = "";

for (const [value] in target) {
    first = value;
}

export const appValue = first;
"#),
    );

    // emit one exact bundled entry with the for in pattern intact
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const target = { a: 1 };
let first = "";
for(const [value] in target) {
    first=value;
}
export const appValue = first;
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify bundled property syntax using identifier property forms when allowed.
#[test]
fn test_minifies_single_file_property_syntax() {
    // collapse quoted property forms into identifier syntax in modern output
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const value = { "plain": 1, "void": 2 };

export const appValue = value["plain"] + value["void"];
"#),
    );

    // emit one exact property minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const value={plain:1,void:2};export const appValue=value.plain+value.void;
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Upgrade lowered nullish and optional forms to ES2020 syntax when the target allows it.
#[test]
fn test_minifies_single_file_es2020_syntax_upgrades() {
    // use the shorter es2020 spellings only when the target explicitly allows them
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const source = { name: "value", "dash-name": "dash" };
const callable = function(value) { return value; };
const fallback = "fallback";
const chosen = source != null ? source : fallback;
const member = source == null ? undefined : source.name;
const index = source == null ? undefined : source["dash-name"];
const call = callable == null ? undefined : callable("call");

export const appValue = { chosen, member, index, call };
"#),
    );

    // emit one exact syntax minified bundle with es2020 upgrades enabled
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
        target.es_target = EsTarget::Es2020;
    });

    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const source={name:"value","dash-name":"dash"},callable=function(value){return value;},fallback="fallback",chosen=source??fallback,member=source?.name,index=source?.["dash-name"],call=callable?.("call");export const appValue={chosen,member,index,call};
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Preserve lowered nullish and optional forms below the ES2020 target gate.
#[test]
fn test_does_not_minify_single_file_to_es2020_syntax_below_target_gate() {
    // keep the older lowered spellings when the output target cannot use es2020 syntax
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const source = { name: "value", "dash-name": "dash" };
const callable = function(value) { return value; };
const fallback = "fallback";
const chosen = source != null ? source : fallback;
const member = source == null ? undefined : source.name;
const index = source == null ? undefined : source["dash-name"];
const call = callable == null ? undefined : callable("call");

export const appValue = { chosen, member, index, call };
"#),
    );

    // emit one exact syntax minified bundle without es2020 upgrades
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
        target.es_target = EsTarget::Es2019;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const source={name:"value","dash-name":"dash"},callable=function(value){return value;},fallback="fallback",chosen=source!=null?source:fallback,member=source==null?void 0:source.name,index=source==null?void 0:source["dash-name"],call=callable==null?void 0:callable("call");export const appValue={chosen,member,index,call};
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Keep reserved property names quoted when ES3 style property output is requested.
#[test]
fn test_preserves_reserved_property_syntax_when_requested() {
    // keep reserved names quoted while still collapsing regular identifier properties
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const value = { "plain": 1, "void": 2 };

export const appValue = value["plain"] + value["void"];
"#),
    );

    // emit one exact reserved-name aware syntax minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.bundle_output.sourcemap = None;
        target
            .bundle_output
            .generated_code
            .get_or_insert_default()
            .reserved_names_as_props = Some(false);
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const value={plain:1,"void":2};export const appValue=value.plain+value["void"];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify bundled identifiers while preserving function and class names when requested.
#[test]
fn test_keeps_function_and_class_names_during_identifier_minification() {
    // preserve runtime class and function names while still shrinking other locals
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
function namedFunction() {
    return 1;
}

class NamedClass {}

const localValue = namedFunction;

export const appValue = [localValue, NamedClass];
"#),
    );

    // emit one exact keep_names bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.identifiers = true;
        target.minify.keep_names = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"function namedFunction() {
    return 1;
}
class NamedClass{
}
const a = namedFunction;
export const appValue = [a, NamedClass];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify full bundled output by stripping unused function and class expression names.
#[test]
fn test_minifies_single_file_full_bundle_expression_names() {
    // mirror the bun expression-name cases under full bundle minification
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const localFunction = function LocalFunction() {};
const returnFunction = function ReturnFunction() { return 1; };
const localClass = class LocalClass {};

export const appValue = [localFunction, returnFunction, localClass];
"#),
    );

    // emit one exact full-minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.minify.identifiers = true;
        target.minify.whitespace = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const b=function(){},c=function(){return 1;},a=class{};export const appValue=[b,c,a];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Preserve function and class expression names under full bundle minification when requested.
#[test]
fn test_keeps_expression_names_during_full_bundle_minification() {
    // mirror the bun keep-names cases with bundled identifier minification enabled
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const localFunction = function LocalFunction() {};
const returnFunction = function ReturnFunction() { return 1; };
const recursiveFunction = function RecursiveFunction() { return RecursiveFunction(); };
const localClass = class LocalClass {};

export const appValue = [localFunction, returnFunction, recursiveFunction, localClass];
"#),
    );

    // emit one exact full-minified keep-names bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.minify.identifiers = true;
        target.minify.whitespace = true;
        target.minify.keep_names = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const b=function LocalFunction(){},d=function ReturnFunction(){return 1;},c=function RecursiveFunction(){return RecursiveFunction();},a=class LocalClass{};export const appValue=[b,d,c,a];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify one full bundled output by merging adjacent mutable bindings.
#[test]
fn test_minifies_single_file_full_bundle_adjacent_bindings() {
    // mirror the bun adjacent-binding case with full bundle minification enabled
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
let alpha = 1;
let beta = 2;
let gamma = 3;

alpha = 4;
const first = [alpha, beta, gamma];
beta = 5;
const second = [alpha, beta, gamma];
gamma = 6;

export const appValue = [first, second, [alpha, beta, gamma]];
"#),
    );

    // emit one exact full-minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.minify.identifiers = true;
        target.minify.whitespace = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"let a=1,b=2,c=3;a=4;const d=[a,b,c];b=5;const e=[a,b,c];c=6;export const appValue=[d,e,[a,b,c]];
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Minify one full bundled output with numeric `Infinity` and `NaN` spellings.
#[test]
fn test_minifies_single_file_full_bundle_infinity_spellings() {
    // mirror the bun infinity case over the real builtin-library surface
    let test =
        TestProgram::memory_sequential_with_prelude();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const values = [
    Infinity,
    -Infinity,
    Infinity + 1,
    -Infinity - 1,
    Infinity / 0,
    -Infinity / 0,
    Infinity * 0,
    -Infinity * 0,
    Infinity % 1,
    -Infinity % 1,
    Infinity ** 1,
    (-Infinity) ** 2,
    ~Infinity,
    ~-Infinity,
];

export const appValue = values;
"#),
    );

    // emit one exact full-minified bundle
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.syntax = true;
        target.minify.identifiers = true;
        target.minify.whitespace = true;
        target.bundle_output.sourcemap = None;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            r#"const a=[1/0,-1/0,1/0,-1/0,1/0,-1/0,NaN,NaN,NaN,NaN,1/0,1/0,-1,-1];export const appValue=a;
"#,
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Emit one external source map reference for bundled single-file output.
#[test]
fn test_emits_external_source_map_reference_for_single_file_target() {
    // annotate the bundled entry with one external source map reference
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
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
    let dep_map_path = format!("../{dep_path}");
    let main_map_path = format!("../{main_path}");
    let map = source_map(
        &[dep_map_path.as_str(), main_map_path.as_str()],
        SINGLE_FILE_SOURCE_MAP_MAPPINGS,
    );

    // emit one bundled entry plus one external map file
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
    });
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        manifest_map("js.js.map"),
    ]);
    let expected_entry = linked_entry_with_source_map(
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
    test.assert_linked_script_map(&linked, Some(&linked_map(map)));
}

/// Emit one hidden source map sidecar without annotating the bundled entry text.
#[test]
fn test_emits_hidden_source_map_for_single_file_target() {
    // keep the hidden source map out of the bundled entry text
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
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
    let dep_map_path = format!("../{dep_path}");
    let main_map_path = format!("../{main_path}");
    let map = source_map(
        &[dep_map_path.as_str(), main_map_path.as_str()],
        SINGLE_FILE_SOURCE_MAP_MAPPINGS,
    );

    // emit the bundled entry without any source map comment
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
        target.bundle_output.sourcemap = Some(destack_workspace::SourceMapMode::Hidden);
    });
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        manifest_map("js.js.map"),
    ]);
    let expected_entry = linked_entry(&js_output(
        r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
    ));

    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_linked_script_map(&linked, Some(&linked_map(map)));
}

/// Emit one external source map reference relative to one nested single-file output path.
#[test]
fn test_emits_external_source_map_reference_for_nested_single_file_output_path() {
    // keep the map sidecar and reference relative to the final nested output path
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
    let main = test.add_module(
        "app.ts",
        &js(r#"
import { commonValue } from './common.ts';

export const appValue = commonValue;
"#),
    );

    // preserve both input modules relative to the nested map path
    let dep_path = test.module_relative_path(dep);
    let main_path = test.module_relative_path(main);
    let dep_map_path = format!("../../{dep_path}");
    let main_map_path = format!("../../{main_path}");
    let map = source_map(
        &[dep_map_path.as_str(), main_map_path.as_str()],
        SINGLE_FILE_SOURCE_MAP_MAPPINGS,
    );

    // emit one nested entry and nested sidecar map path
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.out_file = Some(PathBuf::from("dist/entries/app-entry.js"));
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/entries/app-entry.js".to_string()],
            TargetOutputName::Maps => vec!["dist/entries/app-entry.js.map".to_string()],
            TargetOutputName::Manifest => vec!["dist/entries/js.manifest.json".to_string()],
        },
        entry: LinkedTextFile {
            path: "dist/entries/app-entry.js".to_string(),
            file_type: destack_source::FileType::JavaScript,
            text: linked_entry_with_source_map(
                &js_output(
                    r#"
export const commonValue = 1;

export const appValue = commonValue;
"#,
                ),
                "./app-entry.js.map",
            )
            .text,
        },
        manifest: {
            let value = destack_artifact::BuildManifest {
                index: None,
                files: vec![
                    manifest_chunk("entries/app-entry.js", "js")
                        .input(&main_path)
                        .entry()
                        .into(),
                    manifest_map("entries/app-entry.js.map"),
                ],
            };
            let text = serde_json::to_string_pretty(&value)
                .unwrap_or_else(|error| panic!("failed to serialize expected manifest: {error}"));
            let text = format!("{text}\n");

            super::LinkedJsonFile {
                path: "dist/entries/js.manifest.json".to_string(),
                file_type: destack_source::FileType::Json,
                text,
                value,
            }
        },
        map: Some(map_output("dist/entries/app-entry.js.map", map)),
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Emit one inline source map for bundled single-file output without a sidecar map file.
#[test]
fn test_emits_inline_source_map_for_single_file_target() {
    // inline the linked source map directly into the bundled entry
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let dep = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
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
    let dep_map_path = format!("../{dep_path}");
    let main_map_path = format!("../{main_path}");
    let map = source_map(
        &[dep_map_path.as_str(), main_map_path.as_str()],
        SINGLE_FILE_SOURCE_MAP_MAPPINGS,
    );
    let inline_reference = inline_source_map_reference(&map);

    // emit one entry file with one inline source map reference
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
        target.bundle_output.sourcemap = Some(destack_workspace::SourceMapMode::Inline);
    });
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
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
        entry: linked_entry_with_source_map(
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
    let dep = test.add_module(
        "common.ts",
        &js(r#"
export const commonValue = 1;
"#),
    );
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
    let dep_map_path = format!("../{dep_path}");
    let main_map_path = format!("../{main_path}");
    let map = source_map(
        &[dep_map_path.as_str(), main_map_path.as_str()],
        SINGLE_FILE_BANNER_FOOTER_SOURCE_MAP_MAPPINGS,
    );

    // emit the wrapped entry before the source map annotation
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.entry = vec![PathBuf::from("app.ts")];
        target.bundle_output.banner = Some("/* banner */".to_string());
        target.bundle_output.footer = Some("/* footer */".to_string());
    });
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
            .entry()
            .into(),
        manifest_map("js.js.map"),
    ]);
    let expected_entry = linked_entry_with_source_map(
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
    test.assert_linked_script_map(&linked, Some(&linked_map(map)));
}

/// Minify bundled single-file JavaScript output when bundle minification is enabled.
#[test]
fn test_minifies_single_file_js_target() {
    // bundle one entry through real syntax and identifier minification
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
const source = { "plain": true, "void": 2 };
let placeholder = undefined;
const selected = false || source;
const missing = typeof placeholder === "undefined";
const kind = typeof null;

export const appValue = {
    selected: selected,
    missing: missing,
    plain: source["plain"],
    void: source["void"],
    kind: kind,
    rest: placeholder,
};
"#),
    );

    // emit one exact minified bundle without source maps
    let main_path = test.module_relative_path(main);
    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        target.minify.enabled = true;
        target.minify.identifiers = true;
        target.bundle_output.sourcemap = None;
        target.es_target = EsTarget::Es2020;
    });
    let expected = LinkedScriptTarget {
        assembly: PackageAssembly::SingleFile,
        output_groups: indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
        entry: linked_entry(
            "const a={plain:!0,void:2};let b;const e=a,d=typeof b>\"u\",c=\"object\";export const appValue={selected:e,missing:d,plain:a.plain,void:a.void,kind:c,rest:b};\n",
        ),
        manifest: manifest(vec![
            manifest_chunk("js.js", "js")
                .input(&main_path)
                .entry()
                .into(),
        ]),
        map: None,
    };

    test.assert_linked_script_target(&linked, &expected);
}

/// Lower switch statements with default cases into linked JavaScript output.
#[test]
fn test_links_single_file_switch_statement() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
let output = 0;

switch (1) {
    case 0:
        output = 1;
        break;
    default:
        output = 2;
}

export const appValue = output;
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
let output = 0;
switch(1) {
    case 0:
        output=1;
        break;
    default:
        output=2;
}
export const appValue = output;
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower do while loops into linked JavaScript output.
#[test]
fn test_links_single_file_do_while_statement() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
let count = 0;

do {
    count = count + 1;
} while (count < 2);

export const appValue = count;
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
let count = 0;
do {
    count=count + 1;
} while(count < 2);
export const appValue = count;
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower async for of loops into linked JavaScript output.
#[test]
fn test_links_single_file_async_for_of_statement() {
    let test =
        TestProgram::memory_sequential_with_prelude();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export async function app(source) {
    for await (const value of source) {
        return value;
    }

    return 0;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export async function app(source) {
    for await (const value of source) {
        return value;
    }
    return 0;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower await expressions inside return statements into linked JavaScript output.
#[test]
fn test_links_single_file_return_await_expression() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export async function app(value) {
    return await value;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export async function app(value) {
    return await value;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower delegated generator yields into linked JavaScript output.
#[test]
fn test_links_single_file_generator_yield_delegate() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export function* app() {
    yield* [1, 2];
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export function* app() {
    yield* [1, 2];
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower bare generator yields into linked JavaScript output.
#[test]
fn test_links_single_file_generator_yield_without_value() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export function* app() {
    yield;
    return 1;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export function* app() {
    yield;
    return 1;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower try finally statements without inventing a catch clause.
#[test]
fn test_links_single_file_try_finally_statement() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
declare function cleanup(): void;

export function app() {
    try {
        cleanup();
    } finally {
        cleanup();
    }

    return 1;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export function app() {
    try {
        cleanup();
    } finally {
        cleanup();
    }
    return 1;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower catch clauses without a binding into linked JavaScript output.
#[test]
fn test_links_single_file_try_catch_without_binding() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
declare function cleanup(): void;

export function app() {
    try {
        cleanup();
    } catch {
        return 0;
    }

    return 1;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export function app() {
    try {
        cleanup();
    } catch {
        return 0;
    }
    return 1;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower labelled blocks through the statement lowering path.
#[test]
fn test_links_single_file_labelled_block_statement() {
    let test = TestProgram::memory_sequential();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export function app() {
    label: {
        break label;
    }

    return 1;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export function app() {
    label: {
        break label;
    }
    return 1;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Lower import meta and new target expressions into linked JavaScript output.
#[test]
fn test_links_single_file_meta_property_expressions() {
    let test =
        TestProgram::memory_sequential_with_prelude();
    test.add_package("test", None);
    let main = test.add_module(
        "app.ts",
        &js(r#"
export function current() {
    return import.meta;
}
"#),
    );

    let linked = test.link_single_file_js_entry(main, "js");
    let expected = linked_entry(&js_output(
        r#"
export function current() {
    return import.meta;
}
"#,
    ));

    test.assert_linked_text_file(&linked, &expected, "linked entry output");
}

/// Keep plain stylesheet imports out of emitted js output.
#[test]
fn test_links_single_file_css_imports() {
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
    let main = test.add_module(
        "app.ts",
        &js(r#"
import "./styles.css";

export const panelState = "ready";
"#),
    );

    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        // stable exact output paths
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
        target.bundle_output.asset_file_names = Some("[name].[ext]".to_string());
    });
    let package_id = test.program.module_descriptor(main).package_id;
    let output = test.package_output(package_id, "js");
    let main_path = test.module_relative_path(main);
    let expected_entry = linked_entry(&js_output(
        r#"
export const panelState = "ready";
"#,
    ));
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
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
    ]);

    test.assert_linked_script_output_groups(
        &linked,
        &indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Assets => vec!["dist/styles.css".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
    );
    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_text_output_at_path(
        package_id,
        &output,
        TargetOutputName::Assets,
        &super::LinkedTextFile {
            path: "dist/styles.css".to_string(),
            file_type: destack_source::FileType::Css,
            text: "body{color:red}\n".to_string(),
        },
        "linked stylesheet asset",
    );
}

/// Keep plain stylesheet imports side effect free on non-browser script runtimes.
#[test]
fn test_links_single_file_css_imports_without_document_injection_on_node() {
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
    let main = test.add_module(
        "app.ts",
        &js(r#"
import "./styles.css";

export const panelState = "ready";
"#),
    );

    let linked = test.link_single_file_js_target_with(main, "js", |target| {
        // stable exact output paths
        target.runtime = Runtime::Js;
        target.source_map_mode = None;
        target.bundle_output.sourcemap = None;
        target.bundle_output.asset_file_names = Some("[name].[ext]".to_string());
    });
    let package_id = test.program.module_descriptor(main).package_id;
    let output = test.package_output(package_id, "js");
    let main_path = test.module_relative_path(main);
    let expected_entry = linked_entry(&js_output(
        r#"
export const panelState = "ready";
"#,
    ));
    let expected_manifest = manifest(vec![
        manifest_chunk("js.js", "js")
            .input(&main_path)
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
    ]);

    test.assert_linked_script_output_groups(
        &linked,
        &indexmap! {
            TargetOutputName::Entry => vec!["dist/js.js".to_string()],
            TargetOutputName::Assets => vec!["dist/styles.css".to_string()],
            TargetOutputName::Manifest => vec!["dist/js.manifest.json".to_string()],
        },
    );
    test.assert_linked_script_entry(&linked, &expected_entry);
    test.assert_linked_script_manifest(&linked, &expected_manifest);
    test.assert_text_output_at_path(
        package_id,
        &output,
        TargetOutputName::Assets,
        &super::LinkedTextFile {
            path: "dist/styles.css".to_string(),
            file_type: destack_source::FileType::Css,
            text: "body{color:red}\n".to_string(),
        },
        "linked stylesheet asset",
    );
}
