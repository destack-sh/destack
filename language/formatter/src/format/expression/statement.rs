use crate::format::analysis::timing;
use crate::format::annotation::statement_wrapper_needs_semicolon;
use crate::format::chain::expression_trivia_anchor_end;
use crate::format::declaration::dependency::sort_dependency_items;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{
    Annotation, AnnotationPosition, Asynchrony, Block, DeclarationDescriptor, DeclarationKind,
    Declarator, DependencyKind, DependencyMode, DestackFormatContext, DestackFormatter, Expression,
    ForEachBinding, ForEachDeclarationKind, ForEachKind, FormatResult, IfCondition, IfKind,
    ImportSource, Keyword, LetKind, LocalNodeId, Mutability, NodeTree, NodeType, Pattern, Span,
    StringId, TypeUnaryOperator, WhileKind, YieldCardinality, block_indent,
    call_arguments_are_multiline_span, detect_for_each_binding_keyword, format_declarator,
    format_expression, format_for_each_binding_pattern, format_if_else_chain, format_match,
    format_statement_body_block, format_ternary, format_with, group, hard_line_break,
    is_empty_statement_block, line_postfix_boundary, list_like, space, token,
    tree_literal_should_break,
};
use destack_ast::{Comment, CommentStyle, Doc, DocumentationStyle, ImportTarget};
use destack_fir::format::{Buffer, Format, FormatError};
use destack_fir::write;

/// Format `with { ... }` arguments for import and export statements.
fn format_dependency_with_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    arguments: &[LocalNodeId<crate::format::expression::Argument>],
) -> FormatResult<()> {
    // source newlines inside `with` should expand the collection
    let should_expand_with_arguments = call_arguments_are_multiline_span(f.context(), arguments);
    let mut with_arguments = list_like("{", "}", ",", arguments);
    with_arguments
        .as_collection()
        .include_space()
        .should_expand(should_expand_with_arguments);

    write!(f, [space(), Keyword::With, space(), with_arguments])
}

