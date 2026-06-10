use crate::check::{
    Condition, ConditionPredicate, Constraint, Dump, DumpContext, PatternRelation, StaticRelation,
};

use super::format::{dump_list, dump_record};

impl Dump for Constraint {
    /// Render one check constraint.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Type {
                relation,
                left,
                right,
                origin,
                condition,
                coercion,
            } => dump_record(
                "Constraint.Type",
                [
                    ("relation", relation.dump(context)),
                    ("left", left.dump(context)),
                    ("right", right.dump(context)),
                    ("origin", origin.dump(context)),
                    ("condition", condition.dump(context)),
                    ("coercion", coercion.dump(context)),
                ],
            ),
            Self::Static {
                relation,
                left,
                right,
                origin,
                condition,
            } => dump_record(
                "Constraint.Static",
                [
                    ("relation", relation.dump(context)),
                    ("left", left.dump(context)),
                    ("right", right.dump(context)),
                    ("origin", origin.dump(context)),
                    ("condition", condition.dump(context)),
                ],
            ),
            Self::Pattern {
                relation,
                value,
                origin,
                condition,
            } => dump_record(
                "Constraint.Pattern",
                [
                    ("relation", relation.dump(context)),
                    ("value", value.dump(context)),
                    ("origin", origin.dump(context)),
                    ("condition", condition.dump(context)),
                ],
            ),
        }
    }
}

impl Dump for StaticRelation {
    /// Render one static relation.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Equal => "equal".to_string(),
            Self::Assignable => "assignable".to_string(),
        }
    }
}

impl Dump for PatternRelation {
    /// Render one pattern relation.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Match(pattern) => dump_record(
                "PatternRelation.Match",
                [(
                    "pattern",
                    context.check.inference.term(*pattern).dump(context),
                )],
            ),
            Self::Assign(pattern) => dump_record(
                "PatternRelation.Assign",
                [(
                    "pattern",
                    context.check.inference.term(*pattern).dump(context),
                )],
            ),
        }
    }
}

impl Dump for Condition {
    /// Render one static condition.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Always => dump_record("Condition.Always", []),
            Self::Never => dump_record("Condition.Never", []),
            Self::When { conditions } => {
                let conditions = conditions
                    .iter()
                    .map(|condition| condition.dump(context))
                    .collect::<Vec<_>>()
                    .join(",");

                dump_record("Condition.When", [("conditions", dump_list(conditions))])
            }
        }
    }
}

impl Dump for ConditionPredicate {
    /// Render one static condition predicate.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "ConditionPredicate",
            [
                ("origin", self.origin.dump(context)),
                ("operand", self.operand.dump(context)),
            ],
        )
    }
}

impl Dump for destack_dir::CastOrigin {
    /// Render one cast origin.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Explicit => "explicit".to_string(),
            Self::Implicit => "implicit".to_string(),
        }
    }
}
