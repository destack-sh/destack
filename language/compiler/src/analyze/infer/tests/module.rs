use super::*;
use crate::{AnalyzeError, CompilePhase, ResolveError, run_to_completion};
use destack_dir::{FloatType, SymbolSpace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParallelValueKind {
    IntLiteral,
    NumberPrimitive,
    IntPrimitive,
    Error,
    Other,
}

fn add_parallel_analyze_stress_modules(test: &TestProgram) -> ModuleId {
    test.add_module(
        "dep.ds",
        r#"
export enum Mode {
    A = 1,
    B = 2,
}

export function pick(flag: boolean): Mode {
    if (flag) {
        return Mode.A;
    }

    return Mode.B;
}
"#,
    );
    test.add_module(
        "bridge.ds",
        r#"
import { pick } from "./dep.ds";

export function wrap(flag: boolean) {
    return pick(flag);
}
"#,
    );
    test.add_module(
        "leaf.ds",
        r#"
import { wrap } from "./bridge.ds";

export const value = wrap(true);
"#,
    )
}

fn value_kind_for_symbol(view: &TestModuleView<'_>, symbol: GlobalSymbolId) -> ParallelValueKind {
    let type_id = view.expect_value_type_id(symbol);
    match view.types().get_type(type_id) {
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
        } => ParallelValueKind::IntLiteral,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        } => ParallelValueKind::NumberPrimitive,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(_)),
        } => ParallelValueKind::IntPrimitive,
        Type::Error => ParallelValueKind::Error,
        _ => ParallelValueKind::Other,
    }
}

fn expect_array_element_type(types: &TypeTable, type_id: LocalTypeId) -> LocalTypeId {
    match types.get_type(type_id) {
        Type::Array {
            element: Some(element),
            ..
        } => *element,
        other => panic!("expected array type, found {other:?}"),
    }
}

fn assert_type_references_symbol(
    types: &TypeTable,
    type_id: LocalTypeId,
    expected_symbol: GlobalSymbolId,
) {
    assert_type!(types, type_id, Type::Reference { symbol, .. } => {
        assert_eq!(*symbol, expected_symbol);
    });
}

fn assert_fixed_array_primitive_type(
    test: &TestProgram,
    view: &TestModuleView<'_>,
    symbol: GlobalSymbolId,
    expected_element: PrimitiveType,
    expected_count: i64,
) {
    let type_id = view.expect_value_type_id(symbol);
    let Type::ArraySized { element, count, .. } = view.types().get_type(type_id) else {
        panic!(
            "expected fixed-size array type, got {:?}",
            view.types().get_type(type_id)
        );
    };

    assert_type!(view.types(), *element, Type::TypeLiteral { value } => {
        assert_eq!(*value, TypeLiteral::Primitive(expected_element));
    });

    let count_literal = test
        .compiler
        .integer_literal_value_for_type_id(*count, view.types());
    assert_eq!(
        count_literal,
        Some(expected_count),
        "expected fixed-size array count to resolve to {expected_count}: count={:?}",
        view.types().get_type(*count),
    );
}

fn assert_struct_field_type_matches_instance_field(
    test: &TestProgram,
    view: &TestModuleView<'_>,
    struct_name_text: &str,
    field_name_text: &str,
) -> LocalTypeId {
    let strings = &test.program.strings;
    let struct_name = strings.intern(struct_name_text);
    let field_name = strings.intern(field_name_text);
    let struct_symbol = view.expect_declaration_symbol(struct_name);
    let instance_type_id = view.expect_instance_type_id(struct_symbol);
    let instance_field_type_id = view.expect_object_field_type(instance_type_id, field_name);
    let field_symbol = view.expect_struct_field_symbol(struct_name, field_name);
    let field_type_id = view.expect_value_type_id(field_symbol);

    assert_eq!(
        view.types().get_type(field_type_id),
        view.types().get_type(instance_field_type_id),
        "expected field symbol type to match instance field type for {struct_name_text}:{field_name_text}",
    );

    field_type_id
}

#[test]
fn test_analyze_import_item_error_keeps_valid_sibling_binding() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "dep.ts",
        r#"
export let bar = 1;
"#,
    );
    let module_id = test.add_module(
        "test.ts",
        r#"
import { foo as, bar } from "./dep";

let x = bar;
"#,
    );

    // run the pipeline and preserve later phases as no-op clean
    test.analyze_module(module_id);
    test.compile();
    test.check_no_diagnostics_for_phases(&[CompilePhase::Resolve, CompilePhase::Analyze]);

    // the valid sibling import still binds and infers through the malformed clause
    let view = test.view(module_id);
    let x_symbol = test
        .resolve_to_symbol("test.ts", "x")
        .expect("expected x symbol");
    let x_type_id = view.expect_value_type_id(x_symbol);
    assert_type!(
        view.types(),
        x_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

#[test]
fn test_analyze_struct_member_error_keeps_valid_sibling_field() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box {
    +
    y: int32
}

let value = Box { y: 1 };
value.y;
"#,
    );

    // run the pipeline and preserve later phases as no-op clean
    test.analyze_module(module_id);
    test.compile();
    test.check_no_diagnostics_for_phases(&[CompilePhase::Resolve, CompilePhase::Analyze]);

    // the struct still keeps its error slot and valid sibling member
    let view = test.view(module_id);
    let box_name = test.program.strings.intern("Box");
    let box_symbol = view.expect_declaration_symbol(box_name);
    let box_instance_type_id = view.expect_instance_type_id(box_symbol);
    let box_declaration_id = view
        .tree()
        .iter_node_ids_of_type::<Declaration>()
        .into_iter()
        .find(|declaration_id| {
            view.tree()
                .get(*declaration_id)
                .descriptor()
                .name
                .map(|name| name.string())
                == Some(box_name)
        })
        .expect("expected Box declaration");
    let Declaration::Struct { members, .. } = view.tree().get(box_declaration_id) else {
        panic!("expected struct declaration");
    };
    assert_eq!(members.len(), 2);
    match view.tree().get(members[0]) {
        Member::Error { .. } => {}
        other => panic!("expected member error slot, got {other:?}"),
    }
    match view.tree().get(members[1]) {
        Member::Field {
            key: Some(DynamicKey::Name(name)),
            ..
        } => {
            assert_string!(test.program, *name, "y");
        }
        other => panic!("expected field member, got {other:?}"),
    }

    // the valid sibling field still publishes both declaration and instance field types
    let y_name = test.program.strings.intern("y");
    let y_field_type_id = assert_struct_field_type_matches_instance_field(&test, &view, "Box", "y");
    let y_instance_type_id = view.expect_object_field_type(box_instance_type_id, y_name);
    assert_eq!(
        view.types().get_type(y_field_type_id),
        view.types().get_type(y_instance_type_id),
    );
    assert_type!(
        view.types(),
        y_field_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );

    // the constructed instance member access also keeps the expected inferred type
    let member_expression_id = view.root_expression_id(2);
    assert_type!(
        view.types(),
        view.expect_inferred_type_id(member_expression_id),
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Keep parallel analyze runs live and diagnostic clean for repeated module-graph work.
#[test]
fn test_parallel_analyze_repeated_runs_stay_live_and_clean() {
    for _ in 0..10 {
        let test = TestProgram::memory_parallel();
        let leaf_id = add_parallel_analyze_stress_modules(&test);

        test.analyze_module_and_check_clean(leaf_id);
    }
}

/// Keep sequential and parallel analyze aligned for a representative cross-module value.
#[test]
fn test_parallel_analyze_matches_sequential_value_kind() {
    let sequential = TestProgram::memory_sequential();
    let sequential_leaf = add_parallel_analyze_stress_modules(&sequential);
    sequential.analyze_module_and_check_clean(sequential_leaf);
    let sequential_view = sequential.view(sequential_leaf);
    let sequential_symbol = sequential
        .resolve_to_symbol("leaf.ds", "value")
        .expect("expected value symbol in sequential run");
    let sequential_kind = value_kind_for_symbol(&sequential_view, sequential_symbol);

    let parallel = TestProgram::memory_parallel();
    let parallel_leaf = add_parallel_analyze_stress_modules(&parallel);
    parallel.analyze_module_and_check_clean(parallel_leaf);
    let parallel_view = parallel.view(parallel_leaf);
    let parallel_symbol = parallel
        .resolve_to_symbol("leaf.ds", "value")
        .expect("expected value symbol in parallel run");
    let parallel_kind = value_kind_for_symbol(&parallel_view, parallel_symbol);

    assert_eq!(parallel_kind, sequential_kind);
    assert_ne!(parallel_kind, ParallelValueKind::Error);
}

/// Report imprecise primitive diagnostics for inferred Number constructor calls.
#[test]
fn test_no_imprecise_primitives_reports_number_constructor_call() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
    test.add_destack_config(
        r#"{
            "compilerOptions": {
                "noAny": false,
                "noImprecisePrimitives": true
            }
        }"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
let value = Number(1);
"#,
    );

    test.analyze_module(module_id);
    test.compile();

    test.check_has_diagnostic("EA806");
}

