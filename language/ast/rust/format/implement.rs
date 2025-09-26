use dyst_fir::format::FormatResult;

use crate::{
    DystFormatter, FormatNode, Implement, Keyword, NodeId, empty_block_with_infix_annotations,
};
use dyst_fir::prelude::*;
use dyst_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Implement> for Implement {
    fn format_node(
        &self,
        node_id: NodeId<Implement>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(node_id)])?;

        // keyword
        write!(f, [Keyword::Implement])?;

        // static arguments
        if let Some(static_arguments) = &self.static_arguments
            && !static_arguments.is_empty()
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
                        .entries(static_arguments)
                        .finish()
                    })),
                    token(">")
                ])]
            )?;
        }

        // receiver
        write!(f, [space(), self.receiver])?;

        // for clause
        if let Some(for_trait) = self.for_trait {
            write!(f, [space(), Keyword::For, space(), for_trait])?;
        }

        // body
        if self.statements.is_empty() {
            write!(f, [space(), empty_block_with_infix_annotations(node_id)])?;
            write!(f, [f.context().any_postfix_annotations(node_id)])?;
            return Ok(());
        }

        // body
        write!(f, [space(), token("{"), hard_line_break()])?;
        write!(
            f,
            [group(&format_args![block_indent(&format_with(|f| f
                .join_with(hard_line_break())
                .entries(&self.statements)
                .finish())),])]
        )?;
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
    fn test_format_implement_empty() {
        assert_format!(
            "implement Foo {}",
            "implement Foo { }",
            |p| p.eat_implement(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_implement_with_for() {
        assert_format!(
            "implement Foo for Bar { let X = 1 }",
            "implement Foo for Bar {\n\tlet X = 1\n}",
            |p| p.eat_implement(),
            DystFormatOptions::default_tab()
        );
    }

    #[test]
    fn test_format_implement_with_static_arguments() {
        assert_format!(
            "implement<T> Foo<T> { }",
            "implement<T> Foo<T> { }",
            |p| p.eat_implement(),
            DystFormatOptions::default()
        );
    }
}
