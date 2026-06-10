use destack_dir as dir;

use crate::check::{
    ConditionBranch, Dump, DumpContext, MappedParameter, TypeOperationTerm, TypePredicateTerm,
};

use super::argument::dump_arguments;
use super::format::{dump_list, dump_record};
use super::operand::dump_type_operands;

impl Dump for TypeOperationTerm {
    /// Render one type operation term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::StringMapping { mapping, argument } => dump_record(
                "TypeOperationTerm.StringMapping",
                [
                    ("mapping", mapping.dump(context)),
                    ("argument", argument.dump(context)),
                ],
            ),
            Self::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => dump_record(
                "TypeOperationTerm.Conditional",
                [
                    ("left", left.dump(context)),
                    ("right", right.dump(context)),
                    ("then", then_type.dump(context)),
                    ("else", else_type.dump(context)),
                ],
            ),
            Self::Mapped {
                parameter,
                modifiers,
                value,
            } => dump_record(
                "TypeOperationTerm.Mapped",
                [
                    ("parameter", parameter.dump(context)),
                    ("modifiers", modifiers.dump(context)),
                    ("value", value.dump(context)),
                ],
            ),
            Self::Index { left, index } => dump_record(
                "TypeOperationTerm.Index",
                [("left", left.dump(context)), ("index", index.dump(context))],
            ),
            Self::TemplateLiteral { strings, spans } => {
                let strings = strings
                    .iter()
                    .map(|string| context.string(*string))
                    .collect::<Vec<_>>()
                    .join(",");
                let spans = dump_type_operands(spans, context);

                dump_record(
                    "TypeOperationTerm.TemplateLiteral",
                    [("strings", dump_list(strings)), ("spans", dump_list(spans))],
                )
            }
            Self::Infer { name, constraint } => {
                let constraint = constraint
                    .map(|constraint| constraint.dump(context))
                    .unwrap_or_else(|| "none".to_string());

                dump_record(
                    "TypeOperationTerm.Infer",
                    [
                        ("name", context.optional_string(*name)),
                        ("constraint", constraint),
                    ],
                )
            }
            Self::KeyOf { target } => dump_record(
                "TypeOperationTerm.KeyOf",
                [("target", target.dump(context))],
            ),
            Self::BestCommon { elements } => dump_record(
                "TypeOperationTerm.BestCommon",
                [("elements", dump_type_operands(elements, context))],
            ),
            Self::Widen { source } => dump_record(
                "TypeOperationTerm.Widen",
                [("source", source.dump(context))],
            ),
            Self::Exclude { source, target } => dump_record(
                "TypeOperationTerm.Exclude",
                [
                    ("source", source.dump(context)),
                    ("target", target.dump(context)),
                ],
            ),
            Self::Narrow { source, predicate } => dump_record(
                "TypeOperationTerm.Narrow",
                [
                    ("source", source.dump(context)),
                    ("predicate", predicate.dump(context)),
                ],
            ),
            Self::Intrinsic { item, arguments } => dump_record(
                "TypeOperationTerm.Intrinsic",
                [
                    ("item", item.key()),
                    ("arguments", dump_arguments(arguments, context)),
                ],
            ),
        }
    }
}

impl Dump for TypePredicateTerm {
    /// Render one type predicate term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::MemberEquality {
                key,
                target,
                branch,
            } => dump_record(
                "TypePredicateTerm.MemberEquality",
                [
                    ("key", key.dump(context)),
                    ("target", target.dump(context)),
                    ("branch", branch.dump(context)),
                ],
            ),
        }
    }
}

impl Dump for ConditionBranch {
    /// Render one condition branch.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::True => "true".to_string(),
            Self::False => "false".to_string(),
        }
    }
}

impl Dump for MappedParameter {
    /// Render one mapped type parameter.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "MappedParameter",
            [
                ("name", context.string(self.name)),
                ("symbol", context.symbol_label(self.symbol)),
                ("constraint", self.constraint.dump(context)),
                ("key_remap", self.key_remap.dump(context)),
            ],
        )
    }
}

impl Dump for dir::StringMapping {
    /// Render one string mapping operator.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Uppercase => "uppercase".to_string(),
            Self::Lowercase => "lowercase".to_string(),
            Self::Capitalize => "capitalize".to_string(),
            Self::Uncapitalize => "uncapitalize".to_string(),
        }
    }
}

impl Dump for dir::MappedTypeModifiers {
    /// Render one mapped type modifier pair.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "MappedTypeModifiers",
            [
                ("readonly", self.readonly.dump(context)),
                ("optional", self.optional.dump(context)),
            ],
        )
    }
}

impl Dump for dir::MappedTypeModifier {
    /// Render one mapped type modifier.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Present => "present".to_string(),
            Self::Add => "add".to_string(),
            Self::Remove => "remove".to_string(),
            Self::None => "none".to_string(),
        }
    }
}
