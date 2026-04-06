use super::*;
use destack_dir::{StaticKey, SymbolSpace};

/// Assert one module exports a type symbol for the requested name.
fn assert_has_type_export(test: &TestProgram, module_id: destack_source::ModuleId, name: &str) {
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir(module_id, profile);
    let exports = &dir.exported_symbols;
    let name_id = test.program.strings.intern(name);
    let key = (SymbolSpace::Type, StaticKey::Name(name_id));

    let Some(export) = exports.get(&key) else {
        panic!("expected type export '{name}'");
    };
    assert!(
        export.symbol.is_some() || export.item.is_some(),
        "expected type export '{name}' to carry a target",
    );
}

/// Assert one module exports a value symbol for the requested name.
fn assert_has_value_export(test: &TestProgram, module_id: destack_source::ModuleId, name: &str) {
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir(module_id, profile);
    let exports = &dir.exported_symbols;
    let name_id = test.program.strings.intern(name);
    let key = (SymbolSpace::Value, StaticKey::Name(name_id));

    let Some(export) = exports.get(&key) else {
        panic!("expected value export '{name}'");
    };
    assert!(export.target.resolved().is_some());
}

/// Assert one module does not export a value symbol for the requested name.
fn assert_missing_value_export(
    test: &TestProgram,
    module_id: destack_source::ModuleId,
    name: &str,
) {
    let profile = test.default_profile_id(module_id);
    let dir = test.artifact_dir(module_id, profile);
    let exports = &dir.exported_symbols;
    let name_id = test.program.strings.intern(name);
    let key = (SymbolSpace::Value, StaticKey::Name(name_id));

    assert!(
        !exports.contains_key(&key),
        "expected missing value export '{name}'"
    );
}

/// Build module graph edges for import dependencies.
#[test]
fn test_module_graph_import_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export const value = 1;
"#;
    let main_source = r#"
import { value } from "./dep.ts";

value;
"#;

    let dep_module_id = test.add_module("dep.ts", dep_source);
    let main_module_id = test.add_module("main.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let graph = test
        .repository
        .module_graph(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
    let dependencies = graph.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module graph to include dep.ts"
    );
}

/// Resolve ts relative .js specifiers through TypeScript extension substitution.
#[test]
fn test_module_graph_typescript_import_js_specifier_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export const value = 1;
"#;
    let main_source = r#"
import { value } from "./dep.js";

value;
"#;

    let dep_module_id = test.add_module("dep.ts", dep_source);
    let main_module_id = test.add_module("main.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let graph = test
        .repository
        .module_graph(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
    let dependencies = graph.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module graph to include dep.ts for ./dep.js import"
    );
}

/// Publish prepared type exports for exported type aliases.
#[test]
fn test_module_exports_type_alias_symbol() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "mod.ds",
        r#"
export type User = { name: string };
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_has_type_export(&test, module_id, "User");
}

/// Publish prepared default type exports for exported interfaces.
#[test]
fn test_module_exports_default_interface_symbol() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "mod.ds",
        r#"
export default interface User {
    name: string;
}
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_has_type_export(&test, module_id, "default");
}

/// Resolve declaration imports with .js specifiers through declaration targets.
#[test]
fn test_module_graph_declaration_import_js_specifier_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export type TaskResultPack = { ok: true };
"#;
    let main_source = r#"
import { TaskResultPack } from "./dep.js";

type Wrapped = TaskResultPack;
"#;

    let dep_module_id = test.add_module("dep.d.ts", dep_source);
    let main_module_id = test.add_module("main.d.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let graph = test
        .repository
        .module_graph(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
    let dependencies = graph.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module graph to include dep.d.ts for ./dep.js import"
    );
}

