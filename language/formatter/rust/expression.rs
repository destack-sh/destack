use dyst_ast::{
    Argument, Asynchrony, DependencyKind, IfStyle, Mutability, NodeTree, Path, PostfixPosition,
    TypeBinaryOperator, TypeUnaryOperator, YieldCardinality,
};
use dyst_container::SmallVec;
use dyst_fir::format::BestFittingMode;
use dyst_fir::prelude::*;
use dyst_fir::{best_fitting, format_args, write};

use crate::argument::list_like;
use crate::block::format_block;
use crate::import::format_dependency_binding;
use crate::r#let::FormatScopedMutability;
use crate::literal::{format_scalar_literal, format_template_literal};
use crate::{
    AssignOperator, BinaryOperator, DystFormatContext, DystFormatter, Expression, FormatNode,
    Keyword, NodeId, Runtime, UnaryOperator, empty_block_with_infix_annotations,
};

/// Tree fragment argument (with `=` instead of `: `)
#[derive(Debug, Clone, PartialEq)]
struct TreeLiteralArgument {
    argument_id: NodeId<Argument>,
}

impl<'ast> Format<DystFormatContext<'ast>> for TreeLiteralArgument {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(self.argument_id)])?;

        let argument = f.context().tree.get(self.argument_id);
        match argument {
            Argument::Named {
                modifiers: _,
                name,
                value,
            } => {
                // name
                write!(f, [name])?;
                // value
                write!(f, [token("="), value])?;
            }
            Argument::Function {
                modifiers: _,
                name,
                value,
            } => {
                // name
                write!(f, [name])?;
                // value
                write!(f, [token("="), value])?;
            }
            Argument::Shorthand { modifiers: _, name } => {
                // name
                write!(f, [name])?;
            }
            Argument::Positional {
                modifiers: _,
                value,
            } => {
                // value
                write!(f, [value])?;
            }
            Argument::Spread {
                modifiers: _,
                name: _,
                value,
            } => {
                // keyword
                write!(f, [token("...")])?;
                // value
                write!(f, [value])?;
            }
            Argument::Dynamic {
                modifiers: _,
                name,
                key,
                value,
            } => {
                // key
                write!(f, [token("[")])?;
                // name
                if let Some(name) = name {
                    write!(f, [name, token(":"), space()])?;
                }
                write!(f, [key, token("]")])?;
                // value
                write!(f, [token("="), value])?;
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
pub(crate) fn format_if_chain<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    node_id: NodeId<Expression>,
) -> FormatResult<()> {
    // walk the chain
    let mut next_if_id = node_id;
    loop {
        let if_node = f.context().tree.get(next_if_id);
        match if_node {
            // if or else if
            Expression::If {
                runtime,
                style: _, // we turn everything into regular ifs
                condition,
                then_expression: then_expression_id,
                else_expression: else_expression_id,
            } => {
                // runtime
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }

                // if <condition>
                write!(f, [Keyword::If, space(), condition, space()])?;

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

                // next node
                if let Some(else_expression) = else_expression_id {
                    if !f.context().has_prefix_annotation(*else_expression)
                        && !f.context().has_postfix_annotation(*then_expression_id)
                    {
                        write!(f, [space()])?;
                    }
                    write!(f, [Keyword::Else, space()])?;
                    match f.context().tree.get(*else_expression) {
                        // else if
                        Expression::If { .. } => {
                            next_if_id = *else_expression;
                        }
                        // else
                        Expression::Block(else_expression_id) => {
                            format_block(f, *else_expression_id)?;
                            break;
                        }
                        // something else
                        _ => {
                            write!(f, [*else_expression])?;
                            break;
                        }
                    }
                } else {
                    // bare if
                    break;
                }
            }
            // shouldn't be anything else
            _ => panic!("invalid if chain: {if_node:?}"),
        }
    }
    Ok(())
}

