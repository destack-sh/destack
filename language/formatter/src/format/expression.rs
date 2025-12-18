use std::cmp::Ordering;

use destack_ast::{
    Argument, Asynchrony, BinaryOperator, Declaration, Declarator, DependencyItem, DependencyKind,
    DependencyMode, Expression, ForEachKind, IfKind, Keyword, LetKind, LocalNodeId, NodeTree,
    Pattern, PostfixPosition, Property, ScalarLiteral, TypeUnaryOperator, WhileKind,
    YieldCardinality,
};
use destack_base::StringId;
use destack_fir::format::{BestFittingMode, FormatError};
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
            Argument::Named { name, value } => {
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
            Argument::Labeled { label, value } => {
                // label
                write!(f, [label])?;
                // value
                write!(f, [token(":"), space(), value])?;
            }
            Argument::Positional { value } => {
                // In tree expressions, expression children need braces too
                let value_expr = f.context().tree.get(*value);
                let needs_braces = !matches!(
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
            Argument::Spread { value } => {
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
                write!(
                    f,
                    [
                        Keyword::If,
                        space(),
                        token("("),
                        condition,
                        token(")"),
                        space()
                    ]
                )?;

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
        allow_arrow_functions: true,
        handle_annotations: true,
    };

    const ARRAY: Self = Self {
        open: "[",
        close: "]",
        force_trailing: false,
        allow_arrow_functions: false,
        handle_annotations: false,
    };

    const TUPLE: Self = Self {
        open: "(",
        close: ")",
        force_trailing: true,
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
        Argument::Positional { value } => Some(*value),
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

        branches.push((*condition, *then_expression));

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
            // arrow function with block body
            if let Declaration::Function {
                body: Some(body_id),
                ..
            } = tree.get(*declaration_id)
            {
                matches!(tree.get(*body_id), Expression::Block(_))
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

    if !is_huggable_expression(tree, value_id, &config) {
        return Ok(false);
    }

    let trailing = if config.force_trailing { "," } else { "" };

    // inline: keep everything on one line
    let inline_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(f, [token(config.open), argument_id])?;
        if config.force_trailing {
            write!(f, [token(",")])?;
        }
        write!(f, [token(config.close)])
    });

    // hugged: argument expands but delimiters hug
    let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if config.handle_annotations {
            write!(f, [f.context().any_prefix_annotations(argument_id)])?;
        }

        match f.context().tree.get(value_id) {
            Expression::ObjectExpression { ty, properties } => {
                write!(f, [token(config.open)])?;
                if let Some(ty) = ty {
                    write!(f, [ty, space()])?;
                }
                write!(
                    f,
                    [
                        group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
                                            .entries(properties)
                                            .finish()?;
                                            write!(f, [if_group_breaks(&token(","))])
                                        }
                                    )),
                                    token("}")
                                ]
                            )
                        }))
                        .should_expand(true),
                        token(trailing),
                        token(config.close)
                    ]
                )?;
            }
            Expression::ArrayExpression { elements } => {
                write!(
                    f,
                    [
                        token(config.open),
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
                                            .entries(elements)
                                            .finish()?;
                                            write!(f, [if_group_breaks(&token(","))])
                                        }
                                    )),
                                    token("]")
                                ]
                            )
                        }))
                        .should_expand(true),
                        token(trailing),
                        token(config.close)
                    ]
                )?;
            }
            Expression::Declaration(declaration_id) if config.allow_arrow_functions => {
                // arrow function: format normally, handles its own expansion
                write!(f, [token(config.open), declaration_id, token(config.close)])?;
            }
            _ => {
                write!(f, [token(config.open), value_id])?;
                if config.force_trailing {
                    write!(f, [token(",")])?;
                }
                write!(f, [token(config.close)])?;
            }
        }

        if config.handle_annotations {
            write!(
                f,
                [f.context().any_infix_or_postfix_annotations(argument_id)]
            )?;
        }
        Ok(())
    });

    best_fitting![inline_format, hugged_format]
        .with_mode(BestFittingMode::AllLines)
        .format(f)?;

    Ok(true)
}

/// Format a tree/JSX attribute value with hugging for objects/arrays.
///
/// Uses `fits_expanded` so inner breaks don't cause the outer tree element to break.
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
            // uses fits_expanded so the outer tree element doesn't break
            let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [
                        token("="),
                        token("{"),
                        fits_expanded(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
                        })),
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
            // uses fits_expanded so the outer tree element doesn't break
            let hugged_format = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                write!(
                    f,
                    [
                        token("="),
                        token("{"),
                        fits_expanded(
                            &group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
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
                            .should_expand(true)
                        ),
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

        // try hugged format for single object/array arguments
        if !format_hugged(f, dynamic_arguments, HugOptions::CALL)? {
            // force expansion when single argument is multi-line JSX
            let force_expand_jsx = has_multiline_jsx_argument(f.context().tree, dynamic_arguments);

            // force expansion when arguments have annotations (comments)
            let has_annotations = f.context().has_infix_annotation(node_id)
                || dynamic_arguments
                    .iter()
                    .any(|arg| f.context().has_annotation(*arg));

            let force_expand = force_expand_jsx || has_annotations;
            write!(
                f,
                [list_like("(", ")", ",", dynamic_arguments).should_expand(force_expand)]
            )?;
        }
    } else {
        debug_assert!(false, "unexpected expression kind for call formatter");
    }
    Ok(())
}

