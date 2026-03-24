use crate::format::analysis::timing;
use crate::format::annotation::statement_wrapper_needs_semicolon;
use crate::format::chain::expression_trivia_anchor_end;
use crate::format::declaration::dependency::sort_dependency_items;
use crate::format::directive::{
    FormatterDirective, FormatterDirectiveKind, FormatterDirectivePosition, directive_for_node,
};
use crate::format::expression::{
    Annotation, AnnotationPosition, Argument, Asynchrony, Block, DeclarationDescriptor,
    DeclarationKind, Declarator, DependencyItem, DependencyKind, DependencyMode,
    DestackFormatContext, DestackFormatter, Expression, ForEachBinding, ForEachDeclarationKind,
    ForEachKind, FormatResult, IfCondition, IfKind, ImportSource, Keyword, LetKind, LocalNodeId,
    Mutability, NodeTree, NodeType, Pattern, Span, StringId, TypeUnaryOperator, WhileKind,
    YieldCardinality, block_indent, call_arguments_are_multiline_span,
    detect_for_each_binding_keyword, format_declarator, format_expression,
    format_expression_without_prefix_annotations, format_for_each_binding_pattern,
    format_if_else_chain, format_match, format_statement_body_block, format_ternary, format_with,
    group, hard_line_break, is_empty_statement_block, line_postfix_boundary, list_like, space,
    token, tree_literal_should_break,
};
use destack_ast::{Comment, CommentStyle, Doc, DocumentationStyle, ImportTarget};
use destack_fir::format::{Buffer, Format, FormatError};
use destack_fir::write;
use destack_workspace::ImportSortOrder;

/// Return one dependency item's mode when it is valid.
fn dependency_item_mode(item: &DependencyItem) -> Option<DependencyMode> {
    match item {
        DependencyItem::Item { mode, .. } => Some(*mode),
        DependencyItem::Error => None,
    }
}

/// Return one dependency item's alias when it is valid.
fn dependency_item_alias(item: &DependencyItem) -> Option<StringId> {
    match item {
        DependencyItem::Item { alias, .. } => *alias,
        DependencyItem::Error => None,
    }
}

/// Return one dependency item's value when it is valid.
fn dependency_item_value(item: &DependencyItem) -> Option<LocalNodeId<Expression>> {
    match item {
        DependencyItem::Item { value, .. } => *value,
        DependencyItem::Error => None,
    }
}

/// Format `with { ... }` arguments for import and export statements.
fn format_dependency_with_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    arguments: &[LocalNodeId<Argument>],
) -> FormatResult<()> {
    // attribute head comments should stay attached to the with head, not drift to statement tails
    let has_attribute_head_annotation = f
        .context()
        .has_dependency_attribute_head_annotation(node_id);
    if has_attribute_head_annotation {
        write!(
            f,
            [f.context().dependency_attribute_head_annotations(node_id)]
        )?;
    }

    // source newlines inside `with` should expand the collection
    let should_expand_attribute_arguments =
        call_arguments_are_multiline_span(f.context(), arguments);
    let mut with_arguments = list_like("{", "}", ",", arguments);
    with_arguments
        .as_collection()
        .include_space()
        .should_expand(should_expand_attribute_arguments);

    if has_attribute_head_annotation {
        write!(f, [Keyword::With, space(), with_arguments])?;
        return Ok(());
    }

    write!(f, [space(), Keyword::With, space(), with_arguments])
}

/// Return whether any dependency item in one list has annotations.
fn dependency_items_have_annotations(
    ctx: &DestackFormatContext<'_>,
    items: &[LocalNodeId<DependencyItem>],
) -> bool {
    items.iter().any(|item| ctx.has_annotation(*item))
}

/// Return dependency items in output order with optional organize-imports sorting.
fn dependency_items_for_output(
    ctx: &DestackFormatContext<'_>,
    items: &[LocalNodeId<DependencyItem>],
    options: DependencyOutputOptions,
) -> Vec<LocalNodeId<DependencyItem>> {
    if options.organize_imports && !options.has_item_annotations {
        return sort_dependency_items(items, ctx.tree, ctx.strings, options.sort_order);
    }

    items.to_vec()
}

