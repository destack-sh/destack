use crate::{TestProgram, run_to_completion};
use destack_artifact::{DirPrepared, EmitFormat, ImportedModuleTable, ModuleEdgeRelation, Runtime};
use destack_dir::{DependencyKind, Expression, ImportSource, ModuleTarget, StaticKey};
use destack_source::DiagnosticSeverity;
use std::time::Duration;

/// Resolve imports from module declarations.
#[test]
fn test_resolve_module_binding_import() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "foo" {
    export const value: number;
}
"#;
    let main_source = r#"
import "./decl.d.ts";
import { value } from "foo";

value;
"#;
    let decl_module_id = test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", main_source);
    test.resolve_module(main_module_id);
    test.compile_check_clean();

    // load symbol data for the main module
    let profile = test.default_profile_id(main_module_id);
    let dir = test.artifact_dir(main_module_id, profile);
    let symbols = &dir.symbols;

    // locate the imported symbol
    let name_id = test.program.strings.intern("value");
    let scope = symbols.get_scope_by_id(dir.namespace_scope);
    let Some(symbol_id) = symbols.find_active_symbol(scope, StaticKey::Name(name_id)) else {
        panic!("expected import binding for value");
    };
    let symbol = symbols.get_symbol(symbol_id);
    let Some(target_symbol) = symbol.target_symbol else {
        panic!("expected import target");
    };

    // assert the import resolves to the module binding declaration
    assert_eq!(target_symbol.module_id, decl_module_id);
}

