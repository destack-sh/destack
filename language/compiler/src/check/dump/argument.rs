use crate::check::{Dump, DumpContext, GenericArgument};

use super::format::{dump_list, dump_record};

impl Dump for GenericArgument {
    /// Render one generic argument.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Type(operand) => {
                dump_record("GenericArgument.Type", [("value", operand.dump(context))])
            }
            Self::Static(operand) => {
                dump_record("GenericArgument.Static", [("value", operand.dump(context))])
            }
            Self::AssociatedType { name, value } => {
                let name = context.string(*name);

                dump_record(
                    "GenericArgument.AssociatedType",
                    [("name", name), ("value", value.dump(context))],
                )
            }
            Self::AssociatedConst { name, value } => {
                let name = context.string(*name);

                dump_record(
                    "GenericArgument.AssociatedConst",
                    [("name", name), ("value", value.dump(context))],
                )
            }
            Self::TypeOrStatic { value } => dump_record(
                "GenericArgument.TypeOrStatic",
                [
                    ("type", value.ty.dump(context)),
                    ("static", value.value.dump(context)),
                ],
            ),
            Self::SpreadType(operand) => dump_record(
                "GenericArgument.SpreadType",
                [("value", operand.dump(context))],
            ),
            Self::SpreadStatic(operand) => dump_record(
                "GenericArgument.SpreadStatic",
                [("value", operand.dump(context))],
            ),
            Self::SpreadTypeOrStatic { value } => dump_record(
                "GenericArgument.SpreadTypeOrStatic",
                [
                    ("type", value.ty.dump(context)),
                    ("static", value.value.dump(context)),
                ],
            ),
        }
    }
}

/// Render one generic argument list.
pub(super) fn dump_arguments(
    arguments: &[GenericArgument],
    context: &DumpContext<'_, '_>,
) -> String {
    let arguments = arguments
        .iter()
        .map(|argument| argument.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(arguments)
}
