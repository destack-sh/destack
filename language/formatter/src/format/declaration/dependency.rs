use crate::format::collection::{TrailingSeparator, separated_entries};
use crate::{DestackFormatContext, DestackFormatter, FormatNode};
use destack_ast::{
    Argument, DeclarationDescriptor, Declarator, DependencyItem, DependencyKind, DependencyMode,
    Expression, ImportSource, ImportTarget, Keyword, LocalNodeId, Name, NodeTree, Pattern,
    ScalarLiteral,
};
use destack_core::{ImmutableStringPool, StringId};
use destack_fir::format::{FormatError, FormatResult};
use destack_fir::prelude::*;
use destack_fir::write;
use destack_query::format::{
    ImportDeclarationKey, categorize_import, sort_dependency_items as query_sort_dependency_items,
    sort_import_declaration_indices,
};
use destack_workspace::ImportSortOrder;

use crate::format::collection::literal::format_scalar_literal;
use destack_source::Span;

/// Format a dependency item name.
fn format_dependency_item_name<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: Name,
) -> FormatResult<()> {
    match name {
        Name::Identifier(name) | Name::Number(name) => {
            write!(f, [name])?;
        }
        Name::String(name) => {
            let literal = ScalarLiteral::String(name);
            let span = Span::empty(f.context().file.id);
            format_scalar_literal(&literal, span, f)?;
        }
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: LocalNodeId<DependencyItem>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let DependencyItem::Item {
            mode,
            kind,
            name,
            alias,
            value: _,
        } = self
        else {
            return Ok(());
        };

        write!(
            f,
            [crate::format::annotation::prefix_annotations(
                f.context(),
                node_id
            )]
        )?;

        // type
        if *kind == Some(DependencyKind::Type) {
            write!(f, [Keyword::Type, space()])?;
        }

        let is_default_binding =
            *mode == DependencyMode::Default || (*mode == DependencyMode::Item && name.is_none());

        // default
        if is_default_binding {
            write!(f, [Keyword::Default])?;
            // alias
            if let Some(alias) = alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        // namespace
        else if *mode == DependencyMode::Namespace {
            write!(f, [token("*")])?;
            if let Some(alias) = alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        // item
        else {
            // name
            if let Some(name) = name {
                format_dependency_item_name(f, *name)?;
            }
            // alias
            if let Some(alias) = alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }

        write!(
            f,
            [crate::format::annotation::infix_or_postfix_annotations(
                f.context(),
                node_id
            )]
        )?;

        Ok(())
    }
}

/// Return the inner import expression, unwrapping statement wrappers when needed.
pub(crate) fn import_expression(
    expr_id: LocalNodeId<Expression>,
    tree: &NodeTree,
) -> Option<&Expression> {
    let expr = tree.get(expr_id);
    match expr {
        Expression::Import {
            source: ImportSource::ImportCall,
            ..
        } => None,
        Expression::Import { target, .. } => {
            if matches!(target, ImportTarget::String(_)) {
                Some(expr)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Check if an expression is an import (unwrapping Statement if needed).
pub(crate) fn is_import(expr_id: LocalNodeId<Expression>, tree: &NodeTree) -> bool {
    import_expression(expr_id, tree).is_some()
}

/// Format `export import ... = require(...)` when modeled as an export let.
pub(crate) fn format_export_import_equals_statement(
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
        ImportTarget::Expression { .. } => return Ok(false),
    };

    let alias = items
        .first()
        .and_then(|item| match tree.get(*item) {
            DependencyItem::Item { alias, .. } => *alias,
            DependencyItem::Error => None,
        })
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

/// Format one `export as namespace` statement.
pub(crate) fn format_export_namespace_statement<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    name: StringId,
) -> FormatResult<()> {
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
    )
}

/// Format one dependency-shaped statement expression.
pub(crate) fn format_dependency_statement_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    expression: &Expression,
) -> FormatResult<bool> {
    match expression {
        Expression::Import {
            source,
            kind,
            target,
            items,
            arguments,
        } => {
            format_import_expression(
                f,
                node_id,
                *source,
                *kind,
                target,
                items,
                arguments.as_deref(),
            )?;
            Ok(true)
        }
        Expression::Export {
            kind,
            target,
            items,
            arguments,
        } => {
            format_export_expression(f, node_id, *kind, *target, items, arguments.as_deref())?;
            Ok(true)
        }
        Expression::ExportNamespace { name } => {
            format_export_namespace_statement(f, *name)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Sort import expressions by group and then alphabetically within each group.
///
/// Side-effect imports (no items) preserve their relative order and stay at the top.
pub(crate) fn sort_imports(
    imports: &[LocalNodeId<Expression>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> Vec<LocalNodeId<Expression>> {
    let mut expression_ids = Vec::new();
    let mut order_keys = Vec::new();

    // collect sortable declaration keys
    for &expr_id in imports {
        if let Some(Expression::Import { items, target, .. }) = import_expression(expr_id, tree) {
            let ImportTarget::String(target) = target else {
                continue;
            };

            let target_str = strings.get(*target);
            expression_ids.push(expr_id);
            order_keys.push(ImportDeclarationKey {
                target: target_str,
                is_side_effect: items.is_empty(),
            });
        }
    }

    // map declaration order back to expression ids
    let order = sort_import_declaration_indices(&order_keys);
    order
        .into_iter()
        .map(|index| expression_ids[index])
        .collect()
}

/// Sort dependency items by kind and configured key order.
pub(crate) fn sort_dependency_items(
    items: &[LocalNodeId<DependencyItem>],
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    sort_order: ImportSortOrder,
) -> Vec<LocalNodeId<DependencyItem>> {
    query_sort_dependency_items(items, tree, strings, sort_order)
}

/// Determine if a blank line should be inserted between two imports.
///
/// Returns true if:
/// - Transitioning from side-effect to regular imports
/// - Different import groups (for regular imports)
pub(crate) fn should_insert_blank_between(
    prev_expr_id: LocalNodeId<Expression>,
    curr_expr_id: LocalNodeId<Expression>,
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> bool {
    let (prev_is_side_effect, prev_group) = match import_expression(prev_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let ImportTarget::String(target) = target else {
                return false;
            };
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    let (curr_is_side_effect, curr_group) = match import_expression(curr_expr_id, tree) {
        Some(Expression::Import { items, target, .. }) => {
            let ImportTarget::String(target) = target else {
                return false;
            };
            let target_str = strings.get(*target);
            (items.is_empty(), categorize_import(target_str))
        }
        _ => return false,
    };

    // blank line between side effect and regular imports
    if prev_is_side_effect && !curr_is_side_effect {
        return true;
    }

    // blank line between different groups (for regular imports)
    if !prev_is_side_effect && !curr_is_side_effect && prev_group != curr_group {
        return true;
    }

    false
}

/// Return whether call arguments span multiple lines in source.
fn call_arguments_are_multiline_span(
    context: &DestackFormatContext<'_>,
    dynamic_arguments: &[LocalNodeId<Argument>],
) -> bool {
    let (Some(first), Some(last)) = (dynamic_arguments.first(), dynamic_arguments.last()) else {
        return false;
    };

    let first_span = context.span(*first);
    let last_span = context.span(*last);
    if first_span.file != last_span.file || first_span.start >= last_span.end {
        return false;
    }

    context.has_newline(Span::new(first_span.file, first_span.start, last_span.end))
}

/// Return one dependency item's mode when it is valid.
fn dependency_item_mode(item: &DependencyItem) -> Option<DependencyMode> {
    match item {
        DependencyItem::Item { mode, .. } => Some(*mode),
        DependencyItem::Error => None,
    }
}

/// Return one dependency item's alias when it is valid.
fn dependency_item_alias(item: &DependencyItem) -> Option<destack_core::StringId> {
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
    let has_attribute_head_annotation = f.context().has_infix_annotation(node_id);
    if has_attribute_head_annotation {
        write!(
            f,
            [crate::format::annotation::block_infix_annotations(
                f.context(),
                node_id
            )]
        )?;
    }

    let should_expand_attribute_arguments =
        call_arguments_are_multiline_span(f.context(), arguments);
    let trailing_separator = match f.context().options.trailing_comma {
        destack_workspace::TrailingComma::None => TrailingSeparator::Omit,
        destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::All => {
            TrailingSeparator::Allowed
        }
    };
    let format_arguments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        if f.context().options.bracket_spacing {
            write!(f, [if_group_fits_on_line(&space())])?;
        }

        write!(
            f,
            [separated_entries(",", arguments, trailing_separator, None)]
        )?;

        if f.context().options.bracket_spacing {
            write!(f, [if_group_fits_on_line(&space())])?;
        }

        Ok(())
    });
    let with_arguments = format_with(|f: &mut DestackFormatter<'ast, '_>| {
        write!(
            f,
            [token("{"), soft_block_indent(&format_arguments), token("}")]
        )
    });
    let with_arguments = group(&with_arguments).should_expand(should_expand_attribute_arguments);

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
    organize_imports: bool,
    sort_order: ImportSortOrder,
    has_item_annotations: bool,
) -> Vec<LocalNodeId<DependencyItem>> {
    if organize_imports && !has_item_annotations {
        return sort_dependency_items(items, ctx.tree, ctx.strings, sort_order);
    }

    items.to_vec()
}

/// Write one dependency item collection list with stable expansion rules.
fn write_dependency_item_collection<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<DependencyItem>],
    should_expand: bool,
) -> FormatResult<()> {
    let trailing_separator = match f.context().options.trailing_comma {
        destack_workspace::TrailingComma::None => TrailingSeparator::Omit,
        destack_workspace::TrailingComma::Es5 | destack_workspace::TrailingComma::All => {
            TrailingSeparator::Allowed
        }
    };

    write!(
        f,
        [group(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
            write!(
                f,
                [
                    token("{"),
                    soft_block_indent(&format_with(|f: &mut DestackFormatter<'ast, '_>| {
                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        write!(f, [separated_entries(",", items, trailing_separator, None)])?;

                        if f.context().options.bracket_spacing {
                            write!(f, [if_group_fits_on_line(&space())])?;
                        }

                        Ok(())
                    })),
                    token("}")
                ]
            )
        }))
        .should_expand(should_expand)]
    )
}

/// Write one dependency item collection using shared output ordering options.
fn write_dependency_items_for_output<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    items: &[LocalNodeId<DependencyItem>],
    organize_imports: bool,
    sort_order: ImportSortOrder,
    has_item_annotations: bool,
) -> FormatResult<()> {
    let sorted_items = dependency_items_for_output(
        f.context(),
        items,
        organize_imports,
        sort_order,
        has_item_annotations,
    );
    write_dependency_item_collection(f, &sorted_items, has_item_annotations)
}

/// Write one quoted dependency source target.
fn write_dependency_target<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: destack_core::StringId,
) -> FormatResult<()> {
    write!(f, [token("\""), target, token("\"")])
}

/// Write one `from "<target>"` dependency source clause.
fn write_dependency_from_target_clause<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    target: destack_core::StringId,
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

                write_import_call_target(f, target)?;
                if total_items > 1 {
                    write!(f, [token(",")])?;
                }

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
    let has_item_annotations = dependency_items_have_annotations(f.context(), items);
    let organize_imports = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;

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

    write!(f, [Keyword::Import, space()])?;
    if source == ImportSource::ImportEquals {
        if kind == DependencyKind::Type {
            write!(f, [Keyword::Type, space()])?;
        }

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
    let first_item = items.first().map(|item| tree.get(*item));

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
    } else if let Some(first_item) = first_item
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
            write_dependency_items_for_output(
                f,
                rest_items,
                organize_imports,
                sort_order,
                has_item_annotations,
            )?;
        }
    } else if !items.is_empty() {
        write_dependency_items_for_output(
            f,
            items,
            organize_imports,
            sort_order,
            has_item_annotations,
        )?;
    } else if import_type_empty_items {
        write!(f, [token("{"), token("}")])?;
    }

    if !items.is_empty() || import_type_empty_items {
        write_dependency_from_target_clause(f, target)?;
    } else {
        write_dependency_target(f, target)?;
    }

    write_dependency_attribute_clause(f, node_id, arguments)?;

    Ok(())
}

