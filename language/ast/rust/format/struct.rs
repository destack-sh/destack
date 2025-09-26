use dyst_fir::format::FormatResult;
use dyst_fir::format_args;

use crate::{
    DystFormatter, FormatNode, Keyword, NodeId, Struct, StructField, StructStyle, Visibility,
    empty_block_with_infix_annotations,
};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, Struct> for Struct {
    fn format_node(
        &self,
        node_id: NodeId<Struct>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // split tuple / struct fields
        let tuple_fields: &[NodeId<StructField>] = if self.style == StructStyle::Tuple {
            &self.fields
        } else {
            &[]
        };
        let struct_fields: &[NodeId<StructField>] = if self.style == StructStyle::Struct {
            &self.fields
        } else {
            &[]
        };

        // visibility
        if let Some(visibility) = self.visibility {
            let keyword = match visibility {
                Visibility::Public => Keyword::Public,
                Visibility::Private => Keyword::Private,
            };
            write!(f, [keyword, space()])?;
        }

        // keyword
        write!(f, [Keyword::Struct])?;
        if let Some(representation_type) = self.representation_type {
            write!(f, [token("("), representation_type, token(")")])?;
        }

        // name
        if let Some(name) = self.name {
            write!(f, [space()])?;
            write!(f, [name])?;

            // static parameters
            if let Some(static_parameters) = &self.static_parameters
                && !static_parameters.is_empty()
            {
                write!(
                    f,
                    [group(&format_args![
                        token("<"),
                        soft_block_indent(&format_with(|f| f
                            .join_with(&format_args![
                                if_group_fits_on_line(&token(",")),
                                soft_line_break_or_space()
                            ])
                            .entries(static_parameters)
                            .finish())),
                        token(">")
                    ])]
                )?;
            }
        }

        // tuple
        if self.style == StructStyle::Tuple {
            if tuple_fields.is_empty() {
                write!(f, [token("("), token(")")])?;
            } else {
                write!(
                    f,
                    [group(&format_args![
                        token("("),
                        soft_block_indent(&format_with(|f| f
                            .join_with(&format_args![
                                if_group_fits_on_line(&token(",")),
                                soft_line_break_or_space()
                            ])
                            .entries(tuple_fields)
                            .finish())),
                        token(")")
                    ])]
                )?;
            }
        }

        // super types
        if let Some(super_types) = &self.super_types
            && !super_types.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token(": "),
                    soft_block_indent(&format_with(|f| f
                        .join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(super_types)
                        .finish()))
                ])]
            )?;
        }

        write!(f, [space()])?;

        // empty body
        if struct_fields.is_empty() && self.statements.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id)])?;
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
            return Ok(());
        }

        // body
        write!(f, [token("{"), hard_line_break()])?;

        // fields
        if !struct_fields.is_empty() {
            write!(
                f,
                [group(&format_args![block_indent(&format_with(|f| f
                    .join_with(hard_line_break())
                    .entries(struct_fields)
                    .finish())),])]
            )?;
        }

        // blank line between fields and statements
        if !struct_fields.is_empty() && !self.statements.is_empty() {
            write!(f, [hard_line_break()])?;
            if !f.context().has_blank_prefix_annotation(self.statements[0]) {
                write!(f, [empty_line()])?;
            }
        }

        // statements
        if !self.statements.is_empty() {
            write!(
                f,
                [group(&format_args![block_indent(&format_with(|f| f
                    .join_with(hard_line_break())
                    .entries(&self.statements)
                    .finish())),])]
            )?;
        }

        write!(f, [f.context().block_infix_annotations(node_id)])?;

        write!(f, [hard_line_break(), token("}")])?;

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

impl<'ast> FormatNode<'ast, StructField> for StructField {
    fn format_node(
        &self,
        node_id: NodeId<StructField>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // name
        if let Some(name) = self.name {
            write!(f, [name, token(": ")])?;
        }

        // type
        write!(f, [self.r#type])?;

        // default
        if let Some(default) = self.default {
            write!(f, [token(" = "), default])?;
        }

        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_struct_empty() {
        assert_format!(
            "struct { }",
            "struct { }",
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_fields() {
        assert_format!(
            "struct { a: int32, b: boolean }",
            "struct {\n\ta: int32\n\tb: boolean\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_name() {
        assert_format!(
            "struct Foo { a: int32 }",
            "struct Foo {\n\ta: int32\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_representation_type() {
        assert_format!(
            "struct(uint64) Foo { a: int32 }",
            "struct(uint64) Foo {\n\ta: int32\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_fields() {
        assert_format!(
            "struct Foo(int32, boolean) { }",
            "struct Foo(int32, boolean) { }",
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_struct_with_tuple_body_statements() {
        assert_format!(
            "struct Foo(int32) { let X = 2 }",
            "struct Foo(int32) {\n\tlet X = 2\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_defaults() {
        assert_format!(
            "struct { a: int32 = 42, b: boolean }",
            "struct {\n\ta: int32 = 42\n\tb: boolean\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_statements() {
        assert_format!(
            r"struct { let X = 1 }",
            r"struct {
	let X = 1
}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_fields_and_statements() {
        assert_format!(
            "struct { a: int32, let X = 1 }",
            "struct {\n\ta: int32\n\n\tlet X = 1\n}",
            |p| p.eat_struct(None),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_struct_with_static_parameters_and_super_types() {
        assert_format!(
            "struct Foo<T: Numeric>: Bar, Baz { }",
            "struct Foo<T: Numeric>: Bar, Baz { }",
            |p| p.eat_struct(None),
            DystFormatOptions::default()
        );
    }
}
