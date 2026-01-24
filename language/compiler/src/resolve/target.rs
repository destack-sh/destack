use destack_base::StringId;
use destack_dir::ModuleTarget;
use destack_source::{ModuleId, PackageId};
use destack_workspace::{
    ModuleBindingReference, ModuleBindingRegistry, ModuleBindingTable, ModuleBindingTableKey,
    ProfileId,
};

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
            let Some(registry) = self.program.index.module_binding_registry.get(&package_id) else {
                self.program
                    .index
                    .module_binding_tables
                    .insert(key.clone(), ModuleBindingTable::new());
                return Ok(key);
            };
            let cache = self.build_module_binding_table_from_registry(&registry);
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

    /// Build the module binding table from a package registry.
    fn build_module_binding_table_from_registry(
        &self,
        registry: &ModuleBindingRegistry,
    ) -> ModuleBindingTable {
        let mut cache = ModuleBindingTable::new();
        for (module_id, version) in &registry.module_versions {
            cache.module_versions.insert(*module_id, *version);
        }
        for (specifier, bindings) in &registry.bindings_by_specifier {
            cache
                .bindings_by_specifier
                .entry(*specifier)
                .or_default()
                .extend(bindings.iter().copied());
        }
        cache
    }

    /// Whether a module binding cache is missing any bound modules.
    fn module_binding_table_is_stale(
        &self,
        package_id: PackageId,
        cache: &ModuleBindingTable,
    ) -> bool {
        let Some(registry) = self.program.index.module_binding_registry.get(&package_id) else {
            return !cache.bindings_by_specifier.is_empty();
        };
        for (module_id, cached_version) in &cache.module_versions {
            let Some(registry_version) = registry.module_versions.get(module_id) else {
                return true;
            };
            if cached_version != registry_version {
                return true;
            }
        }
        for module_id in registry.module_versions.keys() {
            if !cache.module_versions.contains_key(module_id) {
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
}
