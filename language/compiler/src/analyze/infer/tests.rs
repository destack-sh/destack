use crate::{AnalyzeOptions, InferContext, TestProgram, assert_string, assert_type};
use destack_dir::{
    BinaryOperator, Declaration, Expression, FlowEdgeKind, FlowGraphBuilder, InferTable,
    PrimitiveType, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, SymbolKind,
    SymbolSpace, SymbolType, Type, TypeLiteral, TypeUnaryOperator,
};
use destack_workspace::DsConfigCompilerOptions;

/// Analyze number literal.
#[test]
fn test_analyze_number_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "42");

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };

    // read inferred type
    let ty = types
        .get_inferred_type(expression_id.into_global_any(module.id))
        .unwrap();

    // integer literals now have literal types, not widened primitive types
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
    );
}

/// Analyze string literal.
#[test]
fn test_analyze_string_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", r#""hello""#);

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };

    // read inferred type
    let ty = types
        .get_inferred_type(expression_id.into_global_any(module.id))
        .unwrap();

    // string literal has literal type (e.g., "hello" has type "hello")
    assert!(matches!(
        ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    ));
}

/// Analyze boolean literal.
#[test]
fn test_analyze_boolean_literal() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "true");

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };

    // read inferred type
    let ty = types
        .get_inferred_type(expression_id.into_global_any(module.id))
        .unwrap();

    // boolean literal has literal type (e.g., true has type true)
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        }
    );
}

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
    test.analyze_module(main_id);
    test.compile_dump_clean();

    // load module data for inspection
    let module = test.program.modules.get(main_id);
    let module = module.read();
    let profile = test.default_profile_id(main_id);
    let dir = module.dir(profile);
    let symbols = dir.symbols.read();
    let types = dir.types.read();
    let thing_name = test.program.strings.intern("thing");
    let global_key = StaticKey::Name(test.program.strings.intern("GlobalThing"));
    let global_group = test
        .compiler
        .get_global_symbol_group(main_id, profile, global_key, SymbolSpace::Type)
        .expect("missing global group for GlobalThing");
    assert_eq!(global_group.len(), 2);

    for global_symbol in &global_group {
        let remote_profile = test.default_profile_id(global_symbol.module_id);
        let remote_module = test.program.modules.get(global_symbol.module_id);
        let remote_module = remote_module.read();
        let remote_types = remote_module.dir(remote_profile).types.read();
        assert!(
            remote_types.get_instance_type_id(*global_symbol).is_some(),
            "expected instance type for GlobalThing in module {:?}",
            global_symbol.module_id,
        );
    }

    // locate the bound symbol for thing in the module scope
    let namespace_scope = symbols.get_scope_by_id(dir.namespace_scope);
    let thing_symbol = namespace_scope
        .named_symbols
        .iter()
        .find_map(|(key, symbol_id)| match key {
            StaticKey::Name(name_id) if *name_id == thing_name => Some(*symbol_id),
            _ => None,
        })
        .expect("missing symbol for thing")
        .into_global(main_id);
    let thing_entry = symbols.get_symbol(thing_symbol.local_id);
    assert!(
        thing_entry.target_symbol.is_none(),
        "unexpected target_symbol on thing binding",
    );
    assert!(
        thing_entry.canonical_symbol.is_none(),
        "unexpected canonical_symbol on thing binding",
    );
    let canonical_symbol =
        test.compiler
            .canonical_symbol_id(&module, &symbols, profile, thing_symbol);
    assert_eq!(canonical_symbol, thing_symbol);

    // assert the binding uses a nominal reference type
    let value_ty_id = types
        .get_value_type_id(thing_symbol)
        .expect("missing value type for thing");
    let value_ty = types.get_type(value_ty_id);
    let Type::Reference {
        symbol: global_thing,
        ..
    } = value_ty
    else {
        panic!("expected reference type for thing");
    };
    assert_eq!(global_thing.ty(), SymbolType::Interface);

    // assert the merged instance type includes both fields
    let instance_ty_id = types
        .get_instance_type_id(*global_thing)
        .expect("missing instance type for GlobalThing");
    let instance_ty = types.get_type(instance_ty_id);
    let Type::Object { fields, .. } = instance_ty else {
        panic!("expected object instance type for GlobalThing");
    };
    let value_key = StaticKey::Name(test.program.strings.intern("value"));
    let label_key = StaticKey::Name(test.program.strings.intern("label"));
    assert!(fields.iter().any(|field| field.key.matches(&value_key)));
    assert!(fields.iter().any(|field| field.key.matches(&label_key)));
}

/// Analyze binary number operation.
#[test]
fn test_analyze_binary_number_operation() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "1 + 2");

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };

    // read inferred type
    let ty = types
        .get_inferred_type(expression_id.into_global_any(module.id))
        .unwrap();

    // constant folding: 1 + 2 evaluates to literal type 3
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(3))
        }
    );
}

