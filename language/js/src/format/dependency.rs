use crate::{DependencyBinding, DependencyForm, DependencyItem, Keyword, LocalNodeId, Name};
use destack_core::StringId;
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;
use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::format::argument::list_like;
use crate::format::literal::format_string_literal_with_source_span;
use crate::{FormatNode, JsFormatter};

/// Format a dependency item name.
fn format_dependency_item_name<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    name: Name,
    source_span: Option<Span>,
) -> FormatResult<()> {
    match name {
        Name::Identifier(name) => {
            if let Some(source_span) = source_span {
                source_position(source_span.start).format(f)?;
            }

            write!(f, [name])?;

            if let Some(source_span) = source_span {
                source_position(source_span.end).format(f)?;
            }
        }
        Name::String(name) => {
            format_string_literal_with_source_span(name, source_span, f)?;
        }
    }

    Ok(())
}

/// Format a dependency item alias.
fn format_dependency_item_alias<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    alias: destack_core::StringId,
    source_span: Option<Span>,
) -> FormatResult<()> {
    if let Some(source_span) = source_span {
        source_position(source_span.start).format(f)?;
    }

    write!(f, [alias])?;

    if let Some(source_span) = source_span {
        source_position(source_span.end).format(f)?;
    }

    Ok(())
}

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: LocalNodeId<DependencyItem>,
        f: &mut JsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        let name_span = f
            .context()
            .source_part_span(node_id.id, NodeSpanType::Region(NodeSpanRegion::Type));
        let alias_span = f.context().source_part_span(node_id.id, NodeSpanType::Main);

        // type
        if self.form == Some(DependencyForm::Type) {
            write!(f, [Keyword::Type, space()])?;
        }

        // default binding
        if self.binding == DependencyBinding::Default {
            write!(f, [Keyword::Default])?;
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space()])?;
                format_dependency_item_alias(f, alias, alias_span)?;
            }
        }
        // named binding
        else {
            if let Some(name) = self.name {
                format_dependency_item_name(f, name, name_span)?;
            }
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space()])?;
                format_dependency_item_alias(f, alias, alias_span)?;
            }
        }
        Ok(())
    }
}

/// Format an import binding (like `"foo"` or `{ bar, baz } from "foo"` or `* as foo from "foo"`).
pub(crate) fn format_import_binding<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    target: StringId,
    items: &[LocalNodeId<DependencyItem>],
    target_span: Option<Span>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let first_item = items.first().map(|item| tree.get(*item));

    // namespace (like `import * as foo from "bar"`)
    if items.len() == 1
        && let Some(first_item) = first_item
        && first_item.binding == DependencyBinding::Namespace
    {
        write!(f, [token("*"), space(), Keyword::As, space()])?;

        if let Some(item_id) = items.first()
            && let Some(alias) = first_item.alias
        {
            let alias_span = f.context().source_part_span(item_id.id, NodeSpanType::Main);
            format_dependency_item_alias(f, alias, alias_span)?;
        }
    }
    // items
    else {
        // default (like `import foo, { bar } from "baz"`)
        if let Some(first_item) = first_item
            && first_item.binding == DependencyBinding::Default
        {
            if let Some(item_id) = items.first()
                && let Some(alias) = first_item.alias
            {
                let alias_span = f.context().source_part_span(item_id.id, NodeSpanType::Main);
                format_dependency_item_alias(f, alias, alias_span)?;
            }

            write!(f, [token(","), space()])?;
            let rest_items: Vec<LocalNodeId<DependencyItem>> =
                items.iter().skip(1).copied().collect();
            if !rest_items.is_empty() {
                write!(f, [list_like("{", "}", ",", &rest_items).include_space()])?;
            }
        }
        // named items
        else if !items.is_empty() {
            let items_vec: Vec<LocalNodeId<DependencyItem>> = items.to_vec();
            write!(f, [list_like("{", "}", ",", &items_vec).include_space()])?;
        }
    }

    // from target
    if !items.is_empty() {
        write!(f, [space(), Keyword::From, space()])?;
    }

    format_string_literal_with_source_span(target, target_span, f)?;

    Ok(())
}

/// Format an export binding (like `{ bar, baz }` or `{ bar } from "foo"` or `* from "foo"`).
pub(crate) fn format_export_binding<'ast>(
    f: &mut JsFormatter<'ast, '_>,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
    target_span: Option<Span>,
) -> FormatResult<()> {
    let tree = f.context().tree;
    let first_item = items.first().map(|item| tree.get(*item));

    // value export (like `export = foo`)
    if let Some(first_item) = first_item
        && let Some(value) = first_item.value
    {
        write!(f, [token("="), space(), value])?;
        return Ok(());
    }

    // namespace (like `export * from "foo"` or `export * as bar from "foo"`)
    if items.len() == 1
        && let Some(first_item) = first_item
        && first_item.binding == DependencyBinding::Namespace
    {
        write!(f, [token("*")])?;
        if let Some(item_id) = items.first()
            && let Some(alias) = first_item.alias
        {
            let alias_span = f.context().source_part_span(item_id.id, NodeSpanType::Main);
            write!(f, [space(), Keyword::As, space()])?;
            format_dependency_item_alias(f, alias, alias_span)?;
        }
    }
    // items (like `export { foo, bar }`)
    else if !items.is_empty() {
        let items_vec: Vec<LocalNodeId<DependencyItem>> = items.to_vec();
        write!(f, [list_like("{", "}", ",", &items_vec).include_space()])?;
    }

    // from target
    if let Some(target) = target {
        write!(f, [space(), Keyword::From, space()])?;

        format_string_literal_with_source_span(target, target_span, f)?;
    }

    Ok(())
}
