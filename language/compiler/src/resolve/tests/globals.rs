use destack_dir::{DependencySource, StaticKey};
use destack_workspace::TargetId;

use crate::TestProgram;

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
        .build_global_symbol_table_freestanding(&[module_id], profile)
        .unwrap();

    // check that the global symbols were collected
    let test_global_key = StaticKey::Name(test.program.strings.intern("TestGlobal"));
    assert!(cache.symbols.contains_key(&test_global_key),);
    let test_fn_key = StaticKey::Name(test.program.strings.intern("testGlobalFn"));
    assert!(cache.symbols.contains_key(&test_fn_key),);
    let test_iface_key = StaticKey::Name(test.program.strings.intern("TestGlobalInterface"));
    assert!(cache.symbols.contains_key(&test_iface_key),);
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
    let normalized_target = test.compiler.resolve_target_for_dependency_source(
        DependencySource::ReferencePathDirective,
        bare_target,
    );
    let normalized_text = test.program.strings.get(normalized_target);
    assert_eq!(normalized_text.as_ref(), "./global.d.ts");

    // keep explicit relative path targets unchanged
    let explicit_target = test.program.strings.intern("./global.d.ts");
    let explicit_result = test.compiler.resolve_target_for_dependency_source(
        DependencySource::ReferencePathDirective,
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
        .build_global_symbol_table_freestanding(&[module_id], profile)
        .unwrap();

    let buffer_key = StaticKey::Name(test.program.strings.intern("Buffer"));
    assert!(cache.symbols.contains_key(&buffer_key),);
}

/// Select a global table target from profile context when no default target is configured.
#[test]
fn test_build_global_symbol_table_key_uses_profile_target_without_default() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("main.ds", "export const value: i32 = 1;");
    test.add_target(module_id, "js");
    test.add_target(module_id, "native");

    let package_id = {
        let module = test.program.modules.get(module_id);
        let module = module.read();
        module.package_id
    };
    let js_target = TargetId::new(package_id, "js");
    let js_profile = test
        .program
        .profile_id_for_target(module_id, &js_target)
        .expect("missing profile for js target");
    let key = test
        .compiler
        .build_global_symbol_table_key(module_id, js_profile)
        .expect("expected profile target selection to avoid default-target error");

    assert_eq!(key.target_id, js_target);
}
