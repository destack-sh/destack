use std::cmp::Ordering;

use destack_ast::{
    Argument, AssignOperator, Asynchrony, BinaryOperator, Declaration, Declarator, DependencyItem,
    DependencyKind, DependencyMode, Expression, ForEachBinding, ForEachKind, FunctionKind,
    IfCondition, IfKind, ImportSource, Keyword, LetKind, LocalNodeId, MatchCase, MatchKind,
    MatchSelector, Member, Mutability, NodeTree, NodeType, OperatorPrecedence, Parameter, Pattern,
    PostfixPosition, Property, ScalarLiteral, TypeLiteral, TypeModifier, TypePredicateSubject,
    TypeUnaryOperator, WhereClause, WhileKind, YieldCardinality,
};
use destack_base::StringId;
use destack_fir::format::{BestFittingMode, FormatError, GroupId};
use destack_fir::prelude::*;
use destack_fir::{best_fitting, format_args, write};
use destack_workspace::ImportSortOrder;
use smallvec::{SmallVec, smallvec};

use crate::argument::list_like;
use crate::block::format_block;
use crate::literal::{format_scalar_literal, format_template_literal};
use crate::{
    DestackFormatContext, DestackFormatter, FormatNode, empty_block_with_infix_annotations,
};

// chain head promotion limits
const MAX_CHAIN_HEAD_OPS: usize = 2;
const MAX_CHAIN_HEAD_LEN_DIVISOR: usize = 2;
const DECLARATOR_PREFIX_PADDING: usize = 6;
const ASSIGNMENT_CHAIN_TAIL_RESERVE: usize = 8;

/// Compare two strings using natural sort order (numbers ordered as integers).
/// Example: `"a1" < "a2" < "a10"` (not `"a1" < "a10" < "a2"`).
fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ac), Some(bc)) => {
                // both are digits: compare as numbers
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    let mut a_num: u64 = 0;
                    while let Some(&c) = a_chars.peek()
                        && c.is_ascii_digit()
                    {
                        a_num = a_num
                            .saturating_mul(10)
                            .saturating_add((c as u64) - ('0' as u64));
                        a_chars.next();
                    }
                    let mut b_num: u64 = 0;
                    while let Some(&c) = b_chars.peek()
                        && c.is_ascii_digit()
                    {
                        b_num = b_num
                            .saturating_mul(10)
                            .saturating_add((c as u64) - ('0' as u64));
                        b_chars.next();
                    }
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }

                // compare characters case-insensitively
                let ac_lower = ac.to_ascii_lowercase();
                let bc_lower = bc.to_ascii_lowercase();
                match ac_lower.cmp(&bc_lower) {
                    Ordering::Equal => {
                        // same letter different case: uppercase comes first
                        match ac.cmp(bc) {
                            Ordering::Equal => {
                                a_chars.next();
                                b_chars.next();
                            }
                            other => return other,
                        }
                    }
                    other => return other,
                }
            }
        }
    }
}

/// Sort dependency items (import/export specifiers) according to the given sort order.
fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &NodeTree,
    strings: &destack_base::ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    let mut sorted: Vec<_> = items.to_vec();

    sorted.sort_by(|a, b| {
        let a_item = tree.get(*a);
        let b_item = tree.get(*b);

        // type imports come before value imports
        let a_is_type = a_item.kind == Some(DependencyKind::Type);
        let b_is_type = b_item.kind == Some(DependencyKind::Type);
        match (a_is_type, b_is_type) {
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            _ => {}
        }

        // sort key: use alias if present, otherwise name
        let a_key = a_item
            .alias
            .or(a_item.name)
            .map(|s| strings.get(s))
            .unwrap_or("");
        let b_key = b_item
            .alias
            .or(b_item.name)
            .map(|s| strings.get(s))
            .unwrap_or("");

        match sort_order {
            ImportSortOrder::Natural => natural_cmp(a_key, b_key),
            ImportSortOrder::Alphabetical => a_key.cmp(b_key),
        }
    });

    sorted
}

/// Tree fragment argument (with `=` instead of `: `)
#[derive(Debug, Clone, PartialEq)]
struct TreeExpressionArgument {
    argument_id: LocalNodeId<Argument>,
}

impl<'ast> Format<DestackFormatContext<'ast>> for TreeExpressionArgument {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(self.argument_id)])?;

        let argument = f.context().tree.get(self.argument_id);
        match argument {
            Argument::Named { name, value, .. } => {
                let value_expr = f.context().tree.get(*value);
                if let Expression::ScalarLiteral(ScalarLiteral::Boolean(true)) = value_expr {
                    // boolean shorthand
                    write!(f, [name])?;
                } else {
                    write!(f, [name])?;
                    let is_string_literal = matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::ScalarLiteral(ScalarLiteral::Character(_))
                    );
                    if is_string_literal {
                        write!(f, [token("="), value])?;
                    } else {
                        // try hugged format for object/array attribute values
                        format_tree_attribute_value(f, *value)?;
                    }
                }
            }
            Argument::Labeled { label, value, .. } => {
                // label
                write!(f, [label])?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { value, .. } => {
                // In tree expressions, expression children need braces too
                let value_expr = f.context().tree.get(*value);
                let argument_span = f.context().get_span(self.argument_id);
                let argument_span_str = f.context().get_span_str(argument_span);
                let argument_is_braced = argument_span_str.trim_start().starts_with('{')
                    && argument_span_str.trim_end().ends_with('}');
                let needs_braces = argument_is_braced
                    || !matches!(
                        value_expr,
                        Expression::ScalarLiteral(ScalarLiteral::String(_))
                            | Expression::TreeExpression { .. }
                    );
                if needs_braces {
                    // for comment-only containers (stub), use soft_block_indent
                    // for regular expressions, keep inline to preserve ternary formatting
                    let is_comment_only = matches!(value_expr, Expression::Stub);
                    if is_comment_only {
                        write!(
                            f,
                            [group(&format_args![
                                token("{"),
                                soft_block_indent(&value),
                                token("}")
                            ])]
                        )?;
                    } else {
                        write!(f, [token("{"), value, token("}")])?;
                    }
                } else {
                    write!(f, [value])?;
                }
            }
            Argument::Spread { value, .. } => {
                // Spread in JSX needs braces: {...props}
                write!(f, [token("{"), token("..."), value, token("}")])?;
            }
        }

        write!(
            f,
            [f.context()
                .any_infix_or_postfix_annotations(self.argument_id)]
        )?;

        Ok(())
    }
}

/// Walk a chain of if expressions and collect the if/else if/else nodes.
pub(crate) fn format_if_else_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                kind: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                // if <condition>
                match condition {
                    IfCondition::Expression { condition } => {
                        write!(
                            f,
                            [
                                Keyword::If,
                                space(),
                                token("("),
                                *condition,
                                token(")"),
                                space()
                            ]
                        )?;
                    }
                    IfCondition::Let {
                        kind,
                        mutability: _,
                        declarator,
                    } => {
                        write!(f, [Keyword::If, space()])?;
                        match kind {
                            LetKind::Let => write!(f, [Keyword::Let])?,
                            LetKind::Var => write!(f, [Keyword::Var])?,
                            LetKind::Const => write!(f, [Keyword::Const])?,
                        }
                        write!(f, [space()])?;
                        format_declarator(f, f.context().tree, *declarator)?;
                        write!(f, [space()])?;
                    }
                }

                // then block
                let then_expression = f.context().tree.get(*then_expression_id);
                match then_expression {
                    Expression::Block(block_id) => {
                        write!(f, [f.context().any_prefix_annotations(*then_expression_id)])?;
                        format_block(f, *block_id)?;
                        write!(
                            f,
                            [f.context()
                                .any_infix_or_postfix_annotations(*then_expression_id)]
                        )?;
                    }
                    // something else
                    _ => write!(f, [*then_expression_id])?,
                }

                // postfix annotations
                if next_if_id != node_id {
                    write!(f, [f.context().any_postfix_annotations(next_if_id)])?;
                }

                // next node
                if let Some(else_expression) = else_expression_id {
                    if !f.context().has_prefix_annotation(*else_expression)
                        && !f.context().has_postfix_annotation(*then_expression_id)
                    {
                        write!(f, [space()])?;
                    }
                    match f.context().tree.get(*else_expression) {
                        // else if
                        Expression::If { .. } => {
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            // (postfix is covered by the next if above)
                            next_if_id = *else_expression;
                        }
                        // else
                        Expression::Block(else_block_id) => {
                            write!(f, [f.context().any_prefix_annotations(*else_expression)])?;
                            write!(f, [Keyword::Else, space()])?;
                            format_block(f, *else_block_id)?;
                            write!(f, [f.context().any_postfix_annotations(*else_expression)])?;
                            break;
                        }
                        // something else
                        _ => {
                            write!(f, [Keyword::Else, space(), *else_expression])?;
                            break;
                        }
                    }
                } else {
                    // bare if
                    break;
                }
            }
            // shouldn't be anything else
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for if chain",
                });
            }
        }
    }
    Ok(())
}

/// Format a member expression without considering chaining.
#[inline]
fn format_member_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Member {
        left,
        name,
        static_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left, token("."), *name])?;
        if let Some(static_arguments) = static_arguments {
            write!(f, [list_like("<", ">", ",", static_arguments)])?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for member formatter");
    }
    Ok(())
}

/// Format a type index expression without considering chaining.
#[inline]
fn format_type_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    left: LocalNodeId<Expression>,
    index: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let needs_parentheses = matches!(
        f.context().tree.get(left),
        Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
    );
    if needs_parentheses {
        write!(f, [token("("), left, token(")")])?;
    } else {
        write!(f, [left])?;
    }
    write!(f, [token("["), index, token("]")])?;
    Ok(())
}

/// Format a type template literal expression.
fn format_type_template_literal<'ast>(
    strings: &[StringId],
    spans: &[LocalNodeId<Expression>],
    f: &mut DestackFormatter<'ast, '_>,
) -> FormatResult<()> {
    debug_assert_eq!(strings.len(), spans.len().saturating_add(1));

    write!(f, [token("`")])?;

    let mut string_segments = strings.iter();
    if let Some(first_segment) = string_segments.next() {
        write!(f, [*first_segment])?;
    }

    for (span, segment) in spans.iter().zip(string_segments) {
        write!(
            f,
            [
                group(&format_args![
                    token("${"),
                    indent(&format_args![soft_line_break(), *span]),
                    soft_line_break(),
                    token("}")
                ]),
                *segment,
            ]
        )?;
    }

    write!(f, [token("`")])
}

/// Format an index expression without considering chaining.
#[inline]
fn format_index_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Index {
        position,
        left,
        index,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(index) = index {
            write!(f, [token("["), *index, token("]")])?;
        } else {
            write!(f, [token("[]")])?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for index formatter");
    }
    Ok(())
}

/// Hugging configuration for different delimiter contexts.
struct HugOptions {
    /// The opening delimiter.
    open: &'static str,
    /// The closing delimiter.
    close: &'static str,
    /// Whether to force a trailing comma.
    force_trailing: bool,
    /// Whether to include a trailing comma when the group breaks.
    trailing_if_breaks: bool,
    /// Whether to allow arrow functions.
    allow_arrow_functions: bool,
    /// Whether to handle annotations.
    handle_annotations: bool,
}

impl HugOptions {
    const CALL: Self = Self {
        open: "(",
        close: ")",
        force_trailing: false,
        trailing_if_breaks: false,
        allow_arrow_functions: true,
        handle_annotations: true,
    };

    const ARRAY: Self = Self {
        open: "[",
        close: "]",
        force_trailing: false,
        trailing_if_breaks: false,
        allow_arrow_functions: false,
        handle_annotations: false,
    };

    const TUPLE: Self = Self {
        open: "(",
        close: ")",
        force_trailing: true,
        trailing_if_breaks: false,
        allow_arrow_functions: false,
        handle_annotations: false,
    };
}

/// Extract the value expression from a positional argument.
#[inline]
fn get_argument_value(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Positional { value, .. } => Some(*value),
        _ => None,
    }
}

/// Collect ternary chain into a flat list of (condition, then) pairs plus final else.
#[allow(clippy::type_complexity)]
fn collect_ternary_chain(
    tree: &NodeTree,
    node_id: LocalNodeId<Expression>,
) -> (
    Vec<(LocalNodeId<Expression>, LocalNodeId<Expression>)>,
    Option<LocalNodeId<Expression>>,
) {
    let mut branches = Vec::new();
    let mut current = node_id;

    loop {
        let Expression::If {
            kind: IfKind::Ternary,
            condition,
            then_expression,
            else_expression,
            ..
        } = tree.get(current)
        else {
            break;
        };

        let condition_id = match condition {
            IfCondition::Expression { condition } => *condition,
            IfCondition::Let { .. } => break,
        };
        branches.push((condition_id, *then_expression));

        // check if else is another ternary
        let Some(else_id) = else_expression else {
            return (branches, None);
        };

        if matches!(
            tree.get(*else_id),
            Expression::If {
                kind: IfKind::Ternary,
                ..
            }
        ) {
            current = *else_id;
        } else {
            return (branches, Some(*else_id));
        }
    }

    (branches, None)
}

/// Format a ternary expression with Prettier-style breaking.
/// Nested ternaries get progressive indentation when they break.
fn format_ternary(
    f: &mut DestackFormatter<'_, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let (branches, final_else) = collect_ternary_chain(tree, node_id);

    if branches.len() == 1 {
        // simple ternary
        let (condition, then_expr) = branches[0];
        write!(
            f,
            [group(&format_args![
                condition,
                indent(&format_args![
                    soft_line_break_or_space(),
                    token("?"),
                    space(),
                    then_expr,
                    soft_line_break_or_space(),
                    token(":"),
                    space(),
                    final_else
                ]),
            ])]
        )
    } else {
        // nested ternary chain: all branches at same indent level
        write!(
            f,
            [group(&format_with(|f| {
                for (condition, then_expr) in branches.iter() {
                    write!(f, [condition])?;
                    write!(
                        f,
                        [indent(&format_args![
                            soft_line_break_or_space(),
                            token("?"),
                            space(),
                            then_expr,
                            soft_line_break_or_space(),
                            token(":"),
                            space(),
                        ])]
                    )?;
                }
                write!(f, [final_else])
            }))]
        )
    }
}

/// Check if an argument list contains a single multi-line JSX element.
/// Multi-line JSX (with children) should not be hugged in function calls.
#[inline]
fn has_multiline_jsx_argument(tree: &NodeTree, arguments: &[LocalNodeId<Argument>]) -> bool {
    if arguments.len() != 1 {
        return false;
    }

    let Some(value_id) = get_argument_value(tree, arguments[0]) else {
        return false;
    };

    // check if it's a JSX element with children
    if let Expression::TreeExpression { elements, .. } = tree.get(value_id) {
        elements.as_ref().is_some_and(|e| !e.is_empty())
    } else {
        false
    }
}

/// Get the value expression for any tree attribute argument variant.
fn tree_attribute_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> Option<LocalNodeId<Expression>> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => Some(*value),
    }
}

/// Check whether a property value is complex enough to force breaks.
fn property_has_complex_value(
    context: &DestackFormatContext<'_>,
    property_id: LocalNodeId<Property>,
) -> bool {
    let tree = context.tree;

    // annotations on the property force complexity
    if context.has_annotation(property_id) {
        return true;
    }

    let property = tree.get(property_id);

    // field values and defaults can be complex
    if let Property::Field { value, default, .. } = property {
        // inspect the field value
        let value_is_complex = value.is_some_and(|value_id| {
            let value_expr = tree.get(value_id);
            is_complex_expression(tree, value_expr) || context.has_annotation(value_id)
        });

        // inspect the field default
        let default_is_complex = default.is_some_and(|default_id| {
            let default_expr = tree.get(default_id);
            is_complex_expression(tree, default_expr) || context.has_annotation(default_id)
        });

        return value_is_complex || default_is_complex;
    }

    // methods with bodies are always complex in object literals
    if let Property::Method { body, .. } = property {
        return body.is_some();
    }

    // spread properties inherit complexity from their value
    if let Property::Spread { value, .. } = property {
        let value_expr = tree.get(*value);
        return is_complex_expression(tree, value_expr) || context.has_annotation(*value);
    }

    false
}

/// Decide whether tree attributes should force the element to break.
fn should_force_break_tree_attributes(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> bool {
    let tree = context.tree;
    let line_width = usize::from(context.options.line_width);
    let complex_len_threshold = (line_width / 2).max(24);

    // comments on attributes force a break
    if arguments
        .iter()
        .copied()
        .any(|argument_id| context.has_annotation(argument_id))
    {
        return true;
    }

    for argument_id in arguments {
        let Some(value_id) = tree_attribute_value_id(tree, *argument_id) else {
            continue;
        };

        // comments on attribute values force a break
        if context.has_annotation(value_id) {
            return true;
        }

        // preserve explicit multiline attribute values
        let value_span = context.get_span(value_id);
        if context.has_newline(value_span) {
            return true;
        }

        // collect value signals for complexity checks
        let value_source_len = expression_source_len(context, value_id);
        let value_expr = tree.get(value_id);

        // complex object and array values should break the element
        match value_expr {
            Expression::ObjectExpression { properties, .. } => {
                // collect object signals
                let has_many_properties = properties.len() > 1;
                let has_complex_property = properties
                    .iter()
                    .copied()
                    .any(|property_id| property_has_complex_value(context, property_id));

                // break when the object is clearly complex
                let should_break_object = has_many_properties
                    && (has_complex_property || value_source_len > complex_len_threshold);

                if should_break_object {
                    return true;
                }
            }
            Expression::ArrayExpression { elements } => {
                // collect array signals
                let has_many_elements = elements.len() > 1;
                let has_complex_element = elements.iter().any(|element_id| {
                    // annotations on the element force complexity
                    let element_has_annotation = context.has_annotation(*element_id);

                    // inspect the element value when present
                    let element_value_is_complex = tree_attribute_value_id(tree, *element_id)
                        .is_some_and(|element_value_id| {
                            let element_expr = tree.get(element_value_id);
                            is_complex_expression(tree, element_expr)
                                || context.has_annotation(element_value_id)
                        });

                    element_has_annotation || element_value_is_complex
                });

                // break when the array is clearly complex
                let should_break_array = has_many_elements
                    && (has_complex_element || value_source_len > complex_len_threshold);

                if should_break_array {
                    return true;
                }
            }
            Expression::TreeExpression { elements, .. } => {
                // nested trees with children force a break
                let has_children = elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty());

                if has_children {
                    return true;
                }
            }
            _ => {}
        }
    }

    false
}

