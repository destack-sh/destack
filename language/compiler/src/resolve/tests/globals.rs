use std::sync::Arc;

use destack_artifact::ArtifactKey;
use destack_dir::{ImportSource, StaticKey};

use crate::{ResolveError, TestProgram};

/// Test that symbols inside `declare global { }` blocks are collected as globals.
#[test]
fn test_collect_global_symbols_declare_global() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.d.ts",
        r#"
declare global {
    var TestGlobal: string;
    function testGlobalFn(): void;
    interface TestGlobalInterface {}
}
export {};
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let cache = test
        .compiler
        .build_global_symbol_table_freestanding(test.current_revision(), &[module_id], profile)
        .unwrap();

    // check that the global symbols were collected
    let test_global_key = StaticKey::Name(test.program.strings.intern("TestGlobal"));
    assert!(cache.symbols.contains_key(&test_global_key),);
    let test_fn_key = StaticKey::Name(test.program.strings.intern("testGlobalFn"));
    assert!(cache.symbols.contains_key(&test_fn_key),);
    let test_iface_key = StaticKey::Name(test.program.strings.intern("TestGlobalInterface"));
    assert!(cache.symbols.contains_key(&test_iface_key),);
}

/// Rebuild the cached global symbol table when the module graph snapshot changes.
#[test]
fn test_global_symbol_table_rebuilds_after_module_graph_change() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "main.d.ts",
        r#"
declare global {
    interface FirstGlobal {}
}
export {};
"#,
    );

    // build and cache the initial global symbol table
    test.resolve_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let initial_table = test
        .compiler
        .global_symbol_table_for_module(test.current_revision(), module_id, profile)
        .unwrap_or_else(|error| panic!("failed to build initial global symbol table: {error:?}"));

    // publish a distinct module graph snapshot for the same profile
    let mut graph = test
        .compiler
        .module_graph(profile)
        .unwrap_or_else(|| panic!("expected module graph for test profile"))
        .as_ref()
        .clone();
    let graph_version = test.compiler.artifact_version_for_revision(
        test.program.current_revision(),
        &ArtifactKey::module_graph(profile),
    );
    let module_version = graph
        .module_version_for(module_id)
        .unwrap_or_else(|| panic!("expected module graph version for {module_id:?}"));
    graph.update_module_dependencies(
        module_id,
        destack_artifact::ModuleKind::Code,
        module_version,
        vec![module_id],
    );
    test.compiler
        .artifacts
        .publish_module_graph(graph_version, graph);

    let rebuilt_table = test
        .compiler
        .global_symbol_table_for_module(test.current_revision(), module_id, profile)
        .unwrap_or_else(|error| panic!("failed to rebuild global symbol table: {error:?}"));

    // a changed graph snapshot must not reuse the old cached table
    assert!(!Arc::ptr_eq(&initial_table, &rebuilt_table));
}

/// Resolve `export as namespace` globals in both value and type spaces.
#[test]
fn test_collect_export_namespace_globals_in_type_space() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.d.ts",
        r#"
export as namespace babel;

