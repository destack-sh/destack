use super::function::{
    FormatContentWithCacheMode, function_can_omit_lambda_parameter_parentheses,
    function_parameter_container_span, function_parameters, write_cached_function_return_type,
    write_function_ambient_prefix, write_function_export_prefix, write_function_generic_parameters,
};
use crate::annotation::{
    FormatTrailingComments, block_infix_annotations, format_leading_comments, postfix_annotations,
};
use crate::chain::{is_lambda_expression, transparent_inner_expression};
use crate::declaration::signature::{
    expression_body_requires_head_space, format_where_clause_with_break,
    write_function_header_prefix, write_grouped_parameters_with_return_type,
};
use crate::declaration::statement::format_block;
use crate::expression::ExpressionLeftSide;
use crate::operator::AssignmentLikeLayout;
use crate::{DestackFormatContext, DestackFormatter};
use destack_dir::{
    Argument, Declaration, ExportKind, Expression, FunctionDeclaration, FunctionForm,
    FunctionSignature, IfForm, LocalNodeId, Name, NodeType, Parameter, TemplateLiteral,
};
use destack_fir::format::{FormatResult, RemoveSoftLinesBuffer};
use destack_fir::prelude::*;
use destack_fir::{format_args, write};
use destack_repository::TrailingComma;
use destack_source::Span;

/// The grouped call-argument layout shared with lambda formatting.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GroupedCallArgumentLayout {
    /// Group the first call argument.
    GroupedFirstArgument,
    /// Group the last call argument.
    GroupedLastArgument,
}

impl GroupedCallArgumentLayout {
    /// Return whether this layout groups the first call argument.
    pub(crate) fn is_grouped_first(self) -> bool {
        matches!(self, Self::GroupedFirstArgument)
    }

    /// Return whether this layout groups the last call argument.
    pub(crate) fn is_grouped_last(self) -> bool {
        matches!(self, Self::GroupedLastArgument)
    }
}

/// The caching mode for function signature and body content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FunctionCacheMode {
    /// Format without caching the content.
    #[default]
    NoCache,
    /// Cache the content on the next format.
    Cache,
}

/// Options for formatting one lambda declaration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct FormatLambdaDeclarationOptions {
    /// The assignment-like layout, when this lambda sits on one assignment rhs.
    pub assignment_layout: Option<AssignmentLikeLayout>,
    /// The grouped call-argument layout, when this lambda is being formatted as one.
    pub call_argument_layout: Option<GroupedCallArgumentLayout>,
    /// The signature and body cache mode for grouped call formatting.
    pub cache_mode: FunctionCacheMode,
}

/// The cached lambda body wrapper keyed by the body container span.
#[derive(Clone, Copy, Debug)]
struct FormatMaybeCachedLambdaBody {
    body_id: LocalNodeId<Expression>,
    cache_mode: FunctionCacheMode,
}

impl<'ast> Format<DestackFormatContext<'ast>> for FormatMaybeCachedLambdaBody {
    fn format(&self, f: &mut DestackFormatter<'ast, '_>) -> FormatResult<()> {
        let body_id = self.body_id;
        let body_span = f.context().span(body_id);

        // body content
        let content = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
            let body_expression = f.context().tree.get(body_id);

            // block body
            if let Expression::Block(block_id) = body_expression {
                return format_block(f, *block_id);
            }

            write!(f, [body_id])
        });

        FormatContentWithCacheMode::new(body_span, content, self.cache_mode).format(f)
    }
}

/// Return whether one lambda declaration appears in statement context.
fn lambda_declaration_is_statement_context(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> bool {
    let Some((declaration_expression_id, parent_type)) = context.parent(node_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }
    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_id);

    let Some((_, parent_type)) = context.parent(declaration_expression_id) else {
        return false;
    };

    parent_type == NodeType::Block
}