/// Check if an expression is huggable with the given configuration.
#[inline]
fn is_huggable_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    config: &HugOptions,
) -> bool {
    match tree.get(expression_id) {
        Expression::ObjectExpression { .. } | Expression::ArrayExpression { .. } => true,
        Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
            // arrow functions stay hugged when used as the only call argument
            if let Declaration::Function {
                signature,
                body: Some(_),
                ..
            } = tree.get(*declaration_id)
            {
                signature.kind == FunctionKind::Lambda
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Format a single-element argument list with hugging for expandable elements.
///
/// When a single object/array (or arrow function for calls) is the only argument,
/// format as `foo({...})` instead of `foo(\n    {...},\n)`.
/// Returns true if hugging was applied, false if regular list_like should be used.
fn format_hugged<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
    config: HugOptions,
    group_id: Option<GroupId>,
    force_expand: bool,
) -> FormatResult<bool> {
    // only hug single positional arguments
    if arguments.len() != 1 {
        return Ok(false);
    }

    let argument_id = arguments[0];
    let tree = f.context().tree;

    let Some(value_id) = get_argument_value(tree, argument_id) else {
        return Ok(false);
    };
    let value_id = transparent_inner_expression(f.context(), value_id);

    if !is_huggable_expression(tree, value_id, &config) {
        return Ok(false);
    }

    let arrow_declaration_id = match tree.get(value_id) {
        Expression::Declaration(declaration_id) => match tree.get(*declaration_id) {
            Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda => {
                Some(*declaration_id)
            }
            _ => None,
        },
        _ => None,
    };
    let is_arrow_function = arrow_declaration_id.is_some();
    let (arrow_body_is_block, arrow_body_is_tree) = arrow_declaration_id
        .and_then(|declaration_id| {
            let Declaration::Function {
                body: Some(body_id),
                ..
            } = tree.get(declaration_id)
            else {
                return None;
            };

            let body_id = transparent_inner_expression(f.context(), *body_id);
            let body_expr = tree.get(body_id);
            let is_block = matches!(body_expr, Expression::Block(_));
            let is_tree = matches!(body_expr, Expression::TreeExpression { .. });
            Some((is_block, is_tree))
        })
        .unwrap_or((false, false));
    let arrow_force_expand = arrow_declaration_id
        .is_some_and(|declaration_id| lambda_expression_should_break(f.context(), declaration_id))
        || (is_arrow_function
            && expression_source_len(f.context(), value_id)
                > usize::from(f.context().options.line_width));
    let trailing_if_breaks = config.trailing_if_breaks
        || (is_arrow_function && !arrow_body_is_block && !arrow_body_is_tree);

    // inline: keep everything on one line
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let inline_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(f, [token(config.open), argument_id])?;
            if config.force_trailing {
                write!(f, [token(",")])?;
            }
            write!(f, [token(config.close)])
        });

        if let Some(group_id) = group_id {
            group(&inline_inner).with_id(Some(group_id)).format(f)
        } else {
            write!(f, [inline_inner])
        }
    });

    // hugged: argument expands but delimiters hug
    let hugged_format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if config.handle_annotations {
            write!(f, [f.context().any_prefix_annotations(argument_id)])?;
        }

        if arrow_force_expand {
            write!(f, [expand_parent()])?;
        }

        write!(f, [token(config.open)])?;

        match f.context().tree.get(value_id) {
            Expression::ObjectExpression { ty, properties } => {
                if let Some(ty) = ty {
                    write!(f, [ty, space()])?;
                }
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("{"),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(properties)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("}")
                            ]
                        )
                    }))
                    .should_expand(true),]
                )?;
            }
            Expression::ArrayExpression { elements } => {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        write!(
                            f,
                            [
                                token("["),
                                block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    f.join_with(&format_args![
                                        token(","),
                                        soft_line_break_or_space()
                                    ])
                                    .entries(elements)
                                    .finish()?;
                                    write!(f, [if_group_breaks(&token(","))])
                                })),
                                token("]")
                            ]
                        )
                    }))
                    .should_expand(true)]
                )?;
            }
            Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
                // arrow function: format normally, handles its own expansion
                if let Some(group_id) = group_id {
                    write!(
                        f,
                        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(f, [declaration_id])
                        }))
                        .with_id(Some(group_id))]
                    )?;
                } else {
                    write!(f, [declaration_id])?;
                }
            }
            _ => {
                write!(f, [value_id])?;
            }
        }

        if config.force_trailing {
            write!(f, [token(",")])?;
        } else if is_arrow_function {
            if !arrow_body_is_block && !arrow_body_is_tree {
                write!(f, [token(",")])?;
            }
        } else if trailing_if_breaks {
            let comma = token(",");
            let trailing_comma = if let Some(group_id) = group_id {
                if_group_breaks(&comma).with_group_id(Some(group_id))
            } else {
                if_group_breaks(&comma)
            };
            write!(f, [trailing_comma])?;
        }

        if is_arrow_function && !arrow_body_is_block {
            write!(f, [hard_line_break()])?;
        }

        write!(f, [token(config.close)])?;

        if config.handle_annotations {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(argument_id)]
            )?;
        }
        Ok(())
    });

    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if trailing_if_breaks || group_id.is_some() {
            write!(
                f,
                [group(&hugged_format_inner)
                    .with_id(group_id)
                    .should_expand(arrow_force_expand)]
            )?;
        } else {
            write!(f, [hugged_format_inner])?;
        }
        Ok(())
    });

    if force_expand {
        hugged_format.format(f)?;
    } else {
        best_fitting![inline_format, hugged_format]
            .with_mode(BestFittingMode::AllLines)
            .format(f)?;
    }

    Ok(true)
}

/// Format a tree/JSX attribute value with hugging for objects/arrays.
fn format_tree_attribute_value<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let value_expr = tree.get(value_id);

    match value_expr {
        Expression::ObjectExpression { ty, properties } => {
            let ty = *ty;
            let properties = properties.clone();

            // inline: ={value} all on one line
            let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [token("="), token("{"), value_id, token("}")])
            });

            // hugged: ={{ with expanded object contents then }}
            let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [
                        token("="),
                        token("{"),
                        format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            if let Some(ty) = ty {
                                write!(f, [ty, space()])?;
                            }
                            write!(
                                f,
                                [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                                    write!(
                                        f,
                                        [
                                            token("{"),
                                            block_indent(&format_with(
                                                |f: &mut DestackFormatter<'ast, '_>| {
                                                    f.join_with(&format_args![
                                                        token(","),
                                                        soft_line_break_or_space()
                                                    ])
                                                    .entries(&properties)
                                                    .finish()?;
                                                    write!(f, [if_group_breaks(&token(","))])
                                                }
                                            )),
                                            token("}")
                                        ]
                                    )
                                }))
                                .should_expand(true)]
                            )
                        }),
                        token("}")
                    ]
                )
            });

            best_fitting![inline_format, hugged_format]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
        }
        Expression::ArrayExpression { elements } => {
            let elements = elements.clone();

            // inline: ={value} all on one line
            let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(f, [token("="), token("{"), value_id, token("}")])
            });

            // hugged: ={[ with expanded array contents then ]}
            let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [
                        token("="),
                        token("{"),
                        group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                            write!(
                                f,
                                [
                                    token("["),
                                    block_indent(&format_with(
                                        |f: &mut DestackFormatter<'ast, '_>| {
                                            f.join_with(&format_args![
                                                token(","),
                                                soft_line_break_or_space()
                                            ])
                                            .entries(&elements)
                                            .finish()?;
                                            write!(f, [if_group_breaks(&token(","))])
                                        }
                                    )),
                                    token("]")
                                ]
                            )
                        }))
                        .should_expand(true),
                        token("}")
                    ]
                )
            });

            best_fitting![inline_format, hugged_format]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
        }
        _ => {
            // regular format for non-huggable values
            write!(f, [token("="), token("{"), value_id, token("}")])?;
        }
    }

    Ok(())
}

/// Returns the precedence group for a binary operator.
#[inline]
fn binary_operator_precedence_group(operator: BinaryOperator) -> u8 {
    // first two digits of discriminant encode precedence
    (operator as u16 / 100) as u8
}

/// Checks if two binary operators should be flattened together.
#[inline]
fn should_flatten_binary(left_operator: BinaryOperator, right_operator: BinaryOperator) -> bool {
    binary_operator_precedence_group(left_operator)
        == binary_operator_precedence_group(right_operator)
}

/// Represents a flattened binary expression operand with its preceding operator.
struct BinaryOperand {
    /// The operator before this operand (None for first).
    operator: Option<BinaryOperator>,
    /// The expression node.
    expression: LocalNodeId<Expression>,
}

/// Flattens a binary expression chain into a list of operands.
///
/// For `a + b + c`, returns [(None, a), (Some(+), b), (Some(+), c)].
fn flatten_binary_expression(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
) -> Vec<BinaryOperand> {
    let mut operands = Vec::new();
    flatten_binary_recursive(tree, expression_id, target_operator, &mut operands, None);
    operands
}

fn flatten_binary_recursive(
    tree: &NodeTree,
    expression_id: LocalNodeId<Expression>,
    target_operator: BinaryOperator,
    operands: &mut Vec<BinaryOperand>,
    preceding_operator: Option<BinaryOperator>,
) {
    if let Expression::Binary {
        left,
        operator,
        right,
    } = tree.get(expression_id)
        && should_flatten_binary(*operator, target_operator)
    {
        // recursively flatten the left side
        flatten_binary_recursive(tree, *left, target_operator, operands, None);

        // add the right operand with its operator
        operands.push(BinaryOperand {
            operator: Some(*operator),
            expression: *right,
        });
        return;
    }

    // not a binary expression or different precedence - add as-is
    operands.push(BinaryOperand {
        operator: preceding_operator,
        expression: expression_id,
    });
}

/// Whether an expression variant is type specific.
#[inline]
fn is_type_expression_variant(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::TypeUnary { .. }
            | Expression::TypeBinary { .. }
            | Expression::TypeConditional { .. }
            | Expression::TypeMapped { .. }
            | Expression::TypeIndex { .. }
            | Expression::TypeTemplateLiteral { .. }
            | Expression::TypeImport { .. }
            | Expression::TypeInfer { .. }
            | Expression::TypePredicate { .. }
    )
}

/// Whether a binary expression is in a type position.
fn is_type_context(context: &DestackFormatContext<'_>, node_id: LocalNodeId<Expression>) -> bool {
    let mut current_id = node_id.id;

    // walk ancestors and check for type slots
    while let Some((parent_id, parent_type)) = context.get_parent_by_id(current_id) {
        match parent_type {
            // type specific expressions imply type context
            NodeType::Expression => {
                let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
                if is_type_expression_variant(parent_expr) {
                    return true;
                }
            }

            // declarator type annotation
            NodeType::Declarator => {
                let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
                if declarator.ty.is_some_and(|ty| ty.id == current_id) {
                    return true;
                }
            }

            // parameter type annotation
            NodeType::Parameter => {
                let parameter = context.tree.get(LocalNodeId::<Parameter>::new(parent_id));
                let parameter_ty = match parameter {
                    Parameter::Named { ty, .. }
                    | Parameter::Pattern { ty, .. }
                    | Parameter::Variadic { ty, .. } => *ty,
                };
                if parameter_ty.is_some_and(|ty| ty.id == current_id) {
                    return true;
                }
            }

            // where clause constraint
            NodeType::WhereClause => {
                let where_clause = context.tree.get(LocalNodeId::<WhereClause>::new(parent_id));
                if where_clause.right.id == current_id {
                    return true;
                }
            }

            // property field type
            NodeType::Property => {
                let property = context.tree.get(LocalNodeId::<Property>::new(parent_id));
                if let Property::Field { value, .. } = property
                    && value.is_some_and(|value| value.id == current_id)
                {
                    return true;
                }
            }

            // member type slots
            NodeType::Member => {
                let member = context.tree.get(LocalNodeId::<Member>::new(parent_id));
                let is_type_slot = match member {
                    Member::Type {
                        name, ty, value, ..
                    } => {
                        name.id == current_id
                            || ty.is_some_and(|ty| ty.id == current_id)
                            || value.is_some_and(|value| value.id == current_id)
                    }
                    Member::Field { value, .. } => {
                        value.is_some_and(|value| value.id == current_id)
                    }
                    Member::Embed { value, .. } => value.id == current_id,
                    Member::Method { .. }
                    | Member::StaticBlock { .. }
                    | Member::ComptimeBlock { .. } => false,
                };
                if is_type_slot {
                    return true;
                }
            }

            // declaration type slots
            NodeType::Declaration => {
                let declaration = context.tree.get(LocalNodeId::<Declaration>::new(parent_id));
                let in_type_slot = match declaration {
                    Declaration::Type { value, .. } => value.id == current_id,
                    Declaration::Struct { heritage, .. }
                    | Declaration::Class { heritage, .. }
                    | Declaration::Interface { heritage, .. }
                    | Declaration::Enum { heritage, .. } => {
                        heritage
                            .extends_types
                            .as_ref()
                            .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
                            || heritage
                                .implements_types
                                .as_ref()
                                .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
                    }
                    Declaration::Extension {
                        target_type,
                        heritage,
                        ..
                    } => {
                        target_type.id == current_id
                            || heritage
                                .extends_types
                                .as_ref()
                                .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
                            || heritage
                                .implements_types
                                .as_ref()
                                .is_some_and(|types| types.iter().any(|ty| ty.id == current_id))
                    }
                    Declaration::Function { signature, .. } => signature
                        .return_type
                        .is_some_and(|return_type| return_type.id == current_id),
                    Declaration::Global { .. } | Declaration::Namespace { .. } => false,
                };
                if in_type_slot {
                    return true;
                }
            }

            _ => {}
        }

        current_id = parent_id;
    }

    false
}

/// Format call arguments with list-group awareness.
fn format_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    call_node_id: LocalNodeId<Expression>,
    dynamic_arguments: &Vec<LocalNodeId<Argument>>,
) -> FormatResult<()> {
    // set up the list group id for conditional formatting
    let group_id = f.group_id("call_args");
    let previous_group_id = f.context().current_argument_group_id;
    f.context_mut().current_argument_group_id = Some(group_id);

    let result = (|| {
        let force_hugged_expand = if dynamic_arguments.len() == 1 {
            let argument_id = dynamic_arguments[0];
            let value_id = argument_value_id(f.context().tree, argument_id);
            let value_id = transparent_inner_expression(f.context(), value_id);
            let is_arrow_argument = matches!(
                f.context().tree.get(value_id),
                Expression::Declaration(declaration_id)
                    if matches!(
                        f.context().tree.get(*declaration_id),
                        Declaration::Function { signature, .. }
                            if signature.kind == FunctionKind::Lambda
                    )
            );

            if is_arrow_argument {
                let line_width = usize::from(f.context().options.line_width);
                let threshold = line_width.saturating_sub(1);
                expression_source_len(f.context(), call_node_id) >= threshold
            } else {
                false
            }
        } else {
            false
        };

        // try hugged format for single object/array arguments
        if format_hugged(
            f,
            dynamic_arguments,
            HugOptions::CALL,
            Some(group_id),
            force_hugged_expand,
        )? {
            return Ok(());
        }

        // decide if the argument list must expand
        let force_expand_jsx = has_multiline_jsx_argument(f.context().tree, dynamic_arguments);
        let has_annotations = f.context().has_infix_annotation(call_node_id)
            || dynamic_arguments
                .iter()
                .any(|argument_id| f.context().has_annotation(*argument_id));

        // expand argument lists when any argument is complex
        let force_expand_complex = dynamic_arguments.len() > 1
            && dynamic_arguments.iter().any(|argument_id| {
                let value_id = argument_value_id(f.context().tree, *argument_id);
                let value_id = transparent_inner_expression(f.context(), value_id);

                // multiline jsx handling is delegated to force_expand_jsx
                if matches!(
                    f.context().tree.get(value_id),
                    Expression::TreeExpression { .. }
                ) {
                    return false;
                }

                let argument = f.context().tree.get(*argument_id);
                is_complex_argument(f.context().tree, argument)
            });
        let force_expand = force_expand_jsx || has_annotations || force_expand_complex;

        let list_format = format_with(|f| {
            let mut list = list_like("(", ")", ",", dynamic_arguments);
            list.with_group_id(Some(group_id))
                .should_expand(force_expand);
            write!(f, [list])
        });

        let can_hug_last_argument = !force_expand
            && dynamic_arguments.len() > 1
            && dynamic_arguments
                .last()
                .is_some_and(|argument_id| is_block_lambda_argument(f.context(), *argument_id));

        if can_hug_last_argument {
            let hug_last_format = format_with(|f| {
                write!(f, [token("(")])?;

                for (index, argument_id) in dynamic_arguments.iter().enumerate() {
                    if index > 0 {
                        write!(f, [token(","), space()])?;
                    }

                    write!(f, [*argument_id])?;
                }

                write!(f, [token(")")])
            });

            best_fitting![hug_last_format, list_format]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
            return Ok(());
        }

        list_format.format(f)
    })();

    f.context_mut().current_argument_group_id = previous_group_id;

    result
}

/// Format a call expression without considering chaining.
#[inline]
fn format_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Call {
        position,
        left,
        static_arguments,
        dynamic_arguments,
    } = f.context().tree.get(node_id)
    {
        write!(f, [*left])?;
        if *position == PostfixPosition::Indirect {
            write!(f, [token(".")])?;
        }
        if let Some(static_arguments) = static_arguments {
            write!(f, [list_like("<", ">", ",", static_arguments)])?;
        }
        format_call_arguments(f, node_id, dynamic_arguments)?;
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Get the precedence of an expression.
///
/// Returns the operator precedence for expressions that have one, or `u16::MAX` for
/// atomic/primary expressions (identifiers, literals, parenthesized, etc.).
fn expression_precedence(expr: &Expression) -> u16 {
    match expr {
        // postfix operators (2000)
        Expression::Call { .. }
        | Expression::Member { .. }
        | Expression::Index { .. }
        | Expression::Maybe { .. }
        | Expression::Must { .. } => OperatorPrecedence::Postfix as u16,

        // postfix unary (2000)
        Expression::Unary { operator, .. } if operator.is_postfix() => {
            OperatorPrecedence::Postfix as u16
        }

        // prefix unary (1900)
        Expression::Unary { .. } => OperatorPrecedence::Prefix as u16,

        // prefix expressions (1900)
        Expression::Await { .. }
        | Expression::AwaitMaybe { .. }
        | Expression::Yield { .. }
        | Expression::Delete { .. }
        | Expression::Throw { .. }
        | Expression::Return { .. } => OperatorPrecedence::Prefix as u16,

        // type unary: use operator's precedence
        Expression::TypeUnary { operator, .. } => operator.precedence(),

        // binary: use operator's precedence
        Expression::Binary { operator, .. } => operator.precedence(),
        Expression::TypeBinary { operator, .. } => operator.precedence(),

        // assignment: use operator's precedence
        Expression::Assign { operator, .. } => operator.precedence(),

        // ternary: lower than all binary/assignment operators
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => OperatorPrecedence::AssignmentBoolean as u16 - 1,

        // atomic/primary expressions: highest precedence (never need parens)
        _ => u16::MAX,
    }
}

/// Returns true if the expression needs parentheses when used as the operand
/// of a postfix operator like `?` or `!`.
///
/// Postfix operators (precedence 2000) bind tighter than all other operators.
/// For example, `await x?` parses as `await (x?)`, not `(await x)?`.
/// So when formatting `Maybe { left: Await { expr } }`, we need to output `(await expr)?`.
#[inline]
fn needs_parens_in_postfix_position(tree: &NodeTree, expr_id: LocalNodeId<Expression>) -> bool {
    expression_precedence(tree.get(expr_id)) < OperatorPrecedence::Postfix as u16
}

/// Format a maybe expression without considering chaining.
#[inline]
fn format_maybe_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Maybe { left, position } = f.context().tree.get(node_id) {
        let needs_parentheses = needs_parens_in_postfix_position(f.context().tree, *left);
        if needs_parentheses {
            write!(f, [token("("), *left, token(")")])?;
        } else {
            write!(f, [*left])?;
        }
        match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
        }
    } else {
        debug_assert!(false, "unexpected expression kind for maybe formatter");
    }
    Ok(())
}

