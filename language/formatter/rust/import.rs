use dyst_ast::Keyword;
use dyst_fir::format::FormatResult;
use dyst_source::StringId;

use crate::argument::list_like;
use crate::{DystFormatContext, DystFormatter, FormatNode, ImportItem, ImportTarget, NodeId};
use dyst_fir::format::Format;
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> Format<DystFormatContext<'ast>> for ImportTarget {
    #[inline]
    fn format(&self, f: &mut DystFormatter<'ast, '_>) -> FormatResult<()> {
        match self {
            ImportTarget::Path(path) => write!(f, [path]),
            ImportTarget::Virtual(string) => write!(f, [token("\""), string, token("\"")]),
        }
    }
}

impl<'ast> FormatNode<'ast, ImportItem> for ImportItem {
    fn format_node(
        &self,
        node_id: NodeId<ImportItem>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

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
pub(crate) fn format_import_binding<'ast>(
    f: &mut DystFormatter<'ast, '_>,
    target: Option<&ImportTarget>,
    alias: Option<StringId>,
    items: Option<&Vec<NodeId<ImportItem>>>,
) -> FormatResult<()> {
    // items with maybe target
    if let Some(items) = items
        && !items.is_empty()
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
        }
        // target
        write!(f, [target])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_import_simple() {
        assert_format!(
            "import foo",
            "import foo",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_alias() {
        assert_format!(
            "import foo as bar",
            "import * as bar from foo",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_items() {
        assert_format!(
            "import foo.{bar, baz}",
            "import { bar, baz } from foo",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }
    #[test]
    fn test_format_import_with_items_from() {
        assert_format!(
            "import {bar, baz} from foo",
            "import { bar, baz } from foo",
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_import_with_physical_target() {
        assert_format!(
            "import \"foo\"",
            "import \"foo\"",
            |p| p.eat_expression(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_overflow() {
        let source = r#"import {
    StructuredObject,
    StructuredObjectOptions,
    StructuredObjectOptions2,
    StructuredObjectOptions3,
} from lib"#;
        assert_format!(
            source,
            source,
            |p| p.eat_expression(),
            DystFormatOptions::default_with_line_width(60)
        );
    }
}