/// Write one lambda arrow token with separator comments and local infix spacing.
pub(crate) fn write_lambda_arrow_with_infix_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    _cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    // arrow token
    write!(f, [block_infix_annotations(f.context(), node_id)])?;
    if !f.context().has_infix_annotation(node_id) {
        write!(f, [space()])?;
    }

    write!(f, [token("=>")])
}
/// Write one lambda parameter list and return type.
fn write_lambda_parameters_and_return_type<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    parameters: &[LocalNodeId<Parameter>],
    can_omit_parens: bool,
    cache_mode: FunctionCacheMode,
) -> FormatResult<()> {
    let format_parameters = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        super::function::write_function_parameters(
            f,
            node_id,
            signature,
            parameters,
            can_omit_parens,
        )
    });
    let format_parameters = FormatContentWithCacheMode::new(
        function_parameter_container_span(f.context(), node_id),
        format_parameters,
        cache_mode,
    );
    let format_return_type = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_cached_function_return_type(f, node_id, signature, body, parameters, cache_mode)
    });
    let format_parameter_head = format_with(|_f: &mut DestackFormatter<'ast, '_>| Ok(()));

    write_grouped_parameters_with_return_type(
        f,
        &signature.generic_parameters,
        parameters.len(),
        signature.return_type,
        format_parameter_head,
        format_parameters,
        format_return_type,
        false,
        false,
    )
}

/// Return whether one lambda declaration needs a trailing semicolon.
fn lambda_declaration_needs_trailing_semicolon(
    context: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportKind>,
) -> bool {
    export.is_some() || lambda_declaration_is_statement_context(context, node_id)
}

/// Return whether one expression is a multiline template that starts on the same line.
fn expression_is_multiline_template_starting_on_same_line(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);
    let expression_span = context.span(expression_id);

    if !matches!(
        expression,
        Expression::TemplateExpression { .. } | Expression::TaggedTemplateExpression { .. }
    ) {
        return false;
    }

    context.source_text().contains_newline(expression_span)
        && !context
            .source_text()
            .has_newline_before(expression_span.start)
}

/// Return the lambda declaration for one declaration id.
fn lambda_declaration<'ast>(
    context: &'ast DestackFormatContext<'_>,
    node_id: LocalNodeId<Declaration>,
) -> &'ast FunctionDeclaration {
    let Declaration::Function(function) = context.tree.get(node_id) else {
        unreachable!();
    };

    debug_assert_eq!(function.signature.form, FunctionForm::Lambda);
    function
}

/// Return whether one parameter is simple enough for inline arrow chains.
fn lambda_parameter_is_simple(
    context: &DestackFormatContext<'_>,
    parameter_id: LocalNodeId<Parameter>,
    allow_type_annotations: bool,
) -> bool {
    match context.tree.get(parameter_id) {
        Parameter::Named {
            declared_type,
            default,
            ..
        } => default.is_none() && (allow_type_annotations || declared_type.is_none()),
        _ => false,
    }
}

/// Return whether one lambda has only simple parameters.
fn lambda_has_only_simple_parameters(
    context: &DestackFormatContext<'_>,
    signature: &FunctionSignature,
    allow_type_annotations: bool,
) -> bool {
    signature.parameters.iter().copied().all(|parameter_id| {
        lambda_parameter_is_simple(context, parameter_id, allow_type_annotations)
    })
}

/// Return whether one lambda chain should break at its signatures.
fn lambda_chain_should_break(
    context: &DestackFormatContext<'_>,
    signature: &FunctionSignature,
) -> bool {
    if !signature.generic_parameters.is_empty() {
        return true;
    }

    if !lambda_has_only_simple_parameters(context, signature, true) {
        return true;
    }

    !signature.parameters.is_empty() && signature.return_type.is_some()
}

/// Return the next lambda declaration in one chain body.
fn next_lambda_chain_declaration(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
    options: FormatLambdaDeclarationOptions,
) -> Option<LocalNodeId<Declaration>> {
    if options
        .call_argument_layout
        .is_some_and(GroupedCallArgumentLayout::is_grouped_first)
    {
        return None;
    }

    let function = lambda_declaration(context, declaration_id);
    let body_id = function.body?;
    let body_expression_id = transparent_inner_expression(context, body_id);
    let Expression::Declaration(next_id) = context.tree.get(body_expression_id) else {
        return None;
    };

    match context.tree.get(*next_id) {
        Declaration::Function(function) if function.signature.form == FunctionForm::Lambda => {
            Some(*next_id)
        }
        _ => None,
    }
}

/// Return whether one lambda declaration sits in call-like callee position.
fn lambda_declaration_is_call_like_callee(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Some((declaration_expression_id, parent_type)) = context.parent(declaration_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_id);
    let Some((parent_id, parent_type)) = context.parent(declaration_expression_id) else {
        return false;
    };
    if parent_type != NodeType::Expression {
        return false;
    }

    let parent_id = LocalNodeId::<Expression>::new(parent_id);
    match context.tree.get(parent_id) {
        Expression::Call { left, .. } => *left == declaration_expression_id,
        Expression::TaggedTemplateExpression { tag, .. } => *tag == declaration_expression_id,
        _ => false,
    }
}