/// Merge global interface members across imported modules.
#[test]
fn test_merge_global_declarations_across_imports() {
    // arrange test modules
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    test.add_module(
        "a.ds",
        r#"
declare global {
    interface GlobalThing {
        value: number
    }
}
"#,
    );
    test.add_module(
        "b.ds",
        r#"
declare global {
    interface GlobalThing {
        label: string
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import "./a.ds";
import "./b.ds";

const thing: GlobalThing = { value: 1, label: "ok" };
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);

    // load module data for inspection
    let view = test.view(main_id);
    let module = test.program.module_descriptor(main_id);
    let module = module.as_ref();
    let profile = view.profile_id();
    let dir = test.artifact_dir(main_id, profile);
    let thing_name = test.program.strings.intern("thing");
    let global_key = StaticKey::Name(test.program.strings.intern("GlobalThing"));
    let global_group = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<_, ResolveError>(compiler.get_global_symbol_group(
                _context.revision(),
                main_id,
                profile,
                global_key,
                SymbolSpace::Type,
            ))
        },
    )
    .expect("failed to read global symbol group");
    let global_group = global_group.expect("missing global group for GlobalThing");
    assert_eq!(global_group.len(), 2);

    for global_symbol in &global_group {
        let remote_profile = test.default_profile_id(global_symbol.module_id);
        let remote_dir = test
            .compiler
            .require_artifact_dir_declared(
                test.program.current_revision(),
                global_symbol.module_id,
                remote_profile,
            )
            .expect("declare stage should be ready for merged global module test");
        assert!(
            remote_dir
                .types
                .get_instance_type_id(*global_symbol)
                .is_some(),
            "expected instance type for GlobalThing in module {:?}",
            global_symbol.module_id,
        );
    }

    // locate the bound symbol for thing in the module scope
    let namespace_scope = view.symbols().get_scope_by_id(dir.namespace_scope);
    let thing_symbol = view
        .symbols()
        .active_named_symbols(namespace_scope)
        .find_map(|(key, symbol_id)| match key {
            StaticKey::Name(name_id) if name_id == thing_name => Some(symbol_id),
            _ => None,
        })
        .expect("missing symbol for thing")
        .into_global(main_id);
    let thing_entry = view.symbols().get_symbol(thing_symbol.local_id);
    assert!(
        thing_entry.target_symbol.is_none(),
        "unexpected target_symbol on thing binding",
    );
    assert!(
        thing_entry.canonical_symbol.is_none(),
        "unexpected canonical_symbol on thing binding",
    );
    let canonical_symbol = canonical_symbol_id(
        &test.compiler,
        module,
        view.symbols(),
        profile,
        thing_symbol,
        CanonicalSymbolMode::FollowAliases,
    );
    assert_eq!(canonical_symbol, thing_symbol);

    // assert the binding uses a nominal reference type
    let value_ty_id = view
        .types()
        .get_value_type_id(thing_symbol)
        .expect("missing value type for thing");
    let value_ty = view.types().get_type(value_ty_id);
    let Type::Reference {
        symbol: global_thing,
        ..
    } = value_ty
    else {
        panic!("expected reference type for thing");
    };
    assert_eq!(global_thing.ty(), SymbolType::Interface);

    // assert the merged instance type includes both fields
    let instance_ty_id = view
        .types()
        .get_instance_type_id(*global_thing)
        .expect("missing instance type for GlobalThing");
    let instance_ty = view.types().get_type(instance_ty_id);
    let Type::Object { fields, .. } = instance_ty else {
        panic!("expected object instance type for GlobalThing");
    };
    let value_key = StaticKey::Name(test.program.strings.intern("value"));
    let label_key = StaticKey::Name(test.program.strings.intern("label"));
    assert!(fields.iter().any(|field| field.key.matches(&value_key)));
    assert!(fields.iter().any(|field| field.key.matches(&label_key)));
}

/// Merge global interface members from local and remote declarations.
#[test]
fn test_merge_global_interface_members_include_local_and_remote_fields() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
declare global {
    interface GlobalThing {
        left: number
    }
}
"#,
    );
    test.add_module(
        "b.ds",
        r#"
declare global {
    interface GlobalThing {
        right: string
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import "./a.ds";
import "./b.ds";

declare global {
    interface GlobalThing {
        local: boolean
    }
}

const thing: GlobalThing = { left: 1, right: "ok", local: true };
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);

    // load module data for inspection
    let view = test.view(main_id);
    let profile = view.profile_id();
    let global_key = StaticKey::Name(test.program.strings.intern("GlobalThing"));
    let global_group = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<_, ResolveError>(compiler.get_global_symbol_group(
                _context.revision(),
                main_id,
                profile,
                global_key,
                SymbolSpace::Type,
            ))
        },
    )
    .expect("failed to read global symbol group");
    let global_group = global_group.expect("missing global group for GlobalThing");
    assert_eq!(global_group.len(), 3);
    let global_thing = global_group
        .iter()
        .find(|symbol| symbol.module_id == main_id)
        .copied()
        .expect("missing local GlobalThing symbol");
    let instance_ty_id = view.expect_instance_type_id(global_thing);
    let instance_ty = view.types().get_type(instance_ty_id);
    let Type::Object { fields, .. } = instance_ty else {
        panic!("expected object instance type for GlobalThing");
    };

    // assert merged fields include local and remote members
    let left_key = StaticKey::Name(test.program.strings.intern("left"));
    let right_key = StaticKey::Name(test.program.strings.intern("right"));
    let local_key = StaticKey::Name(test.program.strings.intern("local"));
    assert!(fields.iter().any(|field| field.key.matches(&left_key)));
    assert!(fields.iter().any(|field| field.key.matches(&right_key)));
    assert!(fields.iter().any(|field| field.key.matches(&local_key)));
}

