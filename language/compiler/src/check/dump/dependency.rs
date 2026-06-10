use crate::check::{DecisionKey, Dependency, Dump, DumpContext};

use super::format::dump_record;

impl Dump for Dependency {
    /// Render one solver dependency.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Constraint(constraint) => {
                let state = if context.check.inference.is_constraint_complete(*constraint) {
                    "complete"
                } else {
                    "open"
                };
                let value = context.check.inference.constraint_by_id(*constraint);

                dump_record(
                    "Dependency.Constraint",
                    [
                        ("id", format!("constraint#{}", constraint.index)),
                        ("state", state.to_string()),
                        ("value", value.dump(context)),
                    ],
                )
            }
            Self::Variable(variable) => {
                let solution = context.check.variable_solution(*variable);
                let state = if solution.is_some() { "solved" } else { "open" };
                let solution = solution
                    .map(|solution| solution.dump(context))
                    .unwrap_or_else(|| "none".to_string());

                dump_record(
                    "Dependency.Variable",
                    [
                        ("value", variable.dump(context)),
                        ("state", state.to_string()),
                        ("solution", solution),
                    ],
                )
            }
            Self::Decision(decision) => {
                dump_record("Dependency.Decision", [("value", decision.dump(context))])
            }
        }
    }
}

impl Dump for DecisionKey {
    /// Render one solver decision key.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Callable(source) => dump_record(
                "Decision.Callable",
                [("source", context.node_label(*source))],
            ),
            Self::Call(source) => {
                dump_record("Decision.Call", [("source", context.node_label(*source))])
            }
            Self::Construct(source) => dump_record(
                "Decision.Construct",
                [("source", context.node_label(*source))],
            ),
            Self::Operator(source) => dump_record(
                "Decision.Operator",
                [("source", context.node_label(*source))],
            ),
            Self::Identity(source) => dump_record(
                "Decision.Identity",
                [("source", context.node_label(*source))],
            ),
            Self::Layout(source) => {
                dump_record("Decision.Layout", [("source", context.node_label(*source))])
            }
            Self::Member(source) => {
                dump_record("Decision.Member", [("source", context.node_label(*source))])
            }
            Self::Pattern(source) => dump_record(
                "Decision.Pattern",
                [("source", context.node_label(*source))],
            ),
            Self::Receiver(source) => dump_record(
                "Decision.Receiver",
                [("source", context.node_label(*source))],
            ),
        }
    }
}
