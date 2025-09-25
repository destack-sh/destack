use dyst_fir::format::FormatResult;

use crate::{
    DystFormatter, FormatNode, Keyword, NodeId, StructStyle, Type, Union, UnionField, Visibility,
    empty_block_with_infix_annotations,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Union> for Union {
    fn format_node(
        &self,
        node_id: NodeId<Union>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // visibility
        if let Some(visibility) = self.visibility {
            let keyword = match visibility {
                Visibility::Public => Keyword::Public,
                Visibility::Private => Keyword::Private,
            };
            write!(f, [keyword, space()])?;
        }

        // keyword
        write!(f, [Keyword::Union])?;

        // tag and representation type
        if self.tag_type.is_some() || self.representation_type.is_some() {
            write!(f, [token("(")])?;
            let mut join = f.join_with(token(", "));
            if let Some(tag_type) = self.tag_type {
                join.entry(&tag_type);
            }
            if let Some(representation_type) = self.representation_type {
                join.entry(&representation_type);
            }
            join.finish()?;
            write!(f, [token(")")])?;
        }

        // name
        if let Some(name) = self.name {
            write!(f, [space()])?;
            write!(f, [name])?;
        }

        // static parameters
        if let Some(static_parameters) = &self.static_parameters
            && !static_parameters.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token("<"),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(static_parameters)
                        .finish()
                    })),
                    token(">")
                ])]
            )?;
        }

        // super types
        if let Some(super_types) = &self.super_types
            && !super_types.is_empty()
        {
            write!(
                f,
                [group(&format_args![
                    token(": "),
                    soft_block_indent(&format_with(|f| {
                        f.join_with(&format_args![
                            if_group_fits_on_line(&token(",")),
                            soft_line_break_or_space()
                        ])
                        .entries(super_types)
                        .finish()
                    }))
                ])]
            )?;
        }

        // space before body braces
        write!(f, [space()])?;

        // empty body (same line)
        if self.fields.is_empty() && self.statements.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id)])?;
            return Ok(());
        }

        // body
        write!(f, [token("{"), hard_line_break()])?;

        // fields
        if !self.fields.is_empty() {
            write!(
                f,
                [group(&format_args![block_indent(&format_with(|f| f
                    .join_with(hard_line_break())
                    .entries(&self.fields)
                    .finish())),])]
            )?;
        }

        // blank line between fields and statements
        if !self.fields.is_empty() && !self.statements.is_empty() {
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

        // body closing braces
        write!(f, [hard_line_break(), token("}")])
    }
}

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
        if let Some(ty) = self.r#type {
            let payload = f.context().get_node(ty).clone();
            match payload {
                Type::InlineStruct(struct_id) => {
                    let struct_ = f.context().get_node(struct_id).clone();
                    match struct_.style {
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
                                        .entries(&struct_.fields)
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
                                        .entries(&struct_.fields)
                                        .finish())),
                                    hard_line_break(),
                                    token("}")
                                ])]
                            )?;
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

        write!(f, [f.context().any_postfix_annotations(node_id)])?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
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
            "union Foo: Bar, Baz { }",
            "union Foo: Bar, Baz { }",
            |p| p.eat_union(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_union_with_statements() {
        assert_format!(
            "union Foo { let X = 1 }",
            "union Foo {\n\tlet X = 1\n}",
            |p| p.eat_union(None),
            DystFormatOptions::default_tab()
        );
    }
}