export namespace types {
    export interface Expression {
        kind: string;
    }
}
"#,
    );
    let module_id = test.add_module(
        "main.ts",
        r#"
import { types } from "./a";

type Expression = babel.types.Expression;
const expression: types.Expression = { kind: "ok" };
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Resolve triple slash path directives to same-directory declaration modules.
#[test]
fn test_collect_global_symbols_from_triple_slash_declaration_script() {
    let test = TestProgram::memory_sequential();

    // normalize one bare reference path target to same-directory relative form
    let bare_target = test.program.strings.intern("global.d.ts");
    let normalized_target = test
        .compiler
        .resolve_target_for_dependency_source(ImportSource::ReferencePathDirective, bare_target);
    let normalized_text = test.program.strings.get(normalized_target);
    assert_eq!(normalized_text.as_ref(), "./global.d.ts");

    // keep explicit relative path targets unchanged
    let explicit_target = test.program.strings.intern("./global.d.ts");
    let explicit_result = test.compiler.resolve_target_for_dependency_source(
        ImportSource::ReferencePathDirective,
        explicit_target,
    );
    assert_eq!(explicit_result, explicit_target);
}

/// Resolve triple slash type package directives through @types package lookup.

#[test]
fn test_collect_global_symbols_from_triple_slash_types_package() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "node_modules/@types/runner-types/package.json",
        r#"{
  "name": "@types/runner-types",
  "types": "./index.d.ts"
}"#,
    );
    test.add_module(
        "node_modules/@types/runner-types/index.d.ts",
        r#"
interface RunnerGlobal {
    id: string;
}
"#,
    );
    test.add_module(
        "ambient.d.ts",
        r#"
/// <reference types="runner-types" />

export interface Markup {
    value: RunnerGlobal;
}
"#,
    );
    let module_id = test.add_module(
        "main.ts",
        r#"
import type { Markup } from "./ambient";

const markup: Markup = {
    value: {
        id: "ok",
    },
};
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Resolve triple slash lib directives through builtin library loading.
#[test]
fn test_collect_global_symbols_from_triple_slash_lib_directive() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "ambient.d.ts",
        r#"
/// <reference lib="esnext.disposable" />

export interface ResourceHolder {
    resource: Disposable;
}
"#,
    );
    let module_id = test.add_module(
        "main.ts",
        r#"
import type { ResourceHolder } from "./ambient";

declare const holder: ResourceHolder;
holder.resource;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Resolve export-namespace globals when value imports use declaration companions.
#[test]
fn test_collect_export_namespace_globals_from_companion_type_target() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "babel.js",
        r#"
export const types = {};
"#,
    );
    test.add_module(
        "babel.d.ts",
        r#"
export as namespace babel;

export namespace types {
    export interface Expression {
        kind: string;
    }

    export interface V8IntrinsicIdentifier {
        intrinsic: string;
    }
}
"#,
    );
    let module_id = test.add_module(
        "main.ts",
        r#"
import "./babel.js";

type BabelType = babel.types.Expression | babel.types.V8IntrinsicIdentifier;
const value: BabelType | null = null;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Resolve export-namespace globals when members are export aliases.
#[test]
fn test_collect_export_namespace_globals_from_export_alias() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "types.d.ts",
        r#"
export interface Expression {
    kind: string;
}

export interface V8IntrinsicIdentifier {
    intrinsic: string;
}
"#,
    );
    test.add_module(
        "babel.js",
        r#"
export {};
"#,
    );
    test.add_module(
        "babel.d.ts",
        r#"
import * as t from "./types";

export { t as types };
export as namespace babel;
"#,
    );
    let module_id = test.add_module(
        "main.ts",
        r#"
import "./babel.js";

type BabelType = babel.types.Expression | babel.types.V8IntrinsicIdentifier;
const value: BabelType | null = null;
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();
}

/// Test that symbols inside nested `declare module "x" { global { } }` are collected.
#[test]
fn test_collect_global_symbols_nested_in_module() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.d.ts",
        r#"
declare module "buffer" {
    global {
        var Buffer: string;
    }
}
"#,
    );
    test.resolve_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let cache = test
        .compiler
        .build_global_symbol_table_freestanding(test.current_revision(), &[module_id], profile)
        .unwrap();

    let buffer_key = StaticKey::Name(test.program.strings.intern("Buffer"));
    assert!(cache.symbols.contains_key(&buffer_key),);
}

