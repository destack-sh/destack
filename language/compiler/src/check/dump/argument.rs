use destack_dir as dir;

use crate::check::{Dump, DumpContext, GenericArgument, GenericInstance};

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
            Self::TypeOrStatic { source } => dump_record(
                "GenericArgument.TypeOrStatic",
                [
                    (
                        "source",
                        context.node_label(source.source.clone().into_any()),
                    ),
                    ("ty", source.ty.dump(context)),
                    (
                        "static",
                        source
                            .r#static
                            .map(|operand| operand.dump(context))
                            .unwrap_or_else(|| "none".to_string()),
                    ),
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
            Self::SpreadTypeOrStatic { source } => dump_record(
                "GenericArgument.SpreadTypeOrStatic",
                [
                    (
                        "source",
                        context.node_label(source.source.clone().into_any()),
                    ),
                    ("ty", source.ty.dump(context)),
                    (
                        "static",
                        source
                            .r#static
                            .map(|operand| operand.dump(context))
                            .unwrap_or_else(|| "none".to_string()),
                    ),
                ],
            ),
        }
    }
}

impl Dump for GenericInstance {
    /// Render one generic instance.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "GenericInstance",
            [
                ("template", context.generic_template_label(self.template)),
                ("arguments", dump_arguments(&self.arguments, context)),
            ],
        )
    }
}

impl Dump for dir::GenericInstance {
    /// Render one committed generic instance.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        let arguments = self
            .arguments
            .iter()
            .map(|argument| argument.dump(context))
            .collect::<Vec<_>>()
            .join(",");

        dump_record(
            "GenericInstance",
            [
                ("template", context.generic_template_label(self.template)),
                ("arguments", dump_list(arguments)),
            ],
        )
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
