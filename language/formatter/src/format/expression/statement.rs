use crate::format::analysis::timing::tags;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::dependency::{format_export_expression, format_import_expression};
use crate::format::expression::{
    Annotation, AnnotationPosition, Asynchrony, Block, DeclarationDescriptor, DeclarationKind,
    Declarator, DependencyKind, DestackFormatContext, DestackFormatter, Expression, ForEachBinding,
    ForEachDeclarationKind, ForEachKind, FormatResult, IfKind, ImportSource, Keyword, LetKind,
    LocalNodeId, Mutability, NodeTree, NodeType, Pattern, TypeUnaryOperator, WhileKind,
    YieldCardinality, block_indent, detect_for_each_binding_keyword,
    expression_has_leading_prefix_comment, format_declarator, format_expression,
    format_for_each_binding_pattern, format_if_else_chain, format_match,
    format_statement_body_block, format_ternary, format_with, group, hard_line_break,
    is_empty_statement_block, space, token, tree_literal_should_break,
};
use destack_ast::{Comment, CommentStyle, ImportTarget};
use destack_fir::format::{Buffer, Format, FormatError};
use destack_fir::write;

/// Format `export import ... = require(...)` when modeled as an export let.
fn format_export_import_equals(
    f: &mut DestackFormatter<'_, '_>,
    tree: &NodeTree,
    descriptor: &DeclarationDescriptor,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<bool> {
    // descriptor.export is only set for export forms
    let Some(export) = descriptor.export else {
        return Ok(false);
    };

    // expect single declarator: const Alias = importEquals
    if declarators.len() != 1 {
        return Ok(false);
    }

    let Declarator {
        pattern,
        ty: None,
        value: Some(value),
    } = tree.get(declarators[0])
    else {
        return Ok(false);
    };

    if !matches!(tree.get(*pattern), Pattern::Binding { .. }) {
        return Ok(false);
    }

    let Expression::Import {
        source,
        kind,
        target,
        items,
        ..
    } = tree.get(*value)
    else {
        return Ok(false);
    };

    if *source != ImportSource::ImportEquals {
        return Ok(false);
    }
    let target = match target {
        ImportTarget::String(target) => *target,
        ImportTarget::Expression { .. } => {
            return Ok(false);
        }
    };

    let alias =
        items
            .first()
            .and_then(|item| tree.get(*item).alias)
            .ok_or(FormatError::SyntaxError {
                message: "import equals requires an alias",
            })?;

    write!(f, [export, space(), Keyword::Import, space()])?;
    if *kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }
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
    Ok(true)
}

/// Return whether a statement wrapper should print a trailing semicolon.
fn statement_expression_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);

    let is_declaration_statement = matches!(expression, Expression::Declaration(_));
    let is_block_statement = matches!(expression, Expression::Block(_));
    let is_control_flow_statement = matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        } | Expression::While { .. }
            | Expression::ForEach { .. }
            | Expression::For { .. }
            | Expression::Loop { .. }
            | Expression::Match { .. }
    );

    let is_do_while_statement = matches!(
        expression,
        Expression::While {
            kind: WhileKind::DoWhile,
            ..
        }
    );

    if is_do_while_statement {
        return true;
    }

    !(is_declaration_statement || is_block_statement || is_control_flow_statement)
}

/// Return whether one expression has a multiline block postfix annotation.
fn expression_has_multiline_block_postfix_annotation(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(annotation_ids) = context.annotations(expression_id) else {
        return false;
    };

    annotation_ids.into_iter().any(|annotation_id| {
        let Annotation::Comment { node, position } = context.annotation(annotation_id) else {
            return false;
        };
        if !matches!(
            position,
            AnnotationPosition::LinePostfixBoundary | AnnotationPosition::BlockPostfix
        ) {
            return false;
        }

        let comment = context.tree.get::<Comment>(node);
        if comment.style != CommentStyle::Star {
            return false;
        }

        context.has_newline(context.annotation_span(annotation_id))
    })
}

