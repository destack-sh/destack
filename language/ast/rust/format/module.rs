use dyst_language_fir::format::{FormatResult, group};
use dyst_language_fir::{format_args, write};

use crate::{DystFormatter, FormatNode, Keyword, Module, ModuleFormat, NodeId};
use dyst_language_fir::prelude::*;

impl<'ast> FormatNode<'ast, Module> for Module {
    fn format_node(
        &self,
        _node_id: NodeId<Module>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // implicit module (whole file)
        match self.format {
            ModuleFormat::Implicit => {
                write!(
                    f,
                    [format_with(|f| f
                        .join_with(hard_line_break())
                        .entries(&self.statements)
                        .finish())]
                )
            }
            // declaration module (module x;)
            ModuleFormat::Forward => {
                write!(f, [Keyword::Module, space(), self.name])
            }
            // explicit module (module { ... })
            ModuleFormat::Inline => {
                // header
                if let Some(name) = self.name {
                    write!(f, [Keyword::Module, space(), name, space()])?;
                } else {
                    write!(f, [Keyword::Module, space()])?;
                }
                // empty body
                if self.statements.is_empty() {
                    write!(f, [token("{ }")])?;
                    return Ok(());
                }
                // body
                write!(
                    f,
                    [group(&format_args![
                        token("{"),
                        hard_line_break(),
                        format_with(|f| f
                            .join_with(hard_line_break())
                            .entries(&self.statements)
                            .finish()),
                        hard_line_break(),
                        token("}")
                    ])]
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_empty_anonymous_inline_module() {
        assert_format!(
            "module { }",
            "module { }",
            |p| p.eat_module(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_empty_named_forward_module() {
        assert_format!(
            "module x",
            "module x",
            |p| p.eat_module(None),
            DystFormatOptions::default()
        );
    }
}
