use crate::{DestackFormatter, FormatNode};
use destack_ast::{DependencyItem, DependencyKind, DependencyMode, Keyword, LocalNodeId};
use destack_fir::format::FormatResult;
use destack_fir::prelude::*;
use destack_fir::write;

impl<'ast> FormatNode<'ast, DependencyItem> for DependencyItem {
    fn format_node(
        &self,
        node_id: LocalNodeId<DependencyItem>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // type
        if self.kind == Some(DependencyKind::Type) {
            write!(f, [Keyword::Type, space()])?;
        }

        // default
        if self.mode == DependencyMode::Default {
            write!(f, [Keyword::Default])?;
            // alias
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }
        // item
        else {
            // name
            write!(f, [self.name])?;
            // alias
            if let Some(alias) = self.alias {
                write!(f, [space(), Keyword::As, space(), alias])?;
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{DestackFormatOptions, TestFormatter, assert_format};

    #[test]
    fn test_format_import() {
        assert_format!(
            "import \"foo\"",
            "import \"foo\"",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_alias() {
        assert_format!(
            "import * as foo from \"foo\"",
            "import * as foo from \"foo\"",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_items_from() {
        assert_format!(
            "import {bar, baz} from \"foo\"",
            "import { bar, baz } from \"foo\"",
            |p| p.eat_expression(),
            DestackFormatOptions::default_with_line_width(60)
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
            DestackFormatOptions::default_with_line_width(60)
        );
    }

    #[test]
    fn test_format_export_glob() {
        assert_format!(
            r#"export * from "./foo""#,
            r#"export * from "./foo""#,
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_import_with_default_and_block() {
        assert_format!(
            "import Default, { type Item } from \"foo\"",
            "import Default, { type Item } from \"foo\"",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }

    #[test]
    fn test_format_export_with_default_and_block() {
        assert_format!(
            "export { default, default as bar, foo } from \"foo\"",
            "export { default, default as bar, foo } from \"foo\"",
            |p| p.eat_expression(),
            DestackFormatOptions::default()
        );
    }
}