/// The head of a chain before any postfix operations.
#[derive(Clone)]
enum ChainExpressionBaseHead {
    Path {
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    Expression(LocalNodeId<Expression>),
}

/// The initial portion of the chain including direct postfix ops.
#[derive(Clone)]
struct ChainExpressionBase {
    head: ChainExpressionBaseHead,
    body: Vec<ChainExpression>,
}

/// One operation in an expression chain.
#[derive(Clone)]
enum ChainExpression {
    /// Member expression.
    Member {
        node_id: LocalNodeId<Expression>,
        segment: StringId,
        static_arguments: Option<Vec<LocalNodeId<Argument>>>,
    },
    /// Call expression.
    Call {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        dynamic_arguments: Vec<LocalNodeId<Argument>>,
    },
    /// Index expression.
    Index {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
        index: Option<LocalNodeId<Expression>>,
    },
    /// Maybe expression.
    Maybe {
        node_id: LocalNodeId<Expression>,
        position: PostfixPosition,
    },
}

/// Format the base portion of the chain.
fn format_chain_base<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    base: &ChainExpressionBase,
) -> FormatResult<()> {
    match &base.head {
        ChainExpressionBaseHead::Path {
            segment,
            static_arguments,
        } => {
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", arguments)])?;
            }
        }
        ChainExpressionBaseHead::Expression(node_id) => {
            let expression = f.context().tree.get(*node_id);
            debug_assert!(
                !matches!(
                    expression,
                    Expression::Member { .. }
                        | Expression::Call { .. }
                        | Expression::Index { .. }
                        | Expression::Maybe { .. }
                ),
                "chain base expression should not be another chain node"
            );
            write!(f, [*node_id])?;
        }
    }

    for op in &base.body {
        format_chain_expression(f, op)?;
    }

    Ok(())
}

/// Format one chained operation.
fn format_chain_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    op: &ChainExpression,
) -> FormatResult<()> {
    // output any line prefix annotations before the operation
    let node_id = match op {
        ChainExpression::Member { node_id, .. }
        | ChainExpression::Call { node_id, .. }
        | ChainExpression::Index { node_id, .. }
        | ChainExpression::Maybe { node_id, .. } => *node_id,
    };
    write!(f, [f.context().line_prefix_annotations(node_id)])?;

    match op {
        ChainExpression::Member {
            segment,
            static_arguments,
            ..
        } => {
            write!(f, [token(".")])?;
            write!(f, [*segment])?;
            if let Some(arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", arguments)])?;
            }
        }
        ChainExpression::Call {
            node_id: call_node_id,
            position,
            dynamic_arguments,
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            format_call_arguments(f, *call_node_id, dynamic_arguments)?;
        }
        ChainExpression::Index {
            position, index, ..
        } => {
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            if let Some(index) = index {
                write!(f, [token("["), *index, token("]")])?;
            } else {
                write!(f, [token("[]")])?;
            }
        }
        ChainExpression::Maybe { position, .. } => match position {
            PostfixPosition::Direct => write!(f, [token("?")])?,
            PostfixPosition::Indirect => {
                write!(f, [token("."), token("?")])?;
            }
        },
    }

    // output any line postfix annotations after the operation
    write!(f, [f.context().line_postfix_annotations(node_id)])?;

    Ok(())
}

/// Format all operations for one chain line.
fn format_chain_expression_line<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    ops: &[ChainExpression],
) -> FormatResult<()> {
    for op in ops {
        format_chain_expression(f, op)?;
    }
    Ok(())
}

/// Group chain operations into the segments that should share lines.
fn group_chain_expression_lines(
    operations: Vec<ChainExpression>,
) -> Vec<SmallVec<[ChainExpression; 2]>> {
    let mut lines = Vec::new();
    let mut iter = operations.into_iter().peekable();
    while let Some(op) = iter.next() {
        let mut line = smallvec![op.clone()];
        match op {
            // (maybe)
            ChainExpression::Maybe { .. } => {
                match iter.peek() {
                    // (maybe, member)
                    Some(ChainExpression::Member { .. }) => {
                        line.push(iter.next().unwrap());
                        // (maybe, member, index | call)
                        if let Some(ChainExpression::Index { .. } | ChainExpression::Call { .. }) =
                            iter.peek()
                        {
                            line.push(iter.next().unwrap());
                        }
                    }
                    // (maybe, index | call)
                    Some(ChainExpression::Index { .. } | ChainExpression::Call { .. }) => {
                        line.push(iter.next().unwrap());
                    }
                    _ => {}
                }
            }
            // (member)
            ChainExpression::Member { .. } => {
                // (member, index | call)
                if let Some(ChainExpression::Call { .. } | ChainExpression::Index { .. }) =
                    iter.peek()
                {
                    line.push(iter.next().unwrap());
                }
            }
            // (index | call)
            ChainExpression::Index { .. } | ChainExpression::Call { .. } => {
                // end of line
            }
        }
        lines.push(line);
    }

    lines
}

/// Check whether the expression is part of a member/call/maybe/index chain.
pub(crate) fn is_expression_chain(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. } => is_chain_expression(tree.get(*left)),
        _ => false,
    }
}

/// Check whether an expression is the root of a chain.
pub(crate) fn is_chain_root(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. } => !is_chain_expression(tree.get(*left)),
        _ => false,
    }
}

/// Walk upward through transparent wrappers to find an assignment-like parent rhs.
fn assignment_like_parent(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<(NodeType, u32)> {
    let mut current_id = node_id.id;

    // walk through transparent wrappers until we reach an assignment-like parent
    while let Some((parent_id, parent_type)) = context.get_parent_by_id(current_id) {
        match parent_type {
            NodeType::Expression => {
                let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));

                // assignment rhs
                if let Expression::Assign { right, .. } = parent_expr
                    && right.id == current_id
                {
                    return Some((NodeType::Expression, parent_id));
                }

                // transparent wrappers around the rhs
                let is_wrapper_parent = matches!(
                    parent_expr,
                    Expression::Await { expression }
                        | Expression::AwaitMaybe { expression }
                        | Expression::Parenthesized { expression }
                        if expression.id == current_id
                );

                if is_wrapper_parent {
                    current_id = parent_id;
                    continue;
                }

                return None;
            }
            NodeType::Declarator => {
                let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
                if declarator.value.is_some_and(|value| value.id == current_id) {
                    return Some((NodeType::Declarator, parent_id));
                }
                return None;
            }
            _ => return None,
        }
    }

    None
}

/// Unwrap transparent wrappers around an expression for classification.
fn transparent_inner_expression(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> LocalNodeId<Expression> {
    let mut current = node_id;

    // peel transparent wrappers when they have no annotations
    loop {
        if context.has_annotation(current) {
            return current;
        }

        let next = match context.tree.get(current) {
            Expression::Await { expression }
            | Expression::AwaitMaybe { expression }
            | Expression::Parenthesized { expression } => Some(*expression),
            _ => None,
        };

        match next {
            Some(next_id) => current = next_id,
            None => return current,
        }
    }
}

/// Get the display width of an assignment operator token.
fn assign_operator_len(operator: &AssignOperator) -> usize {
    match operator {
        AssignOperator::Assign => 1,
        AssignOperator::MultiplyAssign
        | AssignOperator::DivideAssign
        | AssignOperator::RemainderAssign
        | AssignOperator::AddAssign
        | AssignOperator::SubtractAssign
        | AssignOperator::ElementwiseAndAssign
        | AssignOperator::ElementwiseXorAssign
        | AssignOperator::ElementwiseOrAssign => 2,
        AssignOperator::WrappingMultiplyAssign
        | AssignOperator::SaturatingMultiplyAssign
        | AssignOperator::WrappingAddAssign
        | AssignOperator::SaturatingAddAssign
        | AssignOperator::WrappingSubtractAssign
        | AssignOperator::SaturatingSubtractAssign
        | AssignOperator::ShiftLeftAssign
        | AssignOperator::ShiftRightAssign
        | AssignOperator::AndAssign
        | AssignOperator::OrAssign
        | AssignOperator::CoalesceAssign
        | AssignOperator::ExponentAssign => 3,
        AssignOperator::WrappingExponentAssign
        | AssignOperator::SaturatingExponentAssign
        | AssignOperator::SaturatingShiftLeftAssign
        | AssignOperator::UnsignedShiftRightAssign => 4,
    }
}

/// Estimate the remaining inline width for a rhs in an assignment-like parent.
fn assignment_like_remaining_width(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<usize> {
    let line_width = usize::from(context.options.line_width);
    let (parent_type, parent_id) = assignment_like_parent(context, node_id)?;

    // compute remaining width based on the specific parent form
    match parent_type {
        NodeType::Expression => {
            let parent_expr = context.tree.get(LocalNodeId::<Expression>::new(parent_id));
            let Expression::Assign { left, operator, .. } = parent_expr else {
                return None;
            };

            // account for `left <space> op <space>`
            let left_source_len = expression_source_len(context, *left);
            let operator_len = assign_operator_len(operator);
            let inline_overhead = left_source_len
                .saturating_add(operator_len)
                .saturating_add(2);

            Some(line_width.saturating_sub(inline_overhead))
        }
        NodeType::Declarator => {
            let declarator = context.tree.get(LocalNodeId::<Declarator>::new(parent_id));
            let pattern_span = context.get_span(declarator.pattern);
            let pattern_source_len = context.get_span_str(pattern_span).chars().count();
            let type_source_len = declarator
                .ty
                .map(|ty_id| expression_source_len(context, ty_id));
            let header_source_len = type_source_len.map_or(pattern_source_len, |type_len| {
                pattern_source_len
                    .saturating_add(type_len)
                    .saturating_add(2)
            });

            // account for `header <space> = <space>`
            let remaining_width = line_width.saturating_sub(header_source_len.saturating_add(3));

            // be conservative: declarators often have a leading keyword
            Some(remaining_width.saturating_sub(DECLARATOR_PREFIX_PADDING))
        }
        _ => None,
    }
}

/// Get the source length of an expression span.
fn expression_source_len(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> usize {
    let span = context.get_span(expression_id);
    context.get_span_str(span).chars().count()
}

/// Get the root head expression of a chain.
fn chain_head_id(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> LocalNodeId<Expression> {
    let mut current = node_id;

    loop {
        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. } => Some(*left),
            _ => None,
        };

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(next_id) => return next_id,
            None => return current,
        }
    }
}

/// Check whether the chain head is simple and short.
fn is_simple_chain_head(
    context: &DestackFormatContext<'_>,
    head_id: LocalNodeId<Expression>,
) -> bool {
    let threshold = usize::from(context.options.line_width) / 4;
    let expression = context.tree.get(head_id);

    match expression {
        // simple identifier style heads
        Expression::Path {
            path,
            static_arguments,
        } => {
            static_arguments.is_none()
                && !context.has_annotation(head_id)
                && path.segments.len() <= 2
                && expression_source_len(context, head_id) <= threshold.max(6)
        }
        _ => false,
    }
}

/// Check whether an argument is short enough for poorly breakable chains.
fn is_short_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let threshold = usize::from(context.options.line_width) / 4;
    let argument = context.tree.get(argument_id);

    // reject annotated arguments immediately
    if context.has_annotation(argument_id) {
        return false;
    }

    let value_id = match argument {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    };

    let value = context.tree.get(value_id);
    is_trivial_expression(context.tree, value)
        && expression_source_len(context, value_id) <= threshold.max(8)
        && !context.has_annotation(value_id)
}

/// A chain that has no calls at all or only short call arguments.
pub(crate) fn is_poorly_breakable_chain(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    // only chains qualify
    if !is_chain_root(tree, node_id) && !is_expression_chain(tree, node_id) {
        return false;
    }

    // find the chain head
    let head_id = chain_head_id(tree, node_id);
    if !is_simple_chain_head(context, head_id) {
        return false;
    }

    // walk the chain from the node down to the head, checking call arguments
    let mut current = node_id;
    let mut has_call = false;

    loop {
        if context.has_annotation(current) {
            return false;
        }

        match tree.get(current) {
            Expression::Call {
                dynamic_arguments, ..
            } => {
                has_call = true;

                // only allow no args or a single short argument
                let is_breakable_call = match dynamic_arguments.len() {
                    0 => false,
                    1 => !is_short_chain_argument(context, dynamic_arguments[0]),
                    _ => true,
                };

                if is_breakable_call {
                    return false;
                }
            }
            Expression::Index { index, .. } => {
                // indexes with non-trivial expressions are breakable
                if let Some(index_id) = index {
                    let index_expr = tree.get(*index_id);
                    if !is_trivial_expression(tree, index_expr) || context.has_annotation(*index_id)
                    {
                        return false;
                    }
                }
            }
            Expression::Member { .. } | Expression::Maybe { .. } => {}
            _ => break,
        }

        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. } => Some(*left),
            _ => None,
        };

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(_) | None => break,
        }
    }

    // plain member chains are also poorly breakable
    has_call || is_chain_root(tree, node_id)
}

/// Get the value expression of any argument variant.
fn argument_value_id(
    tree: &NodeTree,
    argument_id: LocalNodeId<Argument>,
) -> LocalNodeId<Expression> {
    match tree.get(argument_id) {
        Argument::Named { value, .. }
        | Argument::Labeled { value, .. }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => *value,
    }
}

/// Check whether an expression is a lambda declaration.
fn is_lambda_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return false;
    };

    matches!(
        tree.get(*declaration_id),
        Declaration::Function { signature, .. } if signature.kind == FunctionKind::Lambda
    )
}

/// Check whether an expression is a lambda whose body is another lambda.
fn is_nested_lambda_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    let Expression::Declaration(declaration_id) = tree.get(expression_id) else {
        return false;
    };

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    is_lambda_expression(context, *body_id)
}

/// Check whether a lambda body forces multi-line formatting.
fn lambda_body_forces_multiline(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let tree = context.tree;

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let body_id = transparent_inner_expression(context, *body_id);

    match tree.get(body_id) {
        Expression::Block(_) => true,
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => tree_literal_should_break(context, arguments, elements),
        _ => false,
    }
}

/// Compute the maximum nested callback depth inside an expression.
fn expression_callback_depth(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> usize {
    let tree = context.tree;
    let expression_id = transparent_inner_expression(context, expression_id);

    match tree.get(expression_id) {
        Expression::Call {
            dynamic_arguments, ..
        } => {
            let mut max_depth = 0usize;

            for argument_id in dynamic_arguments {
                let argument = tree.get(*argument_id);
                let value_id = match argument {
                    Argument::Positional { value, .. } | Argument::Spread { value, .. } => *value,
                    Argument::Named { value, .. } | Argument::Labeled { value, .. } => *value,
                };

                let value_id = transparent_inner_expression(context, value_id);

                let Expression::Declaration(declaration_id) = tree.get(value_id) else {
                    continue;
                };

                let Declaration::Function {
                    signature,
                    body: Some(body_id),
                    ..
                } = tree.get(*declaration_id)
                else {
                    continue;
                };

                if signature.kind != FunctionKind::Lambda {
                    continue;
                }

                let nested_depth = expression_callback_depth(context, *body_id);
                max_depth = max_depth.max(1 + nested_depth);
            }

            max_depth
        }
        _ => 0,
    }
}

/// Check whether a chain expression should break in its current context.
fn expression_chain_should_break(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    if !is_chain_expression(tree.get(expression_id)) {
        return false;
    }

    let mut chain = Vec::new();
    let mut current = expression_id;
    loop {
        chain.push(current);

        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. } => Some(*left),
            _ => None,
        };

        match next {
            Some(next_id) if is_chain_expression(tree.get(next_id)) => {
                current = next_id;
            }
            Some(_) | None => break,
        }
    }

    should_break_chain(context, &chain)
}

/// Check whether an expression is nested inside a tree literal child.
fn expression_is_in_tree_literal_child(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let mut current_id = expression_id.id;

    // walk up the parent chain looking for tree element arguments
    while let Some((parent_id, parent_type)) = context.get_parent_by_id(current_id) {
        if parent_type == NodeType::Argument {
            let argument_id = LocalNodeId::<Argument>::new(parent_id);

            // tree elements store children as arguments
            if let Some((grand_id, grand_type)) = context.get_parent_by_id(parent_id)
                && grand_type == NodeType::Expression
            {
                let parent_expression = context.tree.get(LocalNodeId::<Expression>::new(grand_id));
                if let Expression::TreeExpression {
                    elements: Some(elements),
                    ..
                } = parent_expression
                    && elements.contains(&argument_id)
                {
                    return true;
                }
            }
        }

        current_id = parent_id;
    }

    false
}

/// Check whether a lambda expression body should break across lines.
pub(crate) fn lambda_expression_should_break(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let tree = context.tree;

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let body_expr_id = transparent_inner_expression(context, *body_id);
    let body_expr = tree.get(body_expr_id);
    if matches!(body_expr, Expression::Block(_)) {
        return false;
    }

    if let Expression::TreeExpression {
        arguments,
        elements,
        ..
    } = body_expr
    {
        if tree_literal_should_break(context, arguments, elements) {
            return true;
        }

        // tree-returning callbacks in tree literals should break for readability
        if let Some((parent_id, parent_type)) = context.get_parent(declaration_id)
            && parent_type == NodeType::Expression
        {
            let expression_id = LocalNodeId::<Expression>::new(parent_id);
            if expression_is_in_tree_literal_child(context, expression_id) {
                return true;
            }
        }
    }

    if expression_chain_should_break(context, body_expr_id) {
        return true;
    }

    if expression_callback_depth(context, body_expr_id) >= 2 {
        return true;
    }

    false
}

/// Check whether a call argument forces multi-line formatting.
fn argument_forces_multiline(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);

    match context.tree.get(value_id) {
        Expression::Declaration(declaration_id) => {
            lambda_body_forces_multiline(context, *declaration_id)
        }
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => tree_literal_should_break(context, arguments, elements),
        _ => false,
    }
}

/// Check whether an argument is a block-bodied lambda.
fn is_block_lambda_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    let value_id = transparent_inner_expression(context, value_id);

    let Expression::Declaration(declaration_id) = context.tree.get(value_id) else {
        return false;
    };

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = context.tree.get(*declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    matches!(context.tree.get(*body_id), Expression::Block(_))
}

/// Check whether an argument value is function-like.
fn is_function_like_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let value_id = argument_value_id(context.tree, argument_id);
    is_lambda_expression(context, value_id)
}

/// Check whether an argument is simple enough to stay inline in chains.
fn is_simple_chain_argument(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let threshold = usize::from(context.options.line_width) / 4;

    // annotated arguments are never simple
    if context.has_annotation(argument_id) {
        return false;
    }

    let value_id = argument_value_id(context.tree, argument_id);

    // function-like arguments make the call complex
    if is_lambda_expression(context, value_id) || context.has_annotation(value_id) {
        return false;
    }

    let value = context.tree.get(value_id);
    is_trivial_expression(context.tree, value)
        && expression_source_len(context, value_id) <= threshold.max(10)
}

/// Sum the source lengths of argument values.
fn arguments_total_len(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> usize {
    // accumulate argument value lengths
    let mut total_len = 0usize;

    for argument_id in arguments {
        let value_id = argument_value_id(context.tree, *argument_id);
        let value_len = expression_source_len(context, value_id);
        total_len = total_len.saturating_add(value_len);
    }

    total_len
}

/// Estimate the rendered length of arguments when printed inline.
fn arguments_rendered_len(
    context: &DestackFormatContext<'_>,
    arguments: &[LocalNodeId<Argument>],
) -> usize {
    // sum the value widths
    let values_len = arguments_total_len(context, arguments);

    // account for `, ` separators
    let separators_len = arguments.len().saturating_sub(1) * 2;

    values_len.saturating_add(separators_len)
}

/// Check whether an expression is a numeric scalar literal.
fn is_numeric_scalar_literal(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::ScalarLiteral(
            ScalarLiteral::Integer(_) | ScalarLiteral::Bigint(_) | ScalarLiteral::Float(_)
        )
    )
}