/// Format a maybe expression without considering chaining.
#[inline]
fn format_maybe_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    if let Expression::Maybe { left, position } = f.context().tree.get(node_id) {
        write!(f, [*left])?;
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
            // try hugged format for single object/array arguments
            if !format_hugged(f, dynamic_arguments, HugOptions::CALL)? {
                // force expansion when arguments have annotations (comments)
                let has_annotations = f.context().has_infix_annotation(*call_node_id)
                    || dynamic_arguments
                        .iter()
                        .any(|arg| f.context().has_annotation(*arg));

                write!(
                    f,
                    [list_like("(", ")", ",", dynamic_arguments).should_expand(has_annotations)]
                )?;
            }
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
fn is_expression_chain(tree: &NodeTree, node_id: LocalNodeId<Expression>) -> bool {
    match tree.get(node_id) {
        Expression::Member { left, .. }
        | Expression::Call { left, .. }
        | Expression::Index { left, .. }
        | Expression::Maybe { left, .. } => is_chain_expression(tree.get(*left)),
        _ => false,
    }
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
        // NOTE: path segments are synthetic, they come from a single Path expression,
        // so we use root_id for annotations (though they won't have intra-path comments)
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

    // group the chain operations into lines
    let lines = group_chain_expression_lines(body);

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
        group(&format_with(|f| {
            // always print the base first so indentation aligns subsequent lines
            format_chain_base(f, &base)?;
            // indent chained entries so each operation sits on its own line
            // Use indent with manual line breaks instead of block_indent to avoid trailing newline
            // This ensures semicolons stay on the same line as the last chain element
            if !lines.is_empty() {
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
            }
            Ok(())
        }))
        .format(f)
    });
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
    let Expression::Match {
        kind: _,
        value,
        cases,
    } = &match_node
    else {
        return Err(FormatError::SyntaxError {
            message: "invalid match expression",
        });
    };

    if include_prefix {
        // match <expression>
        write!(f, [Keyword::Match, space()])?;
    }

    write!(f, [token("("), value, token(")")])?;

    // empty match body
    if cases.is_empty() {
        write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        return Ok(());
    }

    // match cases
    write!(f, [space(), token("{"), hard_line_break()])?;
    write!(
        f,
        [group(&format_args![block_indent(&format_with(|f| f
            .join_with(hard_line_break())
            .entries(cases)
            .finish())),])]
    )?;
    write!(f, [f.context().block_infix_annotations(node_id)])?;
    write!(f, [hard_line_break(), token("}")])?;

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
        Expression::Binary { .. } | Expression::TypeBinary { .. } => true,
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