/// Merge class and namespace global declarations into one value shape.
#[test]
fn test_merge_global_class_and_namespace_value_shapes() {
    // arrange test modules
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    test.add_module(
        "a.ds",
        r#"
declare global {
    class GlobalWidget {
        ping(): number;
    }
}
"#,
    );
    test.add_module(
        "b.ds",
        r#"
declare global {
    namespace GlobalWidget {
        export const tag: string;
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import "./a.ds";
import "./b.ds";

declare const widget: GlobalWidget;
widget.ping() satisfies number;

const tag = GlobalWidget.tag;
tag satisfies string;
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);

    // assert the projected static member preserves its declared type
    let view = test.view(main_id);
    let tag_symbol = test
        .resolve_to_symbol("main.ds", "tag")
        .expect("missing symbol for tag");
    let tag_ty_id = view.expect_value_type_id(tag_symbol);
    assert_type!(
        view.types(),
        tag_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );
}

/// Merge ambient Array members with local global Array augmentations.
#[test]
fn test_merge_global_array_augmentation_preserves_ambient_members() {
    // arrange test module with ambient libs enabled
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
    let main_id = test.add_module(
        "main.ds",
        r#"
declare global {
    interface Array<T> {
        first(): T | undefined;
    }
}
const values = [1, 2, 3];
const length = values.length;
values.first() satisfies number | undefined;
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);
    let view = test.view(main_id);
    let profile = view.profile_id();

    // resolve the local Array symbol from global groups
    let key = StaticKey::Name(test.program.strings.intern("Array"));
    let type_group = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            Ok::<_, ResolveError>(compiler.get_global_symbol_group(
                _context.revision(),
                main_id,
                profile,
                key,
                SymbolSpace::Type,
            ))
        },
    )
    .expect("failed to read Array group");
    let type_group = type_group.expect("missing Array group");
    let ambient_merge_group = test
        .compiler
        .get_library_symbol_sources_for_merge(profile, key, SymbolSpace::Type)
        .unwrap_or_default();
    let array_symbol = type_group
        .iter()
        .find(|symbol| symbol.module_id == main_id)
        .copied()
        .expect("missing local Array symbol");

    // assert the merged instance shape keeps ambient and local members
    let instance_ty_id = view.expect_instance_type_id(array_symbol);
    let instance_ty = view.types().get_type(instance_ty_id);
    let Type::Object {
        fields: local_fields,
        ..
    } = instance_ty
    else {
        panic!("expected object instance type for Array");
    };
    let first_key = StaticKey::Name(test.program.strings.intern("first"));
    let length_key = StaticKey::Name(test.program.strings.intern("length"));

    // read one ambient Array shape for baseline member preservation
    let ambient_symbol = ambient_merge_group
        .first()
        .copied()
        .expect("missing ambient Array symbol for merge baseline");
    let ambient_dir = test
        .compiler
        .require_artifact_dir_declared(
            test.program.current_revision(),
            ambient_symbol.module_id,
            profile,
        )
        .expect("declare stage should be ready for ambient Array merge baseline");
    let ambient_instance = ambient_dir
        .types
        .get_instance_type_id(ambient_symbol)
        .expect("missing ambient Array instance type");
    let ambient_instance = ambient_dir.types.get_type(ambient_instance);
    let Type::Object {
        fields: ambient_fields,
        ..
    } = ambient_instance
    else {
        panic!("expected ambient Array instance object type");
    };
    let ambient_keys = ambient_fields
        .iter()
        .map(|field| field.key)
        .collect::<Vec<_>>();
    // assert merged shape preserves at least one ambient member
    assert!(
        ambient_keys.iter().any(|ambient_key| local_fields
            .iter()
            .any(|field| field.key.matches(ambient_key))),
        "missing ambient members on merged Array shape",
    );

    // assert merged shape keeps the local augmentation member
    assert!(
        local_fields
            .iter()
            .any(|field| field.key.matches(&first_key)),
        "missing local member 'first' on merged Array shape",
    );
    assert!(
        local_fields
            .iter()
            .any(|field| field.key.matches(&length_key)),
        "missing merged length member",
    );

    // assert ambient length member type stays numeric under global augmentation
    let length_symbol = test
        .resolve_to_symbol("main.ds", "length")
        .expect("missing symbol for length");
    let length_ty_id = view.expect_value_type_id(length_symbol);
    match view.types().get_type(length_ty_id) {
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number),
        } => {}
        other => panic!("unexpected length type: {other:?}"),
    }
}

/// Remap merged generic members into the active receiver parameter space.
#[test]
fn test_merge_generic_interface_members_preserve_receiver_substitutions() {
    // arrange one merged generic global interface
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
declare global {
    interface Box<T> {
        first(): T | undefined;
    }
}
"#,
    );
    test.add_module(
        "b.ds",
        r#"
declare global {
    interface Box<T> {
        last(): T | undefined;
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import "./a.ds";
import "./b.ds";

declare let box: Box<number>;
box.first() satisfies number | undefined;
box.last() satisfies number | undefined;
"#,
    );

    // analyze the entry module
    test.analyze_module_and_check_clean(main_id);
}

/// Analyze cross module type import.
#[test]
fn test_analyze_cross_module_type_import() {
    // import a value from another module and verify its type is correctly imported
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let value = 42;
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { value } from "./lib.ds";
let x = value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);
    let x_symbol = view.expect_binding_symbol(test.program.strings.intern("x"));
    let x_ty_id = view.expect_value_type_id(x_symbol);

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Resolve import type operators to exported type members.
#[test]
fn test_analyze_import_type_accesses_exported_type_member() {
    // arrange producer and consumer modules
    let test = TestProgram::memory_sequential();
    test.add_module(
        "mod.ds",
        r#"
export type User = { name: string };
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
type Alias = import("./mod.ds").User;

const value: Alias = { name: "Ada" };
value.name satisfies string;
"#,
    );

    // analyze should stay clean and resolve the import type member
    test.analyze_module_and_check_clean(module_id);
}

/// Resolve import type operators to default exported type members.
#[test]
fn test_analyze_import_type_accesses_default_export_member() {
    // arrange producer and consumer modules
    let test = TestProgram::memory_sequential();
    test.add_module(
        "mod.ds",
        r#"
export default interface User {
    name: string;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
type Default = import("./mod.ds").default;

const value: Default = { name: "Ada" };
value.name satisfies string;
"#,
    );

    // analyze should stay clean and resolve the default import type member
    test.analyze_module_and_check_clean(module_id);
}

/// Resolve import type operators through re-exported type aliases.
#[test]
fn test_analyze_import_type_accesses_reexported_type_alias_member() {
    // arrange producer, barrel, and consumer modules
    let test = TestProgram::memory_sequential();
    test.add_module(
        "user.ds",
        r#"
export type User = { name: string };
"#,
    );
    test.add_module(
        "index.ds",
        r#"
export type { User } from "./user.ds";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
type Alias = import("./index.ds").User;

const value: Alias = { name: "Ada" };
value.name satisfies string;
"#,
    );

    // analyze should stay clean and resolve the re-exported import type member
    test.analyze_module_and_check_clean(module_id);
}

/// Require type-import infer dependencies before resolving imported type members.
#[test]
fn test_require_type_import_dependencies_for_infer_collects_direct_owner() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "mod.ds",
        r#"
export type User = { name: string };
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
type Alias = import("./mod.ds").User;
"#,
    );

    test.resolve_module(module_id);
    test.compile_check_clean();

    let profile = test.default_profile_id(module_id);
    let module = test.program.module_descriptor(module_id);
    run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            let dir = compiler
                .require_artifact_dir_resolved(_context.revision(), module_id, profile)
                .map_err(AnalyzeError::from)?;
            compiler.require_type_import_dependencies_for_infer(
                test.program.current_revision(),
                module.as_ref(),
                profile,
                &dir.tree,
            )
        },
    )
    .unwrap_or_else(|error| panic!("failed to require type import dependencies: {error:?}"));

    let module = test.program.module_descriptor(module_id);
    let dir = test.dir_resolved(module_id);

    let type_import = dir
        .tree
        .iter_nodes_of_type::<Expression>()
        .find_map(|(expression_id, expression)| match expression {
            Expression::TypeImport {
                target, qualifier, ..
            } => Some((expression_id, *target, qualifier.clone())),
            _ => None,
        })
        .expect("missing type import expression");
    let Expression::ScalarLiteral { value } = dir.tree.get(type_import.1) else {
        panic!("expected string target");
    };
    let ScalarLiteral::String(target) = value else {
        panic!("expected string target");
    };

    let resolved_symbol = run_to_completion(
        &test.compiler,
        test.program.current_revision(),
        |compiler, _context| {
            compiler.resolve_import_type_symbol(
                test.program.current_revision(),
                module.as_ref(),
                profile,
                type_import.0.into_any(),
                *target,
                type_import.2.as_ref(),
            )
        },
    );
    let resolved_symbol = resolved_symbol
        .unwrap_or_else(|error| panic!("failed to resolve import type symbol: {error:?}"));

    assert!(
        resolved_symbol.is_some(),
        "expected resolved import type symbol"
    );
}