/// Check whether an index expression is a numeric literal without annotations.
fn is_numeric_index_expression(
    context: &DestackFormatContext<'_>,
    index_id: LocalNodeId<Expression>,
) -> bool {
    // annotated indexes are never numeric-simple
    if context.has_annotation(index_id) {
        return false;
    }

    let expression = context.tree.get(index_id);
    is_numeric_scalar_literal(expression)
}

/// Check whether an optional index is numeric-simple.
fn is_numeric_index(
    context: &DestackFormatContext<'_>,
    index: &Option<LocalNodeId<Expression>>,
) -> bool {
    match index {
        Some(index_id) => is_numeric_index_expression(context, *index_id),
        None => false,
    }
}

/// Check whether static arguments are simple enough for chain heads.
fn is_simple_chain_static_arguments(
    context: &DestackFormatContext<'_>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    match static_arguments {
        None => true,
        Some(arguments) => {
            arguments.len() <= 1
                && arguments
                    .iter()
                    .copied()
                    .all(|argument_id| is_simple_chain_argument(context, argument_id))
        }
    }
}

/// Check whether a chain call is simple enough to stay in the head.
fn is_simple_chain_call(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    // annotated calls are never simple
    if context.has_infix_annotation(node_id) {
        return false;
    }

    dynamic_arguments.len() <= 1
        && dynamic_arguments
            .iter()
            .copied()
            .all(|argument_id| is_simple_chain_argument(context, argument_id))
}

/// Check whether a chain operation is simple enough for head promotion.
fn is_simple_chain_operation(
    context: &DestackFormatContext<'_>,
    operation: &ChainExpression,
) -> bool {
    match operation {
        ChainExpression::Member {
            node_id,
            static_arguments,
            ..
        } => {
            !context.has_annotation(*node_id)
                && is_simple_chain_static_arguments(context, static_arguments)
        }
        ChainExpression::Call {
            node_id,
            dynamic_arguments,
            ..
        } => is_simple_chain_call(context, *node_id, dynamic_arguments),
        ChainExpression::Index { node_id, index, .. } => {
            !context.has_annotation(*node_id) && is_numeric_index(context, index)
        }
        ChainExpression::Maybe { node_id, .. } => !context.has_annotation(*node_id),
    }
}

/// Estimate the rendered length of static arguments.
fn static_arguments_len(
    context: &DestackFormatContext<'_>,
    static_arguments: &Option<Vec<LocalNodeId<Argument>>>,
) -> usize {
    match static_arguments {
        None => 0,
        Some(arguments) => {
            // compute argument length inside delimiters
            let arguments_len = arguments_rendered_len(context, arguments);

            // account for `<` and `>`
            arguments_len.saturating_add(2)
        }
    }
}

/// Estimate the rendered length of a single chain operation.
fn chain_operation_len(context: &DestackFormatContext<'_>, operation: &ChainExpression) -> usize {
    match operation {
        ChainExpression::Member {
            segment,
            static_arguments,
            ..
        } => {
            // measure the segment name
            let segment_len = context.strings.get(*segment).chars().count();

            // include any static arguments
            let static_len = static_arguments_len(context, static_arguments);

            // account for `.` plus the content
            1usize
                .saturating_add(segment_len)
                .saturating_add(static_len)
        }
        ChainExpression::Call {
            position,
            dynamic_arguments,
            ..
        } => {
            // account for a leading `.` on indirect calls
            let dot_len = usize::from(*position == PostfixPosition::Indirect);

            // measure the arguments within parentheses
            let arguments_len = arguments_rendered_len(context, dynamic_arguments);

            // include `(` and `)`
            dot_len.saturating_add(2).saturating_add(arguments_len)
        }
        ChainExpression::Index {
            position, index, ..
        } => {
            // account for a leading `.` on indirect indexes
            let dot_len = usize::from(*position == PostfixPosition::Indirect);

            // measure the index expression if it exists
            let index_len = match index {
                Some(index_id) => expression_source_len(context, *index_id),
                None => 0,
            };

            // include `[` and `]`
            dot_len.saturating_add(2).saturating_add(index_len)
        }
        ChainExpression::Maybe { position, .. } => match position {
            PostfixPosition::Direct => 1,
            PostfixPosition::Indirect => 2,
        },
    }
}

/// Estimate the rendered length of the chain base.
fn chain_base_len(context: &DestackFormatContext<'_>, base: &ChainExpressionBase) -> usize {
    // measure the base head
    let mut head_len = match &base.head {
        ChainExpressionBaseHead::Path {
            segment,
            static_arguments,
        } => {
            let segment_len = context.strings.get(*segment).chars().count();
            let static_len = static_arguments_len(context, static_arguments);
            segment_len.saturating_add(static_len)
        }
        ChainExpressionBaseHead::Expression(node_id) => expression_source_len(context, *node_id),
    };

    // add any base operations
    for operation in &base.body {
        let operation_len = chain_operation_len(context, operation);
        head_len = head_len.saturating_add(operation_len);
    }

    head_len
}

/// Estimate the inline length of a chain expression.
pub(crate) fn chain_inline_len(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
) -> Option<usize> {
    let tree = context.tree;

    if !is_chain_root(tree, node_id) && !is_expression_chain(tree, node_id) {
        return None;
    }

    // collect the nodes that belong to this chain
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);
        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. } => Some(*left),
            _ => None,
        };

        if let Some(next_id) = next {
            current = next_id;
        } else {
            break;
        }
    }
    chain.reverse();

    let root_id = *chain.first()?;

    let mut total_len = match tree.get(root_id) {
        Expression::Path {
            path,
            static_arguments,
        } => {
            let segments_len = path
                .segments
                .iter()
                .map(|segment| context.strings.get(*segment).chars().count())
                .sum::<usize>();
            let dot_len = path.segments.len().saturating_sub(1);
            let static_len = static_arguments_len(context, static_arguments);

            segments_len
                .saturating_add(dot_len)
                .saturating_add(static_len)
        }
        _ => expression_source_len(context, root_id),
    };

    for &expression_id in &chain[1..] {
        let chain_expression = match tree.get(expression_id) {
            Expression::Member { name, .. } => ChainExpression::Member {
                node_id: expression_id,
                segment: *name,
                static_arguments: None,
            },
            Expression::Call {
                position,
                dynamic_arguments,
                ..
            } => ChainExpression::Call {
                node_id: expression_id,
                position: *position,
                dynamic_arguments: dynamic_arguments.clone(),
            },
            Expression::Index {
                position, index, ..
            } => ChainExpression::Index {
                node_id: expression_id,
                position: *position,
                index: *index,
            },
            Expression::Maybe { position, .. } => ChainExpression::Maybe {
                node_id: expression_id,
                position: *position,
            },
            _ => continue,
        };

        let operation_len = chain_operation_len(context, &chain_expression);
        total_len = total_len.saturating_add(operation_len);
    }

    Some(total_len)
}

/// Split off simple head operations that should stay with the base.
fn split_chain_head_operations(
    context: &DestackFormatContext<'_>,
    base_len: usize,
    operations: &[ChainExpression],
    remaining_width: Option<usize>,
) -> usize {
    // nothing to split when there are no operations
    if operations.is_empty() {
        return 0;
    }

    // keep the base head from growing too large
    let line_width = usize::from(context.options.line_width);
    let mut max_head_len = (line_width / MAX_CHAIN_HEAD_LEN_DIVISOR).max(12);

    // clamp head growth to the known rhs width when available
    if let Some(remaining_width) = remaining_width {
        let remaining_limit = remaining_width
            .saturating_sub(ASSIGNMENT_CHAIN_TAIL_RESERVE)
            .max(8);
        max_head_len = max_head_len.min(remaining_limit);
    }

    // detect whether the chain starts with calls or numeric indexes
    let first_is_call_or_numeric_index = match operations.first() {
        Some(ChainExpression::Call { .. }) => true,
        Some(ChainExpression::Index { index, .. }) => is_numeric_index(context, index),
        _ => false,
    };

    // accumulate simple operations while within the promotion limits
    let mut head_len = base_len;
    let mut head_ops_count = 0usize;
    let mut index = 0usize;

    while index < operations.len() {
        // stop after the configured number of promoted operations
        if head_ops_count >= MAX_CHAIN_HEAD_OPS {
            break;
        }

        let operation = &operations[index];

        // avoid splitting a member from its immediate call or index
        let next_operation = operations.get(index + 1);
        let next_is_call_or_index = matches!(
            next_operation,
            Some(ChainExpression::Call { .. } | ChainExpression::Index { .. })
        );

        if matches!(operation, ChainExpression::Member { .. }) && next_is_call_or_index {
            // only promote the pair when both operations are simple
            let Some(next_operation) = next_operation else {
                break;
            };

            if !is_simple_chain_operation(context, operation)
                || !is_simple_chain_operation(context, next_operation)
            {
                break;
            }

            // respect the promotion count limit for paired operations
            if head_ops_count.saturating_add(2) > MAX_CHAIN_HEAD_OPS {
                break;
            }

            // keep call-start chains restricted to call-like operations
            let next_is_numeric_index = matches!(
                next_operation,
                ChainExpression::Index { index, .. } if is_numeric_index(context, index)
            );
            if first_is_call_or_numeric_index && !next_is_numeric_index {
                break;
            }

            // stop if promoting the pair would make the head too long
            let member_len = chain_operation_len(context, operation);
            let next_len = chain_operation_len(context, next_operation);
            let combined_len = head_len.saturating_add(member_len).saturating_add(next_len);
            if combined_len > max_head_len {
                break;
            }

            head_len = combined_len;
            head_ops_count += 2;
            index += 2;
            continue;
        }

        // only promote simple operations
        if !is_simple_chain_operation(context, operation) {
            break;
        }

        let is_call = matches!(operation, ChainExpression::Call { .. });
        let is_numeric_index_op = matches!(
            operation,
            ChainExpression::Index { index, .. } if is_numeric_index(context, index)
        );

        // when the chain starts with calls, keep only call-like head operations
        if first_is_call_or_numeric_index && !(is_call || is_numeric_index_op) {
            break;
        }

        // when the chain starts with members, stop before the first call
        if !first_is_call_or_numeric_index && is_call {
            break;
        }

        // stop if promoting this operation would make the head too long
        let operation_len = chain_operation_len(context, operation);
        let next_len = head_len.saturating_add(operation_len);
        if next_len > max_head_len {
            break;
        }

        head_len = next_len;
        head_ops_count += 1;
        index += 1;
    }

    head_ops_count
}

/// Summarize the complexity of a call within a chain.
struct ChainCallSummary {
    arguments_complex: bool,
    has_function_like_argument: bool,
    has_multiline_argument: bool,
    arguments_rendered_len: usize,
}

/// Build call summaries for a chain in source order.
fn summarize_chain_calls(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> Vec<ChainCallSummary> {
    let mut summaries = Vec::new();

    for expression_id in chain {
        let Expression::Call {
            dynamic_arguments, ..
        } = context.tree.get(*expression_id)
        else {
            continue;
        };

        // collect per-call signals
        let has_function_like_argument = dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| is_function_like_argument(context, argument_id));

        let has_annotations = context.has_infix_annotation(*expression_id)
            || dynamic_arguments
                .iter()
                .copied()
                .any(|argument_id| context.has_annotation(argument_id));

        // simple calls have at most one simple argument and no annotations
        let arguments_simple = !has_annotations
            && dynamic_arguments.len() <= 1
            && dynamic_arguments
                .iter()
                .copied()
                .all(|argument_id| is_simple_chain_argument(context, argument_id));

        let arguments_complex = has_annotations || !arguments_simple;
        let arguments_rendered_len = arguments_rendered_len(context, dynamic_arguments);
        let has_multiline_argument = dynamic_arguments
            .iter()
            .copied()
            .any(|argument_id| argument_forces_multiline(context, argument_id));

        summaries.push(ChainCallSummary {
            arguments_complex,
            has_function_like_argument,
            has_multiline_argument,
            arguments_rendered_len,
        });
    }

    summaries
}

/// Decide whether a chain should proactively break across lines.
fn should_break_chain(
    context: &DestackFormatContext<'_>,
    chain: &[LocalNodeId<Expression>],
) -> bool {
    let line_width = usize::from(context.options.line_width);
    let chain_root = chain
        .first()
        .copied()
        .expect("chain must contain at least one node");
    let call_summaries = summarize_chain_calls(context, chain);

    // chains with no calls stay inline unless they overflow
    if call_summaries.is_empty() {
        return false;
    }

    let calls_count = call_summaries.len();
    let has_multiline_call = call_summaries
        .iter()
        .any(|summary| summary.has_multiline_argument);

    if calls_count > 1 && has_multiline_call {
        return true;
    }

    let available_width =
        assignment_like_remaining_width(context, chain_root).unwrap_or(line_width);
    let inline_len = chain_inline_len(context, chain_root).unwrap_or(0);

    // avoid forcing breaks when the chain fits inline
    if inline_len <= available_width {
        return false;
    }
    let has_complex_args = call_summaries
        .iter()
        .any(|summary| summary.arguments_complex);

    let has_function_like_before_last = call_summaries
        .get(..calls_count.saturating_sub(1))
        .is_some_and(|summaries| {
            summaries
                .iter()
                .any(|summary| summary.has_function_like_argument)
        });

    let arguments_breaks = |summary: &ChainCallSummary| {
        summary.arguments_complex && summary.arguments_rendered_len > line_width / 3
    };

    let has_breaking_call_before_last = call_summaries
        .get(..calls_count.saturating_sub(1))
        .is_some_and(|summaries| summaries.iter().any(&arguments_breaks));

    let last_call_breakable = call_summaries.last().is_some_and(arguments_breaks);

    has_breaking_call_before_last
        || (calls_count > 2 && has_complex_args)
        || (last_call_breakable && has_function_like_before_last)
}

/// Check whether an assignment chain ends in a nested lambda expression.
fn is_assignment_chain_tail_lambda(
    context: &DestackFormatContext<'_>,
    assignment_id: LocalNodeId<Expression>,
    right_id: LocalNodeId<Expression>,
) -> bool {
    // only assignment-like rhs positions participate in assignment chains
    if assignment_like_parent(context, assignment_id).is_none() {
        return false;
    }

    // intermediate assignments are not chain tails
    if matches!(context.tree.get(right_id), Expression::Assign { .. }) {
        return false;
    }

    is_nested_lambda_expression(context, right_id)
}

/// Format a member/call/maybe/index chain with prettier-style breaking.
pub(crate) fn format_expression_chain<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // collect the nodes that belong to this chain
    let mut chain = Vec::new();
    let mut current = node_id;
    loop {
        chain.push(current);
        let next = match tree.get(current) {
            Expression::Member { left, .. }
            | Expression::Call { left, .. }
            | Expression::Index { left, .. }
            | Expression::Maybe { left, .. } => Some(*left),
            _ => None,
        };

        if let Some(next_id) = next {
            current = next_id;
        } else {
            break;
        }
    }
    chain.reverse();
    let root_id = *chain
        .first()
        .expect("member/call/maybe/index chain must contain at least one node");

    // gather operations while breaking path roots into individual segments
    let mut body: Vec<ChainExpression> = Vec::new();
    let mut base_head = ChainExpressionBaseHead::Expression(root_id);

    // break leading path expression into individual segments
    if let Expression::Path {
        path,
        static_arguments,
    } = tree.get(root_id)
    {
        let mut segments = path.segments.clone().into_iter();
        let static_arguments = static_arguments.clone();
        let first_segment = segments.next().expect("path is not empty");
        let remaining_segments: Vec<StringId> = segments.collect();

        // path base (keeps static arguments if there are no remaining segments)
        let base_static_arguments = if remaining_segments.is_empty() {
            static_arguments.clone()
        } else {
            None
        };
        base_head = ChainExpressionBaseHead::Path {
            segment: first_segment,
            static_arguments: base_static_arguments,
        };

        // path rest (tail segments)
        // (path segments are synthetic, they come from a single Path expression,
        //  so we use root_id for annotations, even though they won't have intra-path comments)
        let tail_len = remaining_segments.len();
        for (index, segment) in remaining_segments.into_iter().enumerate() {
            // last segment keeps static arguments if there are any
            let static_args = if tail_len != 0 && index + 1 == tail_len {
                static_arguments.clone()
            } else {
                None
            };
            body.push(ChainExpression::Member {
                node_id: root_id,
                segment,
                static_arguments: static_args,
            });
        }
    }
    let mut base = ChainExpressionBase {
        head: base_head,
        body: Vec::new(),
    };

    // convert the chain into individual chain expressions, preserving node IDs for annotations
    for &expression_id in &chain[1..] {
        let chain_expression = match tree.get(expression_id) {
            Expression::Member { name, .. } => ChainExpression::Member {
                node_id: expression_id,
                segment: *name,
                static_arguments: None,
            },
            Expression::Call {
                position,
                dynamic_arguments,
                ..
            } => ChainExpression::Call {
                node_id: expression_id,
                position: *position,
                dynamic_arguments: dynamic_arguments.clone(),
            },
            Expression::Index {
                position, index, ..
            } => ChainExpression::Index {
                node_id: expression_id,
                position: *position,
                index: *index,
            },
            Expression::Maybe { position, .. } => ChainExpression::Maybe {
                node_id: expression_id,
                position: *position,
            },
            _ => {
                return Err(FormatError::SyntaxError {
                    message: "unexpected expression kind for chain expression",
                });
            }
        };
        body.push(chain_expression);
    }

    // keep a leading call with the base so alignment stays stable
    if let Some(first_op) = body.first()
        && matches!(
            first_op,
            ChainExpression::Call {
                position: PostfixPosition::Direct,
                ..
            }
        )
    {
        base.body.push(first_op.clone());
        body.remove(0);
    }

    // keep a small head group with the base for prettier style chains
    let base_len = chain_base_len(f.context(), &base);
    let remaining_width = assignment_like_remaining_width(f.context(), node_id);
    let head_ops_count = split_chain_head_operations(f.context(), base_len, &body, remaining_width);
    if head_ops_count > 0 {
        let head_ops: Vec<_> = body.drain(..head_ops_count).collect();
        base.body.extend(head_ops);
    }

    // group the chain operations into lines
    let lines = group_chain_expression_lines(body);

    // indent chain lines consistently, even in assignment rhs positions
    let should_indent_chain = true;

    // compute whether the chain should proactively break
    let chain_should_break = should_break_chain(f.context(), &chain);

    // inline variant keeps everything on one line when it fits
    let format_inline = format_with(|f| {
        format_chain_base(f, &base)?;
        for line in &lines {
            format_chain_expression_line(f, line)?;
        }
        Ok(())
    });
    // chain variant breaks each operation onto its own line
    let format_chain = format_with(|f| {
        // encourage the parent to break when the chain is complex
        if chain_should_break {
            write!(f, [expand_parent()])?;
        }

        group(&format_with(|f| {
            // always print the base first so indentation aligns subsequent lines
            format_chain_base(f, &base)?;
            // indent chained entries so each operation sits on its own line
            // Use indent with manual line breaks instead of block_indent to avoid trailing newline
            // This ensures semicolons stay on the same line as the last chain element
            if !lines.is_empty() {
                if should_indent_chain {
                    write!(
                        f,
                        [indent(&format_with(|f| {
                            // each chain line renders in isolation to mirror prettier style
                            for line in &lines {
                                write!(f, [hard_line_break()])?;
                                format_chain_expression_line(f, line)?;
                            }
                            Ok(())
                        }))]
                    )?;
                } else {
                    write!(
                        f,
                        [format_with(|f| {
                            // avoid extra indentation when the parent already indents after `=`
                            for line in &lines {
                                write!(f, [hard_line_break()])?;
                                format_chain_expression_line(f, line)?;
                            }
                            Ok(())
                        })]
                    )?;
                }
            }
            Ok(())
        }))
        .format(f)
    });

    // chains that are clearly complex should not try the inline layout first
    if chain_should_break {
        write!(f, [group(&format_chain).should_expand(true)])?;
        return Ok(());
    }

    // prefer inline, otherwise chain
    best_fitting![format_inline, format_chain]
        .with_mode(BestFittingMode::AllLines)
        .format(f)
}

