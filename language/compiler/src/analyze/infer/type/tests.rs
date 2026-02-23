use crate::{TestProgram, assert_type};
use destack_dir::{IntType, PrimitiveType, Type, TypeLiteral};

/// Converge one anchored interface cycle to a concrete boundary type.
#[test]
fn test_interface_component_converges_anchored_cycle() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";

export let a: number = b;
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import { c } from "./c.ds";

export let b = c;
"#,
    );
    test.add_module(
        "c.ds",
        r#"
import { a } from "./a.ds";

export let c = a;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { a } from "./a.ds";
import { b } from "./b.ds";
import { c } from "./c.ds";

let x = a;
let y = b;
let z = c;
"#,
    );

    test.analyze_module_and_check_clean(main_id);
    let a_view = test.view(test.module("a.ds").read().id);
    let b_view = test.view(test.module("b.ds").read().id);
    let c_view = test.view(test.module("c.ds").read().id);
    let a_symbol = test
        .resolve_to_symbol("a.ds", "a")
        .expect("missing a symbol");
    let b_symbol = test
        .resolve_to_symbol("b.ds", "b")
        .expect("missing b symbol");
    let c_symbol = test
        .resolve_to_symbol("c.ds", "c")
        .expect("missing c symbol");
    let a_type_id = a_view
        .types()
        .get_value_type_id(a_symbol)
        .expect("missing a value type");
    let b_type_id = b_view
        .types()
        .get_value_type_id(b_symbol)
        .expect("missing b value type");
    let c_type_id = c_view
        .types()
        .get_value_type_id(c_symbol)
        .expect("missing c value type");

    assert_type!(
        a_view.types(),
        a_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
    assert_type!(
        b_view.types(),
        b_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
    assert_type!(
        c_view.types(),
        c_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Reject one unanchored interface cycle after component convergence.
#[test]
fn test_interface_component_rejects_unanchored_cycle() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";

export let a = b;
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import { a } from "./a.ds";

export let b = a;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { a } from "./a.ds";

let value = a;
"#,
    );

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA116");
}

/// Keep interface component solving stable for larger anchored cycles.
#[test]
fn test_interface_component_converges_large_cycle_without_stall() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";
export let a: int32 = b;
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import { c } from "./c.ds";
export let b = c;
"#,
    );
    test.add_module(
        "c.ds",
        r#"
import { d } from "./d.ds";
export let c = d;
"#,
    );
    test.add_module(
        "d.ds",
        r#"
import { a } from "./a.ds";
export let d = a;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { d } from "./d.ds";
let value = d;
"#,
    );

    test.analyze_module_and_check_clean(main_id);
    let d_view = test.view(test.module("d.ds").read().id);
    let d_symbol = test
        .resolve_to_symbol("d.ds", "d")
        .expect("missing d symbol");
    let value_type_id = d_view
        .types()
        .get_value_type_id(d_symbol)
        .expect("missing d value type");

    assert_type!(
        d_view.types(),
        value_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}

/// Reject one re-exported cycle when no declaration anchor exists.
#[test]
fn test_interface_component_rejects_unanchored_reexport_cycle() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import { y } from "./bridge.ds";

export let x = y;
"#,
    );
    test.add_module(
        "bridge.ds",
        r#"
export { y } from "./b.ds";
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import { x } from "./a.ds";

export let y = x;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { x } from "./a.ds";

let value = x;
"#,
    );

    test.analyze_module(main_id);
    test.compile();
    test.check_has_diagnostic("EA116");
}

/// Converge one re-exported cycle when one declaration anchor exists.
#[test]
fn test_interface_component_converges_anchored_reexport_cycle() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import { y } from "./bridge.ds";

export let x: number = y;
"#,
    );
    test.add_module(
        "bridge.ds",
        r#"
export { y } from "./b.ds";
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import { x } from "./a.ds";

export let y = x;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { x } from "./a.ds";

let value = x;
"#,
    );

    test.analyze_module_and_check_clean(main_id);

    let b_view = test.view(test.module("b.ds").read().id);
    let y_symbol = test
        .resolve_to_symbol("b.ds", "y")
        .expect("missing y symbol");
    let y_type_id = b_view
        .types()
        .get_value_type_id(y_symbol)
        .expect("missing y value type");

    assert_type!(
        b_view.types(),
        y_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Converge one anchored cycle through multi-hop re-exports.
#[test]
fn test_interface_component_converges_anchored_multi_hop_reexport_cycle() {
    let test = TestProgram::memory_sequential();
    test.add_module(
        "a.ds",
        r#"
import { y } from "./bridge1.ds";

export let x: number = y;
"#,
    );
    test.add_module(
        "bridge1.ds",
        r#"
export { y } from "./bridge2.ds";
"#,
    );
    test.add_module(
        "bridge2.ds",
        r#"
export { y } from "./b.ds";
"#,
    );
    test.add_module(
        "b.ds",
        r#"
import { x } from "./a.ds";

export let y = x;
"#,
    );
    let main_id = test.add_module(
        "main.ds",
        r#"
import { x } from "./a.ds";

let value = x;
"#,
    );

    test.analyze_module_and_check_clean(main_id);

    let b_view = test.view(test.module("b.ds").read().id);
    let y_symbol = test
        .resolve_to_symbol("b.ds", "y")
        .expect("missing y symbol");
    let y_type_id = b_view
        .types()
        .get_value_type_id(y_symbol)
        .expect("missing y value type");

    assert_type!(
        b_view.types(),
        y_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Reroute non-anchor interface tasks to the canonical component anchor.
#[test]
fn test_interface_component_reroutes_non_anchor_task_to_component_anchor() {
    let test = TestProgram::memory_sequential();
    let a_id = test.add_module(
        "a.ds",
        r#"
import { b } from "./b.ds";

export let a: int32 = b;
"#,
    );
    let b_id = test.add_module(
        "b.ds",
        r#"
import { c } from "./c.ds";

export let b = c;
"#,
    );
    let c_id = test.add_module(
        "c.ds",
        r#"
import { a } from "./a.ds";

export let c = a;
"#,
    );

    // drive analyze from a non-anchor entry module in the same component
    test.analyze_module_and_check_clean(c_id);

    let a_view = test.view(a_id);
    let b_view = test.view(b_id);
    let c_view = test.view(c_id);
    let a_symbol = test
        .resolve_to_symbol("a.ds", "a")
        .expect("missing a symbol");
    let b_symbol = test
        .resolve_to_symbol("b.ds", "b")
        .expect("missing b symbol");
    let c_symbol = test
        .resolve_to_symbol("c.ds", "c")
        .expect("missing c symbol");
    let a_type_id = a_view
        .types()
        .get_value_type_id(a_symbol)
        .expect("missing a value type");
    let b_type_id = b_view
        .types()
        .get_value_type_id(b_symbol)
        .expect("missing b value type");
    let c_type_id = c_view
        .types()
        .get_value_type_id(c_symbol)
        .expect("missing c value type");

    assert_type!(
        a_view.types(),
        a_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
    assert_type!(
        b_view.types(),
        b_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
    assert_type!(
        c_view.types(),
        c_type_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Int(IntType::Int32))
        }
    );
}
