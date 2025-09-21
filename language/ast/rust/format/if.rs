use dyst_language_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, If, Keyword, NodeId};
use dyst_language_fir::prelude::*;
use dyst_language_fir::write;

// nocheckin: multiline ifs

impl<'ast> FormatNode<'ast, If> for If {
    fn format_node(
        &self,
        _node_id: NodeId<If>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            If::If {
                condition,
                then_block,
            } => {
                // if <condition> { <block> }
                write!(f, [Keyword::If, space(), *condition, space(), *then_block])
            }
            If::IfElse {
                condition,
                then_block,
                else_block,
            } => {
                // if <condition> { <block> } else { <block> }
                write!(f, [Keyword::If, space(), *condition, space(), *then_block])?;
                write!(f, [space(), Keyword::Else, space(), *else_block])
            }
            If::IfElseIf {
                condition,
                then_block,
                else_if,
            } => {
                // if <condition> { <block> } else if { <block> }
                write!(f, [Keyword::If, space(), *condition, space(), *then_block])?;
                write!(f, [space(), Keyword::Else, space(), *else_if])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::format::tests::TestFormatter;
    use crate::{DystFormatOptions, assert_format};

    #[test]
    fn test_format_if_basic() {
        assert_format!(
            "if true {}",
            "if true { }",
            |p| p.eat_if(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else() {
        assert_format!(
            "if true {} else {}",
            "if true { } else { }",
            |p| p.eat_if(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else_if() {
        assert_format!(
            "if true {} else if false {}",
            "if true { } else if false { }",
            |p| p.eat_if(),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_with_blocks() {
        assert_format!(
            "if cond { let X = 1 } else { let Y = 2 }",
            "if cond {\n\tlet X = 1\n} else {\n\tlet Y = 2\n}",
            |p| p.eat_if(),
            DystFormatOptions::default_tab()
        );
    }
}