/// Narrow nullish types in an if guard.
#[test]
fn test_narrowing_nullish_if_guard() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const value: string | null = null;
if (value != null) {
    const narrowed: string = value;
} else {
    0;
}
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // locate the if expression
    let (_, if_expression) = tree
        .iter_nodes_of_type::<Expression>()
        .find(|(_, expression)| matches!(expression, Expression::If { .. }))
        .expect("expected if expression");
    let Expression::If {
        then_expression, ..
    } = if_expression
    else {
        panic!("expected if expression");
    };

    // locate the narrowed declaration in the then block
    let then_expression_id = match tree.get(*then_expression) {
        Expression::Block { block } => {
            let block = tree.get(*block);
            *block
                .expressions
                .first()
                .expect("expected then block expression")
        }
        Expression::Statement { statement } => *statement,
        _ => *then_expression,
    };

    let then_expression_id = match tree.get(then_expression_id) {
        Expression::Statement { statement } => *statement,
        _ => then_expression_id,
    };

    let Expression::Let { declarators, .. } = tree.get(then_expression_id) else {
        panic!("expected let expression");
    };

    let declarator_id = declarators.first().expect("expected declarator");
    let declarator = tree.get(*declarator_id);
    let value_expression_id = declarator
        .value
        .expect("expected value expression in declarator");

    // assert nullish types are stripped in the then branch
    let left_type_id = types
        .get_inferred_type_id(value_expression_id.into_global_any(module.id))
        .expect("expected inferred type");
    let left_type = types.get_type(left_type_id);

    let is_nullish = match left_type {
        Type::Union { elements } => elements.iter().any(|element_id| {
            matches!(
                types.get_type(*element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Null | TypeLiteral::Undefined
                }
            )
        }),
        Type::TypeLiteral {
            value: TypeLiteral::Null | TypeLiteral::Undefined,
        } => true,
        _ => false,
    };

    assert!(!is_nullish, "expected nullish to be stripped");
}

/// Analyze binary number comparison.
#[test]
fn test_analyze_binary_number_comparison() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "1 < 2");

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };

    // read inferred type
    let ty = types
        .get_inferred_type(expression_id.into_global_any(module.id))
        .unwrap();

    // constant folding: 1 < 2 evaluates to literal true
    assert_eq!(
        *ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        }
    );
}

/// Analyze let expression infer type.
#[test]
fn test_analyze_let_expression_infer_type() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "let x = 42");

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let let_expr_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

    // no declared type
    assert!(
        types
            .get_declared_type(let_expr_id.into_global_any(module.id))
            .is_none()
    );

    // value_type[x] = literal 42
    let x_ty = types.get_value_type(x_symbol).unwrap();
    assert_eq!(
        *x_ty,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
    );

    // instance_type[x] = undefined
    assert!(types.get_instance_type(x_symbol).is_none());
}

/// Analyze let expression declare type.
#[test]
fn test_analyze_let_expression_declare_type() {
    // use compatible types: string annotation with string value
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", r#"let x: string = "hello""#);

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };
    let let_expression = tree.get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();

    // declared_type[declarator] = string
    let declared = types
        .get_declared_type(declarator_id.into_global(module.id).into())
        .unwrap();
    assert_eq!(
        *declared,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // value_type[x] = string
    let x_ty = types.get_value_type(x_symbol).unwrap();
    assert_eq!(
        *x_ty,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::String)
        }
    );

    // instance_type[x] = undefined
    assert!(types.get_instance_type(x_symbol).is_none());
}

/// Analyze let expression infer tuple type with pattern.
#[test]
fn test_analyze_let_expression_infer_tuple_type_with_pattern() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let (x, y, ...rest, z) = (123, 'abc', true, 456);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let x_symbol = test.resolve_to_symbol("test.ds", "x").unwrap();
    let y_symbol = test.resolve_to_symbol("test.ds", "y").unwrap();
    let rest_symbol = test.resolve_to_symbol("test.ds", "rest").unwrap();
    let z_symbol = test.resolve_to_symbol("test.ds", "z").unwrap();

    // value_type[x] = literal 123
    let x_ty_id = types.get_value_type_id(x_symbol).unwrap();

    assert_type!(
        types,
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(123))
        }
    );

    // value_type[y] = literal 'abc'
    let y_ty_id = types.get_value_type_id(y_symbol).unwrap();
    assert_type!(
        types,
        y_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    );

    // value_type[rest] = (true,)
    let rest_ty_id = types.get_value_type_id(rest_symbol).unwrap();
    assert_type!(types, rest_ty_id, Type::Tuple { elements } => {
        assert_eq!(elements.len(), 1);
        assert_type!(types, elements[0].ty, Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Boolean(true))
        });
    });

    // value_type[z] = literal 456
    let z_ty_id = types.get_value_type_id(z_symbol).unwrap();
    assert_type!(
        types,
        z_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(456))
        }
    );
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
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // x should have literal type 42 (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = types.get_value_type_id(x_symbol).unwrap();

    assert_type!(
        types,
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(42))
        }
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
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // x should have an array element union from the imported literal values
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = types.get_value_type_id(x_symbol).unwrap();

    assert_type!(types, x_ty_id, Type::Array { element: Some(element_id) } => {
        assert_type!(types, *element_id, Type::Union { elements } => {
            // union has three literal elements
            assert_eq!(elements.len(), 3);

            // each element stays a literal integer
            for element_id in elements {
                assert_type!(types, *element_id, Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                });
            }
        });
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
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // x should have literal string type (imported from lib.ds)
    let x_symbol = test.resolve_to_symbol("main.ds", "x").unwrap();
    let x_ty_id = types.get_value_type_id(x_symbol).unwrap();

    assert_type!(
        types,
        x_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        }
    );
}