/// Return the tree argument that contains one lambda declaration.
fn lambda_declaration_tree_argument_id(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> Option<LocalNodeId<Argument>> {
    let (declaration_expression_id, parent_type) = context.parent(declaration_id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let declaration_expression_id = LocalNodeId::<Expression>::new(declaration_expression_id);
    let (argument_id, parent_type) = context.parent(declaration_expression_id)?;
    if parent_type != NodeType::Argument {
        return None;
    }

    let argument_id = LocalNodeId::<Argument>::new(argument_id);

    let (tree_expression_id, parent_type) = context.parent_by_id(argument_id.id)?;
    if parent_type != NodeType::Expression {
        return None;
    }

    let tree_expression_id = LocalNodeId::<Expression>::new(tree_expression_id);
    matches!(
        context.tree.get(tree_expression_id),
        Expression::TreeExpression { .. }
    )
    .then_some(argument_id)
}

/// Return whether one lambda declaration sits inside a tree expression container.
fn lambda_declaration_is_tree_argument(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    lambda_declaration_tree_argument_id(context, declaration_id).is_some()
}

/// Return whether one tree argument lambda should align its closing brace with a soft line.
fn lambda_declaration_tree_argument_should_add_soft_line(
    context: &DestackFormatContext<'_>,
    declaration_id: LocalNodeId<Declaration>,
) -> bool {
    let Some(argument_id) = lambda_declaration_tree_argument_id(context, declaration_id) else {
        return false;
    };

    let declaration_end = context.span(declaration_id).end;
    let argument_end = context.span(argument_id).end;

    !context
        .comments()
        .has_comment_in_range(declaration_end, argument_end)
}

/// Return whether one lambda body has one own-line comment after the arrow.
fn lambda_body_has_leading_own_line_comment(
    context: &DestackFormatContext<'_>,
    body_id: LocalNodeId<Expression>,
) -> bool {
    let body_span = context.span(body_id);

    context
        .comments()
        .has_leading_own_line_comment(body_span.start)
}

/// Return whether one lambda body should keep its own break strategy.
fn lambda_body_has_soft_line_break(
    context: &DestackFormatContext<'_>,
    body_id: LocalNodeId<Expression>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    let body_expression = context.tree.get(body_expression_id);

    match body_expression {
        Expression::ArrayExpression { .. }
        | Expression::ObjectExpression { .. }
        | Expression::StructExpression { .. } => {
            !lambda_body_has_leading_own_line_comment(context, body_id)
        }
        Expression::TreeExpression { .. } => true,
        Expression::Declaration(_) => {
            is_lambda_expression(context, body_expression_id)
                && !lambda_body_has_leading_own_line_comment(context, body_id)
        }
        Expression::TemplateExpression {
            value: TemplateLiteral::InterpolatedString { .. },
        }
        | Expression::TaggedTemplateExpression {
            value: TemplateLiteral::InterpolatedString { .. },
            ..
        } => expression_is_multiline_template_starting_on_same_line(context, body_expression_id),
        _ => false,
    }
}

/// Return whether one lambda body needs parentheses in flat mode.
fn lambda_body_needs_parentheses(
    context: &DestackFormatContext<'_>,
    body_expression_id: LocalNodeId<Expression>,
) -> bool {
    let body_expression = context.tree.get(body_expression_id);

    if !matches!(
        body_expression,
        Expression::If {
            form: IfForm::Ternary,
            ..
        }
    ) {
        return false;
    }

    let mut leftmost = ExpressionLeftSide::new(body_expression_id);
    while let Some(next_left) = leftmost.left(context) {
        leftmost = next_left;
    }

    let leftmost = leftmost.expression_id();

    !matches!(
        context.tree.get(leftmost),
        Expression::ObjectExpression { .. } | Expression::StructExpression { .. }
    )
}

/// Return whether one lambda chain tail body should break onto its own line.
fn lambda_chain_tail_body_is_separate_line(
    context: &DestackFormatContext<'_>,
    tail_id: LocalNodeId<Declaration>,
) -> bool {
    let function = lambda_declaration(context, tail_id);
    let Some(body_id) = function.body else {
        return false;
    };

    let body_expression_id = transparent_inner_expression(context, body_id);
    let body_expression = context.tree.get(body_expression_id);

    !matches!(
        body_expression,
        Expression::Block(_)
            | Expression::ObjectExpression { .. }
            | Expression::StructExpression { .. }
            | Expression::ArrayExpression { .. }
            | Expression::SequenceExpression { .. }
            | Expression::TreeExpression { .. }
    )
}

/// One arrow-layout decision for a lambda declaration.
enum LambdaLayout {
    /// One standalone lambda declaration.
    Single(LocalNodeId<Declaration>),
    /// One chain of nested lambda declarations.
    Chain(LambdaChain),
}

impl LambdaLayout {
    /// Return the layout for one lambda declaration.
    fn for_declaration(
        context: &DestackFormatContext<'_>,
        declaration_id: LocalNodeId<Declaration>,
        options: FormatLambdaDeclarationOptions,
    ) -> Self {
        let mut head = None;
        let mut middle = Vec::new();
        let mut current = declaration_id;
        let mut expand_signatures = false;

        while let Some(next_id) = next_lambda_chain_declaration(context, current, options) {
            let function = lambda_declaration(context, current);
            let next_function = lambda_declaration(context, next_id);

            expand_signatures |= lambda_chain_should_break(context, &function.signature);
            expand_signatures |= lambda_chain_should_break(context, &next_function.signature);

            if head.is_none() {
                head = Some(current);
            } else {
                middle.push(current);
            }

            current = next_id;
        }

        match head {
            Some(head) => Self::Chain(LambdaChain {
                head,
                middle,
                tail: current,
                expand_signatures,
                options,
            }),
            None => Self::Single(declaration_id),
        }
    }
}

/// One chain of nested lambda declarations.
struct LambdaChain {
    head: LocalNodeId<Declaration>,
    middle: Vec<LocalNodeId<Declaration>>,
    tail: LocalNodeId<Declaration>,
    expand_signatures: bool,
    options: FormatLambdaDeclarationOptions,
}

impl LambdaChain {
    /// Iterate over every declaration in this chain.
    fn declarations(&self) -> impl Iterator<Item = LocalNodeId<Declaration>> + '_ {
        std::iter::once(self.head)
            .chain(self.middle.iter().copied())
            .chain(std::iter::once(self.tail))
    }
}

/// Write one lambda declaration body as one standalone arrow expression.
fn write_single_lambda_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    declaration_id: LocalNodeId<Declaration>,
    options: FormatLambdaDeclarationOptions,
) -> FormatResult<()> {
    let function = lambda_declaration(f.context(), declaration_id);
    let signature = function.signature.clone();
    let body = function.body;

    let formatted_signature = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_lambda_head(f, declaration_id, &signature, &body, options, true)?;
        write_lambda_arrow_with_infix_annotations(f, declaration_id, options.cache_mode)
    });

    let Some(body_id) = body else {
        return write!(f, [formatted_signature]);
    };

    let body_expression_id = transparent_inner_expression(f.context(), body_id);
    let body_expression = f.context().tree.get(body_expression_id);
    let format_body = FormatMaybeCachedLambdaBody {
        body_id,
        cache_mode: options.cache_mode,
    };

    // block bodies
    if matches!(body_expression, Expression::Block(_)) {
        write!(f, [formatted_signature])?;

        if expression_body_requires_head_space(f.context(), body_id) {
            write!(f, [space()])?;
        }

        return write!(f, [format_body]);
    }

    write!(f, [formatted_signature])?;

    // self-breaking bodies
    if lambda_body_has_soft_line_break(f.context(), body_id, body_expression_id) {
        return write!(f, [space(), format_body]);
    }

    // flat bodies
    let should_add_parens = lambda_body_needs_parentheses(f.context(), body_expression_id);
    let is_last_call_argument = options
        .call_argument_layout
        .is_some_and(GroupedCallArgumentLayout::is_grouped_last);
    let should_add_trailing_separator =
        is_last_call_argument && matches!(f.context().options.trailing_comma, TrailingComma::All);
    let should_add_soft_line = is_last_call_argument
        || lambda_declaration_tree_argument_should_add_soft_line(f.context(), declaration_id);

    write!(
        f,
        [group(&format_args![
            soft_line_indent_or_space(&format_with(|f| {
                if should_add_parens {
                    write!(f, [if_group_fits_on_line(&token("("))])?;
                }

                write!(f, [format_body])?;

                if should_add_parens {
                    write!(f, [if_group_fits_on_line(&token(")"))])?;
                }

                Ok(())
            })),
            should_add_trailing_separator.then_some(if_group_breaks(&token(","))),
            should_add_soft_line.then_some(soft_line_break())
        ])]
    )
}