/// Format a match expression.
#[inline]
pub(crate) fn format_match<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    node_id: NodeId<Expression>,
    include_prefix: bool,
) -> FormatResult<()> {
    let match_node = f.context().tree.get(node_id);
    let Expression::Match {
        runtime,
        style: _,
        value,
        cases,
    } = &match_node
    else {
        panic!("invalid match expression: {match_node:?}");
    };

    if include_prefix {
        // runtime
        if let Some(runtime) = runtime
            && *runtime == Runtime::Static
        {
            write!(f, [token("@")])?;
        }

        // match <expression>
        write!(f, [Keyword::Match, space()])?;
    }

    write!(f, [value])?;

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

/// Whether an expression is "trivial" (prefers to be fully inline).
pub fn is_trivial_expression(tree: &NodeTree, expression: &Expression) -> bool {
    match expression {
        Expression::ScalarLiteral(_) | Expression::TypeLiteral(_) => true,
        Expression::StructLiteral { ty, fields, .. } => {
            ty.is_none()
                && fields.len() <= 5
                && fields
                    .iter()
                    .all(|field| is_trivial_argument(tree, tree.get(*field)))
        }
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
        Expression::StructLiteral { ty, fields, .. } => ty.is_some() || fields.len() > 1,
        Expression::TreeLiteral { .. } => true,
        _ => false,
    }
}

/// Whether an argument is "trivial" (prefers to be inline).
pub fn is_trivial_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. } => is_trivial_expression(tree, tree.get(*value)),
        Argument::Shorthand { name: _, .. } => true,
        Argument::Positional { value, .. } => is_trivial_expression(tree, tree.get(*value)),
        Argument::Spread { name: _, value, .. } => is_trivial_expression(tree, tree.get(*value)),
        _ => false,
    }
}

/// Whether an argument is "complex" (prefers to be multiline).
pub fn is_complex_argument(tree: &NodeTree, argument: &Argument) -> bool {
    match argument {
        Argument::Named { name: _, value, .. } => is_complex_expression(tree, tree.get(*value)),
        Argument::Shorthand { name: _, .. } => false,
        Argument::Positional { value, .. } => is_complex_expression(tree, tree.get(*value)),
        Argument::Spread { name: _, value, .. } => is_complex_expression(tree, tree.get(*value)),
        _ => false,
    }
}

/// Format a struct literal.
#[inline]
pub(crate) fn format_struct_literal<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    expression_id: NodeId<Expression>,
    ty: &Option<NodeId<Expression>>,
    fields_ids: &Vec<NodeId<Argument>>,
) -> FormatResult<()> {
    if let Some(ty) = ty {
        write!(f, [ty, space()])?;
    }

    let fields = fields_ids
        .iter()
        .map(|field| f.context().tree.get(*field))
        .collect::<SmallVec<_, 3>>();

    let is_trivial = fields.is_empty()
        || fields.len() <= 5
            && fields
                .iter()
                .all(|field| is_trivial_argument(f.context().tree, field));
    let has_annotations = f.context().has_infix_annotation(expression_id)
        || fields_ids
            .iter()
            .any(|field| f.context().has_annotation(*field));
    let keep_newline =
        f.context().has_newline(f.context().get_span(expression_id)) && fields.len() > 1;

    write!(
        f,
        [list_like("{", "}", ",", fields_ids)
            .include_space()
            .should_expand(!is_trivial || has_annotations || keep_newline)]
    )?;
    Ok(())
}