/// Analyze cross module array type import.
#[test]
fn test_analyze_cross_module_array_type_import() {
    // import an array value from another module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let items = [1, 2, 3];
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { items } from "./lib.ds";
let x = items;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have an int32 array element type
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(view.types(), x_ty_id, Type::Array { element: Some(element_id), .. } => {
        match view.types().get_type(*element_id) {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
            } => {}
            Type::Union { elements } => {
                // each element should be int32 or a literal integer
                for element_id in elements {
                    match view.types().get_type(*element_id) {
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)),
                        } => {}
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_)),
                        } => {}
                        other => panic!("unexpected array element type: {other:?}"),
                    }
                }
            }
            other => panic!("unexpected array element type: {other:?}"),
        }
    });
}

/// Analyze cross module string type import.
#[test]
fn test_analyze_cross_module_string_type_import() {
    // import a string value from another module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export let greeting = "hello";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { greeting } from "./lib.ds";
let x = greeting;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // x should have string type (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

    assert_type!(
        view.types(),
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );
}

/// Analyze cross module member access using declared shapes.
#[test]
fn test_analyze_cross_module_member_access_declared_shape() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export interface Box {
    value: number
}

export let boxed: Box = { value: 1 };
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { boxed } from "./lib.ds";

let value = boxed.value;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // value should be number from the declared field type
    let value_symbol = test.resolve_to_symbol("main.ds", "value").unwrap();
    let value_ty_id = view.types().get_value_type_id(value_symbol).unwrap();

    assert_type!(
        view.types(),
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze cross module extension associated comptime value projection.
#[test]
fn test_analyze_cross_module_extension_associated_comptime_value_projection() {
    // arrange contract and owner modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "contract.ds",
        r#"
export interface RetryPolicy {
    comptime const MaxRetries: number;
    type Budget = uint8[this.MaxRetries];
}
"#,
    );
    test.add_module(
        "owner.ds",
        r#"
import { RetryPolicy } from "./contract";

export struct HttpRetryPolicy {}

extension of HttpRetryPolicy implements RetryPolicy {
    comptime const MaxRetries: number = 5;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { HttpRetryPolicy } from "./owner";

declare const budget: HttpRetryPolicy.Budget;
let retries = HttpRetryPolicy.MaxRetries;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // retries should resolve to number
    let retries_symbol = test.resolve_to_symbol("main.ds", "retries").unwrap();
    let retries_ty_id = view.types().get_value_type_id(retries_symbol).unwrap();

    assert_type!(
        view.types(),
        retries_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Report missing inherited abstract associated requirements for imported concrete subclasses.
#[test]
fn test_analyze_reports_missing_associated_requirements_for_imported_concrete_subclass() {
    // arrange owner modules with an abstract requirement and a concrete subclass that omits it
    let test = TestProgram::memory_sequential();
    test.add_module(
        "base.ds",
        r#"
export abstract class Base<T> {
    type Item;
}
"#,
    );
    test.add_module(
        "derived.ds",
        r#"
import { Base } from "./base";

export class Derived extends Base<int32> {}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { Derived } from "./derived";

declare const value: Derived;
value satisfies Derived;
"#,
    );

    // analyze should report missing associated requirement diagnostics
    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA125");
}

/// Enforce implemented contract member compatibility even when the contract declares associated requirements.
#[test]
fn test_analyze_reports_member_mismatch_for_contract_with_associated_requirements() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "contract.ds",
        r#"
export interface Service {
    type Item;
    ping(value: string): number;
}
"#,
    );
    test.add_module(
        "impl.ds",
        r#"
import { Service } from "./contract";

export class Concrete implements Service {
    type Item = string;

    ping(value: int32): number {
        return 0;
    }
}
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { Concrete } from "./impl";

declare const value: Concrete;
value satisfies Concrete;
"#,
    );

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA101");
}

/// Keep strict bind-call-apply arity tied to required parameter syntax, not `undefined` assignability.
#[test]
fn test_strict_call_arity_respects_required_parameter_with_undefined_union() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "main.ds",
        r#"
function invoke(this: { base: number }, value: number | undefined): number {
    return this.base;
}

invoke.call({ base: 1 });
"#,
    );
    test.apply_destack_config(
        module_id,
        r#"{
            "compilerOptions": {
                "strictBindCallApply": true
            }
        }"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA236");
}

/// Restrict strict bind-call-apply arity checks to the builtin function wrapper surface.
#[test]
fn test_strict_call_arity_skips_user_defined_call_members() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es5"]);
    let module_id = test.add_module(
        "main.ds",
        r#"
declare const weird: ((value: number) => number) & {
    call(thisArg: number, value: number): number;
};

weird.call(0);
"#,
    );
    test.apply_destack_config(
        module_id,
        r#"{
            "compilerOptions": {
                "strictBindCallApply": true
            }
        }"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_no_diagnostic_code("EA236");
}

/// Reject value-position uses of type-only exports forwarded through export-star barrels.
#[test]
fn test_export_star_forwarded_type_only_exports_reject_value_use() {
    let test = TestProgram::memory_sequential();
    test.add_file(
        "types.ts",
        r#"
export type User = { name: string };
export const value = 1;
"#,
    );
    test.add_file(
        "module-b.ts",
        r#"
export * from "./types";
"#,
    );
    let main_id = test.add_module(
        "main.ts",
        r#"
import { value, User } from "./module-b";

value satisfies number;
type Alias = User;
User;
"#,
    );
    test.apply_destack_config(
        main_id,
        r#"{
            "compiler": {
                "checkTs": true,
                "checkJs": true
            }
        }"#,
    );

    test.resolve_module(main_id);
    test.compile_check_clean();

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA117");
}

/// Reject value-position uses of type-only reexports in one direct analyze pass.
#[test]
fn test_export_type_reexport_rejects_value_use_in_one_pass() {
    let test = TestProgram::memory_sequential();
    test.add_file(
        "types.ts",
        r#"
export type User = { name: string };
"#,
    );
    let main_id = test.add_module(
        "main.ts",
        r#"
export type { User } from "./types";
import { User } from "./main";

type Alias = User;
User;
"#,
    );
    test.apply_destack_config(
        main_id,
        r#"{
            "compiler": {
                "checkTs": true,
                "checkJs": true
            }
        }"#,
    );

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA117");
}

/// Reject value-position uses of type-only export-star reexports in one direct analyze pass.
#[test]
fn test_export_type_star_reexport_rejects_value_use_in_one_pass() {
    let test = TestProgram::memory_sequential();
    test.add_file(
        "types.ts",
        r#"
export type User = { name: string };
export const value = 1;
"#,
    );
    test.add_file(
        "mod.ts",
        r#"
export * from "./types";
"#,
    );
    let main_id = test.add_module(
        "main.ts",
        r#"
import { value, User } from "./mod";

value satisfies number;
type Alias = User;
User;
"#,
    );
    test.apply_destack_config(
        main_id,
        r#"{
            "compiler": {
                "checkTs": true,
                "checkJs": true
            }
        }"#,
    );

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA117");
}