/// Format a match expression.
#[inline]
pub(crate) fn format_match<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match { kind, value, cases } = &match_node else {
        return Err(FormatError::SyntaxError {
            message: "invalid match expression",
        });
    };
    let kind = *kind;

    if include_prefix {
        // match/switch <expression>
        let keyword = match kind {
            MatchKind::Match => Keyword::Match,
            MatchKind::Switch => Keyword::Switch,
        };
        write!(f, [keyword, space()])?;
    }

    write!(f, [token("("), value, token(")")])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(());
    }

    // match/switch cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| {
            let mut first = true;
            for case_id in cases {
                if !first {
                    write!(f, [hard_line_break()])?;
                }
                first = false;
                format_match_case(f, *case_id, kind)?;
            }
            Ok(())
        })),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;

    Ok(())
}

/// Format a single match/switch case.
///
/// For `match`, uses arrow syntax: `pattern => body`
/// For `switch`, uses colon syntax: `case pattern:` or `default:`
#[allow(clippy::type_complexity)]
fn format_match_case<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    case_id: LocalNodeId<MatchCase>,
    kind: MatchKind,
) -> FormatResult<()> {
    let case = f.context().tree.get(case_id);

    // extract selector and body based on case variant
    let (selector, body_format): (
        &MatchSelector,
        Box<dyn Fn(&mut DestackFormatter<'ast, '_>) -> FormatResult<()> + '_>,
    ) = match case {
        MatchCase::Expression { selector, body } => {
            (selector, Box::new(move |f| write!(f, [*body])))
        }
        MatchCase::Block { selector, body } => (selector, Box::new(move |f| write!(f, [*body]))),
    };

    // write prefix annotations
    write!(f, [f.context().any_prefix_annotations(case_id)])?;

    match kind {
        MatchKind::Match => {
            // match style: pattern => body
            match selector {
                MatchSelector::Pattern { pattern, guard } => {
                    write!(f, [*pattern])?;
                    if let Some(guard) = guard {
                        write!(
                            f,
                            [
                                space(),
                                Keyword::If,
                                space(),
                                token("("),
                                *guard,
                                token(")")
                            ]
                        )?;
                    }
                }
                MatchSelector::Default => {
                    // shouldn't happen in match expressions, but handle gracefully
                    write!(f, [token("_")])?;
                }
            }
            write!(f, [space(), token("=>"), space()])?;
            body_format(f)?;
        }
        MatchKind::Switch => {
            // switch style: case pattern: or default:
            match selector {
                MatchSelector::Pattern { pattern, guard } => {
                    write!(f, [Keyword::Case, space(), *pattern, token(":")])?;
                    if let Some(guard) = guard {
                        write!(
                            f,
                            [
                                space(),
                                Keyword::If,
                                space(),
                                token("("),
                                *guard,
                                token(")")
                            ]
                        )?;
                    }
                }
                MatchSelector::Default => {
                    write!(f, [Keyword::Default, token(":")])?;
                }
            }
            write!(f, [space()])?;
            body_format(f)?;
        }
    }

    // write postfix annotations
    write!(f, [f.context().any_infix_or_postfix_annotations(case_id)])?;

    Ok(())
}

/// Check whether the expression is a chain expression.
fn is_chain_expression(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::Member { .. }
            | Expression::Call { .. }
            | Expression::Index { .. }
            | Expression::Maybe { .. }
    )
}

/// Whether a type expression is object-like (object literals or mapped types).
fn is_object_like_type_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::ObjectExpression { .. } | Expression::TypeMapped { .. }
    )
}

/// Whether a type expression is nullable (null, undefined, or void).
fn is_nullable_union_member(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::TypeLiteral(TypeLiteral::Null | TypeLiteral::Undefined | TypeLiteral::Void)
    )
}

/// Whether a type expression is a simple reference.
fn is_type_reference_expression(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression_id = transparent_inner_expression(context, expression_id);
    matches!(
        context.tree.get(expression_id),
        Expression::Path { .. } | Expression::Member { .. }
    )
}

/// Decide whether a nullable union type should stay inline with `|` separators.
fn should_hug_nullable_union_type(
    context: &DestackFormatContext<'_>,
    operands: &[BinaryOperand],
) -> bool {
    if operands.len() < 2 {
        return false;
    }

    if operands
        .iter()
        .any(|operand| context.has_annotation(operand.expression))
    {
        return false;
    }

    let mut nullable_count = 0;
    let mut has_object_or_ref = false;
    let mut non_nullable_count = 0;

    for operand in operands {
        let expression_id = operand.expression;
        if is_nullable_union_member(context, expression_id) {
            nullable_count += 1;
            continue;
        }

        non_nullable_count += 1;
        if is_object_like_type_expression(context, expression_id)
            || is_type_reference_expression(context, expression_id)
        {
            has_object_or_ref = true;
        } else {
            return false;
        }
    }

    has_object_or_ref && non_nullable_count == 1 && nullable_count == operands.len() - 1
}

/// Whether an expression is "trivial" (prefers to be fully inline).
pub fn is_trivial_expression(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_) | Expression::TypeLiteral(_) => true,
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_none()
                && properties.len() <= 5
                && properties
                    .iter()
                    .all(|property| is_trivial_property(tree, tree.get(*property)))
        }
        Expression::Unary { operator: _, right } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Index { left, index, .. } => {
            is_trivial_expression(tree, tree.get(*left)) && index.is_none()
                || is_trivial_expression(tree, tree.get(*index.as_ref().unwrap()))
        }
        Expression::ReferenceOf {
            mutability: _,
            variance: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::PointerOf {
            mutability: _,
            right,
        } => is_trivial_expression(tree, tree.get(*right)),
        Expression::Member { left, .. } => is_trivial_expression(tree, tree.get(*left)),
        Expression::Path {
            path,
            static_arguments,
        } => path.segments.len() <= 3 && static_arguments.is_none(),
        _ => false,
    }
}

/// Whether an expression is "complex" (prefers to be multiline).
pub fn is_complex_expression(_tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::Statement { .. } => true,
        // only expand objects with many (>3) properties by default
        // let best_fitting handle the rest based on line width
        Expression::ObjectExpression { properties, .. } => properties.len() > 3,
        Expression::TreeExpression { .. } => true,
        _ => false,
    }
}

/// Whether an argument is "trivial" (prefers to be inline).
pub fn is_trivial_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether a property is "trivial" (prefers to be inline).
pub fn is_trivial_property(tree: &NodeTree, property: &Property) -> bool {
    match property {
        Property::Field { value, default, .. } => {
            value.is_none_or(|value| is_trivial_expression(tree, tree.get(value)))
                && default.is_none_or(|default| is_trivial_expression(tree, tree.get(default)))
        }
        Property::Method { body, .. } => {
            body.is_none_or(|body| is_trivial_expression(tree, tree.get(body)))
        }
        Property::Spread { value, .. } => is_trivial_expression(tree, tree.get(*value)),
    }
}

/// Whether an argument is "complex" (prefers to be multiline).
pub fn is_complex_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. }
        | Argument::Labeled {
            label: _, value, ..
        }
        | Argument::Positional { value, .. }
        | Argument::Spread { value, .. } => is_complex_expression(tree, tree.get(*value)),
    }
}

/// Whether the expression can break itself across multiple lines.
pub fn is_expression_breakable(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ArrayExpression { elements, .. } => !elements.is_empty(),
        Expression::TupleExpression { elements, .. } => !elements.is_empty(),
        Expression::SequenceExpression { expressions, .. } => !expressions.is_empty(),
        Expression::ObjectExpression { ty, properties, .. } => {
            ty.is_some_and(|ty| is_expression_breakable(tree, tree.get(ty)))
                || !properties.is_empty()
        }
        Expression::TreeExpression {
            arguments,
            elements,
            ..
        } => {
            arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
                || elements
                    .as_ref()
                    .is_some_and(|elements| !elements.is_empty())
        }
        Expression::Call {
            dynamic_arguments, ..
        } => !dynamic_arguments.is_empty(),
        Expression::Match { .. } => true,
        Expression::Path {
            static_arguments, ..
        } => static_arguments
            .as_ref()
            .is_some_and(|static_arguments| !static_arguments.is_empty()),
        Expression::If { .. }
        | Expression::Loop { .. }
        | Expression::Try { .. }
        | Expression::Block { .. }
        | Expression::ForEach { .. }
        | Expression::For { .. }
        | Expression::While { .. }
        | Expression::Import { .. }
        | Expression::Export { .. } => true,
        Expression::Binary { .. }
        | Expression::TypeBinary { .. }
        | Expression::TypeConditional { .. }
        | Expression::TypeMapped { .. } => true,
        _ => false,
    }
}

/// Check if a pattern can expand to multiple lines (object, array, tuple patterns).
pub fn is_pattern_breakable(tree: &NodeTree, pattern_id: LocalNodeId<Pattern>) -> bool {
    let pattern = tree.get(pattern_id);
    match pattern {
        Pattern::Object { fields } | Pattern::TaggedObject { fields, .. } => !fields.is_empty(),
        Pattern::Array { fields }
        | Pattern::Tuple { fields }
        | Pattern::TaggedTuple { fields, .. } => !fields.is_empty(),
        _ => false,
    }
}

/// Format a struct literal.
#[inline]
pub(crate) fn format_struct_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    ty: &Option<LocalNodeId<Expression>>,
    properties_ids: &Vec<LocalNodeId<Property>>,
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    let properties = properties_ids
        .iter()
        .map(|property| f.context().tree.get(*property))
        .collect::<SmallVec<[_; 3]>>();

    // check for conditions that REQUIRE expansion
    let has_methods = properties
        .iter()
        .any(|property| matches!(property, Property::Method { body: Some(_), .. }));
    let has_annotations = f.context().has_infix_annotation(expression_id)
        || properties_ids
            .iter()
            .any(|property| f.context().has_annotation(*property));
    let keep_newline =
        f.context().has_newline(f.context().get_span(expression_id)) && properties.len() > 1;

    // only force expand for methods, annotations, or explicit newlines
    // otherwise let best_fitting decide based on line width
    let must_expand = has_methods || has_annotations || keep_newline;

    write!(
        f,
        [list_like("{", "}", ",", properties_ids)
            .as_collection()
            .include_space()
            .should_expand(must_expand)]
    )?;
    Ok(())
}

/// Get the raw text content of a tree child when it is a string literal.
fn tree_text_content(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<String> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = tree.get(*value) else {
        return None;
    };

    let span_str = tree_text_span_str(context, argument_id)?;
    if span_str.starts_with('"') || span_str.starts_with('\'') {
        let content = strings.get(*string_id);
        if content.trim().is_empty() {
            return None;
        }
        return Some(content.to_owned());
    }

    let normalized = normalize_tree_text(&span_str);
    if normalized.is_empty() {
        return None;
    }

    Some(normalized)
}

/// Get the span string for a tree text child.
fn tree_text_span_str(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<String> {
    let tree = context.tree;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let Expression::ScalarLiteral(ScalarLiteral::String(_)) = tree.get(*value) else {
        return None;
    };

    let span = context.get_span(*value);
    let span_str = context.file.get_span_str(span).unwrap_or_default();
    Some(span_str.to_owned())
}

/// Normalize tree text by collapsing whitespace to single spaces and trimming edges.
fn normalize_tree_text(text: &str) -> String {
    let mut parts = text.split_whitespace();
    let Some(first) = parts.next() else {
        return String::new();
    };

    let mut normalized = String::from(first);
    for part in parts {
        normalized.push(' ');
        normalized.push_str(part);
    }

    normalized
}

/// Check whether a tree text child is whitespace-only.
fn tree_text_is_whitespace_only(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    if let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = tree.get(*value) {
        let span_str = tree_text_span_str(context, argument_id)?;
        if span_str.starts_with('"') || span_str.starts_with('\'') {
            let content = strings.get(*string_id);
            let has_non_whitespace = content.chars().any(|c| !c.is_whitespace());
            if has_non_whitespace {
                return Some((false, false));
            }
            let has_newline = content.contains(['\n', '\r']);
            return Some((true, has_newline));
        }
    }

    let span_str = tree_text_span_str(context, argument_id)?;
    let has_non_whitespace = span_str.chars().any(|c| !c.is_whitespace());
    if has_non_whitespace {
        return Some((false, false));
    }

    let has_newline = span_str.contains(['\n', '\r']);
    Some((true, has_newline))
}

/// Check whether a tree text child has inline boundary whitespace.
fn tree_text_inline_boundary_whitespace(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> Option<(bool, bool)> {
    let tree = context.tree;
    let strings = context.strings;
    let Argument::Positional { value, .. } = tree.get(argument_id) else {
        return None;
    };

    let span_str = tree_text_span_str(context, argument_id)?;
    if let Expression::ScalarLiteral(ScalarLiteral::String(string_id)) = tree.get(*value)
        && (span_str.starts_with('"') || span_str.starts_with('\''))
    {
        let content = strings.get(*string_id);
        let leading_end = content
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
            .map_or(content.len(), |(index, _)| index);
        let trailing_start = content
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_whitespace())
            .map_or(0, |(index, c)| index + c.len_utf8());

        let leading_whitespace = &content[..leading_end];
        let trailing_whitespace = &content[trailing_start..];

        let has_leading_space =
            !leading_whitespace.is_empty() && !leading_whitespace.contains(['\n', '\r']);
        let has_trailing_space =
            !trailing_whitespace.is_empty() && !trailing_whitespace.contains(['\n', '\r']);
        return Some((has_leading_space, has_trailing_space));
    }

    let leading_end = span_str
        .char_indices()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(span_str.len(), |(index, _)| index);
    let trailing_start = span_str
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_whitespace())
        .map_or(0, |(index, c)| index + c.len_utf8());

    let leading_whitespace = &span_str[..leading_end];
    let trailing_whitespace = &span_str[trailing_start..];

    let has_leading_space =
        !leading_whitespace.is_empty() && !leading_whitespace.contains(['\n', '\r']);
    let has_trailing_space =
        !trailing_whitespace.is_empty() && !trailing_whitespace.contains(['\n', '\r']);
    Some((has_leading_space, has_trailing_space))
}

/// Check whether text starts with closing punctuation.
fn text_starts_with_closing_punctuation(text: &str) -> bool {
    matches!(
        text.chars().next(),
        Some('!' | ',' | '.' | ':' | ';' | '?' | ')' | ']' | '}')
    )
}

/// Check whether text ends with opening punctuation.
fn text_ends_with_opening_punctuation(text: &str) -> bool {
    matches!(text.chars().last(), Some('(' | '[' | '{'))
}

/// Check whether a lambda body is complex enough to force tree breaking.
fn lambda_body_is_complex_for_tree(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let tree = context.tree;

    let Declaration::Function {
        signature,
        body: Some(body_id),
        ..
    } = tree.get(declaration_id)
    else {
        return false;
    };

    if signature.kind != FunctionKind::Lambda {
        return false;
    }

    let body_id = transparent_inner_expression(context, *body_id);

    matches!(
        tree.get(body_id),
        Expression::Block(_) | Expression::TreeExpression { .. }
    )
}

/// Check whether a call has a complex callback argument.
/// Check whether an argument is a lambda with a complex body for tree literals.
fn argument_is_complex_callback(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Argument>,
) -> bool {
    let tree = context.tree;

    // grab the argument value
    let value_id = match tree.get(argument_id) {
        Argument::Positional { value, .. } | Argument::Spread { value, .. } => *value,
        Argument::Named { value, .. } | Argument::Labeled { value, .. } => *value,
    };

    // unwrap transparent wrappers
    let value_id = transparent_inner_expression(context, value_id);

    // only lambda declarations qualify
    let Expression::Declaration(decl_id) = tree.get(value_id) else {
        return false;
    };

    lambda_body_is_complex_for_tree(context, *decl_id)
}

/// Check whether an expression contains a call with a complex callback.
fn expression_has_complex_callback(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let tree = context.tree;

    // unwrap transparent wrappers
    let expression_id = transparent_inner_expression(context, expression_id);

    // walk the expression shape looking for callback lambdas
    match tree.get(expression_id) {
        Expression::Call {
            left,
            dynamic_arguments,
            ..
        }
        | Expression::New {
            left,
            dynamic_arguments,
            ..
        } => {
            // check the call arguments
            let has_complex_argument = dynamic_arguments
                .iter()
                .any(|arg_id| argument_is_complex_callback(context, *arg_id));

            if has_complex_argument {
                return true;
            }

            // check chained receivers
            expression_has_complex_callback(context, *left)
        }
        Expression::Member { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. } => {
            // walk through postfix chains
            expression_has_complex_callback(context, *left)
        }
        Expression::Statement(inner_id) => {
            // peel statement wrappers
            expression_has_complex_callback(context, *inner_id)
        }
        Expression::Block(block_id) => {
            let block = tree.get(*block_id);

            // scan block expressions for complex callbacks
            block
                .expressions
                .iter()
                .any(|expr_id| expression_has_complex_callback(context, *expr_id))
        }
        _ => false,
    }
}

/// Decide whether a tree literal should break across multiple lines.
pub(crate) fn tree_literal_should_break(
    context: &DestackFormatContext<'_>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> bool {
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(context, arguments));

    let Some(elements) = elements else {
        return force_break_attributes;
    };

    if elements.is_empty() {
        return force_break_attributes;
    }

    let tree = context.tree;
    let element_children_count = elements
        .iter()
        .filter(|elem_id| {
            let arg = tree.get(**elem_id);
            if let Argument::Positional { value, .. } = arg {
                matches!(tree.get(*value), Expression::TreeExpression { .. })
            } else {
                false
            }
        })
        .count();

    let has_complex_child = elements.iter().any(|elem_id| {
        let arg = tree.get(*elem_id);
        if let Argument::Positional { value, .. } = arg {
            expression_has_complex_callback(context, *value)
        } else {
            false
        }
    });

    let has_tree_child = element_children_count > 0;
    let has_single_text_child = elements.len() == 1 && !has_tree_child && !has_complex_child;

    force_break_attributes || has_complex_child || (has_tree_child && !has_single_text_child)
}