/// Write one chain of nested lambda declarations.
fn write_lambda_chain_layout<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    chain: &LambdaChain,
) -> FormatResult<()> {
    let tail_function = lambda_declaration(f.context(), chain.tail);
    let tail_body = tail_function.body;
    let is_grouped_call_argument = chain.options.call_argument_layout.is_some();
    let is_callee = lambda_declaration_is_call_like_callee(f.context(), chain.head);
    let is_tree_argument = lambda_declaration_is_tree_argument(f.context(), chain.head);
    let body_on_separate_line = lambda_chain_tail_body_is_separate_line(f.context(), chain.tail);
    let break_signatures = (is_callee && body_on_separate_line)
        || matches!(
            chain.options.assignment_layout,
            Some(AssignmentLikeLayout::ChainTailArrowFunction)
        );
    let has_initial_indent = is_callee
        || chain
            .options
            .assignment_layout
            .is_some_and(|layout| layout != AssignmentLikeLayout::BreakAfterOperator);
    let group_id = f.group_id("lambda-chain");

    let format_signatures = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let join_signatures = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            let mut is_first = true;

            for declaration_id in chain.declarations() {
                let function = lambda_declaration(f.context(), declaration_id);
                let signature = function.signature.clone();
                let body = function.body;
                let formatted_signature = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                    let declaration_start = f.context().span(declaration_id).start;
                    let declaration_comment_key =
                        Span::new(f.context().file.id, declaration_start, declaration_start);
                    let leading_comments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if !is_first
                            && f.context()
                                .comments()
                                .has_comment_before(f.context().span(declaration_id).start)
                        {
                            if is_grouped_call_argument {
                                write!(
                                    f,
                                    [
                                        space(),
                                        format_leading_comments(f.context().span(declaration_id))
                                    ]
                                )?;
                            } else {
                                write!(
                                    f,
                                    [
                                        soft_line_break_or_space(),
                                        format_leading_comments(f.context().span(declaration_id))
                                    ]
                                )?;
                            }
                        }

                        Ok(())
                    });

                    write!(
                        f,
                        [FormatContentWithCacheMode::new(
                            declaration_comment_key,
                            leading_comments,
                            chain.options.cache_mode,
                        )]
                    )?;

                    write_lambda_head(
                        f,
                        declaration_id,
                        &signature,
                        &body,
                        chain.options,
                        is_first,
                    )
                });

                if is_first || has_initial_indent {
                    write!(f, [formatted_signature])?;
                } else {
                    write!(f, [indent(&formatted_signature)])?;
                }

                if declaration_id != chain.tail {
                    write_lambda_arrow_with_infix_annotations(
                        f,
                        declaration_id,
                        chain.options.cache_mode,
                    )?;
                }

                is_first = false;
            }

            Ok(())
        });

        write!(
            f,
            [group(&join_signatures).should_expand(chain.expand_signatures)]
        )
    });

    let format_tail_body_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let Some(body_id) = tail_body else {
            return Ok(());
        };

        let body_expression_id = transparent_inner_expression(f.context(), body_id);
        let should_add_parens = lambda_body_needs_parentheses(f.context(), body_expression_id);
        let format_body = FormatMaybeCachedLambdaBody {
            body_id,
            cache_mode: chain.options.cache_mode,
        };

        if should_add_parens {
            write!(
                f,
                [
                    if_group_fits_on_line(&token("(")),
                    format_body,
                    if_group_fits_on_line(&token(")"))
                ]
            )
        } else {
            write!(f, [format_body])
        }
    });

    let format_tail_body = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        let should_add_soft_line = is_tree_argument;

        if body_on_separate_line {
            write!(
                f,
                [
                    soft_line_indent_or_space(&format_tail_body_inner),
                    should_add_soft_line.then_some(soft_line_break())
                ]
            )?;
        } else {
            write!(f, [space(), format_tail_body_inner])?;
        }

        Ok(())
    });

    let format_inner = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if has_initial_indent {
            write!(
                f,
                [
                    group(&indent(&format_args![soft_line_break(), format_signatures]))
                        .with_id(Some(group_id))
                        .should_expand(break_signatures)
                ]
            )?;
        } else {
            write!(
                f,
                [group(&format_signatures)
                    .with_id(Some(group_id))
                    .should_expand(break_signatures)]
            )?;
        }

        write_lambda_arrow_with_infix_annotations(f, chain.tail, chain.options.cache_mode)?;

        if is_grouped_call_argument {
            write!(f, [group(&format_tail_body)])?;
        } else {
            write!(f, [indent_if_group_breaks(&format_tail_body, group_id)])?;
        }

        if is_callee {
            write!(
                f,
                [if_group_breaks(&soft_line_break()).with_group_id(Some(group_id))]
            )?;
        }

        Ok(())
    });

    write!(f, [group(&format_inner)])
}