/// Reject unannotated export inference cycles in one direct analyze pass.
#[test]
fn test_export_inference_cycle_requires_annotation_in_one_pass() {
    let test = TestProgram::memory_sequential();
    test.add_file(
        "a.ts",
        r#"
import { y } from "./b";

export const x = y;
"#,
    );
    let main_id = test.add_module(
        "b.ts",
        r#"
import { x } from "./a";

export const y = x;
"#,
    );
    test.apply_destack_config(
        main_id,
        r#"{
            "compiler": {
                "checkTs": true,
                "checkJs": true
            }
        }"#,
    );

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA116");
}

/// Analyze cross module extension associated comptime value projection through re-exports.
#[test]
fn test_analyze_cross_module_extension_associated_comptime_value_projection_through_reexport() {
    // arrange contract, owner, and barrel modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "contract.ds",
        r#"
export interface RetryPolicy {
    comptime const MaxRetries: number;
}
"#,
    );
    test.add_file(
        "owner.ds",
        r#"
import { RetryPolicy } from "./contract";

export struct HttpRetryPolicy {}

extension of HttpRetryPolicy implements RetryPolicy {
    comptime const MaxRetries: number = 5;
}
"#,
    );
    test.add_file(
        "barrel.ds",
        r#"
export { HttpRetryPolicy } from "./owner";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { HttpRetryPolicy } from "./barrel";
import * as ownerNs from "./owner";

let retries = HttpRetryPolicy.MaxRetries;
let ownerRetries = ownerNs.HttpRetryPolicy.MaxRetries;
"#,
    );

    // run analyze pipeline
    test.analyze_module_and_check_clean(module_id);

    // load typed module data
    let view = test.view(module_id);

    // retries should resolve to the inferred comptime literal type from the re-export path
    let retries_symbol = test.resolve_to_symbol("main.ds", "retries").unwrap();
    let retries_ty_id = view.types().get_value_type_id(retries_symbol).unwrap();
    assert_type!(
        view.types(),
        retries_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );

    // ownerRetries should resolve to the inferred comptime literal type from the namespace import path
    let owner_retries_symbol = test.resolve_to_symbol("main.ds", "ownerRetries").unwrap();
    let owner_retries_ty_id = view
        .types()
        .get_value_type_id(owner_retries_symbol)
        .unwrap();
    assert_type!(
        view.types(),
        owner_retries_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Analyze associated-comptime projections through multi-hop re-export chains.
#[test]
fn test_analyze_cross_module_associated_comptime_projection_through_multi_hop_reexports() {
    // arrange owner and barrel modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "owner.ds",
        r#"
export class PacketOwner<Row> {
    comptime const Width: number = Row extends string ? 8 : 2;
}

"#,
    );
    test.add_file(
        "barrel1.ds",
        r#"
export { PacketOwner } from "./owner";
"#,
    );
    test.add_file(
        "barrel2.ds",
        r#"
export { PacketOwner } from "./barrel1";
"#,
    );
    test.add_file(
        "barrel3.ds",
        r#"
export * from "./barrel2";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { PacketOwner } from "./barrel3";

let width = PacketOwner<string>.Width;
"#,
    );

    // analyze should preserve associated projections through re-exports
    test.analyze_module_and_check_clean(module_id);

    // width should resolve to int32
    let view = test.view(module_id);
    let width_symbol = test.resolve_to_symbol("main.ds", "width").unwrap();
    let width_ty_id = view.types().get_value_type_id(width_symbol).unwrap();
    assert_type!(
        view.types(),
        width_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Preserve fixed-array disambiguation for namespace imported associated comptime lengths.
#[test]
fn test_analyze_namespace_import_associated_comptime_fixed_array_disambiguation() {
    // arrange layout and namespace-import consumer modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "layout.ds",
        r#"
export class Segment<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import * as api from "./layout";

type Lane<Row> = uint8[api.Segment<Row>.Width as comptime];

declare const lane: Lane<string>;
lane satisfies uint8[8];
"#,
    );

    // analyze and compile the namespace-imported associated comptime projection
    test.analyze_module_and_check_clean(module_id);
    test.compile();

    // preserve the fixed-array count after namespace-imported associated comptime materialization
    let view = test.view(module_id);
    let lane_symbol = test
        .resolve_to_symbol("main.ds", "lane")
        .expect("expected lane symbol");
    let lane_type_id = view.expect_value_type_id(lane_symbol);
    let Type::ArraySized { element, .. } = view.types().get_type(lane_type_id) else {
        panic!(
            "expected namespace-imported lane to analyze as a fixed array, got {:?}",
            view.types().get_type(lane_type_id),
        );
    };

    assert_type!(view.types(), *element, Type::TypeLiteral { value } => {
        assert_eq!(*value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Uint8)));
    });
}

/// Analyze multi-hop re-exported associated contract aliases through imported implementors.
#[test]
fn test_analyze_cross_module_associated_contract_alias_projection_through_multi_hop_reexports() {
    // arrange contract, implementor, and barrel modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "contract.ds",
        r#"
export interface PacketOwner<Row> {
    comptime const Width: number = Row extends string ? 8 : 2;
    type Lane = uint8[this.Width];
    type Packet = this.Lane;
}
"#,
    );
    test.add_file(
        "owner.ds",
        r#"
import type { PacketOwner } from "./contract";

export class Packet<Row> implements PacketOwner<Row> {}
"#,
    );
    test.add_file(
        "barrel1.ds",
        r#"
export { Packet } from "./owner";
"#,
    );
    test.add_file(
        "barrel2.ds",
        r#"
export { Packet } from "./barrel1";
"#,
    );
    test.add_file(
        "barrel3.ds",
        r#"
export * from "./barrel2";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Packet } from "./barrel3";

declare const lane: Packet<string>.Lane;
lane satisfies uint8[8];

declare const packet: Packet<string>.Packet;
packet satisfies uint8[8];
"#,
    );

    // analyze should preserve contract-owned associated alias projections through barrels
    test.analyze_module_and_check_clean(module_id);
}

/// Analyze multi-hop re-exported associated comptime defaults used by contract aliases.
#[test]
fn test_analyze_cross_module_associated_comptime_contract_alias_projection_through_multi_hop_reexports()
 {
    // arrange contract, implementor, and barrel modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "contract.ds",
        r#"
export interface TileShape<Row> {
    comptime const Width: number = Row extends string ? 8 : 4;
    comptime const DoubleWidth: number = this.Width * 2;
    type Tile = uint8[this.DoubleWidth];
}
"#,
    );
    test.add_file(
        "owner.ds",
        r#"
import type { TileShape } from "./contract";

export class Packet<Row> implements TileShape<Row> {}
"#,
    );
    test.add_file(
        "barrel1.ds",
        r#"
export { Packet } from "./owner";
"#,
    );
    test.add_file(
        "barrel2.ds",
        r#"
export { Packet } from "./barrel1";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Packet } from "./barrel2";

const width = Packet<string>.DoubleWidth;
width satisfies number;

declare const tile: Packet<string>.Tile;
tile satisfies uint8[16];
"#,
    );

    // analyze should preserve associated comptime default substitution through barrels
    test.analyze_module_and_check_clean(module_id);
}

/// Analyze inherited class associated comptime aliases through concrete overrides.
#[test]
fn test_analyze_class_associated_comptime_alias_projection_through_abstract_owner() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
abstract class BatchPlan<Row> {
    abstract comptime const SegmentRows: number;
    type SegmentRowsType = SegmentRows;
}

class LogBatch extends BatchPlan<string> {
    comptime const SegmentRows: number = 256;
}

declare const segmentRows: LogBatch.SegmentRowsType;
segmentRows satisfies 256;
"#,
    );
    let view = test.view(module_id);
    let types = view.types();
    let segment_rows_name = test.program.strings.intern("segmentRows");
    let segment_rows_symbol = view.expect_binding_symbol(segment_rows_name);
    let segment_rows_type_id = view.expect_value_type_id(segment_rows_symbol);

    let segment_rows_literal = test
        .compiler
        .integer_literal_value_for_type_id(segment_rows_type_id, types);

    assert_eq!(
        segment_rows_literal,
        Some(256),
        "expected inherited associated comptime alias to resolve to 256: segment_rows={:?}",
        types.get_type(segment_rows_type_id),
    );
}

