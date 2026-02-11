use destack_base::StringId;
use destack_dir::ModuleTarget;
use destack_source::{ModuleId, ModuleVersion, PackageId};
use destack_workspace::{
    ModuleBindingReference, ModuleBindingRegistry, ModuleBindingTable, ModuleBindingTableKey,
    ProfileId,
};
use indexmap::IndexMap;

use crate::{Compiler, ResolveResult};

impl Compiler {
    /// Prepare the module binding table for a module and profile.
    pub(super) fn prepare_module_binding_table(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ResolveResult<ModuleBindingTableKey> {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let package_id = module.package_id;
        drop(module);

        // select the module binding cache key
        let (global_key, _) = self.select_global_symbol_table(module_id, profile_id)?;
        let key = ModuleBindingTableKey {
            target_id: global_key.target_id,
            profile_id: global_key.profile_id,
            entry_module: global_key.entry_module,
        };

        // rebuild when module bindings changed since the cache was built
        let mut rebuild_cache = true;
        if let Some(cache) = self.program.index.module_binding_tables.get(&key) {
            rebuild_cache = self.module_binding_table_is_stale(package_id, &cache);
        }
        if rebuild_cache {
            let cache = self.build_module_binding_table(package_id, profile_id);
            self.program
                .index
                .module_binding_tables
                .insert(key.clone(), cache);
        }

        Ok(key)
    }

    /// Resolve a specifier to a module binding target when available.
    pub(super) fn resolve_module_binding_target(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<ModuleTarget>> {
        let cache_key = self.prepare_module_binding_table(module_id, profile_id)?;
        let Some(cache) = self.program.index.module_binding_tables.get(&cache_key) else {
            return Ok(None);
        };
        if cache.bindings_by_specifier.contains_key(&specifier) {
            return Ok(Some(ModuleTarget::Binding(specifier)));
        }
        Ok(None)
    }

    /// Look up module bindings for a specifier.
    pub(super) fn module_bindings_for_specifier(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
        specifier: StringId,
    ) -> ResolveResult<Option<Vec<ModuleBindingReference>>> {
        let cache_key = self.prepare_module_binding_table(module_id, profile_id)?;
        let Some(cache) = self.program.index.module_binding_tables.get(&cache_key) else {
            return Ok(None);
        };
        Ok(cache.bindings_by_specifier.get(&specifier).cloned())
    }

    /// Build the module binding table for one package and profile.
    fn build_module_binding_table(
        &self,
        package_id: PackageId,
        profile_id: ProfileId,
    ) -> ModuleBindingTable {
        let mut cache = ModuleBindingTable::new();

        // collect module bindings declared in the current package
        if let Some(registry) = self.program.index.module_binding_registry.get(&package_id) {
            self.append_module_binding_registry(&mut cache, &registry);
        }

        // include ambient lib module bindings visible to this profile
        for module_id in self.ambient_binding_module_ids(profile_id) {
            self.append_module_bindings_from_module(&mut cache, module_id);
        }

        cache
    }

    /// Append one package registry to a module binding table.
    fn append_module_binding_registry(
        &self,
        cache: &mut ModuleBindingTable,
        registry: &ModuleBindingRegistry,
    ) {
        for (module_id, version) in &registry.module_versions {
            cache.module_versions.insert(*module_id, *version);
            cache.registry_module_versions.insert(*module_id, *version);
        }

        for (specifier, bindings) in &registry.bindings_by_specifier {
            let entries = cache.bindings_by_specifier.entry(*specifier).or_default();
            for binding in bindings {
                if entries.iter().any(|entry| entry == binding) {
                    continue;
                }
                entries.push(*binding);
            }
        }
    }

    /// Append bindings declared in one module to a module binding table.
    fn append_module_bindings_from_module(
        &self,
        cache: &mut ModuleBindingTable,
        module_id: ModuleId,
    ) {
        let module = self.program.modules.get(module_id);
        let module = module.read();
        cache.module_versions.insert(module_id, module.version);

        let module_bindings = module.dir_base().module_bindings.read();
        for module_binding in module_bindings.iter() {
            let binding_ref = ModuleBindingReference {
                module_id,
                declaration: module_binding.declaration,
            };

            let entries = cache
                .bindings_by_specifier
                .entry(module_binding.specifier)
                .or_default();
            if entries.iter().any(|entry| entry == &binding_ref) {
                continue;
            }
            entries.push(binding_ref);
        }
    }

    /// Collect ambient modules that can contribute module bindings.
    fn ambient_binding_module_ids(&self, profile_id: ProfileId) -> Vec<ModuleId> {
        let Some(builtins) = self.program.builtins.as_ref() else {
            return Vec::new();
        };

        let profile = self.program.profile(profile_id);
        builtins.ambient_libs(&profile.key).unwrap_or_default()
    }

    /// Collect package declared module binding versions for stale checks.
    fn registry_module_binding_versions(
        &self,
        package_id: PackageId,
    ) -> IndexMap<ModuleId, ModuleVersion> {
        let mut versions = IndexMap::new();

        // include package module binding versions
        if let Some(registry) = self.program.index.module_binding_registry.get(&package_id) {
            for (module_id, version) in &registry.module_versions {
                versions.insert(*module_id, *version);
            }
        }

        versions
    }

    /// Return true when package declared module bindings changed since caching.
    fn module_binding_table_is_stale(
        &self,
        package_id: PackageId,
        cache: &ModuleBindingTable,
    ) -> bool {
        let expected_versions = self.registry_module_binding_versions(package_id);
        if expected_versions.len() != cache.registry_module_versions.len() {
            return true;
        }

        for (module_id, expected_version) in expected_versions {
            let Some(cached_version) = cache.registry_module_versions.get(&module_id) else {
                return true;
            };
            if cached_version != &expected_version {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::TestProgram;
    use destack_dir::StaticKey;
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
        let module = test.program.modules.get(main_module_id);
        let module = module.read();
        let profile = test.default_profile_id(main_module_id);
        let dir = module.dir(profile);
        let symbols = dir.symbols.read();

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
        let module = test.program.modules.get(decl_module_id);
        let module = module.read();
        let bindings = module
            .dir_base()
            .module_bindings
            .read()
            .iter()
            .filter(|binding| {
                binding.specifier == test.program.strings.intern("node:worker_threads")
            })
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
        test.compile_with_timeout(Duration::from_secs(30));
        test.check_no_diagnostic_code("ER200");
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
        test.compile_with_timeout(Duration::from_secs(30));
        test.check_no_diagnostic_code("ER101");
        test.check_no_diagnostic_code("ER200");
    }

    /// Resolve default imports from CommonJS `module.exports` assignments.
    #[test]
    fn test_resolve_default_import_from_commonjs_module_exports() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.add_module(
            "cjs.js",
            r#"
function buildValue() {
    return 1;
}

module.exports = buildValue;
"#,
        );
        let main_module_id = test.add_module(
            "main.mjs",
            r#"
import buildValue from "./cjs.js";

buildValue();
"#,
        );

        test.resolve_module(main_module_id);
        test.compile_with_timeout(Duration::from_secs(30));
        test.check_no_diagnostic_code("ER101");
        test.check_no_diagnostic_code("ER200");
    }

    /// Resolve default imports from CommonJS bracket export assignments.
    #[test]
    fn test_resolve_default_import_from_commonjs_bracket_exports() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.add_module(
            "cjs.js",
            r#"
function buildValue() {
    return 1;
}

module["exports"] = buildValue;
"#,
        );
        let main_module_id = test.add_module(
            "main.mjs",
            r#"
import buildValue from "./cjs.js";

buildValue() satisfies number;
"#,
        );

        test.resolve_module(main_module_id);
        test.compile_with_timeout(Duration::from_secs(30));
        test.check_no_diagnostic_code("ER101");
        test.check_no_diagnostic_code("ER200");
    }

    /// Resolve the last CommonJS export assignment as the default import.
    #[test]
    fn test_resolve_default_import_from_commonjs_last_assignment_wins() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.add_module(
            "cjs.js",
            r#"
function second() {
    return;
}

module.exports = 1;
module.exports = second;
"#,
        );
        let main_module_id = test.add_module(
            "main.mjs",
            r#"
import selected from "./cjs.js";

selected();
"#,
        );

        test.resolve_module(main_module_id);
        test.compile_with_timeout(Duration::from_secs(30));
        test.check_no_diagnostic_code("ER101");
        test.check_no_diagnostic_code("ER200");
    }

    /// Resolve default imports from chained CommonJS export assignments.
    #[test]
    fn test_resolve_default_import_from_commonjs_chained_assignment() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.add_module(
            "cjs.js",
            r#"
const holder = {};

function buildValue() {
    return 1;
}

holder.value = module.exports = buildValue;
"#,
        );
        let main_module_id = test.add_module(
            "main.mjs",
            r#"
import selected from "./cjs.js";

selected();
"#,
        );

        test.resolve_module(main_module_id);
        test.compile_with_timeout(Duration::from_secs(30));
        test.check_no_diagnostic_code("ER101");
        test.check_no_diagnostic_code("ER200");
    }

    /// Resolve CommonJS default imports when `module` is declared via global augmentation.
    #[test]
    fn test_resolve_default_import_from_commonjs_with_ambient_module_global() {
        let test = TestProgram::memory_sequential_with_prelude();
        test.add_module(
            "globals.d.ts",
            r#"
export {};

declare global {
    var module: { exports: unknown };
}
"#,
        );
        test.add_module(
            "cjs.js",
            r#"
function buildValue() {
    return 1;
}

module.exports = buildValue;
"#,
        );
        let main_module_id = test.add_module(
            "main.ts",
            r#"
import "./globals.d.ts";
import buildValue from "./cjs";

buildValue() satisfies number;
"#,
        );

        test.resolve_module(main_module_id);
        test.compile_check_clean();
    }
}
