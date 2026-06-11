use crate::check::{
    CallArgument, CallCallee, CallTerm, ConstructTerm, Dump, DumpContext, MemberCallTerm,
    MemberProjectionOrigin, MemberReceiver,
};

use super::argument::dump_arguments;
use super::format::{dump_list, dump_record};

impl Dump for CallTerm {
    /// Render one call term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "CallTerm",
            [
                ("source", context.node_label(self.source)),
                ("callee", self.callee.dump(context)),
                (
                    "generic_arguments",
                    dump_arguments(&self.generic_arguments, context),
                ),
                ("arguments", dump_call_arguments(&self.arguments, context)),
            ],
        )
    }
}

impl Dump for CallArgument {
    /// Render one call argument.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "CallArgument",
            [
                ("source", context.node_label(self.source)),
                ("ty", self.ty.dump(context)),
                ("is_spread", self.is_spread.to_string()),
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
                ("origin", self.origin.dump(context)),
                ("receiver", self.receiver.dump(context)),
                ("key", self.key.dump(context)),
                ("arguments", dump_arguments(&self.arguments, context)),
            ],
        )
    }
}

impl Dump for MemberReceiver {
    /// Render one member receiver.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Value(value) => {
                dump_record("MemberReceiver.Value", [("value", value.dump(context))])
            }
            Self::GenericParameter(parameter) => dump_record(
                "MemberReceiver.GenericParameter",
                [("parameter", context.generic_parameter_label(*parameter))],
            ),
            Self::Declaration {
                origin,
                symbol,
                arguments,
            } => dump_record(
                "MemberReceiver.Declaration",
                [
                    ("origin", origin.dump(context)),
                    ("symbol", context.symbol_label(*symbol)),
                    ("arguments", dump_arguments(arguments, context)),
                ],
            ),
        }
    }
}

impl Dump for MemberProjectionOrigin {
    /// Render one member projection origin.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Expression { source } => dump_record(
                "MemberProjectionOrigin.Expression",
                [("source", context.node_label(*source))],
            ),
            Self::Protocol { protocol } => dump_record(
                "MemberProjectionOrigin.Protocol",
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
                ("arguments", dump_call_arguments(&self.arguments, context)),
            ],
        )
    }
}

/// Render call arguments.
fn dump_call_arguments(arguments: &[CallArgument], context: &DumpContext<'_, '_>) -> String {
    let arguments = arguments
        .iter()
        .map(|argument| argument.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(arguments)
}