/// Format one statement wrapper inner expression with an optional trailing semicolon.
fn format_statement_wrapped_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    needs_semicolon: bool,
) -> FormatResult<()> {
    let expression = f.context().tree.get(node_id);
    let directive = directive_for_node(f.context(), node_id);

    // prefix annotations and core expression
    write!(f, [f.context().any_prefix_annotations(node_id)])?;
    format_expression(f, node_id, expression, directive)?;

    let semicolon_after_multiline_as_const_postfix = needs_semicolon
        && matches!(
            expression,
            Expression::TypeUnary {
                operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                ..
            }
        )
        && expression_has_multiline_block_postfix_annotation(f.context(), node_id);

    // statement terminator should stay attached to the statement expression,
    // not drift after postfix trivia into its own line
    if needs_semicolon && !semicolon_after_multiline_as_const_postfix {
        write!(f, [token(";")])?;
    }

    // regular if chains emit their own edge annotations in control formatter
    let if_chain_handles_annotations = matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        }
    );

    // postfix and infix annotations
    if !if_chain_handles_annotations
        && !matches!(
            directive,
            Some(FormatterDirective {
                kind: FormatterDirectiveKind::IgnoreFormat,
                position: FormatterDirectivePosition::Postfix { .. },
            })
        )
    {
        let call_or_new_handles_empty_infix = matches!(
            expression,
            Expression::Call {
                dynamic_arguments,
                ..
            }
            | Expression::New {
                dynamic_arguments,
                ..
            } if dynamic_arguments.is_empty() && f.context().has_infix_annotation(node_id)
        );

        if matches!(
            expression,
            Expression::TypeUnary {
                operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                ..
            }
        ) {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else if call_or_new_handles_empty_infix {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else {
            write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
        }
    }

    if semicolon_after_multiline_as_const_postfix {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Format a `let` expression.
fn format_let_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: LetKind,
    descriptor: &DeclarationDescriptor,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    // export import equals
    let handled_export_import_equals =
        format_export_import_equals(f, tree, descriptor, declarators)?;

    // keyword header: export + declare + let/var/const
    if !handled_export_import_equals {
        // export
        if let Some(export) = descriptor.export {
            write!(f, [export, space()])?;
        }

        // declare
        if descriptor.kind == DeclarationKind::Declaration {
            write!(f, [Keyword::Declare, space()])?;
        }

        // let kind
        match kind {
            LetKind::Let => write!(f, [Keyword::Let])?,
            LetKind::Var => write!(f, [Keyword::Var])?,
            LetKind::Const => write!(f, [Keyword::Const])?,
        }

        // declarators
        for (index, declarator_id) in declarators.iter().enumerate() {
            if index > 0 {
                write!(f, [token(",")])?;
            }
            write!(f, [space()])?;
            format_declarator(f, tree, *declarator_id)?;
        }
    }

    Ok(())
}

/// Format a `using` expression.
fn format_using_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    asynchrony: Asynchrony,
    descriptor: &DeclarationDescriptor,
    declarators: &[LocalNodeId<Declarator>],
) -> FormatResult<()> {
    let tree = f.context().tree;

    // keyword header: export + declare + await + using
    // export
    if let Some(export) = descriptor.export {
        write!(f, [export, space()])?;
    }

    // declare
    if descriptor.kind == DeclarationKind::Declaration {
        write!(f, [Keyword::Declare, space()])?;
    }

    // await
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Await, space()])?;
    }

    // using
    write!(f, [Keyword::Using])?;

    // declarators
    for (index, declarator_id) in declarators.iter().enumerate() {
        if index > 0 {
            write!(f, [token(",")])?;
        }
        write!(f, [space()])?;
        format_declarator(f, tree, *declarator_id)?;
    }

    Ok(())
}

/// Return whether block annotations include a block prefix annotation.
fn block_has_block_prefix_annotation(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let Some(annotations) = context.annotations(block_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::BlockPrefix,
                ..
            } | Annotation::Doc {
                position: AnnotationPosition::BlockPrefix,
                ..
            } | Annotation::Comment {
                position: AnnotationPosition::BlockPrefix,
                ..
            } | Annotation::Decorator {
                position: AnnotationPosition::BlockPrefix,
                ..
            }
        )
    })
}

/// Return whether block annotations include a line prefix annotation.
fn block_has_line_prefix_annotation(
    context: &DestackFormatContext<'_>,
    block_id: LocalNodeId<Block>,
) -> bool {
    let Some(annotations) = context.annotations(block_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.annotation(annotation_id),
            Annotation::Blank {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Doc {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Comment {
                position: AnnotationPosition::LinePrefix,
                ..
            } | Annotation::Decorator {
                position: AnnotationPosition::LinePrefix,
                ..
            }
        )
    })
}

/// Return whether a control-flow statement body should be preceded by a space.
fn statement_body_requires_head_space(
    context: &DestackFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    if block_has_block_prefix_annotation(context, body) {
        return false;
    }

    if block_has_line_prefix_annotation(context, body) {
        return true;
    }

    !is_empty_statement_block(context, body)
}