/// Format a tree literal.
#[inline]
pub(crate) fn format_tree_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
    let force_break_attributes = arguments
        .as_ref()
        .is_some_and(|arguments| should_force_break_tree_attributes(f.context(), arguments));

    write!(
        f,
        [group(&format_with(|f| {
            // header
            write!(
                f,
                [group(&format_with(|f| {
                    // <
                    write!(f, [token("<")])?;
                    // left
                    if let Some(left) = left {
                        write!(f, [left])?;
                    }
                    // arguments
                    if let Some(arguments) = arguments {
                        let single_attr_per_line = f.context().options.single_attribute_per_line;
                        let bracket_same_line = f.context().options.bracket_same_line;

                        // separator between attributes
                        let attr_separator: &dyn Format<DestackFormatContext<'ast>> =
                            if force_break_attributes
                                || (single_attr_per_line && arguments.len() > 1)
                            {
                                &hard_line_break()
                            } else {
                                &soft_line_break_or_space()
                            };

                        // format attribute list
                        let format_attrs = format_with(|f| {
                            f.join_with(attr_separator)
                                .entries(arguments.iter().map(|argument| TreeExpressionArgument {
                                    argument_id: *argument,
                                }))
                                .finish()
                        });

                        // complex attributes should expand the element
                        if force_break_attributes {
                            write!(f, [expand_parent()])?;
                        }

                        // when bracket_same_line is true, don't add trailing line break before >
                        // when false (default), soft_block_indent adds trailing soft_line_break
                        if bracket_same_line {
                            // avoid a trailing break before `>` when bracket_same_line is enabled
                            if force_break_attributes {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        indent(&format_args![hard_line_break(), format_attrs])
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        indent(&format_args![soft_line_break(), format_attrs])
                                    ]
                                )?;
                            }
                        } else {
                            // force expansion for complex attributes in the default layout
                            if force_break_attributes {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        group(&soft_block_indent(&format_attrs))
                                            .should_expand(true)
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        if_group_fits_on_line(&space()),
                                        soft_block_indent(&format_attrs)
                                    ]
                                )?;
                            }
                        }
                    }
                    // /
                    if elements.is_none() {
                        let bracket_same_line = f.context().options.bracket_same_line;
                        let has_attributes = arguments.is_some();
                        if left.is_some() || has_attributes {
                            // space before /> when inline, or when bracket_same_line is true
                            if bracket_same_line && has_attributes {
                                write!(f, [if_group_breaks(&space())])?;
                            }
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }
                        write!(f, [token("/")])?;
                    }
                    // >
                    write!(f, [token(">")])?;
                    Ok(())
                }))]
            )?;

            // body
            if let Some(elements) = elements {
                // check child types for formatting decisions
                let tree = f.context().tree;
                let element_children_count = elements
                    .iter()
                    .filter(|elem_id| {
                        let arg = tree.get(**elem_id);
                        if let Argument::Positional { value, .. } = arg {
                            matches!(tree.get(*value), Expression::TreeExpression { .. })
                        } else {
                            false
                        }
                    })
                    .count();
                let all_tree_children = element_children_count == elements.len();

                // check if any child has complex content (block expressions)
                let has_complex_child = elements.iter().any(|elem_id| {
                    let arg = tree.get(*elem_id);
                    if let Argument::Positional { value, .. } = arg {
                        expression_has_complex_callback(f.context(), *value)
                    } else {
                        false
                    }
                });

                // force breaking when:
                // - attributes require a break, OR
                // - any child has complex content, such as callbacks with block bodies, OR
                // - there is at least one tree child, unless the only child is plain text
                let has_tree_child = element_children_count > 0;
                let has_single_text_child =
                    elements.len() == 1 && !has_tree_child && !has_complex_child;
                let force_break = force_break_attributes
                    || has_complex_child
                    || (has_tree_child && !has_single_text_child);

                // Format children using TreeExpressionArgument for proper brace handling
                let format_children = format_with(|f| {
                    // when all children are tree elements, keep one element per line
                    if all_tree_children && elements.len() > 1 {
                        for (index, elem_id) in elements.iter().enumerate() {
                            if index > 0 {
                                write!(f, [hard_line_break()])?;
                            }

                            write!(
                                f,
                                [TreeExpressionArgument {
                                    argument_id: *elem_id
                                }]
                            )?;
                        }

                        return Ok(());
                    }

                    // otherwise, use fill so mixed content can share lines when it fits
                    let separators = {
                        let context = f.context();

                        // capture the normalized text content for each child
                        let texts = elements
                            .iter()
                            .map(|elem_id| tree_text_content(context, *elem_id))
                            .collect::<Vec<_>>();
                        let whitespace_flags = elements
                            .iter()
                            .map(|elem_id| tree_text_is_whitespace_only(context, *elem_id))
                            .map(|info| info.unwrap_or((false, false)))
                            .collect::<Vec<_>>();

                        // compute the spacing decisions between adjacent children
                        let mut separators = Vec::with_capacity(elements.len());
                        separators.push((false, false));

                        for index in 1..elements.len() {
                            let prev_argument_id = elements[index - 1];
                            let current_argument_id = elements[index];
                            let prev_text = texts[index - 1].as_deref();
                            let current_text = texts[index].as_deref();
                            let prev_is_whitespace_only = whitespace_flags[index - 1].0;
                            let current_is_whitespace_only = whitespace_flags[index].0;

                            if prev_is_whitespace_only || current_is_whitespace_only {
                                separators.push((false, false));
                                continue;
                            }

                            let omit_space_for_punctuation = current_text
                                .is_some_and(text_starts_with_closing_punctuation)
                                || prev_text.is_some_and(text_ends_with_opening_punctuation);

                            // decide whether a space should be inserted inline
                            let prev_inline =
                                tree_text_inline_boundary_whitespace(context, prev_argument_id);
                            let current_inline =
                                tree_text_inline_boundary_whitespace(context, current_argument_id);
                            let prev_has_inline_trailing_space =
                                prev_inline.is_some_and(|(_, trailing)| trailing);
                            let current_has_inline_leading_space =
                                current_inline.is_some_and(|(leading, _)| leading);
                            let has_inline_boundary_space =
                                prev_has_inline_trailing_space || current_has_inline_leading_space;

                            // insert spaces only for explicit inline whitespace
                            let should_insert_space_inline =
                                has_inline_boundary_space && !omit_space_for_punctuation;

                            separators
                                .push((omit_space_for_punctuation, should_insert_space_inline));
                        }

                        separators
                    };

                    // render children using fill with the precomputed separators
                    let mut fill = f.fill();

                    for (index, elem_id) in elements.iter().enumerate() {
                        let (omit_space_for_punctuation, should_insert_space_inline) =
                            separators[index];

                        // build a separator doc for fill
                        let separator = format_with(|f| {
                            if index == 0 {
                                return Ok(());
                            }

                            if omit_space_for_punctuation {
                                write!(f, [if_group_fits_on_line(&soft_line_break())])?;
                            } else if should_insert_space_inline {
                                write!(f, [soft_line_break_or_space()])?;
                            } else {
                                write!(f, [soft_line_break()])?;
                            }

                            Ok(())
                        });

                        // add the child to the fill output
                        let entry = TreeExpressionArgument {
                            argument_id: *elem_id,
                        };
                        fill.entry(&separator, &entry);
                    }

                    fill.finish()
                });

                if force_break {
                    write!(f, [block_indent(&format_children)])?;
                } else {
                    // Use soft indent - stays on one line if it fits
                    write!(f, [group(&soft_block_indent(&format_children))])?;
                }

                // closing tag
                write!(f, [token("</")])?;
                if let Some(left) = left {
                    write!(f, [left])?;
                }
                write!(f, [token(">")])?;
            }

            Ok(())
        }))]
    )
}

