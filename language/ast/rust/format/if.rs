use dyst_fir::format::FormatResult;

use crate::{DystFormatter, FormatNode, If, Keyword, NodeId, Runtime};
use dyst_fir::prelude::*;
use dyst_fir::write;

impl<'ast> FormatNode<'ast, If> for If {
    fn format_node(
        &self,
        node_id: NodeId<If>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        match self {
            If::If {
                runtime,
                condition,
                then_block,
            } => {
                write!(
                    f,
                    [group(&format_with(|f: &mut DystFormatter<'ast, '_>| {
                        write!(f, [f.context().any_prefix_annotations(node_id)])?;
                        // if <condition> { <block> }
                        if let Some(runtime) = runtime
                            && *runtime == Runtime::Static
                        {
                            write!(f, [token("@")])?;
                        }
                        write!(f, [Keyword::If, space(), *condition, space(), *then_block])?;
                        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
                        Ok(())
                    }))]
                )?;
                Ok(())
            }
            If::IfElse {
                runtime,
                condition,
                then_block,
                else_block,
            } => {
                // if <condition> { <block> } else { <block> }
                write!(
                    f,
                    [group(&format_with(|f: &mut DystFormatter<'ast, '_>| {
                        write!(f, [f.context().any_prefix_annotations(node_id)])?;
                        if let Some(runtime) = runtime
                            && *runtime == Runtime::Static
                        {
                            write!(f, [token("@")])?;
                        }
                        write!(f, [Keyword::If])?;
                        write!(f, [space()])?;
                        write!(f, [*condition])?;
                        write!(f, [space()])?;
                        write!(f, [*then_block])?;
                        if !f.context().has_postfix_annotation(*then_block) {
                            write!(f, [space()])?;
                        }
                        write!(f, [Keyword::Else])?;
                        write!(f, [space()])?;
                        write!(f, [*else_block])?;
                        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
                        Ok(())
                    }))]
                )?;
                Ok(())
            }
            If::IfElseIf {
                runtime,
                condition,
                then_block,
                else_if,
            } => {
                // if <condition> { <block> } else if { <block> }
                write!(
                    f,
                    [group(&format_with(|f: &mut DystFormatter<'ast, '_>| {
                        write!(f, [f.context().any_prefix_annotations(node_id)])?;
                        if let Some(runtime) = runtime
                            && *runtime == Runtime::Static
                        {
                            write!(f, [token("@")])?;
                        }
                        write!(f, [Keyword::If])?;
                        write!(f, [space()])?;
                        write!(f, [*condition])?;
                        write!(f, [space()])?;
                        write!(f, [*then_block])?;
                        if !f.context().has_postfix_annotation(*then_block) {
                            write!(f, [space()])?;
                        }
                        write!(f, [Keyword::Else])?;
                        write!(f, [space()])?;
                        write!(f, [*else_if])?;
                        write!(f, [f.context().any_infix_or_postfix_annotations(node_id)])?;
                        Ok(())
                    }))]
                )?;
                Ok(())
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
            |p| p.eat_if(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else() {
        assert_format!(
            "if true {} else {}",
            "if true { } else { }",
            |p| p.eat_if(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else_if() {
        assert_format!(
            "if true {} else if false {}",
            "if true { } else if false { }",
            |p| p.eat_if(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_with_body() {
        assert_format!(
            "if cond { let X = 1 } else { let Y = 2 }",
            "if cond {\n\tlet X = 1\n} else {\n\tlet Y = 2\n}",
            |p| p.eat_if(None),
            DystFormatOptions::default_tab()
        );
    }

    /// All clauses of an if/else should break if any breaks.
    #[test]
    fn test_format_if_else_breaks_together() {
        let source = r"if cond1 {
    // comment inside cond1
    let X = 1
} else {
    let Y = 2
}";
        assert_format!(
            source,
            source,
            |p| p.eat_if(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_else_if_with_comments() {
        let source = r"if cond1 {
    // comment inside cond1
    let X = 1
}
// comment before cond2
else if cond2 {
    let Y = 2 // comment trailing Y
}
// comment before else
else {
    let Z = 3 // comment trailing Z
}";
        assert_format!(
            source,
            source,
            |p| p.eat_if(None),
            DystFormatOptions::default()
        );
    }

    #[test]
    fn test_format_if_let() {
        let source = r"if let Some(piece) = self.currentPiece {
    let absolutePositions = piece.getAbsolutePositions(pos)
    for blockPos in absolutePositions {
        if self.board.isFilled(blockPos) {
            return true
        }
    }
}";
        assert_format!(
            source,
            source,
            |p| p.eat_if(None),
            DystFormatOptions::default()
        );
    }
}
