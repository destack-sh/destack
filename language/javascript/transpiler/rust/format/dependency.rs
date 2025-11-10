use dyst_ast::StringId;
use dyst_fir::format::FormatResult;
use dyst_fir::prelude::*;
use dyst_fir::write;
use dyst_javascript_ast::{DependencyItem, DependencyKind, Keyword, NodeId};

use crate::format::argument::list_like;
use crate::{FormatNode, JavaScriptFormatter};

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        _node_id: NodeId<DependencyItem>,
        f: &mut JavaScriptFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // type
        if self.kind == Some(DependencyKind::Type) {
            write!(f, [Keyword::Type, space()])?;
        }
        // name and alias
        write!(f, [self.name])?;
        if let Some(alias) = self.alias {
            write!(f, [space(), Keyword::As, space(), alias])?;
        }
        Ok(())
    }
}

/// Format a import binding (like `foo` or `{ bar, baz } from foo` or `* as foo from foo`).
pub(crate) fn format_dependency_binding<'ast>(
    f: &mut JavaScriptFormatter<'ast, '_>,
    target: Option<StringId>,
    alias: Option<StringId>,
    items: Option<&Vec<NodeId<DependencyItem>>>,
    include_glob: bool,
) -> FormatResult<()> {
    // items with maybe target
    if let Some(items) = items
        && (!items.is_empty() || target.is_none() && alias.is_none())
    {
        // items
        write!(f, [list_like("{", "}", ",", items).include_space()])?;
        // target
        if let Some(target) = target {
            write!(f, [space(), Keyword::From, space(), target])?;
        }
    }
    // target only
    else if let Some(target) = target {
        // alias as `* as foo`
        if let Some(alias) = alias {
            write!(
                f,
                [
                    token("*"),
                    space(),
                    Keyword::As,
                    space(),
                    alias,
                    space(),
                    Keyword::From,
                    space(),
                ]
            )?;
        } else if include_glob {
            write!(f, [token("*"), space(), Keyword::From, space()])?;
        }
        // target
        write!(f, [target])?;
    }

    Ok(())
}