/// Write one lambda body and trailing semicolon.
fn write_lambda_body_and_terminator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportKind>,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatLambdaDeclarationOptions,
) -> FormatResult<()> {
    // body
    if body.is_some() {
        match LambdaLayout::for_declaration(f.context(), node_id, options) {
            LambdaLayout::Single(declaration_id) => {
                write_single_lambda_layout(f, declaration_id, options)?;
            }
            LambdaLayout::Chain(chain) => {
                write_lambda_chain_layout(f, &chain)?;
            }
        }
    }

    // terminator
    write!(f, [postfix_annotations(f.context(), node_id)])?;

    if lambda_declaration_needs_trailing_semicolon(f.context(), node_id, export) {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Write one lambda head before the body.
fn write_lambda_head<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatLambdaDeclarationOptions,
    is_first_in_chain: bool,
) -> FormatResult<()> {
    let parameters = function_parameters(signature);
    let parameter_container_span = function_parameter_container_span(f.context(), node_id);
    let has_generic_parameters = !signature.generic_parameters.is_empty();
    let can_omit_parens = function_can_omit_lambda_parameter_parentheses(
        f,
        signature,
        &parameters,
        has_generic_parameters,
    );

    // signature content
    let signature_content = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if options.call_argument_layout.is_some() && !is_first_in_chain {
            write!(f, [space()])?;
        }

        write_function_generic_parameters(f, signature)?;
        write_lambda_parameters_and_return_type(
            f,
            node_id,
            signature,
            body,
            &parameters,
            can_omit_parens,
            options.cache_mode,
        )?;

        let where_clauses = signature.where_clauses.as_slice();
        if !where_clauses.is_empty() {
            format_where_clause_with_break(f, where_clauses)?;
        }

        Ok(())
    });

    // signature head
    let head = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write_function_header_prefix(f, signature, true, false)?;

        if options.call_argument_layout.is_some() && !is_first_in_chain {
            let mut buffer = RemoveSoftLinesBuffer::new(f);
            write!(buffer, [signature_content])?;
        } else {
            write!(f, [signature_content])?;
        }

        Ok(())
    });

    let head = group(&head);
    let cache_key = parameter_container_span;
    let head = FormatContentWithCacheMode::new(cache_key, head, options.cache_mode);
    let comments_before_arrow = format_with(move |f: &mut DestackFormatter<'ast, '_>| {
        let comments_before_arrow = f
            .context()
            .comments()
            .comments_before_character(parameter_container_span.end, b'=');

        write!(f, [FormatTrailingComments::Comments(comments_before_arrow)])
    });
    let comments_before_arrow = FormatContentWithCacheMode::new(
        f.context().span(node_id),
        comments_before_arrow,
        options.cache_mode,
    );

    if options.call_argument_layout.is_some() {
        if is_first_in_chain {
            return write!(f, [head, comments_before_arrow]);
        }

        let mut buffer = RemoveSoftLinesBuffer::new(f);
        return write!(buffer, [head, comments_before_arrow]);
    }

    write!(
        f,
        [
            (!is_first_in_chain).then_some(soft_line_break_or_space()),
            head,
            comments_before_arrow
        ]
    )
}

/// Format one lambda declaration with explicit options.
pub(crate) fn format_lambda_declaration_with_options<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportKind>,
    is_ambient: bool,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
    options: FormatLambdaDeclarationOptions,
) -> FormatResult<()> {
    debug_assert_eq!(signature.form, FunctionForm::Lambda);
    debug_assert!(name.is_none());

    write_function_export_prefix(f, node_id, export)?;
    write_function_ambient_prefix(f, is_ambient)?;

    if body.is_none() {
        write_lambda_head(f, node_id, signature, body, options, true)?;
    }

    write_lambda_body_and_terminator(f, node_id, export, body, options)
}

/// Format one lambda declaration.
pub(crate) fn format_lambda_declaration<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Declaration>,
    export: Option<ExportKind>,
    is_ambient: bool,
    name: Option<Name>,
    signature: &FunctionSignature,
    body: &Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    format_lambda_declaration_with_options(
        f,
        node_id,
        export,
        is_ambient,
        name,
        signature,
        body,
        FormatLambdaDeclarationOptions::default(),
    )
}
