use crate::TestParser;
use tspp_dir::{
    Argument, Catch, Declaration, Declarator, DependencyItem, DocumentationTag, EnumField,
    Expression, GenericArgument, GenericParameter, MatchArm, Member, Parameter, Pattern,
    PatternField, Property, SwitchCase, TupleElement, TypeDeclaration, TypeExpression,
    TypeMappedParameter, TypeMember, WhereClause,
};

/// Parse every source revision produced while typing a documentation line.
#[test]
fn test_parse_documentation_while_typing() {
    let documentation = "/// A simple position.\n";
    let before = r#"class Player {
    foo: string;

    constructor(foo: string) {
        this.foo = foo;
    }
}

"#;
    let after = r#"struct Position {
    x: float64;
    y: float64;
}
"#;

    // parse every authored prefix before the declaration
    for (index, character) in documentation.char_indices() {
        let end = index + character.len_utf8();
        let source = format!("{before}{}{after}", &documentation[..end]);
        let test = TestParser::new(&source);
        let mut parser = test.prepare();
        parser.parse_in_place();
    }

    // require the completed documentation on its declaration
    let test = TestParser::new("/// A simple position.\nstruct Position {}");
    let (parser, roots) = test.parse();
    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };

    assert_eq!(
        test.documentation(&parser, declaration),
        Some("A simple position.")
    );
}

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

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };

    assert_eq!(test.documentation(&parser, root), None);
    assert_eq!(
        test.documentation(&parser, declaration),
        Some("Run work.\nReturn its result.")
    );
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");
    assert_eq!(
        parser.span_str(documentation.span),
        "/// Run work.\n/// Return its result."
    );

    let decorators = parser.tree.get_decorators(declaration.id);
    assert_eq!(decorators.len(), 1);
    assert_eq!(test.documentation(&parser, decorators[0]), None);
}