/// Format a tree literal.
#[inline]
pub(crate) fn format_tree_literal<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    expression_id: LocalNodeId<Expression>,
    left: &Option<LocalNodeId<Expression>>,
    arguments: &Option<Vec<LocalNodeId<Argument>>>,
    elements: &Option<Vec<LocalNodeId<Argument>>>,
) -> FormatResult<()> {
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
                            if single_attr_per_line && arguments.len() > 1 {
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

                        // when bracket_same_line is true, don't add trailing line break before >
                        // when false (default), soft_block_indent adds trailing soft_line_break
                        if bracket_same_line {
                            write!(
                                f,
                                [
                                    if_group_fits_on_line(&space()),
                                    indent(&format_args![soft_line_break(), format_attrs])
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
                // check if source has newlines in the tree body
                let span = f.context().get_span(expression_id);
                let source_has_newline = f.context().has_newline(span);

                // check child types for formatting decisions
                let has_multiple_elements = elements.len() > 1;
                let tree = f.context().tree;
                let element_children_count = elements
                    .iter()
                    .filter(|elem_id| {
                        let arg = tree.get(**elem_id);
                        if let Argument::Positional { value } = arg {
                            matches!(tree.get(*value), Expression::TreeExpression { .. })
                        } else {
                            false
                        }
                    })
                    .count();

                // check if any child has complex content (block expressions)
                let has_complex_child = elements.iter().any(|elem_id| {
                    let arg = tree.get(*elem_id);
                    if let Argument::Positional { value } = arg {
                        // check for call with callback that has block body
                        if let Expression::Call {
                            dynamic_arguments, ..
                        } = tree.get(*value)
                        {
                            dynamic_arguments.iter().any(|arg_id| {
                                if let Argument::Positional { value: arg_val } = tree.get(*arg_id)
                                    && let Expression::Declaration(decl_id) = tree.get(*arg_val)
                                    && let Declaration::Function {
                                        body: Some(body_id),
                                        ..
                                    } = tree.get(*decl_id)
                                {
                                    return matches!(tree.get(*body_id), Expression::Block(_));
                                }
                                false
                            })
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                });

                // force breaking when:
                // - Source has newlines (preserve author's formatting intent), OR
                // - ALL children are tree elements and there are multiple, OR
                // - Any child has complex content (callbacks with block body)
                let all_elements = element_children_count == elements.len();
                let force_break = source_has_newline
                    || (all_elements && has_multiple_elements)
                    || has_complex_child;

                // Format children using TreeExpressionArgument for proper brace handling
                let format_children = format_with(|f| {
                    f.join_with(soft_line_break_or_space())
                        .entries(
                            elements
                                .iter()
                                .map(|elem| TreeExpressionArgument { argument_id: *elem }),
                        )
                        .finish()
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
            kind,
            target,
            items,
            arguments,
        } => {
            let organize = f.context().options.organize_imports.is_enabled();
            let sort_order = f.context().options.import_sort_order;

            // keyword
            write!(f, [Keyword::Import, space()])?;
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
                        let sorted_rest = if organize {
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
                    let sorted_items = if organize {
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
                let sorted_items = if organize {
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

            // Format declarators (comma-separated)
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
            pattern,
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
            write!(
                f,
                [
                    token("("),
                    pattern,
                    space(),
                    keyword,
                    space(),
                    iterator,
                    token(")"),
                    space()
                ]
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

        // type literal
        Expression::TypeLiteral(node) => node.format(f)?,

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
            if !format_hugged(f, elements_ids, HugOptions::ARRAY)? {
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
            } else if !format_hugged(f, elements_ids, HugOptions::TUPLE)? {
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
            TypeUnaryOperator::Maybe | TypeUnaryOperator::Must => {
                write!(f, [right, operator])?;
            }
            TypeUnaryOperator::Newtype
            | TypeUnaryOperator::Type
            | TypeUnaryOperator::Readonly
            | TypeUnaryOperator::Typeof
            | TypeUnaryOperator::Keyof
            | TypeUnaryOperator::Infer
            | TypeUnaryOperator::Asserts => {
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
            if let Some(mutability) = mutability {
                write!(f, [*mutability])?;
            }
            if let Some(variance) = variance {
                write!(f, [variance.to_keyword(), space()])?;
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
            write!(f, [left])?;
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
            // flatten binary expression chain for Prettier-style formatting
            // e.g. `a + b + c` formats as:
            //   a
            //       + b
            //       + c
            // all operands at the same indentation level
            let operands = flatten_binary_expression(f.context().tree, node_id, *operator);

            // format as a group with the first operand inline, rest indented
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
            // Check if the right side is a binary expression - if so, let it handle its own breaking
            let right_is_binary = matches!(f.context().tree.get(*right), Expression::Binary { .. });

            if right_is_binary {
                // Binary expressions handle their own indentation - keep on same line
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
            } else {
                // Other expressions get indented on break
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
    let pattern_breakable = is_pattern_breakable(tree, *pattern);
    let value_breakable = is_expression_breakable(tree, value_expr);

    // string literals are atomic - never break at `=`
    let is_string_literal = matches!(
        value_expr,
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
                fits_expanded(&group(value_id).should_expand(true)),
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

    // for string literals, never break at `=` - just let them exceed line width
    if is_string_literal {
        if pattern_breakable {
            best_fitting![format_inline, format_header_expanded]
                .with_mode(BestFittingMode::AllLines)
                .format(f)?;
        } else {
            write!(f, [format_inline])?;
        }
    } else {
        match (pattern_breakable, value_breakable) {
            (true, true) => {
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
                best_fitting![format_inline, format_header_expanded, format_indented]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
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
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

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
            "long\n\t.base\n\t.path\n\t.followed()\n\t.by()\n\t.many()\n\t.calls()",
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
        assert_format!(
            source,
            source,
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
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_array_of_struct_element() {
        // array attribute values hug: <Menu items={[...]} />
        let source = r#"<Menu
    items=[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]
/>"#;
        let expected = r#"<Menu items={[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]} />"#;
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
        let source = r#"const ast = await parseAsync(text, {
    sourceFileName: "file",
    parserOpts: {
        plugins: ["typescript", "jsx"],
    },
    sourceType: "module",
    configFile: false,
    babelrc: false,
})"#;
        assert_format!(
            source,
            source,
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
}