/// Store shared dependency item output options for import and export formatting.
#[derive(Copy, Clone)]
struct DependencyOutputOptions {
    /// Whether organize imports sorting is enabled for this file.
    organize_imports: bool,
    /// The configured organize imports sort order.
    sort_order: ImportSortOrder,
    /// Whether any dependency item is annotated and must retain source order.
    has_item_annotations: bool,
}

/// Build output options for one dependency item list.
fn dependency_output_options(
    ctx: &DestackFormatContext<'_>,
    items: &[LocalNodeId<DependencyItem>],
) -> DependencyOutputOptions {
    let has_item_annotations = dependency_items_have_annotations(ctx, items);
    let organize_imports = ctx.options.organize_imports.is_enabled();
    let sort_order = ctx.options.import_sort_order;

    DependencyOutputOptions {
        organize_imports,
        sort_order,
        has_item_annotations,
    }
}

/// Write one dependency-item collection list with stable expansion rules.
fn write_dependency_item_collection<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<DependencyItem>],
    should_expand: bool,
) -> FormatResult<()> {
    let mut items_list = list_like("{", "}", ",", items);
    items_list
        .as_collection()
        .include_space()
        .should_expand(should_expand);
    write!(f, [items_list])
}

/// Write one dependency item collection using shared output ordering options.
fn write_dependency_items_for_output<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<DependencyItem>],
    options: DependencyOutputOptions,
) -> FormatResult<()> {
    let sorted_items = dependency_items_for_output(f.context(), items, options);
    write_dependency_item_collection(f, &sorted_items, options.has_item_annotations)
}

/// Write one quoted dependency source target.
fn write_dependency_target<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: StringId,
) -> FormatResult<()> {
    write!(f, [token("\""), target, token("\"")])
}

/// Write one `from "<target>"` dependency source clause.
fn write_dependency_from_target_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: StringId,
) -> FormatResult<()> {
    write!(f, [space(), Keyword::From, space()])?;
    write_dependency_target(f, target)
}

/// Write one optional dependency attribute clause.
fn write_dependency_attribute_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    if let Some(arguments) = arguments {
        return format_dependency_with_arguments(f, node_id, arguments);
    }

    Ok(())
}

/// Return whether one import-call target should force expanded call arguments.
fn import_call_target_requires_expanded_arguments(
    ctx: &DestackFormatContext<'_>,
    target: &ImportTarget,
) -> bool {
    match target {
        ImportTarget::String(_) => false,
        ImportTarget::Expression { target } => {
            ctx.has_annotation(*target) || ctx.node_has_newline(*target)
        }
    }
}

/// Write one import-call target in either string or expression form.
fn write_import_call_target<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: &ImportTarget,
) -> FormatResult<()> {
    match target {
        ImportTarget::String(target) => write_dependency_target(f, *target),
        ImportTarget::Expression { target } => write!(f, [*target]),
    }
}