/// Format a `while` or `do while` expression.
fn format_while_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: WhileKind,
    condition: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    match kind {
        // while (<condition>) <body>
        WhileKind::While => {
            write!(
                f,
                [Keyword::While, space(), token("("), condition, token(")")]
            )?;
            if statement_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, body)?;
        }
        // do <body> while (<condition>)
        WhileKind::DoWhile => {
            write!(f, [Keyword::Do])?;
            if statement_body_requires_head_space(f.context(), body) {
                write!(f, [space()])?;
            }
            format_statement_body_block(f, body)?;
            write!(
                f,
                [
                    space(),
                    Keyword::While,
                    space(),
                    token("("),
                    condition,
                    token(")"),
                ]
            )?;
        }
    }

    Ok(())
}

/// Format a `for each` expression.
fn format_for_each_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    asynchrony: Asynchrony,
    kind: ForEachKind,
    binding: &ForEachBinding,
    iterator: LocalNodeId<Expression>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    let tree = f.context().tree;

    // for header
    write!(f, [Keyword::For, space()])?;
    if asynchrony == Asynchrony::Async {
        write!(f, [Keyword::Await, space()])?;
    }
    let keyword = match kind {
        ForEachKind::In => Keyword::In,
        ForEachKind::Of => Keyword::Of,
    };

    // binding
    write!(f, [token("(")])?;
    match binding {
        ForEachBinding::Pattern {
            pattern,
            declaration_kind,
        } => {
            // explicit declaration kind
            if let Some(declaration_kind) = declaration_kind {
                let keyword = match declaration_kind {
                    ForEachDeclarationKind::Var => Keyword::Var,
                    ForEachDeclarationKind::Let => Keyword::Let,
                    ForEachDeclarationKind::Const => Keyword::Const,
                };
                write!(f, [keyword, space()])?;
                format_for_each_binding_pattern(f, *pattern)?;
            }
            // source keyword recovery
            else {
                let source_keyword =
                    detect_for_each_binding_keyword(f.context(), node_id, *pattern);
                if let Some(keyword) = source_keyword {
                    write!(f, [keyword, space()])?;
                    format_for_each_binding_pattern(f, *pattern)?;
                } else {
                    let pattern_node = tree.get(*pattern);
                    let should_prefix_const = matches!(
                        pattern_node,
                        Pattern::Binding {
                            mutability: Some(Mutability::Immutable),
                            pattern: None,
                            ..
                        }
                    );

                    // keep explicit const for simple immutable bindings
                    if should_prefix_const {
                        write!(f, [Keyword::Const, space()])?;
                    }
                    write!(f, [pattern])?;
                }
            }
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

    // iterator + body
    write!(f, [space(), keyword, space(), iterator, token(")")])?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)?;

    Ok(())
}

/// Format a classic `for` expression.
fn format_for_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    initialization: Option<LocalNodeId<Expression>>,
    condition: Option<LocalNodeId<Expression>>,
    increment: Option<LocalNodeId<Expression>>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
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
            token(")")
        ]
    )?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Format a `loop` expression.
fn format_loop_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    body: LocalNodeId<Block>,
) -> FormatResult<()> {
    write!(f, [Keyword::Loop])?;
    if statement_body_requires_head_space(f.context(), body) {
        write!(f, [space()])?;
    }
    format_statement_body_block(f, body)
}

/// Format a `try` expression.
fn format_try_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    try_expression: LocalNodeId<Expression>,
    catch_pattern: Option<LocalNodeId<Pattern>>,
    catch_ty: Option<LocalNodeId<Expression>>,
    catch_expression: Option<LocalNodeId<Expression>>,
    finally_expression: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    // try block
    write!(f, [Keyword::Try, space(), try_expression])?;

    // catch block
    if let Some(catch_expression) = catch_expression {
        write!(f, [space(), Keyword::Catch, space()])?;
        if let Some(catch_pattern) = catch_pattern {
            write!(f, [token("("), catch_pattern])?;
            if let Some(catch_ty) = catch_ty {
                write!(f, [token(":"), space(), catch_ty])?;
            }
            write!(f, [token(")"), space()])?;
        }
        write!(f, [catch_expression])?;
    }

    // finally block
    if let Some(finally_expression) = finally_expression {
        write!(f, [space(), Keyword::Finally, space(), finally_expression])?;
    }

    Ok(())
}