/// Format an import expression.
pub(crate) fn format_import_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    source: ImportSource,
    kind: DependencyKind,
    target: &ImportTarget,
    items: &[LocalNodeId<crate::format::expression::DependencyItem>],
    arguments: Option<&[LocalNodeId<crate::format::expression::Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let organize = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;
    let items_have_annotations = items.iter().any(|item| f.context().has_annotation(*item));

    // import call
    if source == ImportSource::ImportCall {
        write!(f, [Keyword::Import, token("(")])?;

        let should_expand_import_call_arguments = match target {
            ImportTarget::Expression { target } => {
                f.context().has_annotation(*target) || f.context().node_has_newline(*target)
            }
            ImportTarget::String(_) => false,
        };

        if should_expand_import_call_arguments {
            write!(f, [hard_line_break()])?;
            write!(
                f,
                [group(&block_indent(&format_with(
                    |f: &mut DestackFormatter<'ast, '_>| {
                        let arguments_len = arguments.map_or(0, |items| items.len());
                        let total_items = 1usize + arguments_len;

                        // target item
                        match target {
                            ImportTarget::String(target) => {
                                write!(f, [token("\""), *target, token("\"")])?;
                            }
                            ImportTarget::Expression { target } => {
                                write!(f, [*target])?;
                            }
                        }
                        if total_items > 1 {
                            write!(f, [token(",")])?;
                        }

                        // with-arguments items
                        if let Some(arguments) = arguments {
                            for (index, argument) in arguments.iter().enumerate() {
                                write!(f, [hard_line_break(), *argument])?;
                                if index + 1 < arguments.len() {
                                    write!(f, [token(",")])?;
                                }
                            }
                        }

                        Ok(())
                    }
                )))]
            )?;
            write!(f, [hard_line_break(), token(")")])?;
        } else {
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
        }
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

    let import_type_empty_items = kind == DependencyKind::Type && items.is_empty();

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
        let default_alias = first_item.alias.ok_or(FormatError::SyntaxError {
            message: "default import requires an alias",
        })?;
        let rest_items = &items[1..];
        write!(f, [default_alias])?;

        if rest_items.len() == 1 && tree.get(rest_items[0]).mode == DependencyMode::Namespace {
            let namespace_item = tree.get(rest_items[0]);
            let namespace_alias = namespace_item.alias.ok_or(FormatError::SyntaxError {
                message: "namespace import requires an alias",
            })?;
            write!(
                f,
                [
                    token(","),
                    space(),
                    token("*"),
                    space(),
                    Keyword::As,
                    space(),
                    namespace_alias
                ]
            )?;
        } else if !rest_items.is_empty() {
            // sort named imports when organize_imports is enabled
            let sorted_rest = if organize && !items_have_annotations {
                sort_dependency_items(rest_items, tree, f.context().strings, sort_order)
            } else {
                rest_items.to_vec()
            };
            write!(f, [token(","), space()])?;
            let mut rest_list = list_like("{", "}", ",", &sorted_rest);
            rest_list
                .as_collection()
                .include_space()
                .should_expand(items_have_annotations);
            write!(f, [rest_list])?;
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
        let mut items_list = list_like("{", "}", ",", &sorted_items);
        items_list
            .as_collection()
            .include_space()
            .should_expand(items_have_annotations);
        write!(f, [items_list])?;
    } else if import_type_empty_items {
        write!(f, [token("{"), token("}")])?;
    }

    // from clause
    if !items.is_empty() || import_type_empty_items {
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
pub(crate) fn format_export_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    kind: DependencyKind,
    target: Option<StringId>,
    items: &[LocalNodeId<crate::format::expression::DependencyItem>],
    arguments: Option<&[LocalNodeId<crate::format::expression::Argument>]>,
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

    let export_empty_items_with_target = items.is_empty() && target.is_some();

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
    else if let Some(first_item) = first_item
        && first_item.mode == DependencyMode::Default
        && items.len() == 2
        && target.is_some()
        && tree.get(items[1]).mode == DependencyMode::Namespace
    {
        let default_alias = first_item.alias.ok_or(FormatError::SyntaxError {
            message: "default re-export requires an alias",
        })?;
        let namespace_item = tree.get(items[1]);
        let namespace_alias = namespace_item.alias.ok_or(FormatError::SyntaxError {
            message: "namespace re-export requires an alias",
        })?;

        write!(
            f,
            [
                default_alias,
                token(","),
                space(),
                token("*"),
                space(),
                Keyword::As,
                space(),
                namespace_alias
            ]
        )?;
    }
    // named exports
    else if !items.is_empty() {
        // sort named exports when organize_imports is enabled
        let sorted_items = if organize && !items_have_annotations {
            sort_dependency_items(items, tree, f.context().strings, sort_order)
        } else {
            items.to_vec()
        };
        let mut items_list = list_like("{", "}", ",", &sorted_items);
        items_list
            .as_collection()
            .include_space()
            .should_expand(items_have_annotations);
        write!(f, [items_list])?;
    } else if target.is_none() || export_empty_items_with_target {
        // empty export clause: `export {}`
        write!(f, [token("{"), token("}")])?;
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
            AnnotationPosition::LinePostfix
                | AnnotationPosition::LinePostfixBoundary
                | AnnotationPosition::BlockPostfix
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
        let collection_handles_empty_infix = f.context().has_infix_annotation(node_id)
            && (matches!(
                expression,
                Expression::ObjectExpression { properties, .. } if properties.is_empty()
            ) || matches!(
                expression,
                Expression::ArrayExpression { elements } if elements.is_empty()
            ));

        if matches!(
            expression,
            Expression::TypeUnary {
                operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                ..
            }
        ) {
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
        } else if call_or_new_handles_empty_infix || collection_handles_empty_infix {
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
                [
                    Keyword::While,
                    space(),
                    token("("),
                    condition,
                    line_postfix_boundary(),
                    token(")")
                ]
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
                    line_postfix_boundary(),
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
    write!(
        f,
        [
            space(),
            keyword,
            space(),
            iterator,
            line_postfix_boundary(),
            token(")")
        ]
    )?;
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
            line_postfix_boundary(),
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

/// Return whether one annotation id is one multiline block comment/doc.
fn annotation_is_multiline_block(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = context.annotation(annotation_id);
    let annotation_span = context.annotation_span(annotation_id);

    // block style comments/docs spanning multiple lines are leading comments
    match annotation {
        Annotation::Comment { node, .. } => {
            let comment = context.tree.get::<Comment>(node);
            comment.style == CommentStyle::Star && context.has_newline(annotation_span)
        }
        Annotation::Doc { node, .. } => {
            let doc = context.tree.get::<Doc>(node);
            doc.style == DocumentationStyle::Star && context.has_newline(annotation_span)
        }
        _ => false,
    }
}

/// Return whether one annotation id is followed by a newline before the next token.
fn annotation_is_followed_by_newline(
    context: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation_span = context.annotation_span(annotation_id);
    let Some(next_token) = context.annotation_next_non_whitespace_token(annotation_id) else {
        return false;
    };
    if annotation_span.file != next_token.span.file {
        return false;
    }

    !context
        .file
        .is_same_line(annotation_span.end.saturating_sub(1), next_token.span.start)
}

/// Return whether one expression has leading prefix comment/doc annotations for adjacent wrapping.
fn expression_has_adjacent_leading_comment(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    context
        .visit_annotations(expression_id, |annotations| {
            annotations.iter().any(|annotation_id| {
                let annotation_id = *annotation_id;
                let annotation = context.annotation(annotation_id);
                if !matches!(
                    annotation,
                    Annotation::Comment {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    } | Annotation::Doc {
                        position: AnnotationPosition::LinePrefix | AnnotationPosition::BlockPrefix,
                        ..
                    }
                ) {
                    return false;
                }

                annotation_is_multiline_block(context, annotation_id)
                    || annotation_is_followed_by_newline(context, annotation_id)
            })
        })
        .unwrap_or(false)
}

/// Return the gap span between one member receiver and property token.
fn member_receiver_property_gap_span(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<Span> {
    let left_id = match context.tree.get(expression_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => *left,
        _ => return None,
    };
    let property_span = context.tree.get_main_span(expression_id)?;
    let left_span = context.span(left_id);
    let left_anchor_end = expression_trivia_anchor_end(context, left_id);

    if left_span.file != property_span.file || property_span.start <= left_anchor_end {
        return None;
    }

    Some(Span::new(
        left_span.file,
        left_anchor_end,
        property_span.start,
    ))
}

/// Return whether one comment span starts on its own line or is multiline.
fn comment_span_is_own_line_or_multiline(
    context: &DestackFormatContext<'_>,
    comment_span: Span,
) -> bool {
    if !context
        .file
        .is_same_line(comment_span.start, comment_span.end.saturating_sub(1))
    {
        return true;
    }

    context.span_starts_on_own_line(comment_span)
}

/// Return whether one member expression has own-line or multiline comments between receiver and property.
fn member_has_leading_comment_between_receiver_and_property(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = member_receiver_property_gap_span(context, expression_id) else {
        return false;
    };
    let comment_spans = &context.comment_spans;
    let first_comment_index =
        comment_spans.partition_point(|comment_span| comment_span.end <= gap_span.start);

    for comment_span in &comment_spans[first_comment_index..] {
        if comment_span.file != gap_span.file {
            continue;
        }
        if comment_span.start >= gap_span.end {
            break;
        }
        if comment_span.end <= gap_span.start {
            continue;
        }

        if comment_span_is_own_line_or_multiline(context, *comment_span) {
            return true;
        }
    }

    false
}

/// Return the next left-side expression used for adjacent return/throw comment checks.
fn next_adjacent_argument_left_side(
    context: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match context.tree.get(expression_id) {
        Expression::SequenceExpression { expressions } => expressions.first().copied(),
        Expression::Member { left, .. }
        | Expression::PrivateMember { left, .. }
        | Expression::Index { left, .. }
        | Expression::Call { left, .. }
        | Expression::New { left, .. }
        | Expression::Instantiation { left, .. }
        | Expression::Maybe { left, .. }
        | Expression::Must { left, .. }
        | Expression::TypeBinary { left, .. }
        | Expression::Binary { left, .. }
        | Expression::TypeIndex { left, .. }
        | Expression::Assign { left, .. } => Some(*left),
        Expression::TaggedTemplateExpression { tag, .. } => Some(*tag),
        Expression::If {
            kind: IfKind::Ternary,
            condition: IfCondition::Expression { condition },
            ..
        } => Some(*condition),
        Expression::Statement(expression) => Some(*expression),
        // explicit parentheses already delimit leading trivia for this argument segment
        Expression::Parenthesized { .. } => None,
        _ => None,
    }
}

/// Return whether one adjacent statement argument has leading comments that require wrapping.
fn adjacent_statement_argument_has_leading_comments(
    context: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Expression>,
) -> bool {
    let argument_parent_is_yield =
        context
            .parent(argument_id)
            .is_some_and(|(parent_id, parent_type)| {
                parent_type == NodeType::Expression
                    && matches!(
                        context.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                        Expression::Yield { .. }
                    )
            });

    let mut current_id = argument_id;
    loop {
        current_id = context.transparent_inner_expression(current_id);

        let has_adjacent_leading_comment =
            expression_has_adjacent_leading_comment(context, current_id);
        let has_member_gap_comment =
            member_has_leading_comment_between_receiver_and_property(context, current_id);

        if has_adjacent_leading_comment {
            let should_ignore_for_yield_chain_continuation =
                argument_parent_is_yield && has_member_gap_comment;
            if should_ignore_for_yield_chain_continuation {
                // keep yield member continuation comments in chain form: `yield value\n  // c\n  .m()`
            } else {
                return true;
            }
        }

        if !argument_parent_is_yield && has_member_gap_comment {
            return true;
        }

        let Some(next_id) = next_adjacent_argument_left_side(context, current_id) else {
            break;
        };
        current_id = next_id;
    }

    false
}

/// Format one return/throw/yield adjacent argument.
fn format_adjacent_statement_argument<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    value_id: LocalNodeId<Expression>,
) -> FormatResult<()> {
    let value_check_id = f.context().transparent_inner_expression(value_id);
    let value_expression = f.context().tree.get(value_check_id);
    let value_has_leading_comment =
        adjacent_statement_argument_has_leading_comments(f.context(), value_check_id);
    let value_is_parenthesized = matches!(value_expression, Expression::Parenthesized { .. });
    let value_is_unwrapped_sequence =
        matches!(value_expression, Expression::SequenceExpression { .. });
    let should_wrap_value =
        !value_is_parenthesized && (value_is_unwrapped_sequence || value_has_leading_comment);

    // leading own-line comments on adjacent arguments need one paren wrapper
    if should_wrap_value {
        write!(
            f,
            [
                space(),
                token("("),
                block_indent(&group(&value_check_id).should_expand(true)),
                hard_line_break(),
                token(")")
            ]
        )?;
        return Ok(());
    }

    write!(f, [space(), value_id])?;
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
                format_adjacent_statement_argument(f, value_id)?;
            }
        } else {
            format_adjacent_statement_argument(f, value_id)?;
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
            let needs_semicolon = statement_wrapper_needs_semicolon(f.context(), *node);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_IMPORT);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_EXPORT);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_LET);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_LET);
            format_using_expression(f, *asynchrony, descriptor, declarators)?;
        }

        // if (ternary)
        Expression::If {
            kind: IfKind::Ternary,
            ..
        } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_ternary(f, node_id)?;
        }

        // if (regular)
        Expression::If {
            kind: IfKind::If, ..
        } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
            format_for_expression(f, *initialization, *condition, *increment, *body)?;
        }

        // loop
        Expression::Loop { body } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_CONTROL);
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
                format_adjacent_statement_argument(f, *value)?;
            }

            // block statement yields should terminate like return/throw in statement position
            let yield_parent_is_block = f
                .context()
                .parent(node_id)
                .is_some_and(|(_, parent_type)| parent_type == NodeType::Block);
            if yield_parent_is_block {
                write!(f, [token(";")])?;
            }
        }

        // throw
        Expression::Throw { value } => {
            write!(f, [token("throw")])?;
            format_adjacent_statement_argument(f, *value)?;
        }

        // return
        Expression::Return { value } => {
            let _timing = f
                .context()
                .timing_scope(timing::FORMAT_EXPRESSION_STATEMENT_RETURN);
            format_return_expression(f, node_id, *value)?;
        }
        _ => return Ok(false),
    }

    Ok(true)
}