/// Resolve member access on object literal to field type.
#[test]
fn test_analyze_member_access_object_field() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { x: 42, y: "hello" };
let a = obj.x;
let b = obj.y;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Resolve member access across multiple fields.
#[test]
fn test_analyze_member_access_multiple_fields() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { x: 42, y: "hello", z: true };
let a = obj.x;
let b = obj.y;
let c = obj.z;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Resolve chained member access on nested objects.
#[test]
fn test_analyze_member_access_chained() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
let obj = { inner: { value: 42 } };
let a = obj.inner.value;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile();
    test.check_clean();
}

/// Substitute `this` types for member calls.
#[test]
fn test_analyze_this_type_member_call() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Builder { value: int32 }

extension for Builder {
    combine(other: this): this { return other; }
}

let builder = Builder { value: 0 };
let other = Builder { value: 1 };
let result = builder.combine(other);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let builder_symbol = test.resolve_to_symbol("test.ds", "Builder").unwrap();
    let result_symbol = test.resolve_to_symbol("test.ds", "result").unwrap();
    let result_ty_id = types
        .get_value_type_id(result_symbol)
        .expect("expected result type");

    assert_type!(types, result_ty_id, Type::Value { value } => {
        assert_type!(types, *value, Type::Reference { symbol, .. } => {
            assert_eq!(*symbol, builder_symbol);
        });
    });
}

/// Substitute `this` types inside static type arguments.
#[test]
fn test_analyze_this_type_static_argument() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Box<T> { value: T }

struct Builder { value: int32 }

extension for Builder {
    box(): Box<this> { return { value: this }; }
}

let builder = Builder { value: 0 };
let boxed = builder.box();
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();
    let builder_symbol = test.resolve_to_symbol("test.ds", "Builder").unwrap();
    let boxed_symbol = test.resolve_to_symbol("test.ds", "boxed").unwrap();
    let boxed_ty_id = types
        .get_value_type_id(boxed_symbol)
        .expect("expected boxed type");

    let boxed_ty = types.get_type(boxed_ty_id).clone();
    let (boxed_symbol, boxed_arguments) = match boxed_ty {
        Type::Value { value } => match types.get_type(value).clone() {
            Type::Reference {
                symbol,
                static_arguments,
            } => (symbol, static_arguments),
            other => panic!("expected boxed reference type, got {other:?}"),
        },
        Type::Reference {
            symbol,
            static_arguments,
        } => (symbol, static_arguments),
        other => panic!("expected boxed value type, got {other:?}"),
    };

    assert_eq!(boxed_symbol, box_symbol);
    let boxed_arguments = boxed_arguments.expect("expected static arguments");
    assert_eq!(boxed_arguments.len(), 1);
    let StaticArgument::Evaluated { value, .. } = &boxed_arguments[0] else {
        panic!("expected evaluated static argument");
    };
    let StaticExpression::Type { ty } = value else {
        panic!("expected static type argument");
    };
    let inner_ty = types.get_type(*ty).clone();
    let builder_reference = match inner_ty {
        Type::Reference { symbol, .. } => symbol,
        Type::Value { value } => match types.get_type(value).clone() {
            Type::Reference { symbol, .. } => symbol,
            other => panic!("expected builder reference type, got {other:?}"),
        },
        other => panic!("expected builder value type, got {other:?}"),
    };
    assert_eq!(builder_reference, builder_symbol);
}

/// Infer an inherent extension.
#[test]
fn test_analyze_inherent_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Point { 
    x: number, 
    y: number,
}

extension for Point {
    magnitude(): number { 
        return 0; 
    }
}

extension for Point {
    distance(other: Point): number { 
        return 0; 
    }
}
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    let _point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();
    // NOTE #Incomplete: #Extensions
}

/// Infer a local extension (on a foreign type).
#[test]
fn test_analyze_local_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "point.ds",
        r#"
