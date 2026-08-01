use crate::{CommentRetention, TestParser};
use destack_dir::{
    Argument, Catch, Declaration, Declarator, DependencyItem, EnumField, Expression,
    GenericArgument, GenericParameter, MatchArm, Member, Parameter, Pattern, PatternField,
    Property, SwitchCase, TupleElement, TypeDeclaration, TypeExpression, TypeMappedParameter,
    TypeMember, WhereClause,
};

/// Attach documentation and decorators to the declaration rather than its expression wrapper.
#[test]
fn test_attach_documentation_to_declaration_owner() {
    let test = TestParser::new(
        r#"
/// Run work.
/// Return its result.
@memo
function run(): string {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = parser.unwrap_label_expression(roots[0]);
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };

    assert_eq!(test.documentation(&parser, root), None);
    assert_eq!(
        test.documentation(&parser, declaration),
        Some("Run work.\nReturn its result.")
    );

    let decorators = parser.tree.get_decorators(declaration.id);
    assert_eq!(decorators.len(), 1);
    assert_eq!(test.documentation(&parser, decorators[0]), None);
}

/// Attach documentation to complete value expressions and nested operand expressions.
#[test]
fn test_attach_documentation_to_value_expressions() {
    let test = TestParser::new(
        r#"
/// Combine both values.
left +
/// Compute the right value.
right()
"#,
    );
    let (parser, roots) = test.parse();

    let expression = parser.unwrap_label_expression(roots[0]);
    let (left, right) = match parser.tree.get(expression) {
        Expression::Binary { left, right, .. } => (*left, *right),
        expression => panic!("expected binary expression, got {expression:?}"),
    };

    assert_eq!(
        test.documentation(&parser, expression),
        Some("Combine both values.")
    );
    assert_eq!(test.documentation(&parser, left), None);
    assert_eq!(
        test.documentation(&parser, right),
        Some("Compute the right value.")
    );
}

/// Prefer argument and parameter owners over their child expressions and types.
#[test]
fn test_attach_documentation_to_call_slots() {
    let test = TestParser::new("/// Input value.\n@guard value: string");
    let mut parser = test.prepare();
    let parameter = parser
        .parse_parameter_fragment()
        .expect("expected documented parameter");

    let declared_type = match parser.tree.get(parameter) {
        Parameter::Named {
            declared_type: Some(declared_type),
            ..
        } => *declared_type,
        parameter => panic!("expected named parameter, got {parameter:?}"),
    };
    assert_eq!(test.documentation(&parser, parameter), Some("Input value."));
    assert_eq!(test.documentation(&parser, declared_type), None);

    let decorators = parser.tree.get_decorators(parameter.id);
    assert_eq!(decorators.len(), 1);
    assert_eq!(test.documentation(&parser, decorators[0]), None);

    let test = TestParser::new("/// Argument value.\n@memo value");
    let mut parser = test.prepare();
    let argument = parser
        .parse_argument_fragment()
        .expect("expected documented argument");
    let value = match parser.tree.get(argument) {
        Argument::Positional { value } => *value,
        argument => panic!("expected positional argument, got {argument:?}"),
    };

    assert_eq!(
        test.documentation(&parser, argument),
        Some("Argument value.")
    );
    assert_eq!(test.documentation(&parser, value), None);
}

/// Attach tuple element documentation to the element rather than its child type.
#[test]
fn test_attach_documentation_to_type_slots() {
    let test = TestParser::new(
        r#"
/// Pair values.
type Pair = [
    /// Left value.
    string,
    /// Right value.
    boolean,
]
"#,
    );
    let (parser, roots) = test.parse();

    let root = parser.unwrap_label_expression(roots[0]);
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let value = match parser.tree.get(declaration) {
        Declaration::Type(TypeDeclaration { value, .. }) => *value,
        declaration => panic!("expected type declaration, got {declaration:?}"),
    };
    let elements = match parser.tree.get(value) {
        TypeExpression::Tuple { elements, .. } => elements,
        ty => panic!("expected tuple type, got {ty:?}"),
    };

    assert_eq!(
        test.documentation(&parser, declaration),
        Some("Pair values.")
    );
    assert_eq!(test.documentation(&parser, value), None);
    assert_eq!(
        test.documentation(&parser, elements[0]),
        Some("Left value.")
    );
    assert_eq!(
        test.documentation(&parser, elements[1]),
        Some("Right value.")
    );

    for element in elements {
        let value = match parser.tree.get(*element) {
            TupleElement::Element { value, .. } => *value,
            element => panic!("expected tuple element, got {element:?}"),
        };
        assert_eq!(test.documentation(&parser, value), None);
    }
}

/// Attach documentation to every structural owner parsed around child expressions and types.
#[test]
fn test_attach_documentation_to_structural_owners() {
    let test = TestParser::new(
        r#"
class Box {
    /// Stored value.
    @memo
    value: string
}
"#,
    );
    let (parser, _) = test.parse();
    let members = parser.tree.iter_nodes::<Member>().collect::<Vec<_>>();
    assert_eq!(members.len(), 1);
    assert_eq!(
        test.documentation(&parser, members[0]),
        Some("Stored value.")
    );

    let test = TestParser::new(
        r#"
type Shape = {
    /// Computed area.
    area(): number
}
"#,
    );
    let (parser, _) = test.parse();
    let members = parser.tree.iter_nodes::<TypeMember>().collect::<Vec<_>>();
    assert_eq!(members.len(), 1);
    assert_eq!(
        test.documentation(&parser, members[0]),
        Some("Computed area.")
    );

    let test = TestParser::new(
        r#"
enum Color {
    /// Red channel.
    Red
}
"#,
    );
    let (parser, _) = test.parse();
    let fields = parser.tree.iter_nodes::<EnumField>().collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    assert_eq!(test.documentation(&parser, fields[0]), Some("Red channel."));

    let test = TestParser::new(
        r#"
import {
    /// Imported value.
    value
} from "./value.ds"
"#,
    );
    let (parser, _) = test.parse();
    let items = parser
        .tree
        .iter_nodes::<DependencyItem>()
        .collect::<Vec<_>>();
    assert_eq!(items.len(), 1);
    assert_eq!(
        test.documentation(&parser, items[0]),
        Some("Imported value.")
    );

    let test = TestParser::new(
        r#"
match (value) {
    /// Default result.
    _ => 0
}
"#,
    );
    let (parser, _) = test.parse();
    let arms = parser.tree.iter_nodes::<MatchArm>().collect::<Vec<_>>();
    assert_eq!(arms.len(), 1);
    assert_eq!(
        test.documentation(&parser, arms[0]),
        Some("Default result.")
    );

    let test = TestParser::new(
        r#"
switch (value) {
    /// Default branch.
    default: break
}
"#,
    );
    let (parser, _) = test.parse();
    let cases = parser.tree.iter_nodes::<SwitchCase>().collect::<Vec<_>>();
    assert_eq!(cases.len(), 1);
    assert_eq!(
        test.documentation(&parser, cases[0]),
        Some("Default branch.")
    );

    let test = TestParser::new(
        r#"
const first = 1,
    /// Second binding.
    second = 2
"#,
    );
    let (parser, _) = test.parse();
    let declarators = parser.tree.iter_nodes::<Declarator>().collect::<Vec<_>>();
    assert_eq!(declarators.len(), 2);
    assert_eq!(test.documentation(&parser, declarators[0]), None);
    assert_eq!(
        test.documentation(&parser, declarators[1]),
        Some("Second binding.")
    );
}

/// Attach documentation to generic and mapped type slots.
#[test]
fn test_attach_documentation_to_generic_owners() {
    let test = TestParser::new(
        r#"
function run<
    /// Item type.
    T,
>(value: T): void {}
"#,
    );
    let (parser, _) = test.parse();
    let parameters = parser
        .tree
        .iter_nodes::<GenericParameter>()
        .collect::<Vec<_>>();
    assert_eq!(parameters.len(), 1);
    assert_eq!(
        test.documentation(&parser, parameters[0]),
        Some("Item type.")
    );

    let test = TestParser::new(
        r#"
run<
    /// Item type.
    string,
>()
"#,
    );
    let (parser, _) = test.parse();
    let arguments = parser
        .tree
        .iter_nodes::<GenericArgument>()
        .collect::<Vec<_>>();
    assert_eq!(arguments.len(), 1);
    assert_eq!(
        test.documentation(&parser, arguments[0]),
        Some("Item type.")
    );

    let test = TestParser::new(
        r#"
type Fields<T> = {
    /// Key type.
    [K in keyof T]: T[K]
}
"#,
    );
    let (parser, _) = test.parse();
    let parameters = parser
        .tree
        .iter_nodes::<TypeMappedParameter>()
        .collect::<Vec<_>>();
    assert_eq!(parameters.len(), 1);
    assert_eq!(
        test.documentation(&parser, parameters[0]),
        Some("Key type.")
    );
}

/// Attach documentation to constraint and catch clauses.
#[test]
fn test_attach_documentation_to_clauses() {
    let test = TestParser::new(
        r#"
function run<T>(): void where (
    /// Copy constraint.
    T: Copy,
) {}
"#,
    );
    let (parser, _) = test.parse();
    let clauses = parser.tree.iter_nodes::<WhereClause>().collect::<Vec<_>>();
    assert_eq!(clauses.len(), 1);
    assert_eq!(
        test.documentation(&parser, clauses[0]),
        Some("Copy constraint.")
    );

    let test = TestParser::new(
        r#"
try {}
/// Failure branch.
catch (error) {}
"#,
    );
    let (parser, _) = test.parse();
    let catches = parser.tree.iter_nodes::<Catch>().collect::<Vec<_>>();
    assert_eq!(catches.len(), 1);
    assert_eq!(
        test.documentation(&parser, catches[0]),
        Some("Failure branch.")
    );
}

/// Attach documentation to properties, patterns, and pattern fields.
#[test]
fn test_attach_documentation_to_pattern_owners() {
    let test = TestParser::new(
        r#"
const value = {
    /// Stored value.
    item: 1,
}
"#,
    );
    let (parser, _) = test.parse();
    let properties = parser.tree.iter_nodes::<Property>().collect::<Vec<_>>();
    assert_eq!(properties.len(), 1);
    assert_eq!(
        test.documentation(&parser, properties[0]),
        Some("Stored value.")
    );

    let test = TestParser::new(
        r#"
match (value) {
    {
        /// Item field.
        item:
            /// Bound value.
            bound
    } => bound
}
"#,
    );
    let (parser, _) = test.parse();
    let fields = parser.tree.iter_nodes::<PatternField>().collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    assert_eq!(test.documentation(&parser, fields[0]), Some("Item field."));

    let documented_patterns = parser
        .tree
        .iter_nodes::<Pattern>()
        .filter_map(|pattern| test.documentation(&parser, pattern))
        .collect::<Vec<_>>();
    assert_eq!(documented_patterns, vec!["Bound value."]);
}

/// Leave documentation between decorators and their owner unattached.
#[test]
fn test_leave_documentation_after_decorator_unattached() {
    let test = TestParser::new(
        r#"
@memo
/// Not attached.
function run(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = parser.unwrap_label_expression(roots[0]);
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    assert_eq!(test.documentation(&parser, root), None);
    assert_eq!(test.documentation(&parser, declaration), None);
    assert_eq!(parser.comments().len(), 1);
}

/// Do not attach detached, interrupted, or trailing documentation to later expressions.
#[test]
fn test_leave_unowned_documentation_unattached() {
    let test = TestParser::new(
        r#"
/// Attached.
first

/// Detached.

second

/// Interrupted.
// ordinary
third

fourth /// trailing
fifth
"#,
    );
    let (parser, roots) = test.parse();

    assert_eq!(roots.len(), 5);
    assert_eq!(test.documentation(&parser, roots[0]), Some("Attached."));
    assert_eq!(test.documentation(&parser, roots[1]), None);
    assert_eq!(test.documentation(&parser, roots[2]), None);
    assert_eq!(test.documentation(&parser, roots[3]), None);
    assert_eq!(test.documentation(&parser, roots[4]), None);
    assert_eq!(parser.comments().len(), 5);
}

/// Preserve ordinary comment barriers when only documentation comments are retained.
#[test]
fn test_keep_ordinary_comments_from_extending_documentation() {
    let test = TestParser::new(
        r#"
/// Detached.
// ordinary
first

// ordinary
/// Attached.
second
"#,
    );
    let mut parser = test.prepare_with_comment_retention(CommentRetention::Documentation);
    let roots = parser.parse();

    test.assert_no_errors(&parser);
    assert_eq!(roots.len(), 2);
    assert_eq!(test.documentation(&parser, roots[0]), None);
    assert_eq!(test.documentation(&parser, roots[1]), Some("Attached."));
}