/// Prefer package modules over same-scope module declaration augmentations.
#[test]
fn test_resolve_prefers_package_module_over_module_binding_augmentation() {
    let test = TestProgram::memory_sequential();

    // provide a resolvable external package module target
    test.add_file(
            "node_modules/@vitest/runner/package.json",
            r#"{ "name": "@vitest/runner", "exports": { ".": { "types": "./dist/index.d.ts", "default": "./dist/index.js" } } }"#,
        );
    test.add_module(
        "node_modules/@vitest/runner/dist/index.js",
        r#"
module.exports = {};
"#,
    );
    test.add_module(
        "node_modules/@vitest/runner/dist/index.d.ts",
        r#"
export interface TaskResultPack {
    ok: true;
}
"#,
    );

    // register an in-package module augmentation for the same specifier
    let augmentation_module_id = test.add_module(
        "node_modules/vitest/dist/chunks/global.d.ts",
        r#"
declare module "@vitest/runner" {
    interface TaskMeta {
        benchmark?: boolean;
    }
}
"#,
    );
    test.import_module(augmentation_module_id);
    test.compile_check_clean();

    // resolve an import that needs the real package symbol
    let main_module_id = test.add_module(
        "node_modules/vitest/dist/index.d.ts",
        r#"
import { TaskResultPack } from "@vitest/runner";

type Wrapped = TaskResultPack;
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve missing package symbols through module augmentation fallback.
#[test]
fn test_resolve_module_binding_fallback_for_missing_package_symbol() {
    let test = TestProgram::memory_sequential();

    // provide a resolvable external package module target
    test.add_file(
            "node_modules/@vitest/expect/package.json",
            r#"{ "name": "@vitest/expect", "exports": { ".": { "types": "./dist/index.d.ts", "default": "./dist/index.js" } } }"#,
        );
    test.add_module(
        "node_modules/@vitest/expect/dist/index.js",
        r#"
module.exports = {};
"#,
    );
    test.add_module(
        "node_modules/@vitest/expect/dist/index.d.ts",
        r#"
export interface ExpectStatic {
    matcher: string;
}
"#,
    );

    // register an in-package module augmentation for the same specifier
    let augmentation_module_id = test.add_module(
        "node_modules/vitest/dist/chunks/global_expect.d.ts",
        r#"
declare module "@vitest/expect" {
    interface ExpectPollOptions {
        timeout?: number;
    }
}
"#,
    );
    test.import_module(augmentation_module_id);
    test.compile_check_clean();

    // resolve symbols that come from both module and augmentation
    let main_module_id = test.add_module(
        "node_modules/vitest/dist/index.d.ts",
        r#"
import { ExpectPollOptions, ExpectStatic } from "@vitest/expect";

type Poll = ExpectPollOptions;
type Static = ExpectStatic;
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve type imports from module declarations.
#[test]
fn test_resolve_module_binding_type_import() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "node:worker_threads" {
    export const SHARE_ENV: unique symbol;
}
"#;
    let main_source = r#"
import "./decl.d.ts";

type Share = (typeof import("node:worker_threads"))["SHARE_ENV"];
"#;
    let decl_module_id = test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", main_source);

    // run resolve pipeline
    test.resolve_module(main_module_id);
    test.compile_check_clean();

    // assert the module binding was registered in the declaring module's base DIR
    let dir = test.dir_base(decl_module_id);
    let bindings = dir
        .module_bindings
        .iter()
        .filter(|binding| binding.specifier == test.program.strings.intern("node:worker_threads"))
        .count();
    assert_eq!(
        bindings, 1,
        "expected a module binding for node:worker_threads"
    );

    // assert the binding can be resolved from the importing module via the cache
    let profile = test.default_profile_id(main_module_id);
    let bindings = test
        .compiler
        .module_bindings_for_specifier(
            test.program.current_revision(),
            main_module_id,
            profile,
            test.program.strings.intern("node:worker_threads"),
        )
        .unwrap()
        .unwrap();
    let binding_ref = bindings.first().expect("expected module binding");
    assert_eq!(
        binding_ref.module_id, decl_module_id,
        "expected module binding to resolve to declaration module"
    );
}

/// Test that a single module binding with `export = X` doesn't produce errors.
#[test]
fn test_module_binding_export_assignment_single() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "foo" {
    const foo: number;
    export = foo;
}
"#;
    test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", "import './decl.d.ts';");
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test that multiple module bindings with different specifiers each having
/// `export = X` don't conflict with each other.
#[test]
fn test_module_binding_export_assignment_multiple() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "foo" {
    const foo: number;
    export = foo;
}
declare module "node:foo" {
    import foo = require("foo");
    export = foo;
}
"#;
    test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", "import './decl.d.ts';");
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test that a module binding with only `export * from "X"` doesn't produce errors.
#[test]
fn test_module_binding_export_star_only() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "foo" {
    export const value: number;
}
declare module "node:foo" {
    export * from "foo";
}
"#;
    test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", "import './decl.d.ts';");
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test multiple module bindings: one with `export =` and another with `export * from`.
#[test]
fn test_module_binding_mixed_export_patterns() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "assert" {
    function ok(value: unknown): void;
    export = ok;
}
declare module "node:assert" {
    export * from "assert";
}
"#;
    test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", "import './decl.d.ts';");
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test many module bindings with various export patterns (simulating Node.js typings).
#[test]
fn test_module_binding_many_modules() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "assert" {
    function ok(value: unknown): void;
    export = ok;
}
declare module "node:assert" {
    import assert = require("assert");
    export = assert;
}
declare module "buffer" {
    class Buffer {}
    export { Buffer };
}
declare module "node:buffer" {
    export * from "buffer";
}
declare module "wasi" {
    class WASI {}
}
declare module "node:wasi" {
    export * from "wasi";
}
declare module "zlib" {
    function gzip(data: unknown): unknown;
    export = gzip;
}
declare module "node:zlib" {
    export * from "zlib";
}
"#;
    test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", "import './decl.d.ts';");
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test that namespace members are exported from a module binding with `export = MergedClass`.
#[test]
fn test_module_binding_namespace_member_export() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "stream" {
    class Stream {}
    namespace Stream {
        class Duplex {}
        class Readable {}
    }
    export = Stream;
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import './decl.d.ts';
import { Duplex, Readable, Stream } from 'stream';
const d: Duplex = new Duplex();
const r: Readable = new Readable();
const s: Stream = new Stream();
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test function+namespace merge exports namespace members.
#[test]
fn test_module_binding_function_namespace_merge() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "assert" {
    function assert(value: unknown): void;
    namespace assert {
        interface Assert {}
        var Assert: { new(): Assert };
        class AssertionError {}
    }
    export = assert;
}
    
declare module "assert/strict" {
    import { Assert, AssertionError } from "assert";
    export { Assert, AssertionError };
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import './decl.d.ts';
import { Assert, AssertionError } from 'assert/strict';
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test circular imports between module bindings.
#[test]
fn test_module_binding_circular_imports() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "assert" {
    import strict = require("assert/strict");
    function assert(value: unknown): void;
    namespace assert {
        interface Assert {}
        var Assert: { new(): Assert };
        export { strict };
    }
    export = assert;
}

declare module "assert/strict" {
    import { Assert } from "assert";
    function strict(value: unknown): void;
    namespace strict {
        export { Assert };
    }
    export = strict;
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import './decl.d.ts';
import { Assert } from 'assert';
import { Assert as AssertStrict } from 'assert/strict';
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Test multiple namespace blocks that merge (like real node assert module).
#[test]
fn test_module_binding_multiple_namespace_blocks() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "assert" {
    import strict = require("assert/strict");
    function assert(value: unknown): void;
    // first namespace block with the types
    namespace assert {
        interface Assert {}
        var Assert: { new(): Assert };
        function ok(value: unknown): void;
    }
    // second namespace block that re-exports
    namespace assert {
        export { strict };
    }
    export = assert;
}

declare module "assert/strict" {
    import { Assert } from "assert";
    export { Assert };
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import './decl.d.ts';
import { Assert, ok } from 'assert';
import { Assert as AssertStrict } from 'assert/strict';
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Regression test for exact node assert structure with interface+var Assert.
#[test]
fn test_module_binding_node_assert_pattern() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "assert" {
    import strict = require("assert/strict");
    function assert(value: unknown, message?: string): asserts value;
    const kOptions: unique symbol;
    namespace assert {
        type AssertMethodNames = "ok" | "fail";
        interface AssertOptions {
            strict?: boolean | undefined;
        }
        interface Assert {
            readonly [kOptions]: AssertOptions & { strict: false };
        }
        interface AssertStrict {
            readonly [kOptions]: AssertOptions & { strict: true };
        }
        var Assert: {
            new(options?: AssertOptions & { strict?: true }): AssertStrict;
            new(options: AssertOptions): Assert;
        };
        class AssertionError {
            constructor();
        }
        function ok(value: unknown, message?: string): asserts value;
    }
    namespace assert {
        export { strict };
    }
    export = assert;
}

declare module "node:assert" {
    import assert = require("assert");
    export = assert;
}

declare module "assert/strict" {
    import {
        Assert,
        AssertionError,
        AssertOptions,
        AssertStrict,
        AssertMethodNames,
        ok,
    } from "node:assert";
    export {
        Assert,
        AssertionError,
        AssertOptions,
        AssertStrict,
        AssertMethodNames,
        ok,
    };
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import './decl.d.ts';
import { Assert, AssertionError } from 'assert/strict';
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve named imports through `export =` module-binding aliases.
#[test]
fn test_module_binding_named_import_through_export_assignment_alias() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "decl.d.ts",
        r#"
declare module "path" {
    namespace path {
        interface PlatformPath {
            readonly sep: string;
        }
    }
    const path: path.PlatformPath;
    export = path;
}

declare module "node:path" {
    import path = require("path");
    export = path;
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "./decl.d.ts";
import { sep } from "node:path";

const value = sep;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}
/// Resolve node builtin subpath imports from ambient lib module bindings.
#[test]
fn test_module_binding_node_builtin_subpath_import() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import assert from "node:assert/strict";

assert.ok(true);
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve node builtin named exports from ambient lib module bindings.
#[test]
fn test_module_binding_node_builtin_named_import() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import { createHash, randomUUID } from "node:crypto";

createHash("sha1");
randomUUID();
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve `destack:` protocol imports in native runtime.
#[test]
fn test_protocol_destack_console_in_native_runtime() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "destack:console";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve `platform:` protocol imports in native runtime.
#[test]
fn test_protocol_platform_fs_in_native_runtime() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "platform:fs";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Report `platform:` protocol imports as unsupported in JavaScript targets.
#[test]
fn test_protocol_platform_in_javascript_runtime_reports_error() {
    let mut test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "platform:fs";
"#,
    );
    test.set_module_profile(
        main_module_id,
        Runtime::Node,
        EmitFormat::Js,
        &["js", "esnext"],
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER205");
}

/// Report unknown platform builtins through builtin protocol diagnostics.
#[test]
fn test_protocol_platform_unknown_builtin_in_native_runtime_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "platform:not_a_real_builtin";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER206");
}

/// Resolve protocol imports using the active profile, not the module default profile.
#[test]
fn test_protocol_platform_fs_uses_active_profile_override() {
    let mut test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "platform:fs";
"#,
    );
    test.set_module_profile(
        main_module_id,
        Runtime::NativeManaged,
        EmitFormat::Native,
        &["default"],
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Report internal platform protocol imports when noInternalImport is set to deny.
#[test]
fn test_protocol_platform_in_native_runtime_with_deny_internal_import_policy() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "platform:fs";
"#,
    );
    test.set_module_no_internal_import_policy(main_module_id, "deny");

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER208");
}

/// Report internal platform protocol imports as warnings when noInternalImport is warn.
#[test]
fn test_protocol_platform_in_native_runtime_with_warn_internal_import_policy() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "platform:fs";
"#,
    );
    test.set_module_no_internal_import_policy(main_module_id, "warn");

    test.resolve_module(main_module_id);
    test.compile();
    test.check_no_diagnostic(DiagnosticSeverity::Error);
    test.check_has_diagnostic("ER208");
}

/// Suppress internal platform protocol diagnostics when noInternalImport is allow.
#[test]
fn test_protocol_platform_in_native_runtime_with_allow_internal_import_policy() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "platform:fs";
"#,
    );
    test.set_module_no_internal_import_policy(main_module_id, "allow");

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Report `node:` protocol imports as unsupported in native runtime.
#[test]
fn test_protocol_node_in_native_runtime_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "node:events";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER205");
}

/// Resolve `bun:` protocol imports in Bun runtime.
#[test]
fn test_protocol_bun_in_bun_runtime() {
    let mut test = TestProgram::memory_sequential_with_prelude_and_libs();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "bun:sqlite";
"#,
    );
    test.set_module_profile(
        main_module_id,
        Runtime::Bun,
        EmitFormat::Js,
        &["bun.v1.3", "js", "esnext"],
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Report `bun:` protocol imports as unsupported in native runtime.
#[test]
fn test_protocol_bun_in_native_runtime_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "bun:sqlite";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER205");
}

/// Report unknown bun builtins through builtin protocol diagnostics.
#[test]
fn test_protocol_bun_unknown_builtin_reports_error() {
    let mut test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "bun:not_a_real_builtin";
"#,
    );
    test.set_module_profile(
        main_module_id,
        Runtime::Bun,
        EmitFormat::Js,
        &["js", "esnext"],
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER206");
}

/// Report `deno:` protocol imports as unsupported in native runtime.
#[test]
fn test_protocol_deno_in_native_runtime_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "deno:kv";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER205");
}

/// Report unknown deno builtins through builtin protocol diagnostics.
#[test]
fn test_protocol_deno_unknown_builtin_reports_error() {
    let mut test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "deno:kv";
"#,
    );
    test.set_module_profile(
        main_module_id,
        Runtime::Deno,
        EmitFormat::Js,
        &["js", "esnext"],
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER206");
}

/// Report unknown node builtins through builtin protocol diagnostics.
#[test]
fn test_protocol_node_unknown_builtin_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "node:not_a_real_builtin";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER206");
}

