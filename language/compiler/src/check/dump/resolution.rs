use destack_dir as dir;

use crate::check::{
    CallDecision, CallFailure, CallResolution, CallTargetResolution, CandidateResolution,
    ConstructDecision, ConstructFailure, ConstructResolution, ConstructTargetResolution, Dump,
    DumpContext, IdentityDecision, IdentityFailure, IdentityResolution, LayoutDecision,
    LayoutFailure, LayoutResolution, MemberDecision, MemberFailure, MemberResolution,
    MemberTargetResolution, OperatorDecision, OperatorFailure, OperatorFailureReason,
    OperatorResolution,
};

use super::format::{dump_list, dump_record};

impl Dump for CallDecision {
    /// Render one call decision.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Resolved(resolution) => dump_record(
                "CallDecision.Resolved",
                [("value", resolution.dump(context))],
            ),
            Self::Rejected(failure) => {
                dump_record("CallDecision.Rejected", [("value", failure.dump(context))])
            }
        }
    }
}

impl Dump for CallResolution {
    /// Render one call resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "CallResolution",
            [
                ("source", context.node_label(self.source)),
                ("target", self.target.dump(context)),
                ("function", self.function.dump(context)),
            ],
        )
    }
}

impl Dump for CallTargetResolution {
    /// Render one call target resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Expression { instance } => dump_record(
                "CallTarget.Expression",
                [("instance", instance.dump(context))],
            ),
            Self::Symbol {
                symbol,
                instance,
                receiver,
            } => dump_record(
                "CallTarget.Symbol",
                [
                    ("symbol", context.symbol_label(*symbol)),
                    ("instance", instance.dump(context)),
                    ("receiver", receiver.dump(context)),
                ],
            ),
            Self::Union {
                candidates,
                receiver,
            } => dump_record(
                "CallTarget.Union",
                [
                    ("candidates", dump_candidates(candidates, context)),
                    ("receiver", receiver.dump(context)),
                ],
            ),
        }
    }
}

impl Dump for CandidateResolution {
    /// Render one candidate resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "CandidateResolution",
            [
                ("symbol", context.symbol_label(self.symbol)),
                ("instance", self.instance.dump(context)),
            ],
        )
    }
}

impl Dump for CallFailure {
    /// Render one call failure.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::NotCallable => dump_record("CallFailure.NotCallable", []),
            Self::NoMatch => dump_record("CallFailure.NoMatch", []),
            Self::ArgumentType {
                argument,
                parameter,
            } => dump_record(
                "CallFailure.ArgumentType",
                [
                    ("argument", argument.dump(context)),
                    ("parameter", parameter.dump(context)),
                ],
            ),
        }
    }
}

impl Dump for ConstructDecision {
    /// Render one construct decision.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Resolved(resolution) => dump_record(
                "ConstructDecision.Resolved",
                [("value", resolution.dump(context))],
            ),
            Self::Rejected(failure) => dump_record(
                "ConstructDecision.Rejected",
                [("value", failure.dump(context))],
            ),
        }
    }
}

impl Dump for ConstructResolution {
    /// Render one construct resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "ConstructResolution",
            [
                ("source", context.node_label(self.source)),
                ("target", self.target.dump(context)),
                ("function", self.function.dump(context)),
            ],
        )
    }
}

impl Dump for ConstructTargetResolution {
    /// Render one construct target resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Class {
                symbol,
                constructor,
                instance,
            } => dump_record(
                "ConstructTarget.Class",
                [
                    ("symbol", context.symbol_label(*symbol)),
                    ("constructor", constructor.dump(context)),
                    ("instance", instance.dump(context)),
                ],
            ),
            Self::Newtype { symbol, instance } => dump_record(
                "ConstructTarget.Newtype",
                [
                    ("symbol", context.symbol_label(*symbol)),
                    ("instance", instance.dump(context)),
                ],
            ),
        }
    }
}

impl Dump for ConstructFailure {
    /// Render one construct failure.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::NotConstructible => dump_record("ConstructFailure.NotConstructible", []),
            Self::NoMatch => dump_record("ConstructFailure.NoMatch", []),
        }
    }
}

impl Dump for MemberDecision {
    /// Render one member decision.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Resolved(resolution) => dump_record(
                "MemberDecision.Resolved",
                [("value", resolution.dump(context))],
            ),
            Self::Rejected(failure) => dump_record(
                "MemberDecision.Rejected",
                [("value", failure.dump(context))],
            ),
        }
    }
}

impl Dump for MemberResolution {
    /// Render one member resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "MemberResolution",
            [
                ("source", context.node_label(self.source)),
                ("receiver", self.receiver.dump(context)),
                ("target", self.target.dump(context)),
            ],
        )
    }
}

