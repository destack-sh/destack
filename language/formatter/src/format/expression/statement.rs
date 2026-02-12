use super::super::timing::tags;
use super::*;
use crate::imports::sort_dependency_items;
use destack_ast::ImportTarget;
use destack_fir::write;

/// Return whether a statement wrapper should print a trailing semicolon.
fn statement_expression_needs_semicolon(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let expression = context.tree.get(expression_id);

    let is_type_declaration_statement = matches!(
        expression,
        Expression::Declaration(declaration_id)
            if matches!(context.tree.get(*declaration_id), Declaration::Type { .. })
    );
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

    !(is_type_declaration_statement || is_block_statement || is_control_flow_statement)
}

/// Format `with { ... }` arguments for import and export statements.
fn format_dependency_with_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    // source newlines inside `with` should expand the collection
    let should_expand_with_arguments =
        call_arguments_are_multiline_in_source(f.context(), arguments);
    let mut with_arguments = list_like("{", "}", ",", arguments);
    with_arguments
        .as_collection()
        .include_space()
        .should_expand(should_expand_with_arguments);

    write!(f, [space(), Keyword::With, space(), with_arguments])
}

/// Format an import expression.
fn format_import_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    source: ImportSource,
    kind: DependencyKind,
    target: &ImportTarget,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let organize = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;
    let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

    // import call
    if source == ImportSource::ImportCall {
        write!(f, [Keyword::Import, token("(")])?;

        match target {
            ImportTarget::String(target) => {
                write!(f, [token("\""), *target, token("\"")])?;
            }
            ImportTarget::Expression { target } => {
                write!(f, [*target])?;
            }
        }

        if let Some(arguments) = arguments {
            for argument in arguments {
                write!(f, [token(","), space(), *argument])?;
            }
        }

        write!(f, [token(")")])?;
        return Ok(());
    }

    let target = match target {
        ImportTarget::String(target) => *target,
        ImportTarget::Expression { .. } => {
            return Err(FormatError::SyntaxError {
                message: "import declarations require string targets",
            });
        }
    };

    // keyword
    write!(f, [Keyword::Import, space()])?;
    if source == ImportSource::ImportEquals {
        if kind == DependencyKind::Type {
            write!(f, [Keyword::Type, space()])?;
        }

        // import equals requires a default alias
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
    if kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }

    // items
    let first_item = items.first().map(|item| tree.get(*item));

    // namespace import
    if items.len() == 1 && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace) {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace import requires at least one dependency item",
            });
        };

        write!(
            f,
            [token("*"), space(), Keyword::As, space(), first_item.alias]
        )?;
    }
    // default + named imports
    else if let Some(first_item) = first_item
        && first_item.mode == DependencyMode::Default
    {
        let rest_items = &items[1..];
        write!(f, [first_item.alias])?;
        if !rest_items.is_empty() {
            // sort named imports when organize_imports is enabled
            let sorted_rest = if organize && !items_have_annotations {
                sort_dependency_items(rest_items, tree, f.context().strings, sort_order)
            } else {
                rest_items.to_vec()
            };
            write!(f, [token(","), space()])?;
            write!(
                f,
                [list_like("{", "}", ",", &sorted_rest)
                    .as_collection()
                    .include_space()]
            )?;
        }
    }
    // named imports
    else if !items.is_empty() {
        // sort named imports when organize_imports is enabled
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

    // from clause
    if !items.is_empty() {
        write!(f, [space(), Keyword::From, space()])?;
    }
    write!(f, [token("\""), target, token("\"")])?;

    // with clause
    if let Some(arguments) = arguments {
        format_dependency_with_arguments(f, arguments)?;
    }

    Ok(())
}

/// Format an export expression.
fn format_export_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: DependencyKind,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let organize = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;
    let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

    // keyword
    write!(f, [Keyword::Export, space()])?;
    if kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }

    // items
    let first_item = items.first().map(|item| tree.get(*item));

    // default export with value: export default <value>
    if items.len() == 1
        && first_item
            .is_some_and(|item| item.mode == DependencyMode::Default && item.value.is_some())
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "default export requires at least one dependency item",
            });
        };
        let Some(value) = first_item.value else {
            return Err(FormatError::SyntaxError {
                message: "default export requires a dependency value",
            });
        };
        write!(f, [Keyword::Default, space(), value])?;
    }
    // namespace export: export * as X, export = X
    else if items.len() == 1
        && first_item.is_some_and(|item| item.mode == DependencyMode::Namespace)
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace export requires at least one dependency item",
            });
        };
        if first_item.value.is_some() && target.is_none() {
            let Some(value) = first_item.value else {
                return Err(FormatError::SyntaxError {
                    message: "namespace export assignment requires a dependency value",
                });
            };
            write!(f, [token("="), space(), value])?;
        } else {
            write!(f, [token("*")])?;
            if let Some(alias) = first_item.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
    }
    // named exports
    else if !items.is_empty() {
        // sort named exports when organize_imports is enabled
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

    // with clause
    if let Some(arguments) = arguments {
        format_dependency_with_arguments(f, arguments)?;
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
    let Some(annotations) = context.get_annotations(block_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.tree.get::<Annotation>(annotation_id),
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
    let Some(annotations) = context.get_annotations(block_id) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            context.tree.get::<Annotation>(annotation_id),
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
            let trailing_boundary_comments =
                collect_catch_pattern_trailing_boundary_comments(f.context(), catch_pattern);
            let keep_pattern_node_formatting = trailing_boundary_comments.is_empty()
                || catch_ty.is_some()
                || f.context().has_prefix_annotation(catch_pattern);
            if keep_pattern_node_formatting {
                write!(f, [token("("), catch_pattern])?;
                if let Some(catch_ty) = catch_ty {
                    write!(f, [token(":"), space(), catch_ty])?;
                }
                write!(f, [token(")"), space()])?;
            } else {
                let pattern_source = f
                    .context()
                    .get_span_str(f.context().get_span(catch_pattern));
                let pattern_source = strip_one_wrapping_parentheses(pattern_source);
                write!(f, [token("("), text(pattern_source), token(")")])?;
                for comment in trailing_boundary_comments {
                    write!(f, [space(), text(comment.as_str())])?;
                }
                write!(f, [space()])?;
            }
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
        .get_parent(node_id)
        .is_some_and(|(_, parent_type)| parent_type == NodeType::Block);

    // return keyword
    write!(f, [token("return")])?;

    // return value
    if let Some(value_id) = value {
        let value_expr = tree.get(value_id);

        // jsx returns may need wrapping parens to keep multi line layout stable
        if let Expression::TreeExpression { elements, .. } = value_expr {
            let has_children = elements
                .as_ref()
                .is_some_and(|elements| !elements.is_empty());
            if has_children {
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
                let line_width = usize::from(f.context().options.line_width);
                let inline_width = "return ".len() + expression_source_len(f.context(), value_id);
                if inline_width <= line_width {
                    write!(f, [space(), value_id])?;
                } else {
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
                }
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
pub(super) fn format_statement_expression<'ast>(
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
            if statement_expression_needs_semicolon(f.context(), *node) {
                write!(f, [*node, token(";")])?;
            } else {
                write!(f, [*node])?;
            }
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
                let should_wrap_value = yield_value_has_leading_prefix_comment(f.context(), *value)
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
