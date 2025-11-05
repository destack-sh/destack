use dyst_fir::format::FormatResult;

use crate::argument::list_like;
use crate::{DystFormatter, FormatNode, NodeId, UnionField};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, UnionField> for UnionField {
    fn format_node(
        &self,
        node_id: NodeId<UnionField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        match self {
            UnionField::Unit { name, value } => {
                write!(f, [name])?;
                // discriminator value
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
            }
            UnionField::Tuple {
                name,
                fields,
                value,
            } => {
                write!(f, [name])?;
                // fields
                if fields.is_empty() {
                    write!(f, [token("("), token(")")])?;
                } else {
                    write!(f, [list_like("(", ")", ",", fields)])?;
                }
                // discriminator value
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
            }
            UnionField::Struct {
                name,
                fields,
                value,
            } => {
                write!(f, [name])?;
                // fields
                if fields.is_empty() {
                    write!(f, [token("{"), token("}")])?;
                } else {
                    write!(f, [list_like("{", "}", ",", fields)])?;
                }
                // discriminator value
                if let Some(value) = value {
                    write!(f, [space(), token("="), space(), value])?;
                }
            }
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{LanguageFormatOptions, assert_format};
    use dyst_ast::DefinitionMeta;

    #[test]
    fn test_format_union_empty() {
        assert_format!(
            "union { }",
            "union { }",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default()
        );
    }

    #[test]
    fn test_format_union_with_fields() {
        assert_format!(
            "union { A, B }",
            "union {\n\tA\n\tB\n}",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_union_with_value() {
        assert_format!(
            "union { A = 1 }",
            "union {\n\tA = 1\n}",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_union_with_tag_and_repr() {
        assert_format!(
            "union(uint4, uint60) Foo { A }",
            "union(uint4, uint60) Foo {\n\tA\n}",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_union_with_extends_and_implements() {
        assert_format!(
            "union Foo extends (Bar, Baz) implements Qux { }",
            "union Foo extends Bar, Baz implements Qux { }",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default()
        );
    }

    #[test]
    fn test_format_union_with_extends_types_breaks() {
        assert_format!(
            "union Foo extends BarWithLongName, BazWithEvenLongerName, QuxWithLongestName { }",
            "union Foo extends (\n\tBarWithLongName,\n\tBazWithEvenLongerName,\n\tQuxWithLongestName,\n) { }",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_union_with_expressions() {
        assert_format!(
            "union Foo { const X = 1 }",
            "union Foo {\n\tconst X = 1\n}",
            |p| p.eat_union(DefinitionMeta::default()),
            LanguageFormatOptions::default_tab()
        );
    }
}
