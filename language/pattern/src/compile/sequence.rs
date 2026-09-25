use tspp_dir as dir;
use tspp_source::Span;

use crate::{MetavariableUse, MetavariableUses, Sequence};

/// Adjacent repeated markers without a unique candidate partition.
pub(super) struct SequenceError {
    /// The second repeated marker.
    pub(super) span: Span,
}

impl MetavariableUses {
    /// Reject repeated markers without a deterministic partition.
    pub(super) fn validate_repeated(&self, tree: &dir::Tree) -> Result<(), SequenceError> {
        for metavariable_use in self.iter() {
            let MetavariableUse::Nodes { node, .. } = metavariable_use else {
                continue;
            };
            let Some(sequence) = Sequence::find(tree, *node) else {
                continue;
            };
            self.validate_list(sequence.nodes())?;
        }

        Ok(())
    }

    /// Reject adjacent repeated markers in one ordered node list.
    fn validate_list(&self, nodes: &[dir::LocalNodeIdAny]) -> Result<(), SequenceError> {
        for pair in nodes.windows(2) {
            let left = self.get_node(pair[0]);
            let right = self.get_node(pair[1]);
            let (Some(MetavariableUse::Nodes { .. }), Some(right @ MetavariableUse::Nodes { .. })) =
                (left, right)
            else {
                continue;
            };
            let span = right.span();

            return Err(SequenceError { span });
        }

        Ok(())
    }
}
