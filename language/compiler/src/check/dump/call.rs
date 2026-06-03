use crate::check::{
    CallCallee, CallTerm, ConstructTerm, Dump, DumpContext, MemberCallSource, MemberCallTerm,
};

use super::argument::dump_arguments;
use super::format::{dump_list, dump_record};
use super::operand::dump_type_operands;

impl Dump for CallTerm {
    /// Render one call term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        let argument_values = self
            .argument_values
            .iter()
            .map(|value| context.node_label(value.clone().into()))
            .collect::<Vec<_>>()
            .join(",");

        dump_record(
            "CallTerm",
            [
                ("source", context.node_label(self.source)),
                ("callee", self.callee.dump(context)),
                (
                    "generic_arguments",
                    dump_arguments(&self.generic_arguments, context),
                ),
                ("arguments", dump_type_operands(&self.arguments, context)),
                ("argument_values", dump_list(argument_values)),
            ],
        )
    }
}

impl Dump for CallCallee {
    /// Render one call callee.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Expression(value) => {
                dump_record("CallCallee.Expression", [("value", value.dump(context))])
            }
            Self::Reference { value, symbol } => dump_record(
                "CallCallee.Reference",
                [
                    ("value", value.dump(context)),
                    ("symbol", context.symbol_label(*symbol)),
                ],
            ),
            Self::Member(member) => context.check.inference.term(*member).dump(context),
        }
    }
}

impl Dump for MemberCallTerm {
    /// Render one member call term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "MemberCallTerm",
            [
                ("source", self.source.dump(context)),
                ("receiver", self.receiver.dump(context)),
                ("key", self.key.dump(context)),
                ("arguments", dump_arguments(&self.arguments, context)),
            ],
        )
    }
}

impl Dump for MemberCallSource {
    /// Render one member call source.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Expression { source } => dump_record(
                "MemberCallSource.Expression",
                [("source", context.node_label(*source))],
            ),
            Self::Protocol { protocol } => dump_record(
                "MemberCallSource.Protocol",
                [
                    ("item", protocol.item.key()),
                    ("arguments", dump_arguments(&protocol.arguments, context)),
                ],
            ),
        }
    }
}

impl Dump for ConstructTerm {
    /// Render one construct term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "ConstructTerm",
            [
                ("source", context.node_label(self.source)),
                ("callee", self.callee.dump(context)),
                (
                    "generic_arguments",
                    dump_arguments(&self.generic_arguments, context),
                ),
                ("arguments", dump_type_operands(&self.arguments, context)),
            ],
        )
    }
}