struct Point { 
x: number, 
y: number,
}
"#,
    );
    let module_id = test.add_module(
        "test.ds",
        r#"
import { Point } from "./point.ds";

// local extension on foreign type
extension for Point {
    distance(other: Point): number { return 0; }
}
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    let _point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();
    // NOTE #Incomplete: #Extensions
}

/// Infer a named extension (on a foreign type, from a foreign extension).
#[test]
fn test_analyze_named_extension() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    test.add_file(
        "point.ds",
        r#"
struct Point { 
    x: number, 
    y: number,
}
"#,
    );
    test.add_file(
        "extensions.ds",
        r#"
import { Point } from "./point.ds";

extension PointHelpers for Point {
    distance(other: Point): number { return 0; }
}
"#,
    );
    let module_id = test.add_module(
        "test.ds",
        r#"
import { PointHelpers } from "./extensions.ds";
import { Point } from "./point.ds";
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    let _point_id = test.resolve_to_symbol("test.ds", "Point").unwrap();
    let _point_helpers_id = test.resolve_to_symbol("test.ds", "PointHelpers").unwrap();
    // NOTE #Incomplete: #Extensions
}

/// Analyze infer parameter types from call arguments.
#[test]
fn test_analyze_infer_parameter_types_from_call_arguments() {
    // infers parameter types from call arguments
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function add(a, b) {
    return a + b
}
add(1, 2)
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let fn_symbol = test.expect_first_function_symbol(module.id);
    let fn_ty_id = types
        .get_value_type_id(fn_symbol)
        .expect("expected function type");

    // function type uses literal argument types and a number return type
    assert_type!(types, fn_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // two parameters inferred from call arguments
        assert_eq!(dynamic_parameters.len(), 2);

        // first parameter matches literal 1
        assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        });

        // second parameter matches literal 2
        assert_type!(types, dynamic_parameters[1], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(2))
        });

        // return type is number from arithmetic
        let return_type = return_type.expect("expected return type");
        assert_type!(types, return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze infer parameter type from default.
#[test]
fn test_analyze_infer_parameter_type_from_default() {
    // infers parameter type from default value
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function greet(name = "hi") {
    return name
}
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let fn_symbol = test.expect_first_function_symbol(module.id);
    let fn_ty_id = types
        .get_value_type_id(fn_symbol)
        .expect("expected function type");

    // function type uses default value literal and returns a string
    assert_type!(types, fn_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // one parameter inferred from default value
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter uses the default string literal type
        assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        });

        // return type follows the parameter type
        let return_type = return_type.expect("expected return type");
        assert_type!(types, return_type, Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
        });
    });
}

/// Analyze static type arguments on call.
#[test]
fn test_analyze_static_type_arguments_on_call() {
    // resolves explicit static type arguments at call sites
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
identity<number>(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let call_root_id = module.dir(test.default_profile_id(module_id)).roots[1];
    let call_root = tree.get(call_root_id);
    let &Expression::Statement {
        statement: call_expression_id,
    } = call_root
    else {
        panic!("expected statement");
    };
    let call_ty_id = types
        .get_inferred_type_id(call_expression_id.into_global_any(module.id))
        .expect("expected call type");

    // call expression uses explicit number type argument
    assert_type!(
        types,
        call_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        }
    );
}

/// Analyze static type arguments inferred.
#[test]
fn test_analyze_static_type_arguments_inferred() {
    // infers static type arguments from call arguments
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
let one = identity(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let expression_id = module.dir(test.default_profile_id(module_id)).roots[1];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };
    let let_expression = tree.get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = tree.get(*declarator_id);
    let value_id = declarator.value.expect("expected initializer value");
    let value_ty_id = types
        .get_inferred_type_id(value_id.into_global_any(module.id))
        .expect("expected initializer type");

    // identity returns the literal type of the inferred argument
    assert_type!(
        types,
        value_ty_id,
        Type::TypeLiteral {
            value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
        }
    );
}