/// Format a `return` expression.
fn format_return_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    value: Option<LocalNodeId<Expression>>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let return_parent_is_block = f
        .context()
        .parent(node_id)
        .is_some_and(|(_, parent_type)| parent_type == NodeType::Block);

    // return keyword
    write!(f, [token("return")])?;

    // return value
    if let Some(value_id) = value {
        let value_expr = tree.get(value_id);

        // jsx returns may need wrapping parens to keep multi line layout stable
        if let Expression::TreeExpression {
            arguments,
            elements,
            ..
        } = value_expr
        {
            let has_children = elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty());
            let has_multiple_attributes = arguments
                .as_ref()
                .is_some_and(|arguments| arguments.len() > 1);
            let should_wrap_tree_return = has_children
                || has_multiple_attributes
                || tree_literal_should_break(f.context(), arguments, elements);

            if should_wrap_tree_return {
                write!(
                    f,
                    [
                        space(),
                        token("("),
                        block_indent(&value_id),
                        hard_line_break(),
                        token(")")
                    ]
                )?;
            } else {
                write!(f, [space(), value_id])?;
            }
        } else {
            write!(f, [space(), value_id])?;
        }
    }

    // trailing semicolon
    if return_parent_is_block {
        write!(f, [token(";")])?;
    }

    Ok(())
}

/// Format statement-like expression variants.
pub(crate) fn format_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    match expression {
        // declaration
        Expression::Declaration(node) => node.format(f)?,

        // block
        Expression::Block(node) => node.format(f)?,

        // statement
        Expression::Statement(node) => {
            let needs_semicolon = statement_expression_needs_semicolon(f.context(), *node);
            format_statement_wrapped_expression(f, *node, needs_semicolon)?;
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
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_IMPORT);
            format_import_expression(f, *source, *kind, target, items, arguments.as_deref())?;
        }

        // export
        Expression::Export {
            kind,
            target,
            items,
            arguments,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_EXPORT);
            format_export_expression(f, *kind, *target, items, arguments.as_deref())?;
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
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_LET);
            format_let_expression(f, *kind, descriptor, declarators)?;
        }

        // using
        Expression::Using {
            asynchrony,
            descriptor,
            declarators,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_LET);
            format_using_expression(f, *asynchrony, descriptor, declarators)?;
        }

        // if (ternary)
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_ternary(f, node_id)?;
        }

        // if (regular)
        Expression::If {
            kind: IfKind::If, ..
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_while_expression(f, *kind, *condition, *body)?;
        }

        // for each
        Expression::ForEach {
            asynchrony,
            kind,
            binding,
            iterator,
            body,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_for_each_expression(f, node_id, *asynchrony, *kind, binding, *iterator, *body)?;
        }

        // for condition
        Expression::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_for_expression(f, *initialization, *condition, *increment, *body)?;
        }

        // loop
        Expression::Loop { body } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_loop_expression(f, *body)?;
        }

        // try
        Expression::Try {
            try_expression,
            catch_pattern,
            catch_ty,
            catch_expression,
            finally_expression,
        } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_try_expression(
                f,
                *try_expression,
                *catch_pattern,
                *catch_ty,
                *catch_expression,
                *finally_expression,
            )?;
        }

        // match
        Expression::Match { .. } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_match(f, node_id, true)?;
        }

        // break
        Expression::Break { label, value } => {
            write!(f, [Keyword::Break])?;
            if let Some(label) = label {
                if f.context().options.language_type.is_destack() {
                    write!(f, [space(), token(":"), label])?;
                } else {
                    write!(f, [space(), label])?;
                }
            }
            if let Some(value) = value {
                write!(f, [space(), value])?;
            }
        }

        // continue
        Expression::Continue { label } => {
            write!(f, [Keyword::Continue])?;
            if let Some(label) = label {
                if f.context().options.language_type.is_destack() {
                    write!(f, [space(), token(":"), label])?;
                } else {
                    write!(f, [space(), label])?;
                }
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
                let value_has_leading_prefix_comment =
                    expression_has_leading_prefix_comment(f.context(), *value);
                let should_wrap_value = value_has_leading_prefix_comment
                    && !matches!(
                        f.context().tree.get(*value),
                        Expression::Parenthesized { .. }
                    );
                if should_wrap_value {
                    write!(
                        f,
                        [
                            space(),
                            token("("),
                            block_indent(value),
                            hard_line_break(),
                            token(")")
                        ]
                    )?;
                } else {
                    write!(f, [space(), value])?;
                }
            }
        }

        // throw
        Expression::Throw { value } => {
            write!(f, [token("throw")])?;
            write!(f, [space(), value])?;
        }

        // return
        Expression::Return { value } => {
            let _timing = f
                .context()
                .timing_scope(tags::FORMAT_EXPRESSION_STATEMENT_RETURN);
            format_return_expression(f, node_id, *value)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}