/// Select global roots from profile context when no default target is configured.
#[test]
fn test_select_global_symbol_table_uses_profile_target_without_default() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ds", "export const value: i32 = 1;");
    test.add_target(module_id, "js");
    test.add_target(module_id, "native");

    let package_id = {
        let module = test.program.module_descriptor(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let js_target = test.target_id(package_id, "js");
    let js_profile = test
        .program
        .profile_id_for_target(module_id, &js_target)
        .expect("missing profile for js target");
    let roots = test
        .compiler
        .select_global_symbol_table(test.current_revision(), module_id, js_profile)
        .expect("expected profile target selection to avoid default-target error");

    assert_eq!(roots, vec![module_id].into());
}

/// Report an explicit error when multiple targets map to the same profile.
#[test]
fn test_select_global_symbol_table_errors_on_ambiguous_profile_targets() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ds", "export const value: i32 = 1;");
    test.add_target(module_id, "js");
    test.add_target(module_id, "ts");

    let package_id = {
        let module = test.program.module_descriptor(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let js_target = test.target_id(package_id, "js");
    let ts_target = test.target_id(package_id, "ts");

    // make ts target profile-equivalent to js to force ambiguity
    let _ = ts_target;
    test.copy_target(module_id, "js", "ts");

    let js_profile = test
        .program
        .profile_id_for_target(module_id, &js_target)
        .expect("missing profile for js target");
    let result =
        test.compiler
            .select_global_symbol_table(test.current_revision(), module_id, js_profile);

    let Err(ResolveError::InvalidTargetConfig { message, .. }) = result else {
        panic!("expected invalid target config for ambiguous profile targets");
    };
    assert!(
        message.contains("multiple targets map to the same profile"),
        "expected ambiguity message, got: {message}",
    );
}

/// Use the configured default target when multiple profile-equivalent targets match.
#[test]
fn test_select_global_symbol_table_prefers_default_target_with_ambiguous_profile_matches() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ds", "export const value: i32 = 1;");
    test.add_target(module_id, "js");
    test.add_target(module_id, "ts");
    test.apply_destack_config(module_id, r#"{ "defaultTarget": "ts" }"#);

    let package_id = {
        let module = test.program.module_descriptor(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let js_target = test.target_id(package_id, "js");
    let ts_target = test.target_id(package_id, "ts");

    // make ts target profile-equivalent to js to force ambiguity
    let _ = ts_target;
    test.copy_target(module_id, "js", "ts");

    let js_profile = test
        .program
        .profile_id_for_target(module_id, &js_target)
        .expect("missing profile for js target");
    let roots = test
        .compiler
        .select_global_symbol_table(test.current_revision(), module_id, js_profile)
        .expect("expected default target to disambiguate profile matches");

    assert_eq!(roots, vec![module_id].into());
}

/// Error when default target does not belong to the matching profile target set.
#[test]
fn test_select_global_symbol_table_errors_when_default_target_mismatches_profile_set() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ds", "export const value: i32 = 1;");
    test.add_target(module_id, "js");
    test.add_target(module_id, "ts");
    test.add_target(module_id, "native");
    test.apply_destack_config(module_id, r#"{ "defaultTarget": "native" }"#);

    let package_id = {
        let module = test.program.module_descriptor(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let js_target = test.target_id(package_id, "js");
    let ts_target = test.target_id(package_id, "ts");

    // make ts target profile-equivalent to js to force ambiguity on js profile
    let _ = ts_target;
    test.copy_target(module_id, "js", "ts");

    let js_profile = test
        .program
        .profile_id_for_target(module_id, &js_target)
        .expect("missing profile for js target");
    let result =
        test.compiler
            .select_global_symbol_table(test.current_revision(), module_id, js_profile);

    let Err(ResolveError::InvalidTargetConfig { message, .. }) = result else {
        panic!("expected invalid target config for mismatched default target");
    };
    assert!(
        message.contains("default target does not match profile target set"),
        "expected mismatched-default message, got: {message}",
    );
}

/// Error when destack.json default target references a missing target entry.
#[test]
fn test_select_global_symbol_table_errors_when_default_target_is_missing() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ds", "export const value: i32 = 1;");
    test.add_target(module_id, "js");
    test.add_target(module_id, "native");
    test.apply_destack_config(module_id, r#"{ "defaultTarget": "missing" }"#);

    let package_id = {
        let module = test.program.module_descriptor(module_id);
        let module = module.as_ref();
        module.package_id
    };
    let js_target = test.target_id(package_id, "js");
    let js_profile = test
        .program
        .profile_id_for_target(module_id, &js_target)
        .expect("missing profile for js target");

    // remove js target so js profile no longer matches any current target
    let _ = js_target;
    test.remove_target(module_id, "js");

    let result =
        test.compiler
            .select_global_symbol_table(test.current_revision(), module_id, js_profile);

    let Err(ResolveError::InvalidTargetConfig { message, .. }) = result else {
        panic!("expected invalid target config for missing default target");
    };
    assert!(
        message.contains("default target not found"),
        "expected missing-default message, got: {message}",
    );
}
