use dyst_language_fir::format::FormatResult;

use crate::{Argument, DystFormatter, FormatNode, NodeId, Parameter};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

impl<'ast> FormatNode<'ast, Parameter> for Parameter {
    fn format_node(
        &self,
        _node_id: NodeId<Parameter>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(_node_id)])?;
        // name
        write!(f, [self.name])?;
        // type
        if let Some(r#type) = self.r#type {
            write!(f, [token(": "), r#type])?;
        }
        // default
        if let Some(default) = self.default {
            write!(f, [token(" = "), default])?;
        }
        write!(f, [f.context().any_postfix_annotations(_node_id)])?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Argument> for Argument {
    fn format_node(
        &self,
        _node_id: NodeId<Argument>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [f.context().any_prefix_annotations(_node_id)])?;
        match self {
            Argument::Named { name, value } => {
                write!(f, [name, token(": "), value])?;
            }
            Argument::NamedShorthand { name } => {
                write!(f, [name])?;
            }
            Argument::Positional { value } => {
                write!(f, [value])?;
            }
        }
        write!(f, [f.context().any_postfix_annotations(_node_id)])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_parameter() {
        assert_format!(
            "x: int32",
            "x: int32",
            |p| p.eat_parameter(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_parameter_with_default() {
        assert_format!(
            "x: int32 = 1",
            "x: int32 = 1",
            |p| p.eat_parameter(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named() {
        assert_format!(
            "x: 1",
            "x: 1",
            |p| p.eat_argument(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_argument_named_shorthand() {
        assert_format!("x", "x", |p| p.eat_argument(), DystFormatOptions::default());
    }

    #[test]
    fn test_format_argument_positional() {
        assert_format!("1", "1", |p| p.eat_argument(), DystFormatOptions::default());
    }
}