/// Format a tree literal.
#[inline]
pub(crate) fn format_tree_literal<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    expression_id: NodeId<Expression>,
    path: &Option<Path>,
    arguments: &Option<Vec<NodeId<Argument>>>,
    elements: &Option<Vec<NodeId<Argument>>>,
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
                    // path
                    if let Some(path) = path {
                        write!(f, [path])?;
                    }
                    // arguments
                    if let Some(arguments) = arguments {
                        write!(
                            f,
                            [
                                if_group_fits_on_line(&space()),
                                soft_block_indent(&format_with(|f| {
                                    f.join_with(&format_args![soft_line_break_or_space()])
                                        .entries(arguments.iter().map(|argument| {
                                            TreeLiteralArgument {
                                                argument_id: *argument,
                                            }
                                        }))
                                        .finish()
                                }))
                            ]
                        )?;
                    }
                    // /
                    if elements.is_none() {
                        if path.is_some() {
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
                let span = f.context().get_span(expression_id);
                let force_newline =
                    f.context().has_newline(span) || f.context().is_at_line_start(expression_id.id);

                // elements
                if !force_newline {
                    write!(
                        f,
                        [group(&format_args![soft_block_indent(&format_with(|f| {
                            f.join_with(soft_line_break()).entries(elements).finish()
                        }))])]
                    )?;
                } else {
                    write!(
                        f,
                        [group(&format_args![block_indent(&format_with(|f| {
                            f.join_with(hard_line_break()).entries(elements).finish()
                        }))])]
                    )?;
                }

                // closing tag
                write!(f, [token("</")])?;
                if let Some(path) = path {
                    write!(f, [path])?;
                }
                write!(f, [token(">")])?;
            }

            Ok(())
        }))]
    )
}

impl<'ast> FormatNode<'ast, Expression> for Expression {
    fn format_node(
        &self,
        node_id: NodeId<Expression>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        let tree = f.context().tree;

        match self {
            // definition
            Expression::Definition(node) => node.format(f)?,

            // block
            Expression::Block(node) => node.format(f)?,

            // with
            Expression::With { clauses, body } => {
                // keyword
                write!(f, [Keyword::With])?;
                if clauses.is_empty() {
                    return Ok(());
                }
                write!(f, [space()])?;

                // clauses
                write!(
                    f,
                    [best_fit_parenthesize(&format_with(|f| {
                        f.join_with(&format_args![&token(","), soft_line_break_or_space()])
                            .entries(clauses)
                            .finish()
                    }))]
                )?;

                // scoped body
                if let Some(body) = body {
                    write!(f, [space(), body])?;
                }
            }

            // import
            Expression::Import {
                kind,
                asynchrony,
                target,
                alias,
                items,
                arguments,
            } => {
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }
                write!(f, [Keyword::Import, space()])?;
                if *kind == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_dependency_binding(
                    f,
                    Some(target),
                    alias.as_ref().copied(),
                    items.as_ref(),
                )?;
                if let Some(arguments) = arguments {
                    write!(
                        f,
                        [
                            space(),
                            Keyword::With,
                            space(),
                            list_like("{", "}", ",", arguments).include_space()
                        ]
                    )?;
                }
            }

            // export
            Expression::Export {
                mode,
                kind: ty,
                target,
                alias,
                items,
            } => {
                write!(f, [mode, space()])?;
                if *ty == DependencyKind::Type {
                    write!(f, [Keyword::Type, space()])?;
                }
                format_dependency_binding(
                    f,
                    target.as_ref(),
                    alias.as_ref().copied(),
                    items.as_ref(),
                )?;
            }

            // let
            Expression::Let {
                mutability,
                meta,
                pattern,
                ty,
                value: value_id,
            } => {
                // header
                let header = format_with(|f| {
                    // export
                    if let Some(export) = meta.export {
                        write!(f, [export, space()])?;
                    }
                    // visibility
                    if let Some(visibility) = meta.visibility {
                        write!(f, [visibility, space()])?;
                    }
                    // keyword
                    if mutability.is_immutable() {
                        write!(f, [Keyword::Const])?;
                    } else {
                        write!(f, [Keyword::Let])?;
                    }
                    // pattern
                    write!(f, [space(), pattern])?;
                    // type
                    if let Some(ty) = ty {
                        write!(f, [token(":"), space(), ty])?;
                    }
                    Ok(())
                });

                let Some(value_expression_id) = value_id else {
                    write!(f, [header])?;
                    return Ok(());
                };

                // prefer keeping the value on a single line
                let format_inline = format_with(|f| {
                    write!(
                        f,
                        [header, space(), token("="), space(), *value_expression_id]
                    )?;
                    Ok(())
                });
                // expand inline if possible (like let x = [ ... ])
                let format_inline_expanded = format_with(|f| {
                    write!(
                        f,
                        [
                            header,
                            space(),
                            token("="),
                            space(),
                            fits_expanded(&group(value_expression_id).should_expand(true)),
                        ]
                    )
                });
                // if overall better fit, expand without indenting the value
                let format_indented = format_with(|f| {
                    group(&format_args![
                        header,
                        space(),
                        token("="),
                        block_indent(value_expression_id)
                    ])
                    .format(f)
                });

                best_fitting![format_inline, format_inline_expanded, format_indented]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
            }

            // type
            Expression::LetType {
                mutability,
                meta,
                static_parameters,
                value,
            } => {
                let header = format_with(|f| {
                    // export
                    if let Some(export) = meta.export {
                        write!(f, [export, space()])?;
                    }
                    // visibility
                    if let Some(visibility) = meta.visibility {
                        write!(f, [visibility, space()])?;
                    }
                    // keyword
                    if *mutability == Some(Mutability::Immutable) {
                        // for readonly type expression
                        write!(f, [Keyword::Readonly])?;
                    } else {
                        write!(f, [Keyword::Type])?;
                    }
                    // name
                    if let Some(name) = meta.name {
                        write!(f, [space(), name])?;
                    }
                    // static parameters
                    if let Some(static_parameters) = static_parameters {
                        write!(f, [list_like("<", ">", ",", static_parameters)])?;
                    }
                    Ok(())
                });

                let format_inline =
                    format_with(|f| write!(f, [header, space(), token("="), space(), value]));
                let format_indented = format_with(|f| {
                    group(&format_args![
                        header,
                        space(),
                        token("="),
                        block_indent(&value)
                    ])
                    .format(f)
                });

                best_fitting![format_inline, format_indented]
                    .with_mode(BestFittingMode::AllLines)
                    .format(f)?;
            }

            // if (ternary)
            Expression::If {
                style: IfStyle::Ternary,
                condition,
                then_expression,
                else_expression,
                ..
            } => {
                write!(
                    f,
                    [group(&format_args![
                        condition,
                        if_group_fits_on_line(&space()),
                        soft_block_indent(&format_args![
                            token("?"),
                            space(),
                            then_expression,
                            soft_line_break(),
                            if_group_fits_on_line(&space()),
                            token(":"),
                            space(),
                            else_expression
                        ]),
                    ])]
                )?;
            }

            // if (regular)
            Expression::If {
                style: IfStyle::Regular,
                ..
            } => {
                write!(f, [group(&format_with(|f| format_if_chain(f, node_id)))])?;
            }

            // while
            Expression::While {
                runtime,
                condition,
                body,
            } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
                write!(f, [Keyword::While, space(), condition, space(), body])?;
            }