/// Analyze static type arguments on reference.
#[test]
fn test_analyze_static_type_arguments_on_reference() {
    // resolves explicit static arguments on function references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function identity<T>(value: T): T {
    return value
}
let as_number = identity<number>;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let expression_id = module.dir(test.default_profile_id(module_id)).roots[1];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };
    let let_expression = tree.get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = tree.get(*declarator_id);
    let value_id = declarator.value.expect("expected initializer value");
    let value_ty_id = types
        .get_inferred_type_id(value_id.into_global_any(module.id))
        .expect("expected initializer type");

    // specialized function reference uses number parameter and return type
    assert_type!(types, value_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // single parameter is number
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter uses the explicit number type argument
        assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type matches the parameter
        let return_type = return_type.expect("expected return type");
        assert_type!(types, return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze type reference instance.
#[test]
fn test_analyze_type_reference_instance() {
    // registers instances for type reference annotations
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> {
    value: T
}

declare function makeBox(): Box<number>;

let value: Box<number> = makeBox();
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // locate the type annotation
    let declarator_id = test.expect_first_let_declarator(module.id);
    let declarator = tree.get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected type annotation");

    let instance_id = types
        .get_instance_for_node(type_expression_id.into_global_any(module.id))
        .expect("expected instance");
    let instance = types.get_instance(instance_id);

    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();

    // instance targets Box with a single static type argument
    assert_eq!(instance.symbol_id, box_symbol);
    assert_eq!(instance.static_arguments.len(), 1);

    // static argument is the explicit number type
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    types,
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze type reference default static value.
#[test]
fn test_analyze_type_reference_default_static_value() {
    // applies default static value arguments for type references
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Buffer<T, N: number = 4> {
    value: T
}

declare function makeBuffer(): Buffer<string>;

let buffer: Buffer<string> = makeBuffer();
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let declarator_id = test.expect_first_let_declarator(module.id);
    let declarator = tree.get(declarator_id);
    let type_expression_id = declarator.ty.expect("expected type annotation");

    let instance_id = types
        .get_instance_for_node(type_expression_id.into_global_any(module.id))
        .expect("expected instance");
    let instance = types.get_instance(instance_id);

    let buffer_symbol = test.resolve_to_symbol("test.ds", "Buffer").unwrap();

    // instance targets Buffer with explicit T and default N value
    assert_eq!(instance.symbol_id, buffer_symbol);
    assert_eq!(instance.static_arguments.len(), 2);

    // first static argument is the explicit string type
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    types,
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is the default value N = 4
    match &instance.static_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::ScalarLiteral { value } => {
                assert_eq!(*value, ScalarLiteral::Integer(4));
            }
            _ => panic!("expected scalar literal argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze member instance inherited static arguments.
#[test]
fn test_analyze_member_instance_inherited_static_arguments() {
    // registers inherited static arguments for member instances
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

let result = getContainer().map<string>(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // locate the initializer expression
    let declarator_id = test.expect_first_let_declarator(module.id);
    let declarator = tree.get(declarator_id);
    let value_id = declarator.value.expect("expected initializer value");

    let instance_id = types
        .get_instance_for_node(value_id.into_global_any(module.id))
        .expect("expected instance");
    let instance = types.get_instance(instance_id);

    let map_name = test.program.strings.intern("map");
    let container_symbol = test.resolve_to_symbol("test.ds", "Container").unwrap();
    let map_symbol = test.expect_interface_member_symbol(module.id, container_symbol, map_name);

    // instance targets Container.map with inherited T and explicit U
    assert_eq!(instance.symbol_id, map_symbol);
    assert_eq!(instance.static_arguments.len(), 2);

    // first static argument is inherited Container T = number
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    types,
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is explicit U = string
    match &instance.static_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    types,
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze member instance from member expression.
#[test]
fn test_analyze_member_instance_from_member_expression() {
    // registers static arguments on member expressions
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
interface Container<T> {
    map<U>(value: T): U
}

declare function getContainer(): Container<number>;

let mapper = getContainer().map<string>;
mapper(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // locate the member expression initializer
    let declarator_id = test.expect_first_let_declarator(module.id);
    let declarator = tree.get(declarator_id);
    let value_id = declarator.value.expect("expected initializer value");

    let instance_id = types
        .get_instance_for_node(value_id.into_global_any(module.id))
        .expect("expected instance");
    let instance = types.get_instance(instance_id);

    let map_name = test.program.strings.intern("map");
    let container_symbol = test.resolve_to_symbol("test.ds", "Container").unwrap();
    let map_symbol = test.expect_interface_member_symbol(module.id, container_symbol, map_name);

    // instance targets Container.map with inherited T and explicit U
    assert_eq!(instance.symbol_id, map_symbol);
    assert_eq!(instance.static_arguments.len(), 2);

    // first static argument is inherited Container T = number
    match &instance.static_arguments[0] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    types,
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::Number)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }

    // second static argument is explicit U = string
    match &instance.static_arguments[1] {
        StaticArgument::Evaluated { value, .. } => match value {
            StaticExpression::Type { ty } => {
                assert_type!(
                    types,
                    *ty,
                    Type::TypeLiteral {
                        value: TypeLiteral::Primitive(PrimitiveType::String)
                    }
                );
            }
            _ => panic!("expected type argument"),
        },
        _ => panic!("expected evaluated argument"),
    }
}

/// Analyze substitute type reference arguments.
#[test]
fn test_analyze_substitute_type_reference_arguments() {
    // substitutes type arguments inside type references during inference
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
struct Box<T> { value: T }

declare function wrap<T>(value: T): Box<T>;

let result = wrap(1);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    let result_symbol = test.resolve_to_symbol("test.ds", "result").unwrap();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let result_ty_id = types
        .get_value_type_id(result_symbol)
        .expect("expected result type");
    let box_symbol = test.resolve_to_symbol("test.ds", "Box").unwrap();

    // result type is Box with a literal type argument
    assert_type!(
        types,
        result_ty_id,
        Type::Reference {
            symbol,
            static_arguments,
        } => {
            // type reference targets Box
            assert_eq!(*symbol, box_symbol);
            let static_arguments = static_arguments.as_ref().expect("expected arguments");

            // type reference carries one static argument
            assert_eq!(static_arguments.len(), 1);

            // static argument is the inferred literal type
            match &static_arguments[0] {
                StaticArgument::Evaluated { value, .. } => match value {
                    StaticExpression::Type { ty } => {
                        assert_type!(
                            types,
                            *ty,
                            Type::TypeLiteral {
                                value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(1))
                            }
                        );
                    }
                    _ => panic!("expected type argument"),
                },
                _ => panic!("expected evaluated argument"),
            }
        }
    );
}

/// Analyze contextual lambda from annotation.
#[test]
fn test_analyze_contextual_lambda_from_annotation() {
    // infers lambda signature from contextual function type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const add: (a: number, b: number) => number = (a, b) => a + b;
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };
    let let_expression = tree.get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = tree.get(*declarator_id);
    let value_id = declarator.value.expect("expected function value");
    let value_ty_id = types
        .get_inferred_type_id(value_id.into_global_any(module.id))
        .expect("expected function type");

    // contextual annotation yields number, number to number
    assert_type!(types, value_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // two parameters inherited from annotation
        assert_eq!(dynamic_parameters.len(), 2);

        // first parameter is number
        assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // second parameter is number
        assert_type!(types, dynamic_parameters[1], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type is number
        let return_type = return_type.expect("expected return type");
        assert_type!(types, return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual lambda from argument.
#[test]
fn test_analyze_contextual_lambda_from_argument() {
    // infers lambda signature from argument type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
function apply(fn: (a: number) => number): number {
    return fn(1);
}
apply((a) => a + 1);
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    let call_root_id = module.dir(test.default_profile_id(module_id)).roots[1];
    let call_root = tree.get(call_root_id);
    let &Expression::Statement {
        statement: call_expression_id,
    } = call_root
    else {
        panic!("expected statement");
    };
    let call_expression = tree.get(call_expression_id);
    let Expression::Call {
        dynamic_arguments, ..
    } = call_expression
    else {
        panic!("expected call expression");
    };

    let argument_id = dynamic_arguments.first().expect("expected argument");
    let argument = tree.get(*argument_id);
    let argument_value_id = argument.value();
    let argument_ty_id = types
        .get_inferred_type_id(argument_value_id.into_global_any(module.id))
        .expect("expected argument type");

    // contextual argument yields number to number function type
    assert_type!(types, argument_ty_id, Type::Function { dynamic_parameters, return_type, .. } => {
        // one parameter inherited from argument context
        assert_eq!(dynamic_parameters.len(), 1);

        // parameter is number
        assert_type!(types, dynamic_parameters[0], Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });

        // return type is number
        let return_type = return_type.expect("expected return type");
        assert_type!(types, return_type, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

/// Analyze contextual object literal.
#[test]
fn test_analyze_contextual_object_literal() {
    // infers object literal field types from contextual type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const point: { x: number, y: string } = { x: 1, y: "hi" };
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };
    let let_expression = tree.get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = tree.get(*declarator_id);
    let value_id = declarator.value.expect("expected object value");
    let value_ty_id = types
        .get_inferred_type_id(value_id.into_global_any(module.id))
        .expect("expected object type");

    // object literal fields use contextual field types
    assert_type!(types, value_ty_id, Type::Object { fields, .. } => {
        // two fields are present in the contextual object type
        assert_eq!(fields.len(), 2);

        // scan field types for expected primitives
        let mut saw_number = false;
        let mut saw_string = false;
        for field in fields {
            match types.get_type(field.ty) {
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Number),
                } => saw_number = true,
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String),
                } => saw_string = true,
                _ => {}
            }
        }

        // both number and string fields are present
        assert!(saw_number);
        assert!(saw_string);
    });
}

/// Analyze contextual array literal.
#[test]
fn test_analyze_contextual_array_literal() {
    // infers array literal element types from contextual type
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
const numbers: number[] = [1, 2];
"#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load typed module data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let tree = module.dir(test.default_profile_id(module_id)).tree.read();
    let types = module.dir(test.default_profile_id(module_id)).types.read();

    // select the root expression
    let expression_id = module.dir(test.default_profile_id(module_id)).roots[0];
    let expression = tree.get(expression_id);
    let &Expression::Statement {
        statement: expression_id,
    } = expression
    else {
        panic!("expected statement");
    };
    let let_expression = tree.get(expression_id);
    let Expression::Let { declarators, .. } = let_expression else {
        panic!("expected let expression");
    };
    let declarator_id = declarators.first().unwrap();
    let declarator = tree.get(*declarator_id);
    let value_id = declarator.value.expect("expected array value");
    let value_ty_id = types
        .get_inferred_type_id(value_id.into_global_any(module.id))
        .expect("expected array type");

    // array element type is number from number[] context
    assert_type!(types, value_ty_id, Type::Array { element: Some(element_id) } => {
        assert_type!(types, *element_id, Type::TypeLiteral {
            value: TypeLiteral::Primitive(PrimitiveType::Number)
        });
    });
}

// Binds mapped type parameters for use in value type.
#[test]
fn test_analyze_type_mapped_parameter_scope() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "type Map<T> = { [K in keyof T]: T[K] };");

    test.analyze_module(module_id);
    test.compile_dump_clean();

    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let dir = module.dir(profile);
    let types = dir.types.read();
    let symbols = dir.symbols.read();

    let map_symbol = test
        .resolve_to_symbol("test.ds", "Map")
        .expect("expected Map symbol");
    let map_instance_id = types
        .get_instance_type_id(map_symbol)
        .expect("expected Map instance type");

    assert_type!(types, map_instance_id, Type::Mapped { parameter, value, .. } => {
        assert_string!(test.program, parameter.name, "K");
        assert_type!(types, parameter.constraint, Type::Unary { operator: TypeUnaryOperator::Keyof, right } => {
            assert_type!(types, *right, Type::Reference { symbol, static_arguments } => {
                assert!(static_arguments.is_none());
                let symbol = symbols.get_symbol(symbol.into_local());
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
            });
        });
        assert_type!(types, *value, Type::Index { left, index } => {
            assert_type!(types, *left, Type::Reference { symbol, .. } => {
                let symbol = symbols.get_symbol(symbol.into_local());
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
            });
            assert_type!(types, *index, Type::Reference { symbol, .. } => {
                let symbol = symbols.get_symbol(symbol.into_local());
                assert_eq!(symbol.kind, SymbolKind::Local);
                assert_string!(test.program, symbol.name().expect("expected symbol name"), "K");
            });
        });
    });
}

/// Bind infer variables for use in conditional true branch.
#[test]
fn test_analyze_type_infer_scope() {
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module("test.ds", "type Foo<T> = T extends infer U ? U : never;");

    test.analyze_module(module_id);
    test.compile_dump_clean();

    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let dir = module.dir(profile);
    let types = dir.types.read();
    let symbols = dir.symbols.read();

    let foo_symbol = test
        .resolve_to_symbol("test.ds", "Foo")
        .expect("expected Foo symbol");
    let foo_instance_id = types
        .get_instance_type_id(foo_symbol)
        .expect("expected Foo instance type");

    assert_type!(types, foo_instance_id, Type::Conditional { left, right, then_type, else_type } => {
        assert_type!(types, *left, Type::Reference { symbol, .. } => {
            let symbol = symbols.get_symbol(symbol.into_local());
            assert_string!(test.program, symbol.name().expect("expected symbol name"), "T");
        });
        assert_type!(types, *right, Type::Infer { name, constraint } => {
            assert_string!(test.program, *name, "U");
            assert!(constraint.is_none());
        });
        assert_type!(types, *then_type, Type::Reference { symbol, .. } => {
            let symbol = symbols.get_symbol(symbol.into_local());
            assert_eq!(symbol.kind, SymbolKind::Local);
            assert_string!(test.program, symbol.name().expect("expected symbol name"), "U");
        });
        assert_type!(types, *else_type, Type::TypeLiteral { value: TypeLiteral::Never });
    });
}

/// Build a flow graph with true and false branches for if expressions.
#[test]
fn test_build_flow_graph_if_expression() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        "const value = true; if (value) { 1 } else { 2 };",
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load tree data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let dir = module.dir(profile);
    let tree = dir.tree.read();

    // locate the if expression
    let if_expression_id = dir
        .roots
        .iter()
        .find_map(|root_id| match tree.get(*root_id) {
            Expression::If { .. } => Some(*root_id),
            Expression::Statement { statement } => match tree.get(*statement) {
                Expression::If { .. } => Some(*statement),
                _ => None,
            },
            _ => None,
        })
        .expect("expected if expression");

    // build the flow graph
    let graph = FlowGraphBuilder::new(module.id, &tree).build(if_expression_id);

    let mut has_true_edge = false;
    let mut has_false_edge = false;
    let mut has_join_block = false;
    for block in &graph.blocks {
        if block.predecessors.len() >= 2 {
            has_join_block = true;
        }
        for edge in &block.successors {
            if edge.kind == FlowEdgeKind::True {
                has_true_edge = true;
            }
            if edge.kind == FlowEdgeKind::False {
                has_false_edge = true;
            }
        }
    }

    assert!(has_true_edge);
    assert!(has_false_edge);
    assert!(has_join_block);
}

/// Build a flow graph that narrows the right side of short circuit guards.
#[test]
fn test_build_flow_graph_short_circuit_guard() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
        const value: string | null = null;
        const accepts_string = (input: string): boolean => true;
        if (value !== null && accepts_string(value)) { };
        "#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load tree data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let dir = module.dir(profile);
    let tree = dir.tree.read();
    let symbols = dir.symbols.read();
    let mut types = dir.types.write();

    // locate the if expression
    let if_expression_id = dir
        .roots
        .iter()
        .find_map(|root_id| match tree.get(*root_id) {
            Expression::If { .. } => Some(*root_id),
            Expression::Statement { statement } => match tree.get(*statement) {
                Expression::If { .. } => Some(*statement),
                _ => None,
            },
            _ => None,
        })
        .expect("expected if expression");

    // locate the right side of the condition
    let Expression::If { condition, .. } = tree.get(if_expression_id) else {
        panic!("expected if expression");
    };
    let Expression::Binary {
        left,
        operator,
        right,
    } = tree.get(*condition)
    else {
        panic!("expected binary condition");
    };
    assert_eq!(*operator, BinaryOperator::And);

    let Expression::Binary { left, .. } = tree.get(*left) else {
        panic!("expected binary left guard");
    };

    let Expression::Call {
        dynamic_arguments, ..
    } = tree.get(*right)
    else {
        panic!("expected call expression");
    };
    let argument_id = dynamic_arguments
        .first()
        .copied()
        .expect("expected call argument");
    let argument = tree.get(argument_id);
    let argument_value_id = argument.value();

    // ensure the right side reference uses a distinct node id
    assert_ne!(*left, argument_value_id);

    // build flow data for the condition expression
    // build the flow graph
    let graph = FlowGraphBuilder::new(module.id, &tree).build(*condition);
    let mut infer = InferTable::default();
    let context = InferContext::new(
        profile,
        AnalyzeOptions::from(&DsConfigCompilerOptions::default()),
    );
    let flow = test
        .compiler
        .compute_flow_table_for_graph(
            &module, &graph, &tree, &symbols, &mut types, &mut infer, &context,
        )
        .expect("expected flow table");

    // read the flow environment for the right side argument
    let block_id = graph
        .block_by_node
        .get(&argument_value_id.into_any())
        .copied()
        .expect("expected block for argument");
    let environment_id = flow
        .entry_environment_for_block(block_id)
        .expect("expected entry environment");
    let environment = flow
        .environment(environment_id)
        .expect("expected environment");

    // assert that the value symbol is narrowed to a non null type
    let value_symbol = test
        .resolve_to_symbol("test.ds", "value")
        .expect("expected value symbol");
    let narrowed_type_id = environment
        .bindings
        .get(&value_symbol)
        .copied()
        .expect("expected narrowing for value");
    let is_null = match types.get_type(narrowed_type_id) {
        Type::TypeLiteral {
            value: TypeLiteral::Null,
        } => true,
        Type::Union { elements } => elements.iter().any(|element_id| {
            matches!(
                types.get_type(*element_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Null
                }
            )
        }),
        _ => false,
    };
    assert!(!is_null);
}

/// Build a flow graph with a back edge for for loops.
#[test]
fn test_build_flow_graph_for_loop() {
    // arrange test module
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
        function run() {
            for (let i: number = 0; i < 3; i = i + 1) {
                i = i + 1;
            }
        }
        "#,
    );

    // run analyze pipeline
    test.analyze_module(module_id);
    test.compile_dump_clean();

    // load tree data
    let module = test.program.modules.get(module_id);
    let module = module.read();
    let profile = test.default_profile_id(module_id);
    let dir = module.dir(profile);
    let tree = dir.tree.read();

    // locate the first function body
    let function_body_id = dir
        .roots
        .iter()
        .find_map(|root_id| match tree.get(*root_id) {
            Expression::Declaration { declaration } => match tree.get(*declaration) {
                Declaration::Function {
                    body: Some(body), ..
                } => Some(*body),
                _ => None,
            },
            Expression::Statement { statement } => match tree.get(*statement) {
                Expression::Declaration { declaration } => match tree.get(*declaration) {
                    Declaration::Function {
                        body: Some(body), ..
                    } => Some(*body),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .expect("expected function body");

    // build the flow graph
    let graph = FlowGraphBuilder::new(module.id, &tree).build(function_body_id);

    let mut has_true_edge = false;
    let mut has_false_edge = false;
    let mut has_back_edge = false;
    for block in &graph.blocks {
        for edge in &block.successors {
            if edge.kind == FlowEdgeKind::True {
                has_true_edge = true;
            }
            if edge.kind == FlowEdgeKind::False {
                has_false_edge = true;
            }
            if edge.target.0 < block.id.0 {
                has_back_edge = true;
            }
        }
    }

    assert!(has_true_edge);
    assert!(has_false_edge);
    assert!(has_back_edge);
}
