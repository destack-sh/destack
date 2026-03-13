use destack_dir::DependencySource;
use destack_workspace::ImportEdgeKind;

use crate::resolve::dependency::export::{CommonjsExportState, CommonjsExportValue};

use super::*;

/// Use require conditions for static imports in TypeScript CommonJS modules.
#[test]
fn test_import_edge_kind_for_typescript_commonjs_static_import() {
    let edge_kind =
        Compiler::import_edge_kind_for_dependency(DependencySource::ImportStatement, true);

    assert_eq!(edge_kind, ImportEdgeKind::Require);
}

/// Keep import-call dependencies on import conditions in TypeScript CommonJS modules.
#[test]
fn test_import_edge_kind_for_typescript_commonjs_import_call() {
    let edge_kind = Compiler::import_edge_kind_for_dependency(DependencySource::ImportCall, true);

    assert_eq!(edge_kind, ImportEdgeKind::Import);
}

/// Keep triple slash directives on import conditions in TypeScript CommonJS modules.
#[test]
fn test_import_edge_kind_for_typescript_commonjs_reference_path_directive() {
    let edge_kind =
        Compiler::import_edge_kind_for_dependency(DependencySource::ReferencePathDirective, true);

    assert_eq!(edge_kind, ImportEdgeKind::Import);
}

/// Collect the static CommonJS export state for one module.
fn collect_commonjs_state(
    test: &TestProgram,
    module_id: destack_source::ModuleId,
) -> CommonjsExportState {
    let dir = test.dir_resolved(module_id);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();

    test.compiler.collect_commonjs_export_state(
        module_id,
        dir.namespace_scope,
        dir.namespace_symbol.into_global(module_id),
        &dir.roots,
        &tree,
        &symbols,
    )
}

/// Assert a named CommonJS export points to one unresolved or resolved path name.
fn assert_named_export_targets_name(
    test: &TestProgram,
    module_id: destack_source::ModuleId,
    state: &CommonjsExportState,
    export_name: &str,
    expected_target_name: &str,
) {
    let export_name_id = test.program.strings.intern(export_name);
    let expected_name_id = test.program.strings.intern(expected_target_name);
    let Some(value) = state.named_values.get(&export_name_id) else {
        panic!("expected named export '{export_name}'");
    };

    match value {
        CommonjsExportValue::Expression(expression_id) => {
            let dir = test.dir_resolved(module_id);
            let tree = dir.tree.read();
            let expression = tree.get(*expression_id);

            let path = match expression {
                destack_dir::Expression::UnresolvedPath { path, .. }
                | destack_dir::Expression::ModuleReference { path, .. }
                | destack_dir::Expression::GlobalReference { path, .. } => path,
                _ => panic!("expected path expression target, found {expression:?}"),
            };
            assert_eq!(path.segments.len(), 1);
            assert_eq!(path.segments[0], expected_name_id);
        }
        CommonjsExportValue::Symbol(symbol_id) => {
            let dir = test.dir_resolved(module_id);
            let symbols = dir.symbols.read();
            let symbol = symbols.get_symbol(*symbol_id);
            let Some(symbol_name) = symbol.name() else {
                panic!("expected symbol name for shorthand export");
            };
            assert_eq!(symbol_name, expected_name_id);
        }
    }
}

/// Collect default and named state from direct and property CommonJS writes.
#[test]
fn test_collect_commonjs_state_for_default_and_named_writes() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = selected;
module.exports.helper = helper;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    assert!(state.default_value.is_some());
    assert_named_export_targets_name(&test, module_id, &state, "helper", "helper");
}

/// Capture default value from chained assignments targeting module exports.
#[test]
fn test_collect_commonjs_state_for_chained_module_exports_assignment() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
const holder = {};

function buildValue() {
    return 1;
}

holder.value = module.exports = buildValue;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    assert!(state.default_value.is_some());
}

/// Replace named export state when module exports is replaced with an object literal.
#[test]
fn test_collect_commonjs_state_replaces_named_values_on_module_exports_assignment() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function first() {
    return 1;
}

function second() {
    return 2;
}

exports.first = first;
module.exports = { second };
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    let first_id = test.program.strings.intern("first");
    let second_id = test.program.strings.intern("second");
    assert!(!state.named_values.contains_key(&first_id));
    assert!(state.named_values.contains_key(&second_id));
    assert_named_export_targets_name(&test, module_id, &state, "second", "second");
}

/// Drop exports alias writes after module exports replacement.
#[test]
fn test_collect_commonjs_state_drops_exports_writes_after_replacement() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
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
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    let leaked_id = test.program.strings.intern("leaked");
    assert!(!state.named_values.contains_key(&leaked_id));
}

/// Keep exports property writes when relinked through explicit module exports assignment.
#[test]
fn test_collect_commonjs_state_relinks_exports_alias() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = selected;
exports = module.exports;
exports.helper = helper;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    let helper_id = test.program.strings.intern("helper");
    assert!(state.named_values.contains_key(&helper_id));
    assert_named_export_targets_name(&test, module_id, &state, "helper", "helper");
}

/// Ignore exports writes after assigning exports to a plain value expression.
#[test]
fn test_collect_commonjs_state_does_not_relink_exports_alias_for_value_expression() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

module.exports = selected;
exports = selected;
exports.helper = helper;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    let helper_id = test.program.strings.intern("helper");
    assert!(!state.named_values.contains_key(&helper_id));
}

/// Collect named exports from static numeric bracket keys.
#[test]
fn test_collect_commonjs_state_collects_numeric_bracket_property_name() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function selected() {
    return 1;
}

module.exports[1] = selected;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);

    let key_id = test.program.strings.intern("1");
    assert!(state.named_values.contains_key(&key_id));
    assert_named_export_targets_name(&test, module_id, &state, "1", "selected");
}

/// Ignore CommonJS writes when `module` is shadowed in module scope.
#[test]
fn test_collect_commonjs_state_ignores_shadowed_module_name() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
const module = {};
function selected() {
    return 1;
}

module.exports = selected;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);
    assert!(state.default_value.is_none());
}

/// Ignore CommonJS writes when `exports` is shadowed in module scope.
#[test]
fn test_collect_commonjs_state_ignores_shadowed_exports_name() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
const exports = {};
function helper() {
    return 2;
}

exports.helper = helper;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);
    assert!(state.named_values.is_empty());
}

/// Ignore CommonJS assignments that only occur inside nested function scopes.
#[test]
fn test_collect_commonjs_state_ignores_nested_scope_assignments() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function selected() {
    return 1;
}

function helper() {
    return 2;
}

function assignNested() {
    module.exports = selected;
    exports.helper = helper;
}
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);
    assert!(state.default_value.is_none());
    assert!(state.named_values.is_empty());
}

/// Ignore CommonJS property writes with non-static computed keys.
#[test]
fn test_collect_commonjs_state_ignores_non_static_computed_property_keys() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.js",
        r#"
function helper() {
    return 2;
}

const key = "helper";
module.exports[key] = helper;
"#,
    );

    test.resolve_module(module_id);
    test.compile();

    let state = collect_commonjs_state(&test, module_id);
    assert!(state.named_values.is_empty());
}
