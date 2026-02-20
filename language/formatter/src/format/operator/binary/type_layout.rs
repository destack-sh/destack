use super::super::common::expression_is_trivial_inline_without_annotations;
use super::super::{BinaryOperands, DestackFormatContext};

/// Return whether type-binary operands are structurally complex enough to prefer multiline layout.
pub(super) fn type_binary_operands_are_structurally_complex(
    context: &DestackFormatContext<'_>,
    operands: &BinaryOperands,
) -> bool {
    if operands.len() > 3 {
        return true;
    }

    operands.iter().any(|operand| {
        context.node_has_newline(operand.expression)
            || !expression_is_trivial_inline_without_annotations(context, operand.expression)
    })
}
