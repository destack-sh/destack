use dyst_fir::format::FormatResult;

use crate::{Definition, DystFormatter, Expression, FormatNode, NodeId, StructStyle, UnionField};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, UnionField> for UnionField {
    fn format_node(
        &self,
        node_id: NodeId<UnionField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        // name
        write!(f, [self.name])?;

        // payload type
        if let Some(ty) = self.ty {
            let payload = f.context().get_node(ty).clone();
            match payload {
                Expression::Definition(definition_id) => {
                    let definition = f.context().get_node(definition_id).clone();
                    match definition {
                        Definition::Struct { style, fields, .. } => match style {
                            StructStyle::Tuple => {
                                write!(
                                    f,
                                    [group(&format_args![
                                        token("("),
                                        soft_block_indent(&format_with(|f| {
                                            f.join_with(&format_with(|f| {
                                                if_group_fits_on_line(&token(",")).format(f)?;
                                                soft_line_break_or_space().format(f)
                                            }))
                                            .entries(&fields)
                                            .finish()
                                        })),
                                        token(")")
                                    ])]
                                )?;
                            }
                            StructStyle::Struct => {
                                write!(
                                    f,
                                    [group(&format_args![
                                        token(" {"),
                                        hard_line_break(),
                                        block_indent(&format_with(|f| f
                                            .join_with(hard_line_break())
                                            .entries(&fields)
                                            .finish())),
                                        hard_line_break(),
                                        token("}")
                                    ])]
                                )?;
                            }
                        },
                        _ => {
                            // space separated type? (error?)
                            write!(f, [space(), ty])?;
                        }
                    }
                }
                _ => {
                    // space separated type? (error?)
                    write!(f, [space(), ty])?;
                }
            }
        }

        // default value
        if let Some(value) = self.value {
            write!(f, [space(), token("="), space(), value])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_union_empty() {
        assert_format!(
            "union { }",
            "union { }",
            |p| p.eat_union(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_union_with_fields() {
        assert_format!(
            "union { A, B }",
            "union {\n\tA\n\tB\n}",
            |p| p.eat_union(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_union_with_value() {
        assert_format!(
            "union { A = 1 }",
            "union {\n\tA = 1\n}",
            |p| p.eat_union(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_union_with_tag_and_repr() {
        assert_format!(
            "union(uint4, uint60) Foo { A }",
            "union(uint4, uint60) Foo {\n\tA\n}",
            |p| p.eat_union(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_union_with_super_types() {
        assert_format!(
            "union Foo: (Bar, Baz) { }",
            "union Foo: Bar, Baz { }",
            |p| p.eat_union(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_union_with_super_types_breaks() {
        assert_format!(
            "union Foo: BarWithLongName, BazWithEvenLongerName, QuxWithLongestName { }",
            "union Foo: (\n\tBarWithLongName\n\tBazWithEvenLongerName\n\tQuxWithLongestName\n) { }",
            |p| p.eat_union(None),
            DystFormatOptions::default_tab_with_line_width(40)
        );
    }

    #[test]
    fn test_format_union_with_expressions() {
        assert_format!(
            "union Foo { let X = 1 }",
            "union Foo {\n\tlet X = 1\n}",
            |p| p.eat_union(None),
            DystFormatOptions::default_tab()
        );
    }
}
