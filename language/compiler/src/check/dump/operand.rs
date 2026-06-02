use crate::check::{Dump, DumpContext, StaticOperand, TypeOperand};

use super::format::dump_record;

impl Dump for TypeOperand {
    /// Render one type operand.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Variable(variable) => {
                dump_record("TypeOperand.Variable", [("value", variable.dump(context))])
            }
            Self::Term(term) => dump_record(
                "TypeOperand.Term",
                [("value", context.check.term(*term).dump(context))],
            ),
            Self::Type(ty) => dump_record("TypeOperand.Type", [("value", context.type_label(*ty))]),
        }
    }
}

impl Dump for StaticOperand {
    /// Render one static operand.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Variable(variable) => dump_record(
                "StaticOperand.Variable",
                [("value", variable.dump(context))],
            ),
            Self::Term(term) => dump_record(
                "StaticOperand.Term",
                [("value", context.check.term(*term).dump(context))],
            ),
            Self::Static(value) => dump_record(
                "StaticOperand.Static",
                [("value", context.static_label(*value))],
            ),
        }
    }
}

/// Render one type operand list.
pub(super) fn dump_type_operands(
    operands: &[TypeOperand],
    context: &DumpContext<'_, '_>,
) -> String {
    operands
        .iter()
        .map(|operand| operand.dump(context))
        .collect::<Vec<_>>()
        .join(",")
}