/// Resolve declaration reexports that rename type-only exports from .js specifiers.
#[test]
fn test_module_graph_declaration_reexport_type_alias_from_js_specifier() {
    let test = TestProgram::memory_sequential();
    let task_source = r#"
export interface TaskResultPack {
    ok: true;
}

export { type TaskResultPack as K };
"#;
    let index_source = r#"
export { K as TaskResultPack } from "./tasks.js";
"#;
    let main_source = r#"
import { TaskResultPack } from "./index.js";

type Wrapped = TaskResultPack;
"#;

    test.add_module("tasks.d.ts", task_source);
    test.add_module("index.d.ts", index_source);
    let main_module_id = test.add_module("main.d.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Resolve declaration export-from when the module also imports from the same target.
#[test]
fn test_module_graph_declaration_export_from_with_sibling_import() {
    let test = TestProgram::memory_sequential();
    let tasks_source = r#"
export interface TaskResult {
    state: string;
}

export type TaskResultPack = TaskResult[];

export { type TaskResult as J, type TaskResultPack as K };
"#;
    let runner_source = r#"
import { J as TaskResult } from "./tasks.js";

export { J as TaskResult, K as TaskResultPack } from "./tasks.js";
"#;
    let main_source = r#"
import { TaskResultPack } from "./runner.js";

type Wrapped = TaskResultPack;
"#;

    test.add_module("tasks.d.ts", tasks_source);
    test.add_module("runner.d.ts", runner_source);
    let main_module_id = test.add_module("main.d.ts", main_source);

    test.resolve_module(main_module_id);
    test.compile_check_clean();
}

/// Keep node strict self package behavior for packages without exports.
#[test]
fn test_module_graph_self_package_import_without_exports_reports_error() {
    let test = TestProgram::memory_sequential();

    // build absolute test paths so package discovery can locate package.json
    let root = std::env::current_dir().expect("failed to read current directory");
    let package_json_path = root.join("package.json");
    let consumer_path = root.join("src/consumer.ts");

    // configure package metadata without exports
    test.add_file(
        &package_json_path.to_string_lossy(),
        r#"{ "name": "pkg", "main": "./src/index.ts" }"#,
    );

    // import package self name from package source
    let consumer_module_id = test.add_module(
        &consumer_path.to_string_lossy(),
        r#"
import { value } from "pkg";

value;
"#,
    );

    // resolve module and assert strict self import failure
    test.resolve_module(consumer_module_id);
    test.compile();
    test.check_has_diagnostic("ER200");
}

/// Build module graph edges for self package imports with exports.
#[test]
fn test_module_graph_self_package_import_with_exports_dependency() {
    let test = TestProgram::memory_sequential();

    // build absolute test paths so package discovery can locate package.json
    let root = std::env::current_dir().expect("failed to read current directory");
    let package_json_path = root.join("package.json");
    let package_entry_path = root.join("src/index.ts");
    let consumer_path = root.join("src/consumer.ts");

    // configure package metadata with explicit exports root
    test.add_file(
        &package_json_path.to_string_lossy(),
        r#"{ "name": "pkg", "main": "./src/main.ts", "exports": { ".": "./src/index.ts" } }"#,
    );

    // define exported entry module and consumer
    let package_entry_module_id = test.add_module(
        &package_entry_path.to_string_lossy(),
        r#"
export const value = 1;
"#,
    );
    let consumer_module_id = test.add_module(
        &consumer_path.to_string_lossy(),
        r#"
import { value } from "pkg";

value;
"#,
    );

    // resolve graph and ensure dependency targets exports root
    test.resolve_module(consumer_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(consumer_module_id);
    let graph = test
        .repository
        .module_graph(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
    let dependencies = graph.dependencies_for(consumer_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&package_entry_module_id),
        "expected module graph to include src/index.ts for exported self import"
    );
}

/// Build module graph edges for namespace exports.
#[test]
fn test_module_graph_namespace_export_dependency() {
    let test = TestProgram::memory_sequential();
    let dep_source = r#"
export const value = 1;
"#;
    let export_source = r#"
export * from "./dep.ts";
"#;

    let dep_module_id = test.add_module("dep.ts", dep_source);
    let export_module_id = test.add_module("reexport.ts", export_source);

    test.resolve_module(export_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(export_module_id);
    let graph = test
        .repository
        .module_graph(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
    let dependencies = graph.dependencies_for(export_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&dep_module_id),
        "expected module graph to include dep.ts"
    );
}

/// Build module graph edges for module binding imports.
#[test]
fn test_module_graph_binding_dependency() {
    let test = TestProgram::memory_sequential();
    let decl_source = r#"
declare module "foo" {
    export const value: number;
}
"#;
    let main_source = r#"
import { value } from "foo";

value;
"#;

    let decl_module_id = test.add_module("decl.d.ts", decl_source);
    let main_module_id = test.add_module("main.ts", main_source);

    test.import_module(decl_module_id);
    test.compile_check_clean();

    test.resolve_module(main_module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(main_module_id);
    let graph = test
        .repository
        .module_graph(test.program.current_revision(), profile)
        .unwrap_or_else(|| panic!("missing module graph for profile {profile:?}"));
    let dependencies = graph.dependencies_for(main_module_id);

    // assert dependency edges
    assert!(
        dependencies.contains(&decl_module_id),
        "expected module graph to include module binding module"
    );
}

/// Synthesize named exports from CommonJS property writes.
#[test]
fn test_build_module_exports_collects_commonjs_named_property_writes() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "cjs.js",
        r#"
function buildValue() {
    return 1;
}

exports.buildValue = buildValue;
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_has_value_export(&test, module_id, "buildValue");
}

/// Replace named exports when CommonJS default replacement occurs.
#[test]
fn test_build_module_exports_replaces_commonjs_named_exports_on_module_exports_assignment() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "cjs.js",
        r#"
function first() {
    return 1;
}

function second() {
    return 2;
}

exports.first = first;
module.exports = second;
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_missing_value_export(&test, module_id, "first");
}

/// Ignore exports alias writes after module exports replacement.
#[test]
fn test_build_module_exports_ignores_exports_alias_writes_after_replacement() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "cjs.js",
        r#"
function selected() {
    return 1;
}

function leaked() {
    return 2;
}

module.exports = selected;
exports.leaked = leaked;
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_missing_value_export(&test, module_id, "leaked");
}

/// Skip synthesized CommonJS named exports when export assignment is present.
#[test]
fn test_build_module_exports_skips_commonjs_named_exports_with_export_assignment() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "cjs.cts",
        r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports.helper = helper;
export = selected;
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    assert_missing_value_export(&test, module_id, "helper");
}
