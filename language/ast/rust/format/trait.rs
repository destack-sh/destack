use dyst_fir::format::FormatResult;

use crate::{
    DystFormatter, FormatNode, Keyword, NodeId, Trait, Visibility,
    empty_block_with_infix_annotations,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Trait> for Trait {
    fn format_node(
        &self,
        node_id: NodeId<Trait>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // visibility
        if let Some(visibility) = self.visibility {
            let keyword = match visibility {
                Visibility::Public => Keyword::Public,
                Visibility::Private => Keyword::Private,
            };
            write!(f, [keyword, space()])?;
        }

        // keyword
        write!(f, [Keyword::Trait])?;

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
                        f.join_with(&format_with(|f| {
                            if_group_fits_on_line(&token(",")).format(f)?;
                            soft_line_break_or_space().format(f)
                        }))
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
                        f.join_with(&format_with(|f| {
                            if_group_fits_on_line(&token(",")).format(f)?;
                            soft_line_break_or_space().format(f)
                        }))
                        .entries(super_types)
                        .finish()
                    }))
                ])]
            )?;
        }

        // with clauses
        if !self.withs.is_empty() {
            for with in &self.withs {
                write!(f, [space(), *with])?;
            }
        }

        // space before trait body
        write!(f, [space()])?;

        // empty body
        if self.statements.is_empty() {
            write!(f, [empty_block_with_infix_annotations(node_id)])?;
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
            return Ok(());
        }

        // body
        write!(f, [token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&self.statements)
                .finish())),])]
        )?;
        write!(f, [f.context().block_infix_annotations(node_id)])?;
        write!(f, [hard_line_break(), token("}")])?;

        write!(f, [f.context().any_postfix_annotations(node_id)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_trait_empty() {
        assert_format!(
            "trait {}",
            "trait { }",
            |p| p.eat_trait(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_trait_with_supers() {
        assert_format!(
            "trait Foo: Bar, Baz {}",
            "trait Foo: Bar, Baz { }",
            |p| p.eat_trait(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_trait_with_with_clause() {
        assert_format!(
            "trait Foo with Bar { }",
            "trait Foo with Bar { }",
            |p| p.eat_trait(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_trait_with_body() {
        assert_format!(
            "trait Foo { let X = 1 }",
            "trait Foo {\n\tlet X = 1\n}",
            |p| p.eat_trait(None),
            DystFormatOptions::default_tab()
        );
    }
}