/// Write expanded import-call arguments with one target and optional `with` arguments.
fn write_expanded_import_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: &ImportTarget,
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    write!(f, [hard_line_break()])?;
    write!(
        f,
        [group(&block_indent(&format_with(
            |f: &mut DestackFormatter<'ast, '_>| {
                let arguments_len = arguments.map_or(0, |items| items.len());
                let total_items = 1usize + arguments_len;

                // target item
                write_import_call_target(f, target)?;
                if total_items > 1 {
                    write!(f, [token(",")])?;
                }

                // attribute-arguments items
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
    write!(f, [hard_line_break(), token(")")])
}

/// Write inline import-call arguments with one target and optional `with` arguments.
fn write_inline_import_call_arguments<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: &ImportTarget,
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    write_import_call_target(f, target)?;

    if let Some(arguments) = arguments {
        for argument in arguments {
            write!(f, [token(","), space(), *argument])?;
        }
    }

    write!(f, [token(")")])
}

/// Format one dynamic `import(...)` call expression and report whether it handled output.
fn format_import_call_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    source: ImportSource,
    target: &ImportTarget,
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<bool> {
    if source != ImportSource::ImportCall {
        return Ok(false);
    }

    write!(f, [Keyword::Import, token("(")])?;
    if import_call_target_requires_expanded_arguments(f.context(), target) {
        write_expanded_import_call_arguments(f, target, arguments)?;
    } else {
        write_inline_import_call_arguments(f, target, arguments)?;
    }

    Ok(true)
}

/// Format an import expression.
pub(crate) fn format_import_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    source: ImportSource,
    kind: DependencyKind,
    target: &ImportTarget,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let output_options = dependency_output_options(f.context(), items);

    // import call
    if format_import_call_expression(f, source, target, arguments)? {
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
        let alias = items
            .first()
            .and_then(|item| dependency_item_alias(tree.get(*item)))
            .ok_or(FormatError::SyntaxError {
                message: "import equals requires an alias",
            })?;
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
    if items.len() == 1
        && first_item
            .is_some_and(|item| dependency_item_mode(item) == Some(DependencyMode::Namespace))
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace import requires at least one dependency item",
            });
        };

        let namespace_alias =
            dependency_item_alias(first_item).ok_or(FormatError::SyntaxError {
                message: "namespace import requires an alias",
            })?;

        write!(
            f,
            [token("*"), space(), Keyword::As, space(), namespace_alias]
        )?;
    }
    // default + named imports
    else if let Some(first_item) = first_item
        && dependency_item_mode(first_item) == Some(DependencyMode::Default)
    {
        let default_alias = dependency_item_alias(first_item).ok_or(FormatError::SyntaxError {
            message: "default import requires an alias",
        })?;
        let rest_items = &items[1..];
        write!(f, [default_alias])?;

        if rest_items.len() == 1
            && dependency_item_mode(tree.get(rest_items[0])) == Some(DependencyMode::Namespace)
        {
            let namespace_item = tree.get(rest_items[0]);
            let namespace_alias =
                dependency_item_alias(namespace_item).ok_or(FormatError::SyntaxError {
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
            write!(f, [token(","), space()])?;
            write_dependency_items_for_output(f, rest_items, output_options)?;
        }
    }
    // named imports
    else if !items.is_empty() {
        write_dependency_items_for_output(f, items, output_options)?;
    } else if import_type_empty_items {
        write!(f, [token("{"), token("}")])?;
    }

    // from clause
    if !items.is_empty() || import_type_empty_items {
        write_dependency_from_target_clause(f, target)?;
    } else {
        write_dependency_target(f, target)?;
    }

    // attribute clause
    write_dependency_attribute_clause(f, node_id, arguments)?;

    Ok(())
}

/// Format an export expression.
pub(crate) fn format_export_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    kind: DependencyKind,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let output_options = dependency_output_options(f.context(), items);

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
        && first_item.is_some_and(|item| {
            dependency_item_mode(item) == Some(DependencyMode::Default)
                && dependency_item_value(item).is_some()
        })
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "default export requires at least one dependency item",
            });
        };
        let Some(value) = dependency_item_value(first_item) else {
            return Err(FormatError::SyntaxError {
                message: "default export requires a dependency value",
            });
        };
        write!(f, [Keyword::Default, space(), value])?;
    }
    // namespace export: export * as X, export = X
    else if items.len() == 1
        && first_item
            .is_some_and(|item| dependency_item_mode(item) == Some(DependencyMode::Namespace))
    {
        let Some(first_item) = first_item else {
            return Err(FormatError::SyntaxError {
                message: "namespace export requires at least one dependency item",
            });
        };

        if dependency_item_value(first_item).is_some() && target.is_none() {
            let Some(value) = dependency_item_value(first_item) else {
                return Err(FormatError::SyntaxError {
                    message: "namespace export assignment requires a dependency value",
                });
            };
            write!(f, [token("="), space(), value])?;
        } else {
            write!(f, [token("*")])?;
            if let Some(alias) = dependency_item_alias(first_item) {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
    }
    // named exports
    else if let Some(first_item) = first_item
        && dependency_item_mode(first_item) == Some(DependencyMode::Default)
        && items.len() == 2
        && target.is_some()
        && dependency_item_mode(tree.get(items[1])) == Some(DependencyMode::Namespace)
    {
        let default_alias = dependency_item_alias(first_item).ok_or(FormatError::SyntaxError {
            message: "default re-export requires an alias",
        })?;
        let namespace_item = tree.get(items[1]);
        let namespace_alias =
            dependency_item_alias(namespace_item).ok_or(FormatError::SyntaxError {
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
        write_dependency_items_for_output(f, items, output_options)?;
    } else if target.is_none() || export_empty_items_with_target {
        // empty export clause: `export {}`
        write!(f, [token("{"), token("}")])?;
    }

    // target
    if let Some(target) = target {
        write_dependency_from_target_clause(f, target)?;
    }

    // attribute clause
    write_dependency_attribute_clause(f, node_id, arguments)?;

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

    let alias = items
        .first()
        .and_then(|item| dependency_item_alias(tree.get(*item)))
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
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    if !ctx.has_postfix_annotation(expression_id) {
        return false;
    }

    ctx.visit_annotations(expression_id, |annotation_ids| {
        annotation_ids.iter().copied().any(|annotation_id| {
            let Annotation::Comment { node, position } = ctx.annotation(annotation_id) else {
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

            let comment = ctx.tree.get::<Comment>(node);
            if comment.style != CommentStyle::Star {
                return false;
            }

            ctx.has_newline(ctx.annotation_span(annotation_id))
        })
    })
    .unwrap_or(false)
}

/// Return whether one statement wrapper should delay semicolon emission to after postfix docs.
fn statement_wrapper_delays_semicolon_for_multiline_as_const_postfix(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    needs_semicolon: bool,
) -> bool {
    needs_semicolon
        && matches!(
            expression,
            Expression::TypeUnary {
                operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
                ..
            }
        )
        && expression_has_multiline_block_postfix_annotation(ctx, node_id)
}

/// Return whether wrapper postfix annotations should emit for one directive position.
fn statement_wrapper_should_emit_postfix_annotations(
    directive: Option<FormatterDirective>,
) -> bool {
    !matches!(
        directive,
        Some(FormatterDirective {
            kind: FormatterDirectiveKind::IgnoreFormat,
            position: FormatterDirectivePosition::Postfix { .. },
        })
    )
}

/// Return whether one wrapper expression emits its own edge annotations.
fn statement_wrapper_expression_handles_its_own_edge_annotations(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::If {
            kind: IfKind::If,
            ..
        }
    )
}

/// Return whether wrapper annotation emission should use postfix-only output.
fn statement_wrapper_uses_postfix_only_annotations(
    ctx: &DestackFormatContext<'_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> bool {
    if matches!(
        expression,
        Expression::TypeUnary {
            operator: TypeUnaryOperator::AsConst | TypeUnaryOperator::AsComptime,
            ..
        }
    ) {
        return true;
    }

    let call_or_new_handles_empty_infix = matches!(
        expression,
        Expression::Call {
            dynamic_arguments,
            ..
        }
        | Expression::New {
            dynamic_arguments,
            ..
        } if dynamic_arguments.is_empty() && ctx.has_infix_annotation(node_id)
    );
    if call_or_new_handles_empty_infix {
        return true;
    }

    ctx.has_infix_annotation(node_id)
        && (matches!(
            expression,
            Expression::ObjectExpression { properties, .. } if properties.is_empty()
        ) || matches!(
            expression,
            Expression::ArrayExpression { elements } if elements.is_empty()
        ))
}

/// Write wrapper infix and postfix annotations in the correct phase order.
fn write_statement_wrapper_non_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<()> {
    if statement_wrapper_uses_postfix_only_annotations(f.context(), node_id, expression) {
        return write!(
            f,
            [f.context()
                .any_postfix_except_line_postfix_boundary_annotations(node_id)]
        );
    }

    write!(
        f,
        [f.context()
            .any_infix_or_postfix_except_line_postfix_boundary_annotations(node_id)]
    )
}

/// Write one statement wrapper terminator and boundary annotation phase.
fn write_statement_wrapper_terminator_and_boundary_annotations<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    needs_semicolon: bool,
    semicolon_after_multiline_as_const_postfix: bool,
    should_emit_postfix_annotations: bool,
) -> FormatResult<()> {
    // statement terminator
    if needs_semicolon && !semicolon_after_multiline_as_const_postfix {
        write!(f, [token(";")])?;
    }

    // statement-level boundary comments print after the terminator
    // this matches direct statement-list formatting and prevents wrapper/non-wrapper churn
    if should_emit_postfix_annotations {
        write!(f, [f.context().line_postfix_boundary_annotations(node_id)])?;
    }

    Ok(())
}

/// Write one statement wrapper non-boundary annotation phase.
fn write_statement_wrapper_non_boundary_annotation_phase<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
    should_emit_postfix_annotations: bool,
) -> FormatResult<()> {
    let expression_handles_its_own_edge_annotations =
        statement_wrapper_expression_handles_its_own_edge_annotations(expression);

    // infix and postfix annotations
    if !expression_handles_its_own_edge_annotations && should_emit_postfix_annotations {
        write_statement_wrapper_non_boundary_annotations(f, node_id, expression)?;
    }

    Ok(())
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

    let semicolon_after_multiline_as_const_postfix =
        statement_wrapper_delays_semicolon_for_multiline_as_const_postfix(
            f.context(),
            node_id,
            expression,
            needs_semicolon,
        );
    let should_emit_postfix_annotations =
        statement_wrapper_should_emit_postfix_annotations(directive);

    // statement terminator and boundary annotations
    write_statement_wrapper_terminator_and_boundary_annotations(
        f,
        node_id,
        needs_semicolon,
        semicolon_after_multiline_as_const_postfix,
        should_emit_postfix_annotations,
    )?;

    // infix and postfix annotations
    write_statement_wrapper_non_boundary_annotation_phase(
        f,
        node_id,
        expression,
        should_emit_postfix_annotations,
    )?;

    // delayed semicolon for multiline `as const` postfix comments
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
    ctx: &DestackFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    let Some(annotations) = ctx.annotations(body) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            ctx.annotation(annotation_id),
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
    ctx: &DestackFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    let Some(annotations) = ctx.annotations(body) else {
        return false;
    };

    annotations.into_iter().any(|annotation_id| {
        matches!(
            ctx.annotation(annotation_id),
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
    ctx: &DestackFormatContext<'_>,
    body: LocalNodeId<Block>,
) -> bool {
    if block_has_block_prefix_annotation(ctx, body) {
        return false;
    }

    if block_has_line_prefix_annotation(ctx, body) {
        return true;
    }

    !is_empty_statement_block(ctx, body)
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
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation = ctx.annotation(annotation_id);
    let annotation_span = ctx.annotation_span(annotation_id);

    // block style comments/docs spanning multiple lines are leading comments
    match annotation {
        Annotation::Comment { node, .. } => {
            let comment = ctx.tree.get::<Comment>(node);
            comment.style == CommentStyle::Star && ctx.has_newline(annotation_span)
        }
        Annotation::Doc { node, .. } => {
            let doc = ctx.tree.get::<Doc>(node);
            doc.style == DocumentationStyle::Star && ctx.has_newline(annotation_span)
        }
        _ => false,
    }
}

/// Return whether one annotation id is followed by a newline before the next token.
fn annotation_is_followed_by_newline(
    ctx: &DestackFormatContext<'_>,
    annotation_id: LocalNodeId<Annotation>,
) -> bool {
    let annotation_span = ctx.annotation_span(annotation_id);
    let Some(next_token) = ctx.annotation_next_non_whitespace_token(annotation_id) else {
        return false;
    };
    if annotation_span.file != next_token.span.file {
        return false;
    }

    !ctx.file
        .is_same_line(annotation_span.end.saturating_sub(1), next_token.span.start)
}

/// Return whether one expression has leading prefix comment/doc annotations for adjacent wrapping.
fn expression_has_adjacent_leading_comment(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    ctx.visit_annotations(expression_id, |annotations| {
        annotations.iter().any(|annotation_id| {
            let annotation_id = *annotation_id;
            let annotation = ctx.annotation(annotation_id);
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

            annotation_is_multiline_block(ctx, annotation_id)
                || annotation_is_followed_by_newline(ctx, annotation_id)
        })
    })
    .unwrap_or(false)
}

/// Return the gap span between one member receiver and property token.
fn member_receiver_property_gap_span(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<Span> {
    let left_id = match ctx.tree.get(expression_id) {
        Expression::Member { left, .. } | Expression::PrivateMember { left, .. } => *left,
        _ => return None,
    };
    let property_span = ctx.tree.get_main_span(expression_id)?;
    let left_span = ctx.span(left_id);
    let left_anchor_end = expression_trivia_anchor_end(ctx, left_id);

    if left_span.file != property_span.file || property_span.start <= left_anchor_end {
        return None;
    }

    Some(Span::new(
        left_span.file,
        left_anchor_end,
        property_span.start,
    ))
}

/// Return whether one member expression has own-line or multiline comments between receiver and property.
fn member_has_leading_comment_between_receiver_and_property(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> bool {
    let Some(gap_span) = member_receiver_property_gap_span(ctx, expression_id) else {
        return false;
    };

    ctx.has_own_line_or_multiline_comment(gap_span)
}

/// Return the next left-side expression used for adjacent return/throw comment checks.
fn next_adjacent_argument_left_side(
    ctx: &DestackFormatContext<'_>,
    expression_id: LocalNodeId<Expression>,
) -> Option<LocalNodeId<Expression>> {
    match ctx.tree.get(expression_id) {
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
        Expression::Parenthesized { expression } => Some(*expression),
        _ => None,
    }
}

/// Return whether one adjacent statement argument has leading comments that require wrapping.
fn adjacent_statement_argument_has_leading_comments(
    ctx: &DestackFormatContext<'_>,
    argument_id: LocalNodeId<Expression>,
) -> bool {
    let argument_parent_is_yield =
        ctx.parent(argument_id)
            .is_some_and(|(parent_id, parent_type)| {
                parent_type == NodeType::Expression
                    && matches!(
                        ctx.tree.get(LocalNodeId::<Expression>::new(parent_id)),
                        Expression::Yield { .. }
                    )
            });

    let mut current_id = argument_id;
    loop {
        let has_adjacent_leading_comment = expression_has_adjacent_leading_comment(ctx, current_id);
        let has_member_gap_comment =
            member_has_leading_comment_between_receiver_and_property(ctx, current_id);

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

        let Some(next_id) = next_adjacent_argument_left_side(ctx, current_id) else {
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
    let sequence_value_id = match value_expression {
        Expression::SequenceExpression { .. } => Some(value_check_id),
        Expression::Parenthesized { expression }
            if matches!(
                f.context().tree.get(*expression),
                Expression::SequenceExpression { .. }
            ) =>
        {
            Some(*expression)
        }
        _ => None,
    };
    let value_has_leading_comment =
        adjacent_statement_argument_has_leading_comments(f.context(), value_id);
    if let Some(sequence_value_id) = sequence_value_id
        && value_has_leading_comment
    {
        let prefix_annotation_owner_id = if f.context().has_prefix_annotation(value_id) {
            Some(value_id)
        } else if f.context().has_prefix_annotation(sequence_value_id) {
            Some(sequence_value_id)
        } else {
            None
        };
        let grouped_sequence = format_with(|f: &mut DestackFormatter<'ast, '_>| {
            if let Some(prefix_annotation_owner_id) = prefix_annotation_owner_id {
                write!(
                    f,
                    [f.context()
                        .any_prefix_annotations(prefix_annotation_owner_id)]
                )?;
            }
            write!(f, [token("(")])?;
            format_expression_without_prefix_annotations(f, sequence_value_id)?;
            write!(f, [token(")")])
        });
        write!(
            f,
            [
                space(),
                token("("),
                block_indent(&grouped_sequence),
                hard_line_break(),
                token(")")
            ]
        )?;
        return Ok(());
    }

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
            write!(f, [label, token(":")])?;

            let body_expression = f.context().tree.get(*body);
            let body_is_empty_statement = matches!(
                body_expression,
                Expression::Block(block_id) if is_empty_statement_block(f.context(), *block_id)
            );
            let body_has_prefix_annotation = f.context().has_prefix_annotation(*body);
            if !body_is_empty_statement || body_has_prefix_annotation {
                write!(f, [space()])?;
            }

            match body_expression {
                Expression::Block(block_id) => {
                    format_statement_body_block(f, *block_id)?;
                }
                _ => {
                    write!(f, [*body])?;
                }
            }
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
            format_import_expression(
                f,
                node_id,
                *source,
                *kind,
                target,
                items,
                arguments.as_deref(),
            )?;
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
            format_export_expression(f, node_id, *kind, *target, items, arguments.as_deref())?;
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