/// Bind callable documentation tags to their exact declared parameters.
#[test]
fn test_parse_callable_documentation() {
    let test = TestParser::new(
        r#"
/// Return the provided value.
///
/// # Errors
///
/// Returns `InvalidValue` when validation fails.
///
/// @typeParam Value - The returned value type.
/// @param value - The value to return.
/// @example
/// ```tspp
/// identity<string>("value");
/// ```
function identity<Value>(value: Value): Value {
    return value;
}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let function = match parser.tree.get(declaration) {
        Declaration::Function(function) => function,
        declaration => panic!("expected function declaration, got {declaration:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");

    assert_eq!(
        parser.strings.get(documentation.markdown),
        "Return the provided value.\n\n# Errors\n\nReturns `InvalidValue` when validation fails."
    );
    assert_eq!(documentation.tags.len(), 3);
    let DocumentationTag::TypeParameter {
        parameter,
        markdown,
    } = documentation.tags[0]
    else {
        panic!("expected type parameter documentation");
    };
    assert_eq!(parameter, function.signature.generic_parameters[0]);
    assert_eq!(parser.strings.get(markdown), "The returned value type.");
    let DocumentationTag::Parameter {
        parameter,
        markdown,
    } = documentation.tags[1]
    else {
        panic!("expected parameter documentation");
    };
    assert_eq!(parameter, function.signature.parameters[0]);
    assert_eq!(parser.strings.get(markdown), "The value to return.");
    let DocumentationTag::Example { markdown } = documentation.tags[2] else {
        panic!("expected example documentation");
    };
    assert_eq!(
        parser.strings.get(markdown),
        "```tspp\nidentity<string>(\"value\");\n```"
    );
}

/// Accept common parameter separators while retaining exact parameter bindings.
#[test]
fn test_parse_documentation_parameter_separators() {
    let test = TestParser::new(
        r#"
/// Select one value.
/// @param plain Plain documentation.
/// @param dash - Dash documentation.
/// @param colon: Colon documentation.
function select<Value>(plain: Value, dash: Value, colon: Value): Value {
    return plain;
}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let function = match parser.tree.get(declaration) {
        Declaration::Function(function) => function,
        declaration => panic!("expected function declaration, got {declaration:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");
    let targets = documentation
        .tags
        .iter()
        .map(|tag| tag.target())
        .collect::<Vec<_>>();
    let markdown = documentation
        .tags
        .iter()
        .map(|tag| parser.strings.get(tag.markdown()))
        .collect::<Vec<_>>();

    assert_eq!(
        targets,
        function
            .signature
            .parameters
            .iter()
            .map(|parameter| Some(parameter.into_any()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        markdown,
        vec![
            "Plain documentation.",
            "Dash documentation.",
            "Colon documentation."
        ]
    );
}

/// Keep any unknown line-start tag as one preserved section block.
#[test]
fn test_parse_keeps_an_unknown_line_start_tag_as_a_section() {
    let test = TestParser::new(
        r#"
/// Provides access to the Cache API.
///
/// @category Cache
/// Grouped under the Cache namespace.
/// @since 1.2.0
function open(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");

    assert_eq!(
        parser.strings.get(documentation.markdown),
        "Provides access to the Cache API."
    );
    let markdown = documentation
        .tags
        .iter()
        .map(|tag| {
            assert!(tag.is_section());

            parser.strings.get(tag.markdown())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        markdown,
        vec![
            "@category Cache\nGrouped under the Cache namespace.",
            "@since 1.2.0"
        ]
    );
    assert!(parser.diagnostics().is_empty());
}

/// Keep tag text after the start of a line as plain Markdown prose.
#[test]
fn test_parse_keeps_inline_tag_text_as_prose() {
    let test = TestParser::new(
        r#"
/// Mail admin@example.com when the @memo decorator runs.
function notify(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");

    assert_eq!(
        parser.strings.get(documentation.markdown),
        "Mail admin@example.com when the @memo decorator runs."
    );
    assert!(documentation.tags.is_empty());
    assert!(parser.diagnostics().is_empty());
}

/// Keep braced inline tags as plain Markdown prose.
#[test]
fn test_parse_keeps_a_braced_inline_tag_as_prose() {
    let test = TestParser::new(
        r#"
/// See {@link open} for details.
/// {@linkcode Cache} starts this line.
function close(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");

    assert_eq!(
        parser.strings.get(documentation.markdown),
        "See {@link open} for details.\n{@linkcode Cache} starts this line."
    );
    assert!(documentation.tags.is_empty());
    assert!(parser.diagnostics().is_empty());
}

/// Keep an at sign without a tag identifier as plain Markdown prose.
#[test]
fn test_parse_keeps_a_bare_at_sign_as_prose() {
    let test = TestParser::new(
        r#"
/// @
/// @ mentions ping the author.
/// @!important punctuation stays prose.
function ping(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");

    assert_eq!(
        parser.strings.get(documentation.markdown),
        "@\n@ mentions ping the author.\n@!important punctuation stays prose."
    );
    assert!(documentation.tags.is_empty());
    assert!(parser.diagnostics().is_empty());
}

/// Retain recognized section tags as authored lines including their headers.
#[test]
fn test_parse_documentation_retains_section_tags_as_authored() {
    let test = TestParser::new(
        r#"
/// Read one value.
///
/// @returns The stored value, or `null` when unset.
/// @deprecated Use `readValue` instead.
function read(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };
    let documentation = parser
        .tree
        .get_documentation(declaration.id)
        .expect("function declaration should have documentation");

    assert_eq!(
        parser.strings.get(documentation.markdown),
        "Read one value."
    );
    let markdown = documentation
        .tags
        .iter()
        .map(|tag| {
            assert!(tag.is_section());

            parser.strings.get(tag.markdown())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        markdown,
        vec![
            "@returns The stored value, or `null` when unset.",
            "@deprecated Use `readValue` instead."
        ]
    );
}

/// Attach documentation across intervening ordinary line comments.
#[test]
fn test_attach_documentation_skips_intervening_line_comments() {
    let test = TestParser::new(
        r#"
/// Run work.
// prettier-ignore
function run(): void {}
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
    let declaration = match parser.tree.get(root) {
        Expression::Declaration(declaration) => *declaration,
        expression => panic!("expected declaration expression, got {expression:?}"),
    };

    assert_eq!(test.documentation(&parser, declaration), Some("Run work."));
    assert!(parser.diagnostics().is_empty());
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

    let expression = roots[0];
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
type Pair = (
    /// Left value.
    string,
    /// Right value.
    boolean,
)
"#,
    );
    let (parser, roots) = test.parse();

    let root = roots[0];
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
    let members = parser
        .tree
        .iter_node_ids_of_type::<Member>()
        .collect::<Vec<_>>();
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
    let members = parser
        .tree
        .iter_node_ids_of_type::<TypeMember>()
        .collect::<Vec<_>>();
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
    let fields = parser
        .tree
        .iter_node_ids_of_type::<EnumField>()
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    assert_eq!(test.documentation(&parser, fields[0]), Some("Red channel."));

    let test = TestParser::new(
        r#"
import {
    /// Imported value.
    value
} from "./value.tspp"
"#,
    );
    let (parser, _) = test.parse();
    let items = parser
        .tree
        .iter_node_ids_of_type::<DependencyItem>()
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
    let arms = parser
        .tree
        .iter_node_ids_of_type::<MatchArm>()
        .collect::<Vec<_>>();
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
    let cases = parser
        .tree
        .iter_node_ids_of_type::<SwitchCase>()
        .collect::<Vec<_>>();
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
    let declarators = parser
        .tree
        .iter_node_ids_of_type::<Declarator>()
        .collect::<Vec<_>>();
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
        .iter_node_ids_of_type::<GenericParameter>()
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
        .iter_node_ids_of_type::<GenericArgument>()
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
        .iter_node_ids_of_type::<TypeMappedParameter>()
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
    let clauses = parser
        .tree
        .iter_node_ids_of_type::<WhereClause>()
        .collect::<Vec<_>>();
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
    let catches = parser
        .tree
        .iter_node_ids_of_type::<Catch>()
        .collect::<Vec<_>>();
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
    let properties = parser
        .tree
        .iter_node_ids_of_type::<Property>()
        .collect::<Vec<_>>();
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
    let fields = parser
        .tree
        .iter_node_ids_of_type::<PatternField>()
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 1);
    assert_eq!(test.documentation(&parser, fields[0]), Some("Item field."));

    let documented_patterns = parser
        .tree
        .iter_node_ids_of_type::<Pattern>()
        .filter_map(|pattern| test.documentation(&parser, pattern))
        .collect::<Vec<_>>();
    assert_eq!(documented_patterns, vec!["Bound value."]);
}