/// Format an expression (without prefix and postfix annotations)
pub(crate) fn format_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    let tree = f.context().tree;

    match expression {
        // declaration
        Expression::Declaration(node) => node.format(f)?,

        // block
        Expression::Block(node) => node.format(f)?,

        // statement
        Expression::Statement(node) => {
            write!(f, [*node, token(";")])?;
        }

        // labelled statement
        Expression::Labelled { label, body } => {
            write!(f, [label, token(":"), space(), *body])?;
        }

        // import
        Expression::Import {
            source,
            kind,
            target,
            items,
            arguments,
        } => {
            let organize = f.context().options.organize_imports.is_enabled();
            let sort_order = f.context().options.import_sort_order;
            let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

            // import call
            if *source == ImportSource::ImportCall {
                write!(
                    f,
                    [
                        Keyword::Import,
                        token("("),
                        token("\""),
                        target,
                        token("\""),
                        token(")")
                    ]
                )?;
                return Ok(());
            }

            // keyword
            write!(f, [Keyword::Import, space()])?;
            if *source == ImportSource::ImportEquals {
                let alias = items.first().and_then(|item| tree.get(*item).alias).ok_or(
                    FormatError::SyntaxError {
                        message: "import equals requires an alias",
                    },
                )?;
                write!(
                    f,
                    [
                        alias,
                        space(),
                        token("="),
                        space(),
                        token("require"),
                        token("("),
                        token("\""),
                        target,
                        token("\""),
                        token(")")
                    ]
                )?;
                return Ok(());
            }
            if *kind == DependencyKind::Type {
                write!(f, [Keyword::Type, space()])?;
            }

            // items
            let first_item = items.first().map(|item| tree.get(*item));
            // namespace
            if items.len() == 1
                && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace)
            {
                write!(
                    f,
                    [
                        token("*"),
                        space(),
                        Keyword::As,
                        space(),
                        first_item.unwrap().alias
                    ]
                )?;
            }
            // items
            else {
                // default
                if let Some(first_item) = first_item
                    && first_item.mode == DependencyMode::Default
                {
                    let rest_items: Vec<LocalNodeId<DependencyItem>> =
                        items.iter().skip(1).copied().collect();
                    write!(f, [first_item.alias])?;
                    if !rest_items.is_empty() {
                        // sort rest items if organize_imports is enabled
                        let sorted_rest = if organize && !items_have_annotations {
                            sort_dependency_items(
                                &rest_items,
                                tree,
                                f.context().strings,
                                sort_order,
                            )
                        } else {
                            rest_items
                        };
                        write!(f, [token(","), space()])?;
                        write!(
                            f,
                            [list_like("{", "}", ",", &sorted_rest)
                                .as_collection()
                                .include_space(),]
                        )?;
                    }
                }
                // items
                else if !items.is_empty() {
                    // sort items if organize_imports is enabled
                    let sorted_items = if organize && !items_have_annotations {
                        sort_dependency_items(items, tree, f.context().strings, sort_order)
                    } else {
                        items.to_vec()
                    };
                    write!(
                        f,
                        [list_like("{", "}", ",", &sorted_items)
                            .as_collection()
                            .include_space()]
                    )?;
                }
            }

            // from
            if !items.is_empty() {
                write!(f, [space(), Keyword::From, space()])?;
            }

            // target
            write!(f, [token("\""), target, token("\"")])?;

            // arguments
            if let Some(arguments) = arguments {
                write!(
                    f,
                    [
                        space(),
                        Keyword::With,
                        space(),
                        list_like("{", "}", ",", arguments)
                            .as_collection()
                            .include_space()
                    ]
                )?;
            }
        }

        // export
        Expression::Export {
            kind,
            target,
            items,
        } => {
            let organize = f.context().options.organize_imports.is_enabled();
            let sort_order = f.context().options.import_sort_order;
            let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

            // keyword
            write!(f, [Keyword::Export, space()])?;
            if *kind == DependencyKind::Type {
                write!(f, [Keyword::Type, space()])?;
            }

            // items
            let first_item = items.first().map(|item| tree.get(*item));

            // default export with value (export default <expression>)
            if items.len() == 1
                && first_item.is_some_and(|item| {
                    item.mode == DependencyMode::Default && item.value.is_some()
                })
            {
                write!(
                    f,
                    [
                        Keyword::Default,
                        space(),
                        first_item.unwrap().value.unwrap()
                    ]
                )?;
            }
            // namespace
            else if items.len() == 1
                && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace)
            {
                write!(f, [token("*")])?;
                if let Some(alias) = first_item.unwrap().alias {
                    write!(f, [space(), Keyword::As, space(), alias])?;
                }
            }
            // items
            else if !items.is_empty() {
                // sort items if organize_imports is enabled
                let sorted_items = if organize && !items_have_annotations {
                    sort_dependency_items(items, tree, f.context().strings, sort_order)
                } else {
                    items.to_vec()
                };
                write!(
                    f,
                    [list_like("{", "}", ",", &sorted_items)
                        .as_collection()
                        .include_space()]
                )?;
            }

            // target
            if let Some(target) = target {
                write!(
                    f,
                    [
                        space(),
                        Keyword::From,
                        space(),
                        token("\""),
                        target,
                        token("\"")
                    ]
                )?;
            }
        }

        // export as namespace
        Expression::ExportNamespace { name } => {
            write!(
                f,
                [
                    Keyword::Export,
                    space(),
                    Keyword::As,
                    space(),
                    Keyword::Namespace,
                    space(),
                    name
                ]
            )?;
        }

        // let
        Expression::Let {
            kind,
            descriptor,
            declarators,
            ..
        } => {
            // keyword header (export + const/let/var)
            let keyword_header = format_with(|f| {
                // export
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }
                // keyword (based on LetKind)
                match kind {
                    LetKind::Let => write!(f, [Keyword::Let])?,
                    LetKind::Var => write!(f, [Keyword::Var])?,
                    LetKind::Const => write!(f, [Keyword::Const])?,
                }
                Ok(())
            });

            // format declarators (comma-separated)
            write!(f, [keyword_header])?;
            for (i, declarator_id) in declarators.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(",")])?;
                }
                write!(f, [space()])?;
                format_declarator(f, tree, *declarator_id)?;
            }
        }

        // using
        Expression::Using {
            asynchrony,
            descriptor,
            declarators,
        } => {
            // keyword header (export + await + using)
            let keyword_header = format_with(|f| {
                if let Some(export) = descriptor.export {
                    write!(f, [export, space()])?;
                }
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }
                write!(f, [Keyword::Using])?;
                Ok(())
            });

            // format declarators (comma-separated)
            write!(f, [keyword_header])?;
            for (i, declarator_id) in declarators.iter().enumerate() {
                if i > 0 {
                    write!(f, [token(",")])?;
                }
                write!(f, [space()])?;
                format_declarator(f, tree, *declarator_id)?;
            }
        }

        // if (ternary)
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => {
            format_ternary(f, node_id)?;
        }

        // if (regular)
        Expression::If {
            kind: IfKind::If, ..
        } => {
            write!(
                f,
                [group(&format_with(|f| format_if_else_chain(f, node_id)))]
            )?;
        }

        // while
        Expression::While {
            kind,
            condition,
            body,
        } => match *kind {
            WhileKind::While => {
                write!(
                    f,
                    [
                        Keyword::While,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                        space(),
                        body
                    ]
                )?;
            }
            WhileKind::DoWhile => {
                write!(
                    f,
                    [
                        Keyword::Do,
                        space(),
                        body,
                        space(),
                        Keyword::While,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                    ]
                )?;
            }
        },

        // for each
        Expression::ForEach {
            asynchrony,
            kind,
            binding,
            iterator,
            body,
        } => {
            write!(f, [Keyword::For, space()])?;
            if *asynchrony == Asynchrony::Async {
                write!(f, [Keyword::Await, space()])?;
            }
            let keyword = match kind {
                ForEachKind::In => Keyword::In,
                ForEachKind::Of => Keyword::Of,
            };
            write!(f, [token("(")])?;
            match binding {
                ForEachBinding::Pattern { pattern } => {
                    let pattern_node = tree.get(*pattern);
                    let should_prefix_const = matches!(
                        pattern_node,
                        Pattern::Binding {
                            mutability: Some(Mutability::Immutable),
                            ..
                        }
                    );

                    // keep explicit const for simple bindings
                    if should_prefix_const {
                        write!(f, [Keyword::Const, space()])?;
                    }

                    write!(f, [pattern])?;
                }
                ForEachBinding::Using {
                    asynchrony,
                    pattern,
                } => {
                    if *asynchrony == Asynchrony::Async {
                        write!(f, [Keyword::Await, space()])?;
                    }
                    write!(f, [Keyword::Using, space(), pattern])?;
                }
            }
            write!(
                f,
                [space(), keyword, space(), iterator, token(")"), space()]
            )?;
            write!(f, [body])?;
        }

        // for condition
        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            write!(
                f,
                [
                    Keyword::For,
                    space(),
                    token("("),
                    initialization,
                    token(";"),
                    space(),
                    condition,
                    token(";"),
                    space(),
                    increment,
                    token(")"),
                    space(),
                    body
                ]
            )?;
        }

        // loop
        Expression::Loop { body } => {
            write!(f, [Keyword::Loop, space(), body])?;
        }

        // try
        Expression::Try {
            try_expression,
            catch_pattern,
            catch_expression,
            finally_expression,
        } => {
            // try <expression>
            write!(f, [Keyword::Try, space(), try_expression])?;

            // catch <expression>
            if let Some(catch) = catch_expression {
                write!(f, [space(), Keyword::Catch, space()])?;
                if let Some(catch_pattern) = catch_pattern {
                    write!(f, [token("("), catch_pattern, token(")"), space()])?;
                }
                write!(f, [catch])?;
            }

            // finally <expression>
            if let Some(finally) = finally_expression {
                write!(f, [space(), Keyword::Finally, space(), finally])?;
            }
        }

        // match
        Expression::Match { .. } => {
            format_match(f, node_id, true)?;
        }

        // break
        Expression::Break { label, value } => {
            write!(f, [Keyword::Break])?;
            if let Some(label) = label {
                write!(f, [space(), token(":"), label])?;
            }
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // continue
        Expression::Continue { label } => {
            write!(f, [Keyword::Continue])?;
            if let Some(label) = label {
                write!(f, [space(), token(":"), label])?;
            }
        }

        // await
        Expression::Await { expression } => {
            write!(f, [Keyword::Await, space(), expression])?;
        }

        // await?
        Expression::AwaitMaybe { expression } => {
            write!(f, [Keyword::Await, token("?"), space(), expression])?;
        }

        // comptime
        Expression::Comptime { body } => {
            write!(f, [Keyword::Comptime, space(), body])?;
        }

        // yield
        Expression::Yield { cardinality, value } => {
            write!(f, [Keyword::Yield])?;
            if *cardinality == YieldCardinality::Generator {
                write!(f, [token("*")])?;
            }
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // throw
        Expression::Throw { value } => {
            write!(f, [token("throw")])?;
            write!(f, [space(), value])?;
        }

        // return
        Expression::Return { value } => {
            write!(f, [token("return")])?;
            if let Some(value_id) = value {
                let value_expr = tree.get(*value_id);
                // for JSX returns, wrap in parens when multi-line
                // JSX with children will always be multi-line, so always wrap those
                if let Expression::TreeExpression { elements, .. } = value_expr {
                    let has_children = elements.as_ref().is_some_and(|e| !e.is_empty());
                    if has_children {
                        // multi-line JSX: wrap in parens with block indent
                        write!(
                            f,
                            [
                                space(),
                                token("("),
                                block_indent(value_id),
                                hard_line_break(),
                                token(")")
                            ]
                        )?;
                    } else {
                        // self-closing or no children: use best_fitting
                        let format_inline = format_with(|f| write!(f, [space(), value_id]));
                        let format_wrapped = format_with(|f| {
                            write!(
                                f,
                                [
                                    space(),
                                    token("("),
                                    block_indent(value_id),
                                    hard_line_break(),
                                    token(")")
                                ]
                            )
                        });
                        best_fitting![format_inline, format_wrapped]
                            .with_mode(BestFittingMode::AllLines)
                            .format(f)?;
                    }
                } else {
                    write!(f, [space(), value_id])?;
                }
            }
        }

        // path
        Expression::Path {
            path,
            static_arguments,
        } => {
            write!(f, [path])?;

            // static arguments
            if let Some(static_arguments) = static_arguments
                && !static_arguments.is_empty()
            {
                write!(f, [list_like("<", ">", ",", static_arguments)])?;
            }
        }

        // this
        Expression::This => {
            write!(f, [Keyword::This])?;
        }

        // scalar literal
        Expression::ScalarLiteral(node) => {
            format_scalar_literal(node, tree.get_span(node_id), f)?;
        }

        // template literal
        Expression::TemplateExpression { value } => {
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // tagged template literal
        Expression::TaggedTemplateExpression { tag, value } => {
            write!(f, [tag])?;
            format_template_literal(value, tree.get_span(node_id), f)?;
        }

        // type template literal
        Expression::TypeTemplateLiteral { strings, spans } => {
            format_type_template_literal(strings, spans, f)?;
        }

        // type literal
        Expression::TypeLiteral(node) => node.format(f)?,

        // type import
        Expression::TypeImport {
            target,
            qualifier,
            static_arguments,
        } => {
            write!(
                f,
                [
                    Keyword::Import,
                    token("("),
                    token("\""),
                    target,
                    token("\""),
                    token(")")
                ]
            )?;
            if let Some(qualifier) = qualifier {
                write!(f, [token("."), qualifier])?;
            }
            if let Some(static_arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", static_arguments)])?;
            }
        }

        // type infer
        Expression::TypeInfer { name, constraint } => {
            write!(f, [Keyword::Infer, space(), *name])?;
            if let Some(constraint) = constraint {
                write!(f, [space(), Keyword::Extends, space(), *constraint])?;
            }
        }

        // type predicate
        Expression::TypePredicate {
            asserts,
            subject,
            target,
        } => {
            if *asserts {
                write!(f, [Keyword::Asserts, space()])?;
            }
            match subject {
                TypePredicateSubject::Identifier(name) => {
                    write!(f, [*name])?;
                }
                TypePredicateSubject::This => {
                    write!(f, [Keyword::This])?;
                }
            }
            if let Some(target) = target {
                write!(f, [space(), Keyword::Is, space(), *target])?;
            }
        }

        // type conditional
        Expression::TypeConditional {
            left,
            right,
            then_type,
            else_type,
        } => {
            write!(
                f,
                [group(&format_args![
                    left,
                    indent(&format_args![
                        soft_line_break_or_space(),
                        Keyword::Extends,
                        space(),
                        right,
                        soft_line_break_or_space(),
                        token("?"),
                        space(),
                        then_type,
                        soft_line_break_or_space(),
                        token(":"),
                        space(),
                        else_type
                    ])
                ])]
            )?;
        }

        // type mapped
        Expression::TypeMapped {
            parameter,
            modifiers,
            value,
        } => {
            let format_parameter_clause = |f: &mut DestackFormatter<'ast, '_>,
                                           break_between_name_and_in: bool|
             -> FormatResult<()> {
                write!(f, [token("["), parameter.name])?;

                // allow a stable break inside the bracket clause
                if break_between_name_and_in {
                    write!(
                        f,
                        [indent(&format_args![
                            hard_line_break(),
                            Keyword::In,
                            space(),
                            parameter.constraint
                        ])]
                    )?;
                } else {
                    write!(f, [space(), Keyword::In, space(), parameter.constraint])?;
                }

                if let Some(key_remap) = parameter.key_remap {
                    write!(f, [space(), Keyword::As, space(), key_remap])?;
                }

                write!(f, [token("]")])
            };

            let inner_break_parameter = format_with(|f| {
                match modifiers.readonly {
                    TypeModifier::Add => {
                        write!(f, [token("readonly"), space()])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-readonly"), space()])?;
                    }
                    TypeModifier::None => {}
                }

                format_parameter_clause(f, true)?;

                match modifiers.optional {
                    TypeModifier::Add => {
                        write!(f, [token("?")])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-?")])?;
                    }
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space(), *value, token(",")])
            });

            let inner_flat = format_with(|f| {
                match modifiers.readonly {
                    TypeModifier::Add => {
                        write!(f, [token("readonly"), space()])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-readonly"), space()])?;
                    }
                    TypeModifier::None => {}
                }

                format_parameter_clause(f, false)?;

                match modifiers.optional {
                    TypeModifier::Add => {
                        write!(f, [token("?")])?;
                    }
                    TypeModifier::Remove => {
                        write!(f, [token("-?")])?;
                    }
                    TypeModifier::None => {}
                }

                write!(f, [token(":"), space(), *value])
            });

            let mapped_break_parameter = format_with(|f| {
                write!(
                    f,
                    [
                        token("{"),
                        indent(&format_args![
                            soft_line_break_or_space(),
                            inner_break_parameter
                        ]),
                        soft_line_break_or_space(),
                        token("}")
                    ]
                )
            });

            let mapped_flat = format_with(|f| {
                write!(
                    f,
                    [
                        token("{"),
                        indent(&format_args![soft_line_break_or_space(), inner_flat]),
                        soft_line_break_or_space(),
                        token("}")
                    ]
                )
            });

            // prefer breaking inside the bracket clause before breaking the value type
            write!(
                f,
                [group(
                    &best_fitting!(mapped_flat, mapped_break_parameter)
                        .with_mode(BestFittingMode::AllLines)
                )]
            )?;
        }

        // type index
        Expression::TypeIndex { left, index } => {
            format_type_index_expression(f, *left, *index)?;
        }

        // range literal
        Expression::RangeExpression {
            start,
            end,
            is_inclusive,
        } => {
            if *is_inclusive {
                write!(f, [start, token("..="), end,])?;
            } else {
                write!(f, [start, token(".."), end,])?;
            }
        }

        // array literal
        Expression::ArrayExpression {
            elements: elements_ids,
        } => {
            // try hugged format for single object/array elements
            if !format_hugged(f, elements_ids, HugOptions::ARRAY, None, false)? {
                let span = f.context().get_span(node_id);
                let elements = elements_ids
                    .iter()
                    .map(|id| tree.get(*id))
                    .collect::<SmallVec<[_; 3]>>();

                // check for annotations that require expansion
                let has_annotations = f.context().has_infix_annotation(node_id)
                    || elements_ids
                        .iter()
                        .any(|element| f.context().has_annotation(*element));

                let should_expand = has_annotations
                    || (elements.len() > 1
                        && elements
                            .iter()
                            .any(|element| is_complex_argument(tree, element)))
                    || (f.context().has_newline(span) && elements.len() > 1);
                write!(
                    f,
                    [list_like("[", "]", ",", elements_ids)
                        .as_collection()
                        .should_expand(should_expand)]
                )?;
            }
        }

        // tuple literal
        Expression::TupleExpression {
            elements: elements_ids,
        } => {
            if elements_ids.is_empty() {
                write!(f, [token("()")])?;
            } else if !format_hugged(f, elements_ids, HugOptions::TUPLE, None, false)? {
                // not a single huggable element - use regular formatting
                let span = f.context().get_span(node_id);
                let elements = elements_ids
                    .iter()
                    .map(|id| tree.get(*id))
                    .collect::<SmallVec<[_; 3]>>();

                // check for annotations that require expansion
                let has_annotations = f.context().has_infix_annotation(node_id)
                    || elements_ids
                        .iter()
                        .any(|element| f.context().has_annotation(*element));

                let should_expand = has_annotations
                    || (elements.len() > 1
                        && elements
                            .iter()
                            .any(|element| is_complex_argument(tree, element)))
                    || (f.context().has_newline(span) && elements.len() > 1);
                // trailing comma disambiguates tuples from parenthesized expressions
                write!(
                    f,
                    [list_like("(", ")", ",", elements_ids)
                        .as_collection()
                        .force_trailing_separator()
                        .should_expand(should_expand)]
                )?;
            }
        }

        // sequence expression (JS/TS comma operator)
        Expression::SequenceExpression { expressions } => {
            if expressions.is_empty() {
                write!(f, [token("()")])?;
            } else {
                write!(f, [list_like("(", ")", ",", expressions)])?;
            }
        }

        // struct literal
        Expression::ObjectExpression { ty, properties } => {
            format_struct_literal(f, node_id, ty, properties)?;
        }

        // tree literal
        Expression::TreeExpression {
            left,
            arguments,
            elements,
        } => {
            format_tree_literal(f, node_id, left, arguments, elements)?;
        }

        // parenthesized
        Expression::Parenthesized { expression } => {
            let inner_expression = tree.get(*expression);
            if let Expression::TreeExpression { .. } = inner_expression {
                write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
            } else {
                write!(f, [token("("), expression, token(")")])?;
            }
        }

        // unary
        Expression::Unary { operator, right } => {
            if operator.is_prefix() {
                write!(f, [operator, right])?;
            } else {
                write!(f, [right, operator])?;
            }
        }

        // type unary
        Expression::TypeUnary { operator, right } => match operator {
            TypeUnaryOperator::Not => {
                write!(f, [operator, right])?;
            }
            TypeUnaryOperator::Must => {
                write!(f, [right, operator])?;
            }
            TypeUnaryOperator::Newtype
            | TypeUnaryOperator::Type
            | TypeUnaryOperator::Readonly
            | TypeUnaryOperator::Typeof
            | TypeUnaryOperator::Keyof => {
                write!(f, [operator, space(), right])?;
            }
            TypeUnaryOperator::AsConst => {
                write!(f, [right, token(" as const")])?;
            }
        },

        // value
        Expression::ValueOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("^")])?;
            if let Some(mutability) = mutability {
                write!(f, [*mutability])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // reference
        Expression::ReferenceOf {
            mutability,
            variance,
            right,
        } => {
            write!(f, [token("&")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Mutable
            {
                write!(f, [*mutability])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
            }
            right.format(f)?;
        }

        // pointer
        Expression::PointerOf { mutability, right } => {
            write!(f, [token("*")])?;
            if let Some(mutability) = mutability
                && *mutability == Mutability::Mutable
            {
                write!(f, [*mutability])?;
            }
            right.format(f)?;
        }

        // member
        Expression::Member { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_member_expression(f, node_id)?;
            }
        }

        // index
        Expression::Index { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_index_expression(f, node_id)?;
            }
        }

        // call
        Expression::Call { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_call_expression(f, node_id)?;
            }
        }

        // new
        Expression::New {
            left,
            static_arguments,
            dynamic_arguments,
        } => {
            write!(f, [token("new"), space(), left])?;
            if let Some(static_arguments) = static_arguments {
                write!(f, [list_like("<", ">", ",", static_arguments)])?;
            }
            write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
        }

        // delete
        Expression::Delete { value } => {
            write!(f, [token("delete"), space(), value])?;
        }

        // maybe
        Expression::Maybe { .. } => {
            if is_expression_chain(tree, node_id) {
                format_expression_chain(f, node_id)?;
            } else {
                format_maybe_expression(f, node_id)?;
            }
        }

        // must
        Expression::Must { position, left } => {
            let needs_parentheses = needs_parens_in_postfix_position(tree, *left);
            if needs_parentheses {
                write!(f, [token("("), left, token(")")])?;
            } else {
                write!(f, [left])?;
            }
            if *position == PostfixPosition::Indirect {
                write!(f, [token(".")])?;
            }
            write!(f, [token("!")])?;
        }

        // binary
        Expression::Binary {
            left: _,
            operator,
            right: _,
        } => {
            let in_type_context = is_type_context(f.context(), node_id);
            let is_type_intersection =
                in_type_context && *operator == BinaryOperator::ElementwiseAnd;
            let is_type_union = in_type_context && *operator == BinaryOperator::ElementwiseOr;

            // flatten binary expression chain for Prettier-style formatting
            // e.g. `a + b + c` formats as:
            //   a
            //       + b
            //       + c
            // all operands at the same indentation level
            let operands = flatten_binary_expression(f.context().tree, node_id, *operator);

            // nullable union types stay inline with `|` separators
            if is_type_union && should_hug_nullable_union_type(f.context(), &operands) {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                        for operand in &operands {
                            if let Some(op) = operand.operator {
                                let has_postfix = prev_expression
                                    .is_some_and(|e| f.context().has_postfix_annotation(e));
                                if !has_postfix {
                                    write!(f, [space()])?;
                                }
                                write!(f, [op, space(), operand.expression])?;
                            } else {
                                write!(f, [operand.expression])?;
                            }
                            prev_expression = Some(operand.expression);
                        }
                        Ok(())
                    }))]
                )?;
                return Ok(());
            }

            // type intersections: prefer trailing `&` with object-like heuristics
            if is_type_intersection {
                write!(
                    f,
                    [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                        let mut prev_object_like = false;
                        let mut prev_has_annotation = false;

                        for operand in &operands {
                            if let Some(op) = operand.operator {
                                let has_postfix = prev_expression
                                    .is_some_and(|e| f.context().has_postfix_annotation(e));
                                let is_object_like =
                                    is_object_like_type_expression(f.context(), operand.expression);
                                let current_has_annotation =
                                    f.context().has_annotation(operand.expression);
                                let allow_break = !(prev_object_like || is_object_like)
                                    || prev_has_annotation
                                    || current_has_annotation;

                                if !has_postfix {
                                    write!(f, [space()])?;
                                }

                                write!(f, [op])?;

                                if allow_break {
                                    write!(
                                        f,
                                        [indent(&format_with(
                                            |f: &mut DestackFormatter<'ast, '_>| {
                                                write!(
                                                    f,
                                                    [
                                                        soft_line_break_or_space(),
                                                        operand.expression
                                                    ]
                                                )
                                            }
                                        ))]
                                    )?;
                                } else {
                                    write!(f, [space(), operand.expression])?;
                                }

                                prev_object_like = is_object_like;
                                prev_has_annotation = current_has_annotation;
                            } else {
                                write!(f, [operand.expression])?;
                                prev_object_like =
                                    is_object_like_type_expression(f.context(), operand.expression);
                                prev_has_annotation =
                                    f.context().has_annotation(operand.expression);
                            }

                            prev_expression = Some(operand.expression);
                        }

                        Ok(())
                    }))]
                )?;
                return Ok(());
            }

            // default: leading operator on break
            write!(
                f,
                [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let mut prev_expression: Option<LocalNodeId<Expression>> = None;
                    for operand in &operands {
                        if let Some(op) = operand.operator {
                            // check if previous operand has postfix annotation (it adds its own space)
                            let has_postfix = prev_expression
                                .is_some_and(|e| f.context().has_postfix_annotation(e));
                            // subsequent operands: soft break, operator, space, operand
                            write!(
                                f,
                                [indent(&format_with(
                                    |f: &mut DestackFormatter<'ast, '_>| {
                                        if !has_postfix {
                                            write!(f, [soft_line_break_or_space()])?;
                                        }
                                        write!(f, [op, space(), operand.expression])
                                    }
                                ))]
                            )?;
                        } else {
                            // first operand has no preceding operator
                            write!(f, [operand.expression])?;
                        }
                        prev_expression = Some(operand.expression);
                    }
                    Ok(())
                }))]
            )?;
        }

        // type binary
        Expression::TypeBinary {
            left,
            operator,
            right,
        } => {
            let has_postfix = f.context().has_postfix_annotation(*left);
            write!(
                f,
                [group(&format_args![
                    left,
                    indent(&format_with(|f| {
                        if !has_postfix {
                            write!(f, [soft_line_break_or_space()])?;
                        }
                        write!(f, [operator, space(), right])
                    }))
                ])]
            )?;
        }

        // assign
        Expression::Assign {
            left,
            operator,
            right,
        } => {
            let has_postfix = f.context().has_postfix_annotation(*left);
            let inner_right_id = transparent_inner_expression(f.context(), *right);
            let inner_right_expr = f.context().tree.get(inner_right_id);

            // binaries, chains, and nested lambda tails handle their own breaking
            let right_is_binary = matches!(inner_right_expr, Expression::Binary { .. });
            let right_is_chain_root = is_chain_root(f.context().tree, inner_right_id);
            let right_is_chain =
                is_expression_chain(f.context().tree, inner_right_id) || right_is_chain_root;
            let right_is_chain_tail_lambda =
                is_assignment_chain_tail_lambda(f.context(), node_id, inner_right_id);
            let right_is_lambda = is_lambda_expression(f.context(), inner_right_id);
            let right_handles_its_own_breaking =
                right_is_binary || right_is_chain || right_is_chain_tail_lambda || right_is_lambda;

            // string literals are atomic: never break at `=`
            let is_string_literal = matches!(
                inner_right_expr,
                Expression::ScalarLiteral(ScalarLiteral::String(_))
                    | Expression::TemplateExpression { .. }
            );

            // prefer breaking after `=` for long binary rhs values
            let right_is_long_binary = if right_is_binary {
                let line_width = usize::from(f.context().options.line_width);
                let left_source_len = expression_source_len(f.context(), *left);
                let operator_len = assign_operator_len(operator);
                let inline_overhead = left_source_len
                    .saturating_add(operator_len)
                    .saturating_add(2);
                let remaining_width = line_width.saturating_sub(inline_overhead);
                let right_source_len = expression_source_len(f.context(), inner_right_id);
                let binary_operand_count = match inner_right_expr {
                    Expression::Binary { operator, .. } => {
                        flatten_binary_expression(f.context().tree, inner_right_id, *operator).len()
                    }
                    _ => 0,
                };
                let is_very_long_binary = right_source_len > line_width.saturating_sub(4);

                right_source_len > remaining_width
                    && is_very_long_binary
                    && binary_operand_count > 2
            } else {
                false
            };

            if is_string_literal {
                write!(
                    f,
                    [group(&format_args![
                        left,
                        format_with(|f| {
                            if !has_postfix {
                                write!(f, [space()])?;
                            }
                            Ok(())
                        }),
                        operator,
                        space(),
                        right
                    ])]
                )?;
                return Ok(());
            }

            // format the space before the operator, respecting postfix comments
            let space_before_operator = format_with(|f| {
                if !has_postfix {
                    write!(f, [space()])?;
                }
                Ok(())
            });

            // break long binary rhs values after the operator
            if right_is_long_binary {
                let format_break_after_operator = format_with(|f| {
                    // avoid double indentation when the rhs breaks internally
                    let dedented_right = dedent(&right);
                    group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        indent(&format_args![hard_line_break(), dedented_right])
                    ])
                    .should_expand(true)
                    .format(f)
                });

                let format_inline = format_with(|f| {
                    group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        space(),
                        right
                    ])
                    .format(f)
                });

                best_fitting![format_inline, format_break_after_operator]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
                return Ok(());
            }

            if right_handles_its_own_breaking {
                // binaries and chains break internally, keep the assignment inline
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        space(),
                        right
                    ])]
                )?;
            } else {
                // other expressions get indented on break
                write!(
                    f,
                    [group(&format_args![
                        left,
                        space_before_operator,
                        operator,
                        indent(&format_args![soft_line_break_or_space(), right])
                    ])]
                )?;
            }
        }

        // debugger
        Expression::Debugger => {
            write!(f, [token("debugger")])?;
        }

        // stub (placeholder for annotation-only files)
        Expression::Stub => {}

        // error
        Expression::Error => {
            write!(f, [token("/* ERROR */")])?;
        }
    };

    Ok(())
}