            // for each
            Expression::ForEach {
                runtime,
                asynchrony,
                pattern,
                iterator,
                body,
            } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
                write!(f, [Keyword::For, space()])?;
                if *asynchrony == Asynchrony::Async {
                    write!(f, [Keyword::Await, space()])?;
                }
                if let Some(pattern) = pattern {
                    write!(f, [pattern, space(), Keyword::In, space()])?;
                }
                write!(f, [iterator, space()])?;
                write!(f, [body])?;
            }

            // for condition
            Expression::ForCondition {
                runtime,
                initialization,
                condition,
                increment,
                body,
            } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
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
            Expression::Loop { runtime, body } => {
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }
                write!(f, [Keyword::Loop, space(), body])?;
            }

            // try
            Expression::Try {
                runtime,
                try_expression,
                catch_pattern,
                catch_expression,
                finally_expression,
            } => {
                // runtime
                if let Some(runtime) = runtime
                    && *runtime == Runtime::Static
                {
                    write!(f, [token("@")])?;
                }

                // try <expression>
                write!(f, [Keyword::Try, space(), try_expression])?;

                // catch <expression>
                if let Some(catch) = catch_expression {
                    write!(f, [space(), Keyword::Catch, space()])?;
                    if let Some(catch_pattern) = catch_pattern {
                        write!(f, [catch_pattern, space()])?;
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

            // defer
            Expression::Defer { expression } => {
                write!(f, [Keyword::Defer])?;
                if let Some(expression) = expression {
                    write!(f, [space(), expression])?;
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
                write!(f, [space(), value])?;
            }

            // throw
            Expression::Throw { value } => {
                write!(f, [token("throw")])?;
                if let Some(value) = value {
                    write!(f, [space(), value])?;
                }
            }

            // return
            Expression::Return { value } => {
                write!(f, [token("return")])?;
                if let Some(value) = value {
                    write!(f, [space(), value])?;
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
            Expression::TemplateLiteral(node) => {
                format_template_literal(node, tree.get_span(node_id), f)?;
            }

            // type literal
            Expression::TypeLiteral(node) => node.format(f)?,

            // range literal
            Expression::RangeLiteral {
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
            Expression::ArrayLiteral {
                elements: elements_ids,
            } => {
                let span = f.context().get_span(node_id);
                let elements = elements_ids
                    .iter()
                    .map(|id| tree.get(*id))
                    .collect::<SmallVec<_, 3>>();
                let should_expand = elements.len() > 1
                    && elements
                        .iter()
                        .any(|element| is_complex_argument(tree, element))
                    || f.context().has_newline(span) && elements.len() > 1;
                write!(
                    f,
                    [list_like("[", "]", ",", elements_ids).should_expand(should_expand)]
                )?;
            }

            // tuple literal
            Expression::TupleLiteral {
                elements: elements_ids,
            } => {
                if elements_ids.is_empty() {
                    write!(f, [token("()")])?;
                } else {
                    let span = f.context().get_span(node_id);
                    let elements = elements_ids
                        .iter()
                        .map(|id| tree.get(*id))
                        .collect::<SmallVec<_, 3>>();
                    let should_expand = elements.len() > 1
                        && elements
                            .iter()
                            .any(|element| is_complex_argument(tree, element))
                        || f.context().has_newline(span) && elements.len() > 1;
                    write!(
                        f,
                        [list_like("(", ")", ",", elements_ids)
                            .force_trailing_separator()
                            .should_expand(should_expand)]
                    )?;
                }
            }

            // struct literal
            Expression::StructLiteral { ty, fields } => {
                format_struct_literal(f, node_id, ty, fields)?;
            }

            // tree literal
            Expression::TreeLiteral {
                path,
                arguments,
                elements,
            } => {
                format_tree_literal(f, node_id, path, arguments, elements)?;
            }

            // parenthesized
            Expression::Parenthesized { expression } => {
                let inner_expression = tree.get(*expression);
                if let Expression::TreeLiteral { .. } = inner_expression {
                    write!(f, [token("("), soft_block_indent(&expression), token(")")])?;
                } else {
                    write!(f, [token("("), expression, token(")")])?;
                }
            }

            // unary
            Expression::Unary {
                operator,
                expression,
            } => {
                if operator.is_prefix() {
                    write!(f, [operator, expression])?;
                } else {
                    write!(f, [expression, operator])?;
                }
            }

            // type unary
            Expression::TypeUnary { operator, right } => {
                write!(f, [operator, space(), right])?;
            }

            // reference
            Expression::Reference {
                mutability,
                variance,
                right,
            } => {
                write!(f, [token("&")])?;
                if let Some(mutability) = mutability {
                    write!(
                        f,
                        [FormatScopedMutability::implicit_const(mutability.clone())]
                    )?;
                }
                if let Some(variance) = variance {
                    write!(f, [variance.to_keyword(), space()])?;
                }
                right.format(f)?;
            }

            // member
            Expression::Member {
                left: receiver,
                path,
            } => write!(f, [receiver, token("."), path])?,

            // index
            Expression::Index {
                position,
                left: receiver,
                index,
            } => {
                write!(f, [receiver])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                if let Some(index) = index {
                    write!(f, [token("["), index, token("]")])?;
                } else {
                    write!(f, [token("[]")])?;
                }
            }

            // call
            Expression::Call {
                position,
                runtime,
                left: receiver,
                dynamic_arguments,
            } => {
                let runtime = runtime.unwrap_or(Runtime::Dynamic);
                if runtime == Runtime::Static {
                    write!(f, [token("@")])?;
                }
                write!(f, [receiver])?;
                if *position == PostfixPosition::Indirect {
                    write!(f, [token(".")])?;
                }
                if runtime == Runtime::Dynamic || !dynamic_arguments.is_empty() {
                    write!(f, [list_like("(", ")", ",", dynamic_arguments)])?;
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
            Expression::Maybe { left, position } => {
                left.format(f)?;
                match position {
                    PostfixPosition::Direct => write!(f, [token("?")])?,
                    PostfixPosition::Indirect => write!(f, [token("."), token("?")])?,
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
                left,
                operator,
                right,
            } => {
                write!(f, [left])?;
                if !f.context().has_postfix_annotation(*left) {
                    write!(f, [space()])?;
                }
                write!(f, [operator, space(), right])?;
            }

            // type binary
            Expression::TypeBinary {
                left,
                operator,
                right,
            } => {
                write!(f, [left])?;
                if !f.context().has_postfix_annotation(*left) {
                    write!(f, [space()])?;
                }
                write!(f, [operator, space(), right])?;
            }

            // assign
            Expression::Assign {
                left,
                operator,
                right,
            } => {
                write!(f, [left])?;
                if !f.context().has_postfix_annotation(*left) {
                    write!(f, [space()])?;
                }
                write!(f, [operator, space(), right])?;
            }

            // error
            Expression::Error => panic!("invalid expression: {self:?}"),
        };

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for UnaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            UnaryOperator::PostIncrement => token("++"),
            UnaryOperator::PostDecrement => token("--"),
            UnaryOperator::PreIncrement => token("++"),
            UnaryOperator::PreDecrement => token("--"),
            UnaryOperator::Not => token("!"),
            UnaryOperator::Negate => token("-"),
            UnaryOperator::WrappingNegate => token("-%"),
            UnaryOperator::ElementwiseNot => token("~"),
            UnaryOperator::Dereference => token("*"),
            UnaryOperator::Spread => token("..."),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for TypeUnaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            TypeUnaryOperator::Type => token("type"),
            TypeUnaryOperator::Readonly => token("readonly"),
            TypeUnaryOperator::Typeof => token("typeof"),
            TypeUnaryOperator::Keyof => token("keyof"),
            TypeUnaryOperator::Infer => token("infer"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for BinaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            // multiplication
            BinaryOperator::Multiply => token("*"),
            BinaryOperator::WrappingMultiply => token("*%"),
            BinaryOperator::SaturatingMultiply => token("*|"),
            BinaryOperator::Divide => token("/"),
            BinaryOperator::Remainder => token("%"),

            // addition
            BinaryOperator::Add => token("+"),
            BinaryOperator::WrappingAdd => token("+%"),
            BinaryOperator::SaturatingAdd => token("+|"),
            BinaryOperator::Subtract => token("-"),
            BinaryOperator::WrappingSubtract => token("-%"),
            BinaryOperator::SaturatingSubtract => token("-|"),

            // shift
            BinaryOperator::ShiftLeft => token("<<"),
            BinaryOperator::SaturatingShiftLeft => token("<<|"),
            BinaryOperator::ShiftRight => token(">>"),

            // elementwise
            BinaryOperator::ElementwiseAnd => token("&"),
            BinaryOperator::ElementwiseXor => token("^"),
            BinaryOperator::ElementwiseOr => token("|"),

            // comparison
            BinaryOperator::Equal => token("=="),
            BinaryOperator::NotEqual => token("!="),
            BinaryOperator::EqualStrict => token("==="),
            BinaryOperator::NotEqualStrict => token("!=="),
            BinaryOperator::LessThan => token("<"),
            BinaryOperator::LessThanOrEqual => token("<="),
            BinaryOperator::GreaterThan => token(">"),
            BinaryOperator::GreaterThanOrEqual => token(">="),

            // boolean
            BinaryOperator::And => token("&&"),
            BinaryOperator::Or => token("||"),
            BinaryOperator::Coalesce => token("??"),

            // container
            BinaryOperator::In => token("in"),
            BinaryOperator::Of => token("of"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for TypeBinaryOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = match self {
            TypeBinaryOperator::Cast => token("as"),
            TypeBinaryOperator::Is => token("is"),
            TypeBinaryOperator::Instanceof => token("instanceof"),
            TypeBinaryOperator::Satisfies => token("satisfies"),
            TypeBinaryOperator::Extends => token("extends"),
            TypeBinaryOperator::Implements => token("implements"),
        };
        write!(f, [token])
    }
}

impl<'ast> Format<DystFormatContext<'ast>> for AssignOperator {
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        let token = token(match self {
            AssignOperator::Assign => "=",

            // addition
            AssignOperator::AddAssign => "+=",
            AssignOperator::WrappingAddAssign => "+%=",
            AssignOperator::SaturatingAddAssign => "+|=",
            AssignOperator::SubtractAssign => "-=",
            AssignOperator::WrappingSubtractAssign => "-%=",
            AssignOperator::SaturatingSubtractAssign => "-|=",

            // multiplication
            AssignOperator::MultiplyAssign => "*=",
            AssignOperator::WrappingMultiplyAssign => "*%=",
            AssignOperator::SaturatingMultiplyAssign => "*|=",
            AssignOperator::DivideAssign => "/=",
            AssignOperator::RemainderAssign => "%=",

            // shift
            AssignOperator::ShiftLeftAssign => "<<=",
            AssignOperator::SaturatingShiftLeftAssign => "<<|=",
            AssignOperator::ShiftRightAssign => ">>=",

            // elementwise
            AssignOperator::ElementwiseAndAssign => "&=",
            AssignOperator::ElementwiseOrAssign => "|=",
            AssignOperator::ElementwiseXorAssign => "^=",

            // boolean
            AssignOperator::AndAssign => "&&=",
            AssignOperator::OrAssign => "||=",
            AssignOperator::CoalesceAssign => "??=",
        });
        write!(f, [token])
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    /// Simple expressions should stay on one line.
    #[test]
    fn test_format_expression_simple() {
        assert_format!(
            "1 + 2 * 3 - a / b % c",
            "1 + 2 * 3 - a / b % c",
            |p| p.eat_expression(),
            DystFormatOptions::default_tab()
        );
    }

    /// Parenthesized expressions should retain their parentheses.
    #[test]
    fn test_format_expression_parenthesized() {
        assert_format!(
            "(((1 + 2) * 3) - a / (b % c))",
            "(((1 + 2) * 3) - a / (b % c))",
            |p| p.eat_expression(),
            DystFormatOptions::default_tab()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_parenthesis() {
        assert_format!(
            "(((())))",
            "(((())))",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    /// Empty parenthesis/arguments/tuplestuples should be respected.
    #[test]
    fn test_format_expression_nested_empty_arguments() {
        assert_format!(
            "foo<()>(((())))",
            "foo<()>(((())))",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_struct_literal_trivial() {
        assert_format!(
            "{ a: 1, ..B }",
            "{ a: 1, ...B }",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_struct_literal_spread() {
        assert_format!(
            "Foo { ..B }",
            "Foo { ...B }",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_if_ternary() {
        assert_format!(
            "true ? 1 : 2",
            "true ? 1 : 2",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_index_call_mixed_postfix() {
        assert_format!(
            "x?.[f]?.[2]?.(a, b)",
            "x?.[f]?.[2]?.(a, b)",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_without_arguments() {
        assert_format!(
            "<Entity/>",
            "<Entity />",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_arguments() {
        assert_format!(
            "<Entity a=1, b = 2 />",
            "<Entity a=1 b=2 />",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_parenthesized() {
        let source = r"(
    <Entity a=1 b=2>
        <Entity a=1 b=2 />
    </Entity>
)";
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_nested() {
        let source = r#"<A x=4 y=4>
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
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_expression_tree_literal_with_array_of_struct_element() {
        let source = r#"<Menu
    items=[
        { to: "/posts" },
        { to: "/posts/$postId", params: { postId: "postId" } },
    ]
/>"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DystFormatOptions::default()
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
            DystFormatOptions::default_with_line_width(40)
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
            DystFormatOptions::default_with_line_width(40)
        );
    }
}
