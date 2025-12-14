use crate::{DependencyItem, DependencyKind, DependencyMode, Keyword, LocalNodeId};
use destack_base::StringId;
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

use crate::format::argument::list_like;
use crate::{CodegenJsFormatter, FormatNode};

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        _node_id: LocalNodeId<DependencyItem>,
        f: &mut CodegenJsFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // type
        if self.kind == Some(DependencyKind::Type) {
            write!(f, [Keyword::Type, space()])?;
        }

        // default mode
        if self.mode == DependencyMode::Default {
            write!(f, [Keyword::Default])?;
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        // item mode (regular)
        else {
            if let Some(name) = self.name {
                write!(f, [name])?;
            }
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        Ok(())
    }
}

/// Format an import binding (like `"foo"` or `{ bar, baz } from "foo"` or `* as foo from "foo"`).
pub(crate) fn format_import_binding<'ast>(
    f: &mut CodegenJsFormatter<'ast, '_>,
    target: StringId,
    items: &[LocalNodeId<DependencyItem>],
) -> FormatResult<()> {
    let tree = f.context().tree;
    let first_item = items.first().map(|item| tree.get(*item));

    // namespace (like `import * as foo from "bar"`)
    if items.len() == 1
        && let Some(first_item) = first_item
        && first_item.mode == DependencyMode::Namespace
    {
        write!(
            f,
            [token("*"), space(), Keyword::As, space(), first_item.alias]
        )?;
    }
    // items
    else {
        // default (like `import foo, { bar } from "baz"`)
        if let Some(first_item) = first_item
            && first_item.mode == DependencyMode::Default
        {
            write!(f, [first_item.alias, token(","), space()])?;
            let rest_items: Vec<LocalNodeId<DependencyItem>> =
                items.iter().skip(1).copied().collect();
            if !rest_items.is_empty() {
                write!(f, [list_like("{", "}", ",", &rest_items).include_space()])?;
            }
        }
        // regular items (like `import { foo, bar } from "baz"`)
        else if !items.is_empty() {
            let items_vec: Vec<LocalNodeId<DependencyItem>> = items.to_vec();
            write!(f, [list_like("{", "}", ",", &items_vec).include_space()])?;
        }
    }

    // from target
    if !items.is_empty() {
        write!(f, [space(), Keyword::From, space()])?;
    }
    write!(f, [target])?;

    Ok(())
}

/// Format an export binding (like `{ bar, baz }` or `{ bar } from "foo"` or `* from "foo"`).
pub(crate) fn format_export_binding<'ast>(
    f: &mut CodegenJsFormatter<'ast, '_>,
    target: Option<StringId>,
    items: &[LocalNodeId<DependencyItem>],
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
        && first_item.mode == DependencyMode::Namespace
    {
        write!(f, [token("*")])?;
        if let Some(alias) = first_item.alias {
            write!(f, [space(), Keyword::As, space(), alias])?;
        }
    }
    // items (like `export { foo, bar }`)
    else if !items.is_empty() {
        let items_vec: Vec<LocalNodeId<DependencyItem>> = items.to_vec();
        write!(f, [list_like("{", "}", ",", &items_vec).include_space()])?;
    }

    // from target
    if let Some(target) = target {
        write!(f, [space(), Keyword::From, space(), target])?;
    }

    Ok(())
}