/// Prefer package resolution before bare node builtin compatibility fallback.
#[test]
fn test_bare_node_builtin_prefers_package_module_when_available() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    test.add_file(
        "node_modules/fs/package.json",
        r#"{ "name": "fs", "exports": { ".": { "types": "./dist/index.d.ts", "default": "./dist/index.js" } } }"#,
    );
    test.add_module(
        "node_modules/fs/dist/index.js",
        r#"
module.exports = { packageOnlySymbol: 1 };
"#,
    );
    test.add_module(
        "node_modules/fs/dist/index.d.ts",
        r#"
export const packageOnlySymbol: number;
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import { packageOnlySymbol } from "fs";

packageOnlySymbol;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Report unknown protocol schemes with dedicated diagnostics.
#[test]
fn test_protocol_unknown_scheme_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude();
    let main_module_id = test.add_module(
        "main.ds",
        r#"
import "custom:console";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile();
    test.check_has_diagnostic("ER204");
}

/// Resolve javascript default imports from node builtin modules.
#[test]
fn test_module_binding_node_builtin_default_import_in_javascript() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let main_module_id = test.add_module(
        "main.js",
        r#"
import http from "http";

http.request;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Keep typescript default imports from node builtin modules strict.
#[test]
fn test_module_binding_node_builtin_default_import_in_typescript_reports_error() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import http from "http";

http.request;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_with_timeout(Duration::from_secs(30));
    test.check_has_diagnostic("ER101");
}

/// Allow typescript default imports from node builtins with destack.json interop enabled.
#[test]
fn test_module_binding_node_builtin_default_import_in_typescript_with_destack_config_interop() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    test.add_destack_config(
        r#"
{ "compilerOptions": { "esModuleInterop": true } }
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import http from "http";

http.request;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve parent-directory imports that target an index module.
#[test]
fn test_resolve_parent_directory_index_import() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "lib/compile/index.ts",
        r#"
export const SchemaEnv = 1;

export function getCompilingSchema() {
    return SchemaEnv;
}
"#,
    );
    let main_module_id = test.add_module(
        "lib/compile/jtd/parse.ts",
        r#"
import { SchemaEnv, getCompilingSchema } from "..";

SchemaEnv;
getCompilingSchema();
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve current-directory imports that target an index module.
#[test]
fn test_resolve_current_directory_index_import() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "lib/compile/jtd/index.ts",
        r#"
export const SchemaEnv = 1;
"#,
    );
    let main_module_id = test.add_module(
        "lib/compile/jtd/parse.ts",
        r#"
import { SchemaEnv } from ".";

SchemaEnv;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve TypeScript value import symbols from declaration companions.
#[test]
fn test_resolve_typescript_value_import_prefers_declaration_symbols() {
    let test = TestProgram::memory_sequential_with_prelude();
    test.add_module(
        "react.js",
        r#"
module.exports = {};
"#,
    );
    test.add_module(
        "react.d.ts",
        r#"
export declare function useCallback(): void;
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import { useCallback } from "./react.js";

useCallback();
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Report unresolved declaration imports when skipLibCheck is disabled.
#[test]
fn test_resolve_declaration_unresolved_module_without_skip_lib_check_reports_error() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "types.d.ts",
        r#"
import type { Missing } from "missing-package";

export interface Value {
    item: Missing;
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "./types.d.ts";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_with_timeout(Duration::from_secs(30));
    test.check_has_diagnostic("ER200");
}

/// Suppress unresolved declaration imports when skipLibCheck is enabled.
#[test]
fn test_resolve_declaration_unresolved_module_with_skip_lib_check_skips_error() {
    let test = TestProgram::memory_sequential();
    test.add_destack_config(
        r#"
{ "compilerOptions": { "skipLibCheck": true } }
"#,
    );
    test.add_module(
        "types.d.ts",
        r#"
import type { Missing } from "missing-package";

export interface Value {
    item: Missing;
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "./types.d.ts";
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Suppress dependency declaration unresolved-module diagnostics when skipLibCheck is enabled.
#[test]
fn test_resolve_dependency_declaration_unresolved_module_with_skip_lib_check_skips_error() {
    let test = TestProgram::memory_sequential();
    test.add_destack_config(
        r#"
{ "compilerOptions": { "skipLibCheck": true } }
"#,
    );
    test.add_file(
        "node_modules/pkg/package.json",
        r#"{ "name": "pkg", "types": "index.d.ts" }"#,
    );
    test.add_module(
        "node_modules/pkg/index.d.ts",
        r#"
import type { Missing } from "missing-package";

export type Value = Missing;
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import type { Value } from "./node_modules/pkg/index.d.ts";

export type RootValue = Value;
"#,
    );

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve explicitly externalized bare package reexports as external link targets.
#[test]
fn test_resolve_externalized_bare_package_reexport_as_external_target() {
    let test = TestProgram::memory_sequential();
    let main_module_id = test.add_module(
        "main.ts",
        r#"
export * from "react";
"#,
    );

    test.configure_target(main_module_id, "js", |target| {
        target.bundle_dependencies.never_bundle = vec!["react".to_string()];
    });

    // direct import resolution should already preserve the external target
    let profile_id = test.default_profile_id(main_module_id);
    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.require_dir_prepared(_context.revision(), main_module_id, profile_id)
        },
    )
    .unwrap();
    let dir = test
        .compiler
        .require_artifact_dir_prepared(test.program.current_revision(), main_module_id, profile_id)
        .unwrap();
    let dir: &DirPrepared = dir.as_ref();
    let module = test.program.module_descriptor(main_module_id);
    let module = module.as_ref();
    let anchor = dir.anchor_node.into_global(main_module_id);
    let target = test.program.strings.intern("react");
    let mut imported_modules = ImportedModuleTable::default();
    let resolved_target = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.resolve_import(
                _context.revision(),
                module,
                &mut imported_modules,
                profile_id,
                anchor,
                ImportSource::ExportStatement,
                target,
                DependencyKind::Value,
            )
        },
    )
    .unwrap();
    assert_eq!(resolved_target, destack_dir::ModuleTarget::External(target));

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let dir = test.dir_resolved(main_module_id);
    let cache_key = (
        Some(main_module_id),
        target,
        ModuleEdgeRelation::Import,
        None,
    );
    let imported = dir
        .imported_modules
        .get(&cache_key)
        .unwrap_or_else(|| panic!("expected cached import entry for react"));

    assert_eq!(
        imported.value,
        Some(destack_dir::ModuleTarget::External(
            test.program.strings.intern("react"),
        ))
    );
}

/// Prefer ambient module bindings over external package deferral.
#[test]
fn test_resolve_prefers_module_binding_before_external_package_target() {
    let test = TestProgram::memory_sequential();
    let declaration_module_id = test.add_module(
        "react.d.ts",
        r#"
declare module "react" {
    export const useValue: number;
}
"#,
    );
    let main_module_id = test.add_module(
        "main.ts",
        r#"
import "./react.d.ts";
import { useValue } from "react";

useValue;
"#,
    );

    test.configure_target(main_module_id, "js", |target| {
        target.bundle_dependencies.never_bundle = vec!["react".to_string()];
    });

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    // imported module cache
    let dir = test.dir_resolved(main_module_id);
    let cache_key = (
        Some(main_module_id),
        test.program.strings.intern("react"),
        ModuleEdgeRelation::Import,
        None,
    );
    let imported = dir
        .imported_modules
        .get(&cache_key)
        .unwrap_or_else(|| panic!("expected cached import entry for react"));

    assert_eq!(
        imported.value,
        Some(destack_dir::ModuleTarget::Binding(
            test.program.strings.intern("react"),
        ))
    );

    // imported symbol
    let profile = test.default_profile_id(main_module_id);
    let dir = test.artifact_dir(main_module_id, profile);
    let symbols = &dir.symbols;
    let name_id = test.program.strings.intern("useValue");
    let scope = symbols.get_scope_by_id(dir.namespace_scope);
    let Some(symbol_id) = symbols.find_active_symbol(scope, StaticKey::Name(name_id)) else {
        panic!("expected import binding for useValue");
    };
    let symbol = symbols.get_symbol(symbol_id);
    let Some(target_symbol) = symbol.target_symbol else {
        panic!("expected import target");
    };

    assert_eq!(target_symbol.module_id, declaration_module_id);
}

/// Resolve one static dynamic import target expression to one module target.
#[test]
fn test_resolve_dynamic_import_string_expression_target() {
    let test = TestProgram::memory_sequential();
    let feature_module_id = test.add_module("feature.ts", "export const featureValue = 1;");
    let main_module_id = test.add_module(
        "main.ts",
        r#"
export const featurePromise = import("./feature.ts");
"#,
    );
    test.resolve_module(main_module_id);
    test.compile_check_clean();

    // resolved dynamic import
    let dir = test.dir_resolved(main_module_id);
    let tree = &dir.tree;
    let found_import = tree
        .iter_nodes_of_type::<Expression>()
        .find(|(_, expression)| {
            matches!(
                expression,
                Expression::Import {
                    source: ImportSource::ImportCall,
                    ..
                }
            )
        });
    let Some((_, import_expression)) = found_import else {
        panic!("expected resolved dynamic import expression");
    };

    let Expression::Import {
        source,
        target,
        target_module,
        ..
    } = import_expression
    else {
        panic!("expected resolved dynamic import expression");
    };

    assert_eq!(*source, ImportSource::ImportCall);
    assert_eq!(test.program.strings.get(*target), "./feature.ts");
    assert_eq!(*target_module, ModuleTarget::Module(feature_module_id));

    // unresolved imports should be gone after resolve
    let has_unresolved_import = tree
        .iter_nodes_of_type::<Expression>()
        .any(|(_, expression)| matches!(expression, Expression::UnresolvedImport { .. }));

    assert!(!has_unresolved_import);
}