/// Analyze interface associated comptime aliases through one concrete implementor.
#[test]
fn test_analyze_interface_associated_comptime_alias_projection_through_concrete_implementor() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
interface PartitionedStore<Row> {
    comptime const SegmentBytes: number;
    type Segment = Row[this.SegmentBytes];
}

class AuditStore implements PartitionedStore<string> {
    comptime const SegmentBytes: number = 1024;
}

declare const segment: AuditStore.Segment;
segment satisfies string[1024];
"#,
    );
    let view = test.view(module_id);
    let segment_name = test.program.strings.intern("segment");
    let segment_symbol = view.expect_binding_symbol(segment_name);

    assert_fixed_array_primitive_type(&test, &view, segment_symbol, PrimitiveType::String, 1024);
}

/// Analyze one shared associated comptime implementation across compatible contracts.
#[test]
fn test_analyze_multi_contract_associated_comptime_alias_projection_from_shared_implementation() {
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
interface RowsContract<T> {
    comptime const Width: number;
    type Row = T[this.Width];
}

interface TileContract<T> {
    comptime const Width: number;
    type Tile = T[this.Width][2];
}

class DensePlan implements RowsContract<float32>, TileContract<float32> {
    comptime const Width: number = 8;
}

declare const row: DensePlan.Row;
row satisfies float32[8];

declare const tile: DensePlan.Tile;
tile satisfies float32[8][2];
"#,
    );
    let view = test.view(module_id);
    let row_name = test.program.strings.intern("row");
    let row_symbol = view.expect_binding_symbol(row_name);
    assert_fixed_array_primitive_type(
        &test,
        &view,
        row_symbol,
        PrimitiveType::Float(FloatType::Float32),
        8,
    );

    let tile_name = test.program.strings.intern("tile");
    let tile_symbol = view.expect_binding_symbol(tile_name);
    let tile_type_id = view.expect_value_type_id(tile_symbol);
    let Type::ArraySized {
        element: inner_element,
        count: outer_count,
        ..
    } = view.types().get_type(tile_type_id)
    else {
        panic!(
            "expected nested fixed-size tile type, got {:?}",
            view.types().get_type(tile_type_id)
        );
    };
    let Type::ArraySized {
        element: scalar_element,
        count: inner_count,
        ..
    } = view.types().get_type(*inner_element)
    else {
        panic!(
            "expected nested fixed-size row type, got {:?}",
            view.types().get_type(*inner_element)
        );
    };

    assert_type!(view.types(), *scalar_element, Type::TypeLiteral { value } => {
        assert_eq!(*value, TypeLiteral::Primitive(PrimitiveType::Float(FloatType::Float32)));
    });

    let inner_count_literal = test
        .compiler
        .integer_literal_value_for_type_id(*inner_count, view.types());
    assert_eq!(inner_count_literal, Some(8));

    let outer_count_literal = test
        .compiler
        .integer_literal_value_for_type_id(*outer_count, view.types());
    assert_eq!(outer_count_literal, Some(2));
}

/// Preserve concrete inherited associated aliases through method and projection use sites.
#[test]
fn test_analyze_interface_inheritance_preserves_concrete_associated_aliases() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Source<T> {
    type Item;
}

interface Stream<T> extends Source<T> {
    next(): Source<T>.Item;
}

class Counter implements Stream<int32> {
    value: int32 = 0;

    type Item = int32;

    next(): Item {
        return this.value;
    }
}

declare const value: Counter.Item;
value satisfies int32;
"#,
    );

    test.analyze_module_and_check_clean(module_id);

    let view = test.view(module_id);
    let value_symbol = test
        .resolve_to_symbol("test.ds", "value")
        .expect("expected value symbol");
    let value_type_id = view.expect_value_type_id(value_symbol);

    assert_type!(view.types(), value_type_id, Type::TypeLiteral { value } => {
        assert_eq!(*value, TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32)));
    });
}

/// Reject unresolved imported generic associated comptime projections in value position.
#[test]
fn test_analyze_cross_module_associated_comptime_projection_rejects_unresolved_imported_generic_value_usage()
 {
    // arrange owner module with a generic associated comptime member
    let test = TestProgram::memory_sequential();
    test.add_file(
        "plan.ds",
        r#"
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { SegmentPlan } from "./plan";

function unresolved<Row>() {
    SegmentPlan<Row>.SegmentBytes;
}
"#,
    );

    // run analyze and require unresolved projection diagnostics
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA125");
}

/// Reject unresolved imported generic associated comptime projections through re-exports.
#[test]
fn test_analyze_cross_module_associated_comptime_projection_rejects_unresolved_imported_generic_value_usage_through_reexports()
 {
    // arrange owner module and re-export barrel for a generic associated comptime member
    let test = TestProgram::memory_sequential();
    test.add_file(
        "plan.ds",
        r#"
export class SegmentPlan<Row> {
    comptime const SegmentBytes: number = Row extends string ? 4096 : 1024;
}
"#,
    );
    test.add_file(
        "barrel.ds",
        r#"
export { SegmentPlan } from "./plan";
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { SegmentPlan } from "./barrel";

function unresolved<Row>() {
    SegmentPlan<Row>.SegmentBytes;
}
"#,
    );

    // run analyze and require unresolved projection diagnostics
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA125");
}

/// Report one static-argument cycle for recursive cross-module associated comptime projections.
#[test]
fn test_analyze_cross_module_associated_comptime_projection_cycle_reports_static_argument_cycle() {
    // arrange owner modules with mutually recursive associated comptime members
    let test = TestProgram::memory_sequential();
    test.add_file(
        "a.ds",
        r#"
import type { Right } from "./b";

export class Left<T> {
    comptime const Width: number = Right<T>.Height;
}
"#,
    );
    test.add_file(
        "b.ds",
        r#"
import type { Left } from "./a";

export class Right<T> {
    comptime const Height: number = Left<T>.Width;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Left } from "./a";

function unresolved() {
    Left<string>.Width;
}
"#,
    );

    // run analyze and require a deterministic static cycle diagnostic
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA112");
}

/// Report static-argument cycles through re-exported associated-comptime projections.
#[test]
fn test_analyze_cross_module_associated_comptime_projection_cycle_through_reexports_reports_static_argument_cycle()
 {
    // arrange mutually recursive projections with a multi-hop re-export
    let test = TestProgram::memory_sequential();
    test.add_file(
        "a.ds",
        r#"
import type { Right } from "./bridge-a";

export class Left<T> {
    comptime const Width: number = Right<T>.Height;
}
"#,
    );
    test.add_file(
        "bridge-a.ds",
        r#"
export { Right } from "./bridge-b";
"#,
    );
    test.add_file(
        "bridge-b.ds",
        r#"
export { Right } from "./b";
"#,
    );
    test.add_file(
        "b.ds",
        r#"
import type { Left } from "./a";

export class Right<T> {
    comptime const Height: number = Left<T>.Width;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Left } from "./a";

function unresolved() {
    Left<string>.Width;
}
"#,
    );

    // analyze should report one deterministic static cycle diagnostic
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA112");
}

/// Prefer static-cycle diagnostics over unresolved-projection diagnostics for generic cycles.
#[test]
fn test_analyze_cross_module_generic_associated_comptime_cycle_reports_static_argument_cycle_without_unresolved_projection()
 {
    // arrange mutually recursive generic projections
    let test = TestProgram::memory_sequential();
    test.add_file(
        "a.ds",
        r#"
import type { Right } from "./b";

export class Left<Row> {
    comptime const Width: number = Right<Row>.Height;
}
"#,
    );
    test.add_file(
        "b.ds",
        r#"
import type { Left } from "./a";

export class Right<Row> {
    comptime const Height: number = Left<Row>.Width;
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Left } from "./a";

function unresolved<Row>() {
    Left<Row>.Width;
}
"#,
    );

    // analyze should report one static cycle and avoid unresolved projection fallback diagnostics
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA112");
    test.check_no_diagnostic_code("EA125");
}

