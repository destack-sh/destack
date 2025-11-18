use dyst_ast::{DependencyItem, DependencyKind, Keyword, NodeId};
use dyst_fir::format::FormatResult;
use dyst_source::StringId;

use crate::argument::list_like;
use crate::{DystFormatter, FormatNode};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: NodeId<DependencyItem>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // type
        if self.kind == Some(DependencyKind::Type) {
            write!(f, [Keyword::Type, space()])?;
        }

        // name and alias
        write!(f, [self.name])?;
        if let Some(alias) = self.alias {
            write!(f, [space(), Keyword::As, space(), alias])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

/// Format a import binding (like `foo` or `{ bar, baz } from foo` or `* as foo from foo`).
pub(crate) fn format_dependency_binding<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    target: Option<&StringId>,
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
        write!(f, [token("\""), target, token("\"")])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{DystFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_import() {
        assert_format!(
            "import \"foo\"",
            "import \"foo\"",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_alias() {
        assert_format!(
            "import \"foo\" as bar",
            "import * as bar from \"foo\"",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_items_from() {
        assert_format!(
            "import {bar, baz} from \"foo\"",
            "import { bar, baz } from \"foo\"",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_import_with_overflow() {
        let source = r#"import {
    StructuredObject,
    StructuredObjectOptions,
    StructuredObjectOptions2,
    StructuredObjectOptions3,
} from "lib""#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_export_glob() {
        assert_format!(
            r#"export * from "./foo""#,
            r#"export * from "./foo""#,
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }
}
