use crate::tests::{TestProgram, root_expression_id};
use destack_dir as dir;
use dir::{Expression, TypeExpression};

/// Reify scalar constructor calls into tagged scalar expressions.
#[test]
fn test_reify_nominal_scalar_constructor_call() {
    // build a scalar newtype constructor call
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype UserId = int64;

let value = UserId(42);
"#,
    );

    // run the elaborate pipeline
    test.elaborate_module(module_id);
    test.compile_check_clean();
    // surface output remains a constructor call
    test.assert_elaborated(
        module_id,
        r#"
newtype UserId = int64;
let value = UserId(42);
"#,
    );

    // inspect the initializer expression
    test.with_dir_read(
        module_id,
        |_module, _profile, dir, tree, _symbols, _types| {
            // locate the let initializer
            let roots = dir.roots.clone();
            let expression_id = root_expression_id(&roots, tree, 1);
            let Expression::Let { declarators, .. } = tree.get(expression_id) else {
                panic!("expected let expression");
            };

            // read the initializer expression
            let declarator_id = declarators.first().copied().expect("expected declarator");
            let declarator = tree.get(declarator_id);
            let value_id = declarator.value.expect("expected initializer value");

            // ensure a tagged scalar expression was created
            let Expression::TaggedScalarExpression { ty: _, value: _ } = tree.get(value_id) else {
                panic!("expected tagged scalar expression");
            };
        },
    );
}

/// Reify tuple constructor calls into tagged tuple expressions.
#[test]
fn test_reify_nominal_tuple_constructor_call() {
    // build a tuple newtype constructor call
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Point = (float32, float32);

let value = Point(1.0, 2.0);
"#,
    );

    // run the elaborate pipeline
    test.elaborate_module(module_id);
    test.compile_check_clean();
    // surface output remains a constructor call
    test.assert_elaborated(
        module_id,
        r#"
newtype Point = (float32, float32,);
let value = Point(1, 2);
"#,
    );

    // inspect the initializer expression
    test.with_dir_read(
        module_id,
        |_module, _profile, dir, tree, _symbols, _types| {
            // locate the let initializer
            let roots = dir.roots.clone();
            let expression_id = root_expression_id(&roots, tree, 1);
            let Expression::Let { declarators, .. } = tree.get(expression_id) else {
                panic!("expected let expression");
            };

            // read the initializer expression
            let declarator_id = declarators.first().copied().expect("expected declarator");
            let declarator = tree.get(declarator_id);
            let value_id = declarator.value.expect("expected initializer value");

            // ensure a tagged tuple expression was created
            let Expression::TaggedTuple { ty: _, elements } = tree.get(value_id) else {
                panic!("expected tagged tuple expression");
            };
            assert_eq!(elements.len(), 2);
        },
    );
}

/// Move static arguments onto tagged tuple constructor expressions.
#[test]
fn test_reify_nominal_constructor_static_arguments() {
    // build a generic newtype constructor call
    let test = TestProgram::memory_sequential();
    let module_id = test.add_module(
        "test.ds",
        r#"
newtype Box<T> = (T,);

let value = Box<int32>(1);
"#,
    );

    // run the elaborate pipeline
    test.elaborate_module(module_id);
    test.compile_check_clean();
    // surface output remains a constructor call
    test.assert_elaborated(
        module_id,
        r#"
newtype Box<T> = (T,);
let value = Box<int32>(1);
"#,
    );

    // inspect the initializer expression
    test.with_dir_read(
        module_id,
        |_module, _profile, dir, tree, _symbols, _types| {
            // locate the let initializer
            let roots = dir.roots.clone();
            let expression_id = root_expression_id(&roots, tree, 1);
            let Expression::Let { declarators, .. } = tree.get(expression_id) else {
                panic!("expected let expression");
            };

            // read the initializer expression
            let declarator_id = declarators.first().copied().expect("expected declarator");
            let declarator = tree.get(declarator_id);
            let value_id = declarator.value.expect("expected initializer value");

            // verify tagged tuple constructor uses static arguments on the tag
            let Expression::TaggedTuple { ty, elements: _ } = tree.get(value_id) else {
                panic!("expected tagged tuple expression");
            };

            let ty_expression = tree.get(*ty);
            let arguments = match ty_expression {
                TypeExpression::LocalReference {
                    generic_arguments: arguments,
                    ..
                }
                | TypeExpression::ModuleReference {
                    generic_arguments: arguments,
                    ..
                }
                | TypeExpression::GlobalReference {
                    generic_arguments: arguments,
                    ..
                }
                | TypeExpression::Member {
                    generic_arguments: arguments,
                    ..
                } => arguments,
                _ => {
                    panic!("expected tag reference with generic arguments, got {ty_expression:?}");
                }
            };

            assert_eq!(arguments.len(), 1);
        },
    );
}