/// Format a declarator (pattern, optional type, optional value).
fn format_declarator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    tree: &NodeTree,
    declarator_id: LocalNodeId<Declarator>,
) -> FormatResult<()> {
    let declarator = tree.get(declarator_id);
    let Declarator { pattern, ty, value } = declarator;

    // header: pattern + optional type
    let header = format_with(|f| {
        write!(f, [pattern])?;
        if let Some(ty_id) = ty {
            write!(f, [token(":"), space(), ty_id])?;
        }
        Ok(())
    });

    let Some(value_id) = value else {
        write!(f, [header])?;
        return Ok(());
    };

    let value_expr = tree.get(*value_id);
    let value_inner_id = transparent_inner_expression(f.context(), *value_id);
    let value_inner_expr = tree.get(value_inner_id);

    let pattern_breakable = is_pattern_breakable(tree, *pattern);
    let value_breakable = is_expression_breakable(tree, value_expr);
    let value_is_binary = matches!(value_inner_expr, Expression::Binary { .. });
    let value_is_chain_root = is_chain_root(tree, value_inner_id);
    let value_is_chain = is_expression_chain(tree, value_inner_id) || value_is_chain_root;
    let value_is_poor_chain =
        value_is_chain && is_poorly_breakable_chain(f.context(), value_inner_id);
    let value_is_lambda = is_lambda_expression(f.context(), value_inner_id);
    let value_is_declaration = matches!(value_inner_expr, Expression::Declaration(_));
    let value_handles_its_own_breaking =
        value_is_binary || value_is_chain || value_is_lambda || value_is_declaration;
    let should_force_expand_value = value_breakable && !value_is_poor_chain;

    // estimate remaining width if the declarator stayed inline
    let line_width = usize::from(f.context().options.line_width);
    let pattern_span = f.context().get_span(*pattern);
    let pattern_source_len = f.context().get_span_str(pattern_span).chars().count();
    let type_source_len = ty.map(|ty_id| expression_source_len(f.context(), ty_id));
    let header_source_len = type_source_len.map_or(pattern_source_len, |type_len| {
        pattern_source_len
            .saturating_add(type_len)
            .saturating_add(2)
    });
    let remaining_width = line_width.saturating_sub(header_source_len.saturating_add(3));

    let value_source_len = expression_source_len(f.context(), *value_id);
    let value_is_long = value_source_len > remaining_width;
    let value_binary_operand_count = match value_inner_expr {
        Expression::Binary { operator, .. } => {
            flatten_binary_expression(tree, value_inner_id, *operator).len()
        }
        _ => 0,
    };
    let value_is_very_long_binary = value_source_len > line_width.saturating_sub(4);
    let value_is_long_binary = value_is_binary
        && value_is_long
        && value_is_very_long_binary
        && value_binary_operand_count > 2;

    // string literals are atomic - never break at `=`
    let is_string_literal = matches!(
        value_inner_expr,
        Expression::ScalarLiteral(ScalarLiteral::String(_)) | Expression::TemplateExpression { .. }
    );

    // prefer keeping the value on a single line
    let format_inline = format_with(|f| {
        write!(f, [header, space(), token("="), space(), *value_id])?;
        Ok(())
    });
    // expand inline if value is breakable (like let x = [\n ... ])
    let format_value_expanded = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                fits_expanded(&group(value_id).should_expand(should_force_expand_value)),
            ]
        )
    });

    // expand inline without a fits boundary for chains and binaries
    let format_value_expanded_strict = format_with(|f| {
        write!(
            f,
            [
                header,
                space(),
                token("="),
                space(),
                group(value_id).should_expand(should_force_expand_value),
            ]
        )
    });
    // expand the header (pattern) while keeping value inline
    // e.g., const { a, b } = value becomes:
    // const {
    //     a,
    //     b,
    // } = value;
    let format_header_expanded = format_with(|f| {
        write!(
            f,
            [
                fits_expanded(&group(&header).should_expand(true)),
                space(),
                token("="),
                space(),
                *value_id,
            ]
        )
    });
    // expand and indent the value on a new line (last resort)
    let format_indented = format_with(|f| {
        group(&format_args![
            header,
            space(),
            token("="),
            block_indent(value_id)
        ])
        .format(f)
    });
    let format_break_after_operator_for_binary = format_with(|f| {
        // avoid double indentation when the rhs breaks internally
        let dedented_value = dedent(value_id);
        group(&format_args![
            header,
            space(),
            token("="),
            indent(&format_args![hard_line_break(), dedented_value])
        ])
        .should_expand(true)
        .format(f)
    });

    // for string literals, never break at `=` - just let them exceed line width
    if is_string_literal {
        if pattern_breakable {
            best_fitting![format_inline, format_header_expanded]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
        } else {
            write!(f, [format_inline])?;
        }
    } else if value_is_long_binary {
        best_fitting![format_inline, format_break_after_operator_for_binary]
            .with_mode(BestFittingMode::AllLines)
            .format(f)?;
    } else if value_handles_its_own_breaking {
        // binaries and chains break internally, avoid breaking after `=`
        match pattern_breakable {
            true => {
                best_fitting![
                    format_inline,
                    format_value_expanded_strict,
                    format_header_expanded
                ]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
            }
            false => {
                best_fitting![format_inline, format_value_expanded_strict]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
            }
        }
    } else {
        match (pattern_breakable, value_breakable) {
            (true, true) => {
                // both sides breakable: prefer inline, then expanded variants
                best_fitting![
                    format_inline,
                    format_value_expanded,
                    format_header_expanded,
                    format_indented
                ]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
            }
            (true, false) => {
                // keep long atomic rhs values inline when possible
                if value_is_long {
                    best_fitting![format_inline, format_header_expanded]
                        .with_mode(BestFittingMode::AllLines)
                        .format(f)?;
                } else {
                    best_fitting![format_inline, format_header_expanded, format_indented]
                        .with_mode(BestFittingMode::AllLines)
                        .format(f)?;
                }
            }
            (false, true) => {
                best_fitting![format_inline, format_value_expanded, format_indented]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
            }
            (false, false) => {
                best_fitting![format_inline, format_indented].format(f)?;
            }
        }
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declarator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        format_declarator(f, f.context().tree, node_id)?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: LocalNodeId<Expression>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        format_expression(f, node_id, self)?;

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatContext, DestackFormatOptions, TestFormatter, assert_format};
    use destack_ast::{
        Argument, Declaration, DeclarationDescriptor, Expression, NodeParentIndex, NodeTree,
    };

    /// Simple expressions should stay on one line.
    #[test]
    fn test_format_expression_simple() {
        assert_format!(
            "1 + 2 * 3 - a / b % c",
            "1 + 2 * 3 - a / b % c",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Parenthesized expressions should retain their parentheses.
    #[test]
    fn test_format_expression_parenthesized() {
        assert_format!(
            "(((1 + 2) * 3) - a / (b % c))",
            "(((1 + 2) * 3) - a / (b % c))",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_parenthesis() {
        assert_format!(
            "(((())))",
            "(((())))",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_arguments() {
        assert_format!(
            "foo<()>(((())))",
            "foo<()>(((())))",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_struct_literal_trivial() {
        assert_format!(
            "({ a: 1, ...B })",
            "({ a: 1, ...B })",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_struct_literal_spread() {
        assert_format!(
            "Foo { ...B }",
            "Foo { ...B }",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_if_ternary() {
        assert_format!(
            "true ? 1 : 2",
            "true ? 1 : 2",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_index_call_mixed_postfix() {
        assert_format!(
            "x?.[f]?.[2]?.(a, b)",
            "x?.[f]?.[2]?.(a, b)",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Tree children with map callbacks that return trees force a break.
    #[test]
    fn test_tree_child_map_callback_breaks() {
        let input = "<List>{items.map((item) => <Item key={item.id} />)}</List>";
        let (formatter, expression_id) =
            TestFormatter::parse(input, |p| p.eat_expression()).expect("parse tree expression");

        // find the tree child expression
        let Expression::TreeExpression { elements, .. } = formatter.tree.get(expression_id) else {
            panic!("expected tree expression");
        };
        let elements = elements.as_ref().expect("expected elements");
        let child_id = elements.first().expect("expected child element");
        let Argument::Positional { value, .. } = formatter.tree.get(*child_id) else {
            panic!("expected positional child");
        };

        // build a context to run the helper on
        let context = DestackFormatContext {
            options: DestackFormatOptions::default(),
            file: &formatter.file,
            tree: &formatter.tree,
            source_map: &formatter.tree.source_map,
            parents: NodeParentIndex::from_tree(&formatter.tree),
            tokens: &formatter.tokens,
            side_tokens: &formatter.side_tokens,
            side_span: &formatter.side_span,
            strings: &formatter.strings,
            current_argument_group_id: None,
        };

        assert!(super::expression_has_complex_callback(&context, *value));
    }

    /// Redundant const on borrows is omitted in type formatting.
    #[test]
    fn test_format_type_redundant_const_borrow() {
        assert_format!("&const Foo", "&Foo", |p| p.eat_expression());
    }

    /// Redundant const on pointers is omitted in type formatting.
    #[test]
    fn test_format_type_redundant_const_pointer() {
        assert_format!(
            "type T = *const Foo",
            "*Foo",
            |p| p.eat_type(p.mark(), DeclarationDescriptor::default()),
            |tree: &NodeTree, expr_id| {
                let expression = tree.get(expr_id);
                let Expression::Declaration(declaration_id) = expression else {
                    panic!("expected type declaration expression");
                };

                let declaration = tree.get(*declaration_id);
                let Declaration::Type { value, .. } = declaration else {
                    panic!("expected type declaration");
                };

                *value
            },
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_member_call_chain_line() {
        assert_format!(
            "call().followed().by().many().calls()",
            "call().followed().by().many().calls()",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(100)
        );
    }

    #[test]
    fn test_format_member_call_chain_retains_breaks() {
        assert_format!(
            "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
            "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_member_call_chain_breaks() {
        assert_format!(
            "call().followed().by().many().calls()",
            "call()\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_member_call_chain_breaks_with_maybe_and_index() {
        assert_format!(
            "call().followed()?.by()[0]?.many()?.calls()",
            "call()\n\t.followed()\n\t?.by()\n\t[0]\n\t?.many()\n\t?.calls()",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_path_member_call_chain_breaks() {
        assert_format!(
            "long.base.path.followed().by().many().calls()",
            "long.base\n\t.path\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_index_member_chain_breaks() {
        assert_format!(
            "identifier1.identifier2.identifier3[indexA].identifier4[indexB]?.[indexC][indexD]",
            "identifier1\n\t.identifier2\n\t.identifier3[indexA]\n\t.identifier4[indexB]\n\t?.[indexC]\n\t[indexD]",
            |p| p.eat_expression(),
            DestackFormatOptions::default_tab_with_line_width(20)
        );
    }

    #[test]
    fn test_format_expression_tree_literal_without_arguments() {
        assert_format!(
            "<Entity/>",
            "<Entity />",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_arguments() {
        assert_format!(
            "<Entity a={1} b = {2} />",
            "<Entity a={1} b={2} />",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_parenthesized() {
        let source = r"(
    <Entity a={1} b={2}>
        <Entity a={1} b={2} />
    </Entity>
)";
        let expected = r"(
    <Entity a={1} b={2}>
        <Entity a={1} b={2} />
    </Entity>
)";
        assert_format!(
            source,
            expected,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_nested() {
        let source = r#"<A x={4} y={4}>
    <B x="hey">
        <C>
            <D />
            2
        </C>
    </B>
</A>"#;
        let expected = r#"<A x={4} y={4}>
    <B x="hey">
        <C>
            <D />2
        </C>
    </B>
</A>"#;
        assert_format!(
            source,
            expected,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_array_of_struct_element() {
        // multiline array attributes break the element
        let source = r#"<Menu
    items=[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]
/>"#;
        let expected = r#"<Menu
    items={[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]}
/>"#;
        assert_format!(
            source,
            expected,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_call_with_struct_literal() {
        let source = r#"Destack.serve({
    fetch: function (req: Request) {
        return Response("Success!")
    },
    run: true,
})"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_expression_let_call() {
        let input = r#"const ast = await parseAsync(text, {
    sourceFileName: "file",
    parserOpts: {
        plugins: ["typescript", "jsx"],
    },
    sourceType: "module",
    configFile: false,
    babelrc: false,
})"#;
        let expected = r#"const ast = await parseAsync(
    text,
    {
        sourceFileName: "file",
        parserOpts: {
            plugins: [
                "typescript",
                "jsx",
            ],
        },
        sourceType: "module",
        configFile: false,
        babelrc: false,
    },
)"#;
        assert_format!(
            input,
            expected,
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_chained_assignment() {
        assert_format!(
            "a = b = c = 1",
            "a = b = c = 1",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_chained_assignment_long() {
        assert_format!(
            "veryLongName = anotherLongName = thirdLongName = 42",
            "veryLongName =\n    anotherLongName =\n    thirdLongName =\n    42",
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(30)
        );
    }

    #[test]
    fn test_format_jsx_with_comment() {
        assert_format!(
            "<Container>{/* XOXO */}</Container>",
            "<Container>\n    {\n        /* XOXO */\n    }\n</Container>",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_jsx_conditional_child() {
        assert_format!(
            "<div>{loading && <Spinner />}</div>",
            "<div>{loading && <Spinner />}</div>",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_jsx_in_function_call() {
        assert_format!(
            "render(<App />)",
            "render(<App />)",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_deeply_nested_callbacks() {
        assert_format!(
            "fetch(url).then((res) => res.json()).then((data) => process(data))",
            "fetch(url)\n    .then((res) => res.json())\n    .then((data) => process(data))",
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(40)
        );
    }

    #[test]
    fn test_format_optional_chain_with_nullish() {
        assert_format!(
            r#"user?.profile?.name ?? "Anonymous""#,
            r#"user?.profile?.name ?? "Anonymous""#,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Formats type conditionals with infer bindings.
    #[test]
    fn test_format_type_conditional_with_infer() {
        assert_format!(
            "type Result = T extends infer U ? U : never",
            "type Result = T extends infer U ? U : never;",
            |p| p.eat_expression()
        );
    }

    /// Formats type conditionals with constrained infer bindings.
    #[test]
    fn test_format_type_conditional_with_constrained_infer() {
        assert_format!(
            "type Result = T extends infer U extends string ? U : never",
            "type Result = T extends infer U extends string ? U : never;",
            |p| p.eat_expression()
        );
    }

    /// Formats nested conditional types with explicit parentheses.
    #[test]
    fn test_format_type_conditional_nested() {
        assert_format!(
            "type Result = T extends U ? (U extends V ? X : Y) : Z",
            "type Result = T extends U ? (U extends V ? X : Y) : Z;",
            |p| p.eat_expression()
        );
    }

    /// Formats type intersections with trailing operators in Destack.
    #[test]
    fn test_format_type_intersection_trailing_operator_destack() {
        assert_format!(
            "type Combined = HasName & HasAge & HasEmail",
            "type Combined = HasName &\n    HasAge &\n    HasEmail;",
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(30)
        );
    }

    /// Formats mapped types with modifiers and key remaps.
    #[test]
    fn test_format_type_mapped_with_remap() {
        let source = r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] }"#;
        let expected = r#"type Remap = { readonly [K in keyof T as `${K}`]-?: T[K] };"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats mapped types with removal modifiers.
    #[test]
    fn test_format_type_mapped_with_removals() {
        let source = r#"type Mutable = { -readonly [K in keyof T]-?: T[K] }"#;
        let expected = r#"type Mutable = { -readonly [K in keyof T]-?: T[K] };"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats mapped types without modifiers.
    #[test]
    fn test_format_type_mapped_without_modifiers() {
        let source = r#"type Plain = { [K in keyof T]: T[K] }"#;
        let expected = r#"type Plain = { [K in keyof T]: T[K] };"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats mapped types with optional modifiers.
    #[test]
    fn test_format_type_mapped_with_optional() {
        let source = r#"type Optional = { [K in keyof T]?: T[K] }"#;
        let expected = r#"type Optional = { [K in keyof T]?: T[K] };"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats chained type index expressions.
    #[test]
    fn test_format_type_index() {
        let source = r#"type Value = T[K][P]"#;
        let expected = r#"type Value = T[K][P];"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats type template literals with single spans.
    #[test]
    fn test_format_type_template_literal() {
        let source = r#"type Key = `on${K}`"#;
        let expected = r#"type Key = `on${K}`;"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats type template literals with multiple spans.
    #[test]
    fn test_format_type_template_literal_multiple_spans() {
        let source = r#"type Key = `on${K}:${V}`"#;
        let expected = r#"type Key = `on${K}:${V}`;"#;
        assert_format!(source, expected, |p| p.eat_expression());
    }

    /// Formats type imports with qualifiers.
    #[test]
    fn test_format_type_import() {
        assert_format!(
            r#"type Imported = import("mod").Type"#,
            r#"type Imported = import("mod").Type;"#,
            |p| p.eat_expression()
        );
    }

    /// Formats type imports without qualifiers.
    #[test]
    fn test_format_type_import_without_qualifier() {
        assert_format!(
            r#"type Imported = import("mod")"#,
            r#"type Imported = import("mod");"#,
            |p| p.eat_expression()
        );
    }

    /// Formats standalone infer expressions.
    #[test]
    fn test_format_type_infer_expression() {
        assert_format!("type Result = infer U", "type Result = infer U;", |p| p
            .eat_expression());
    }

    #[test]
    fn test_format_async_arrow() {
        assert_format!(
            "async (event) => await processEvent(event)",
            "async (event) => await processEvent(event)",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_return_jsx_inline() {
        // Short JSX returns stay inline
        assert_format!(
            "return <App />",
            "return <App />",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_return_jsx_multiline() {
        assert_format!(
            "return <App prop=\"value\" another=\"thing\" />",
            "return (\n    <App\n        prop=\"value\"\n        another=\"thing\"\n    />\n)",
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(30)
        );
    }

    #[test]
    fn test_format_nested_ternary() {
        assert_format!(
            "const x = isFirst ? firstValue : isSecond ? secondValue : defaultValue",
            "const x = isFirst\n    ? firstValue\n    : isSecond\n    ? secondValue\n    : defaultValue",
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(50)
        );
    }

    #[test]
    fn test_format_jsx_bracket_same_line_true() {
        let mut options = DestackFormatOptions::default_with_line_width(30);
        options.bracket_same_line = true;
        assert_format!(
            r#"<Button variant="primary" size="large" disabled />"#,
            "<Button\n    variant=\"primary\"\n    size=\"large\"\n    disabled />",
            |p| p.eat_expression(),
            options
        );
    }

    #[test]
    fn test_format_jsx_bracket_same_line_false() {
        let mut options = DestackFormatOptions::default_with_line_width(30);
        options.bracket_same_line = false;
        assert_format!(
            r#"<Button variant="primary" size="large" disabled />"#,
            "<Button\n    variant=\"primary\"\n    size=\"large\"\n    disabled\n/>",
            |p| p.eat_expression(),
            options
        );
    }

    /// Prefix expressions inside postfix operators get parenthesized.
    #[test]
    fn test_format_await_inside_maybe_gets_parenthesized() {
        // this tests the case where we have Maybe { left: Await { expr } }
        // which should format as (await expr)? not await expr?
        assert_format!(
            "(await foo())?",
            "(await foo())?",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Unary prefix inside maybe gets parenthesized.
    #[test]
    fn test_format_unary_inside_maybe_gets_parenthesized() {
        assert_format!(
            "(-x)?",
            "(-x)?",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Postfix inside postfix doesn't need extra parentheses.
    #[test]
    fn test_format_postfix_inside_maybe_no_extra_parens() {
        assert_format!(
            "x.foo?",
            "x.foo?",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Call expression inside maybe doesn't need parentheses.
    #[test]
    fn test_format_call_inside_maybe_no_parens() {
        assert_format!(
            "foo()?",
            "foo()?",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    /// Await? syntactic sugar stays as is.
    #[test]
    fn test_format_await_maybe_sugar() {
        assert_format!(
            "await? foo()",
            "await? foo()",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }
}
