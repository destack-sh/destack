use crate::annotation::{infix_or_postfix_annotations, prefix_annotations};
use crate::operator::format_declarator_assignment;
use crate::{DestackFormatter, FormatNode};
use destack_ast::{Declarator, LocalNodeId, NodeTree};
use destack_fir::format::{Buffer, FormatResult};
use destack_fir::write;

/// Format one declarator through the shared assignment-like owner.
pub(crate) fn format_declarator<'ast>(
    f: &mut DestackFormatter<'ast, '_>,
    _tree: &NodeTree,
    node_id: LocalNodeId<Declarator>,
) -> FormatResult<()> {
    format_declarator_assignment(f, node_id)
}

impl<'ast> FormatNode<'ast, Declarator> for Declarator {
    fn format_node(
        &self,
        node_id: LocalNodeId<Declarator>,
        f: &mut DestackFormatter<'ast, '_>,
    ) -> FormatResult<()> {
        write!(f, [prefix_annotations(f.context(), node_id)])?;

        format_declarator(f, f.context().tree, node_id)?;

        write!(f, [infix_or_postfix_annotations(f.context(), node_id)])?;

        Ok(())
    }
}