/// Enforce imported generic bounds from declare-published static-parameter constraints.
#[test]
fn test_analyze_cross_module_imported_static_constraint_enforced() {
    // arrange producer and consumer modules
    let test = TestProgram::memory_sequential();
    test.add_file(
        "lib.ds",
        r#"
export function readName<T extends { name: string }>(value: T): string {
    value.name
}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { readName } from "./lib";

let ok = readName({ name: "ok" });
let bad = readName({ name: 1 });
"#,
    );

    // analyze should report one incompatible call argument diagnostic
    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA101");
}

/// Keep associated-comptime projections stable across type-only cross-module cycles.
#[test]
fn test_analyze_cross_module_type_cycle_preserves_associated_comptime_projection() {
    // arrange type-only cyclic imports with associated-comptime projections
    let test = TestProgram::memory_sequential();
    test.add_file(
        "a.ds",
        r#"
import type { Right } from "./b";

export interface Left<T> {
    comptime const Width: number = T extends string ? 4 : 2;
    type Lane = uint8[this.Width];
}

export type LeftLane = Right<string>.Lane;
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import type { Left } from "./a";

export class Right<T> implements Left<T> {}
"#,
    );
    let module_id = test.add_module(
        "main.ds",
        r#"
import { Right } from "./b";

declare const lane: Right<string>.Lane;
lane satisfies uint8[4];
"#,
    );

    // analyze should converge without dropping associated projection metadata
    test.analyze_module_and_check_clean(module_id);
}

/// Analyze circular imports with a declared export anchor.
#[test]
fn test_analyze_cross_module_circular_imports_with_declared_anchor() {
    // arrange circular modules with a declared anchor
    let test = TestProgram::memory_sequential();
    let b_module_id = test.add_module(
        "b.ds",
        r#"
import { a } from "./a.ds";

export let b = a;
"#,
    );
    let a_module_id = test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";

export let a: number = b;
"#,
    );

    // run analyze pipeline for the anchor module
    test.analyze_module_and_check_clean(a_module_id);

    // load typed module data
    let view = test.view(b_module_id);

    // b should pick up the declared number type from a
    let b_symbol = test.resolve_to_symbol("b.ds", "b").unwrap();
    let b_ty_id = view.types().get_value_type_id(b_symbol).unwrap();

    assert_type!(
        view.types(),
        b_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze export chain surface inference.
#[test]
fn test_analyze_export_chain_surface_inference() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.analyze_module_with_source(
        "test.ds",
        r#"
export let a = 1;
export let b = a;
"#,
    );

    // load typed module data
    let view = test.view(module_id);

    // locate exported declarators
    let a_name = test.program.strings.intern("a");
    let b_name = test.program.strings.intern("b");
    let a_declarator_id = view.expect_let_declarator(a_name);
    let b_declarator_id = view.expect_let_declarator(b_name);

    // resolve binding symbols
    let a_pattern = view.tree().get(view.tree().get(a_declarator_id).pattern);
    let b_pattern = view.tree().get(view.tree().get(b_declarator_id).pattern);
    let Pattern::Binding {
        symbol: a_symbol, ..
    } = a_pattern
    else {
        panic!("expected binding");
    };
    let Pattern::Binding {
        symbol: b_symbol, ..
    } = b_pattern
    else {
        panic!("expected binding");
    };

    // read value types
    let a_ty_id = view
        .types()
        .get_value_type_id(a_symbol.into_global(module_id))
        .expect("expected a type");
    let b_ty_id = view
        .types()
        .get_value_type_id(b_symbol.into_global(module_id))
        .expect("expected b type");

    // both exports resolve to the int32 type
    assert_type!(
        view.types(),
        a_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
    assert_type!(
        view.types(),
        b_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Keep export surface initializer commitment aligned with local declarator commitment.
#[test]
fn test_analyze_export_surface_satisfies_uses_same_commitment_as_local_declarators() {
    // arrange module with equivalent local and exported declarators
    let test = TestProgram::memory_parallel();
    let module_id = test.analyze_module_with_source(
        "producer.ds",
        r#"
let local = "alpha" satisfies string;
export let exported = "alpha" satisfies string;
"#,
    );

    // read local and exported value types
    let view = test.view(module_id);
    let local_symbol = test
        .resolve_to_symbol("producer.ds", "local")
        .expect("expected local symbol");
    let exported_symbol = test
        .resolve_to_symbol("producer.ds", "exported")
        .expect("expected exported symbol");
    let local_type_id = view
        .types()
        .get_value_type_id(local_symbol)
        .expect("expected local type");
    let exported_type_id = view
        .types()
        .get_value_type_id(exported_symbol)
        .expect("expected exported type");

    // local and exported commitment paths should produce the same type shape
    let local_ty = view.types().get_type(local_type_id).clone();
    let exported_ty = view.types().get_type(exported_type_id).clone();
    assert_eq!(
        local_ty, exported_ty,
        "expected local and exported declarator commitment to agree",
    );
}

// re-exported data loader overrides keep text semantics at the consumer import site
#[test]
fn test_analyze_reexport_text_loader_attribute_infers_string() {
    let test = TestProgram::memory_parallel();
    test.add_file("data.json", r#"{ "key": "value" }"#);
    test.add_module(
        "bridge.ds",
        r#"
export { default as text } from "./data.json" with { type: "text" };
"#,
    );
    let main_id = test.analyze_module_with_source(
        "main.ds",
        r#"
import { text } from "./bridge.ds";
let value = text;
"#,
    );

    let view = test.view(main_id);
    let text_symbol = test
        .resolve_to_symbol("main.ds", "text")
        .expect("expected imported text symbol");
    let text_ty_id = view
        .types()
        .get_value_type_id(text_symbol)
        .expect("missing value type for imported text symbol");

    assert_type!(
        view.types(),
        text_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );
}

/// Reject incompatible assignment from typeof static method calls.
#[test]
fn test_analyze_reports_unassignable_type_for_typeof_static_method_result() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "main.ds",
        r#"
class Counter {
    static next(value: int32): int32 { return value + 1 }
}

type CounterCtor = typeof Counter;

let ctor: CounterCtor = Counter;
let badNext: string = ctor.next(1);
"#,
    );

    test.analyze_module(module_id);
    test.compile();
    test.check_has_diagnostic("EA101");
}

/// Keep template literal argument inference stable when relation checks defer to post-convergence reporting.
#[test]
fn test_analyze_template_literal_argument_inference_remains_stable_with_relation_obligations() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "main.ds",
        r#"
declare function takeAny<T extends string>(value: `${T}`): T;

declare let value: `prefix-${"a"}`;
let result = takeAny(value);
result satisfies `prefix-${"a"}`;
"#,
    );

    test.analyze_module_and_check_clean(module_id);
}

/// Keep associated projection satisfies checks stable after instance commitments and relation reporting.
#[test]
fn test_analyze_associated_projection_satisfies_after_convergence() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "main.ds",
        r#"
interface LocalCursor<T> {
    type Item;
    read(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension of Counter implements LocalCursor<int32> {
    type Item = int32;

    read(): Item {
        this.value
    }
}

function project<I: LocalCursor<int32>>(owner: I): I.Item {
    owner.read()
}

declare const counter: Counter;
project(counter) satisfies Counter.Item;
"#,
    );

    test.analyze_module_and_check_clean(module_id);
}