impl Dump for MemberTargetResolution {
    /// Render one member target resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Builtin(member) => {
                dump_record("MemberTarget.Builtin", [("member", member.dump(context))])
            }
            Self::Field(key) => dump_record("MemberTarget.Field", [("key", key.dump(context))]),
            Self::Symbol { symbol, instance } => dump_record(
                "MemberTarget.Symbol",
                [
                    ("symbol", context.symbol_label(*symbol)),
                    ("instance", instance.dump(context)),
                ],
            ),
            Self::Union(candidates) => dump_record(
                "MemberTarget.Union",
                [("candidates", dump_candidates(candidates, context))],
            ),
        }
    }
}

impl Dump for dir::BuiltinMember {
    /// Render one builtin member.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Index => "index".to_string(),
            Self::Slice => "slice".to_string(),
        }
    }
}

impl Dump for MemberFailure {
    /// Render one member failure.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Missing => dump_record("MemberFailure.Missing", []),
        }
    }
}

impl Dump for OperatorDecision {
    /// Render one operator decision.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Resolved(resolution) => dump_record(
                "OperatorDecision.Resolved",
                [("value", resolution.dump(context))],
            ),
            Self::Rejected(failure) => dump_record(
                "OperatorDecision.Rejected",
                [("value", failure.dump(context))],
            ),
        }
    }
}

impl Dump for OperatorResolution {
    /// Render one operator resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Builtin {
                source,
                kind,
                receiver,
                argument,
                result,
            } => dump_record(
                "OperatorResolution.Builtin",
                [
                    ("source", context.node_label(*source)),
                    ("kind", kind.dump(context)),
                    ("receiver", receiver.dump(context)),
                    ("argument", argument.dump(context)),
                    ("result", result.dump(context)),
                ],
            ),
            Self::Method {
                source,
                symbol,
                receiver,
                function,
            } => dump_record(
                "OperatorResolution.Method",
                [
                    ("source", context.node_label(*source)),
                    ("symbol", context.symbol_label(*symbol)),
                    ("receiver", receiver.dump(context)),
                    ("function", function.dump(context)),
                ],
            ),
        }
    }
}

impl Dump for OperatorFailure {
    /// Render one operator failure.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "OperatorFailure",
            [
                ("source", context.node_label(self.source)),
                ("kind", self.kind.dump(context)),
                ("reason", self.reason.dump(context)),
            ],
        )
    }
}

impl Dump for OperatorFailureReason {
    /// Render one operator failure reason.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::NoMatch => "no_match".to_string(),
            Self::InvalidStrictEquality => "invalid_strict_equality".to_string(),
        }
    }
}

impl Dump for IdentityDecision {
    /// Render one identity decision.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Resolved(resolution) => dump_record(
                "IdentityDecision.Resolved",
                [("value", resolution.dump(context))],
            ),
            Self::Rejected(failure) => dump_record(
                "IdentityDecision.Rejected",
                [("value", failure.dump(context))],
            ),
        }
    }
}

impl Dump for IdentityResolution {
    /// Render one identity resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "IdentityResolution",
            [
                ("source", context.node_label(self.source)),
                ("left", self.left.dump(context)),
                ("right", self.right.dump(context)),
            ],
        )
    }
}

impl Dump for IdentityFailure {
    /// Render one identity failure.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Incompatible => dump_record("IdentityFailure.Incompatible", []),
        }
    }
}

impl Dump for LayoutDecision {
    /// Render one layout decision.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Resolved(resolution) => dump_record(
                "LayoutDecision.Resolved",
                [("value", resolution.dump(context))],
            ),
            Self::Rejected(failure) => dump_record(
                "LayoutDecision.Rejected",
                [("value", failure.dump(context))],
            ),
        }
    }
}

impl Dump for LayoutResolution {
    /// Render one layout resolution.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "LayoutResolution",
            [
                ("source", context.node_label(self.source)),
                ("target", self.target.dump(context)),
                ("query", self.query.dump(context)),
                ("layout", self.layout.dump(context)),
            ],
        )
    }
}

impl Dump for LayoutFailure {
    /// Render one layout failure.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "LayoutFailure",
            [
                ("source", context.node_label(self.source)),
                ("target", self.target.dump(context)),
                ("query", self.query.dump(context)),
            ],
        )
    }
}

/// Render one candidate list.
fn dump_candidates(candidates: &[CandidateResolution], context: &DumpContext<'_, '_>) -> String {
    let candidates = candidates
        .iter()
        .map(|candidate| candidate.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(candidates)
}
