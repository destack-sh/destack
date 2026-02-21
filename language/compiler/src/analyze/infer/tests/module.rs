use super::*;
use crate::analyze::common::AnalyzeDependencyStage;

/// Merge global interface members across imported modules.
#[test]
fn test_merge_global_declarations_across_imports() {
    // arrange test modules
    let test = TestProgram::memory_sequential();
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
    let module = test.program.modules.get(main_id);
    let module = module.read();
    let profile = view.profile_id();
    let dir = module.dir(profile);
    let thing_name = test.program.strings.intern("thing");
    let global_key = StaticKey::Name(test.program.strings.intern("GlobalThing"));
    let global_group = test
        .compiler
        .get_global_symbol_group(main_id, profile, global_key, SymbolSpace::Type)
        .expect("missing global group for GlobalThing");
    assert_eq!(global_group.len(), 2);

    for global_symbol in &global_group {
        let remote_profile = test.default_profile_id(global_symbol.module_id);
        test.compiler
            .with_module_types_at_stage(
                &module,
                remote_profile,
                global_symbol.module_id,
                AnalyzeDependencyStage::Declare,
                |_, remote_types| {
                    assert!(
                        remote_types.get_instance_type_id(*global_symbol).is_some(),
                        "expected instance type for GlobalThing in module {:?}",
                        global_symbol.module_id,
                    );
                },
            )
            .expect("declare stage should be ready for merged global module test");
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
    let canonical_symbol = test.compiler.canonical_symbol_id(
        &module,
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

    // x should have int32 type (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = view.types().get_value_type_id(x_symbol).unwrap();

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
    test.add_file(
        "owner.ds",
        r#"
import { RetryPolicy } from "./contract";

export struct HttpRetryPolicy {}

extension for HttpRetryPolicy implements RetryPolicy {
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

extension for HttpRetryPolicy implements RetryPolicy {
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
    test.add_file(
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
    // arrange a class constructor alias and incompatible assignment
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

    // run analyze and require unassignable diagnostic
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

extension for Counter implements LocalCursor<int32> {
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