/// Preserve exported struct field semantics through star reexports.
#[test]
fn test_exported_struct_field_types_preserve_star_reexported_newtypes() {
    for skip_lib_check in [false, true] {
        let test = TestProgram::memory_sequential_with_prelude_and_libs();
        if skip_lib_check {
            test.add_destack_config(
                r#"
{ "compilerOptions": { "skipLibCheck": true } }
"#,
            );
        }
        test.add_module(
            "handles.ds",
            r#"
export newtype Handle = uint32;
"#,
        );
        test.add_module(
            "index.ds",
            r#"
export * from "./handles.ds";
"#,
        );
        let module_id = test.add_module(
            "types.ds",
            r#"
import { Handle } from "./index.ds";

export struct Container {
    items: Array<Handle>;
}
"#,
        );
        let consumer_id = test.add_module(
            "consumer.ds",
            r#"
import { Container } from "./types.ds";
import { Handle } from "./handles.ds";

declare let handle: Handle;

let value = Container { items: [handle] };
value.items;
"#,
        );

        test.analyze_module_and_check_clean(consumer_id);

        let view = test.declared_view(module_id);
        let container_symbol =
            view.expect_declaration_symbol(test.program.strings.intern("Container"));
        view.expect_value_type_id(container_symbol);
        let items_type_id =
            assert_struct_field_type_matches_instance_field(&test, &view, "Container", "items");
        let handle_symbol = test.canonical_symbol_for_path("handles.ds", "Handle");
        let element_type_id = expect_array_element_type(view.types(), items_type_id);

        assert_type_references_symbol(view.types(), element_type_id, handle_symbol);
    }
}

/// Publish exported rest-tuple newtype value surfaces without leaving unevaluated state behind.
#[test]
fn test_exported_rest_tuple_newtype_values_publish_cleanly() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    let module_id = test.add_module(
        "policy.ds",
        r#"
export type Capability = "fs.read" | "net.listen";

export newtype Require = (...Capability[]);
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ds",
        r#"
import { Require } from "./policy.ds";

let ctor = Require;
"#,
    );

    test.analyze_module_and_check_clean(consumer_id);

    let view = test.view(module_id);
    let require_symbol = test
        .resolve_to_symbol("policy.ds", "Require")
        .expect("expected Require symbol");
    view.expect_value_type_id(require_symbol);
}

/// Preserve declaration struct field semantics across import forms.
#[test]
fn test_declaration_struct_field_types_preserve_imported_newtypes() {
    let cases = [
        (
            "direct",
            r#"import type { Handle } from "./handles.d.ds";"#,
            None,
        ),
        (
            "reexport",
            r#"import type { Handle } from "./index.d.ds";"#,
            Some(
                r#"
export type { Handle } from "./handles.d.ds";
"#,
            ),
        ),
        ("value", r#"import { Handle } from "./handles.d.ds";"#, None),
    ];

    for (suffix, import_statement, index_source) in cases {
        let test = TestProgram::memory_sequential_with_prelude_and_libs();
        test.add_module(
            "handles.d.ds",
            r#"
export newtype Handle = uint32;
"#,
        );
        if let Some(index_source) = index_source {
            test.add_module("index.d.ds", index_source);
        }
        let module_id = test.add_module(
            &format!("types-{suffix}.d.ds"),
            &format!(
                r#"
{import_statement}

export struct Container {{
    items: Array<Handle>;
}}
"#,
            ),
        );
        let consumer_id = test.add_module(
            &format!("consumer-{suffix}.ds"),
            &format!(
                r#"
import {{ Container }} from "./types-{suffix}.d.ds";
import {{ Handle }} from "./handles.d.ds";

declare let handle: Handle;

let value = Container {{ items: [handle] }};
value.items;
"#,
            ),
        );

        test.analyze_module_and_check_clean(consumer_id);

        let view = test.view(module_id);
        let items_type_id =
            assert_struct_field_type_matches_instance_field(&test, &view, "Container", "items");
        let handle_symbol = test.canonical_symbol_for_path("handles.d.ds", "Handle");
        let element_type_id = expect_array_element_type(view.types(), items_type_id);

        assert_type_references_symbol(view.types(), element_type_id, handle_symbol);
    }
}

/// Preserve exported struct field semantics through imported newtypes and star reexports.
#[test]
fn test_exported_struct_field_types_preserve_imported_newtypes_through_star_reexports() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs();
    test.add_destack_config(
        r#"
{ "compilerOptions": { "skipLibCheck": true } }
"#,
    );
    test.add_module(
        "id.ds",
        r#"
export newtype ResourceId = uint64;

export declare namespace resource {}
"#,
    );
    test.add_module(
        "handles.ds",
        r#"
import { ResourceId } from "./id.ds";

export newtype SocketHandle = ResourceId;
export newtype TransferredHandle = ResourceId;
"#,
    );
    test.add_module(
        "index.ds",
        r#"
export { resource } from "./id.ds";
export * from "./handles.ds";
export * from "./id.ds";
"#,
    );
    let module_id = test.add_module(
        "types.ds",
        r#"
import { SocketHandle, TransferredHandle } from "./index.ds";

export struct Container {
    socket: SocketHandle;
    items: Array<TransferredHandle>;
}
"#,
    );
    let consumer_id = test.add_module(
        "consumer.ds",
        r#"
import { Container } from "./types.ds";
import { SocketHandle, TransferredHandle } from "./index.ds";

declare let socket: SocketHandle;
declare let transferred: TransferredHandle;

let value = Container {
    socket,
    items: [transferred],
};

value.items;
value.socket;
"#,
    );

    test.analyze_module_and_check_clean(consumer_id);

    let view = test.view(module_id);
    let container_symbol = test
        .resolve_to_symbol("types.ds", "Container")
        .expect("expected Container symbol");
    view.expect_value_type_id(container_symbol);
    let socket_type_id =
        assert_struct_field_type_matches_instance_field(&test, &view, "Container", "socket");
    let items_type_id =
        assert_struct_field_type_matches_instance_field(&test, &view, "Container", "items");
    let socket_handle_symbol = test.canonical_symbol_for_path("handles.ds", "SocketHandle");
    let transferred_handle_symbol =
        test.canonical_symbol_for_path("handles.ds", "TransferredHandle");
    let item_type_id = expect_array_element_type(view.types(), items_type_id);

    assert_type_references_symbol(view.types(), socket_type_id, socket_handle_symbol);
    assert_type_references_symbol(view.types(), item_type_id, transferred_handle_symbol);
}

/// Expose core promise and symbol globals through the injected prelude.
#[test]
fn test_prelude_exposes_core_promise_and_symbol_globals() {
    let test = TestProgram::memory_sequential_with_prelude();
    let module_id = test.add_module(
        "main.ds",
        r#"
const iterator = Symbol.iterator;
const promise: Promise<number> = Promise.resolve(1);

promise;
iterator;
"#,
    );

    test.analyze_module_and_check_clean(module_id);
}

/// Prefer selected-lib Promise globals over prelude shims for await typing.
#[test]
fn test_selected_lib_promise_globals_override_prelude_shims() {
    let test = TestProgram::memory_sequential_with_prelude_and_libs()
        .with_profile_libs(&["es5", "es2015.promise"]);
    let module_id = test.add_module(
        "main.ds",
        r#"
declare const value: Promise<number>;

async function read(): Promise<number> {
    const inner = await value;
    inner satisfies number;
    return inner;
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);
}

/// Infer async for of bindings from yielded element types.
#[test]
fn test_infers_async_for_of_binding_from_element_type() {
    let test =
        TestProgram::memory_sequential_with_prelude_and_libs().with_profile_libs(&["es2020", "js"]);
    let module_id = test.add_module(
        "main.ts",
        r#"
declare const source: AsyncIterable<number>;

async function app() {
    for await (const value of source) {
        value satisfies number;
        return value;
    }

    return 0;
}
"#,
    );

    test.analyze_module_and_check_clean(module_id);
}