/// Format an export expression.
pub(crate) fn format_export_expression<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    node_id: LocalNodeId<Expression>,
    kind: DependencyKind,
    target: Option<destack_core::StringId>,
    items: &[LocalNodeId<DependencyItem>],
    arguments: Option<&[LocalNodeId<Argument>]>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let has_item_annotations = dependency_items_have_annotations(f.context(), items);
    let organize_imports = f.context().options.organize_imports.is_enabled();
    let sort_order = f.context().options.import_sort_order;
    let mut needs_trailing_semicolon = false;

    write!(f, [Keyword::Export, space()])?;
    if kind == DependencyKind::Type {
        write!(f, [Keyword::Type, space()])?;
    }

    let export_empty_items_with_target = items.is_empty() && target.is_some();
    let first_item = items.first().map(|item| tree.get(*item));

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
        needs_trailing_semicolon = true;
    } else if items.len() == 1
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
            needs_trailing_semicolon = true;
        } else {
            write!(f, [token("*")])?;
            if let Some(alias) = dependency_item_alias(first_item) {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
    } else if let Some(first_item) = first_item
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
    } else if !items.is_empty() {
        write_dependency_items_for_output(
            f,
            items,
            organize_imports,
            sort_order,
            has_item_annotations,
        )?;
    } else if target.is_none() || export_empty_items_with_target {
        write!(f, [token("{"), token("}")])?;
    }

    if let Some(target) = target {
        write_dependency_from_target_clause(f, target)?;
    }

    write_dependency_attribute_clause(f, node_id, arguments)?;

    if needs_trailing_semicolon {
        write!(f, [token(";")])?;
    }

    Ok(())
}
