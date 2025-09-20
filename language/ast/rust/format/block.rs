use dyst_language_fir::format::FormatResult;

use crate::{Block, Break, Continue, Defer, DystFormatter, FormatNode, Keyword, NodeId, Return};
use dyst_language_fir::prelude::*;
use dyst_language_fir::{format_args, write};

impl<'ast> FormatNode<'ast, Block> for Block {
    fn format_node(
        &self,
        _node_id: NodeId<Block>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        // label
        if let Some(label) = &self.label {
            write!(f, [label, token(": ")])?;
        }
        // statements
		if self.statements.is_empty() {
			write!(f, [token("{ }")])?;
			return Ok(());
		}
        write!(
            f,
            [group(&format_args![
                token("{"),
                hard_line_break(),
                block_indent(&format_with(|f| f
                    .join_with(hard_line_break())
                    .entries(&self.statements)
                    .finish())),
                hard_line_break(),
                token("}")
            ])]
        )?;
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Break> for Break {
    fn format_node(
        &self,
        _node_id: NodeId<Break>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Break])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [token(": "), label])?;
        }
        // value
        if let Some(value) = &self.value {
            write!(f, [token(" "), value])?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Continue> for Continue {
    fn format_node(
        &self,
        _node_id: NodeId<Continue>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Continue])?;
        // label
        if let Some(label) = &self.label {
            write!(f, [token(": "), label])?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Return> for Return {
    fn format_node(
        &self,
        _node_id: NodeId<Return>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Return])?;
        // value
        if let Some(value) = &self.value {
            write!(f, [token(" "), value])?;
        }
        Ok(())
    }
}

impl<'ast> FormatNode<'ast, Defer> for Defer {
    fn format_node(
        &self,
        _node_id: NodeId<Defer>,
        f: &mut DystFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [Keyword::Defer])?;
        match self {
            Defer::Expression(expression) => {
                write!(f, [token(" "), expression])?;
            }
            Defer::Block(block) => {
                write!(f, [token(" "), block])?;
            }
        }
        Ok(())
    }
}
