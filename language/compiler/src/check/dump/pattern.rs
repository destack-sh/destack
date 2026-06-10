use destack_dir as dir;

use crate::check::{
    AssignPatternField, AssignPatternTerm, Dump, DumpContext, PatternField, PatternSource,
    PatternTarget, PatternTerm, TermId,
};

use super::format::{dump_list, dump_record};

impl Dump for PatternTerm {
    /// Render one pattern term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "PatternTerm",
            [
                ("source", self.source.dump(context)),
                ("target", self.target.dump(context)),
            ],
        )
    }
}

impl Dump for PatternSource {
    /// Render one pattern source.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Node(node) => dump_record("PatternSource.Node", [("value", node.dump(context))]),
            Self::Synthetic(origin) => dump_record(
                "PatternSource.Synthetic",
                [("origin", origin.dump(context))],
            ),
        }
    }
}

impl Dump for PatternTarget {
    /// Render one pattern target.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Wildcard => dump_record("PatternTarget.Wildcard", []),
            Self::Must { pattern } => dump_record(
                "PatternTarget.Must",
                [("pattern", dump_pattern(*pattern, context))],
            ),
            Self::Assign { pattern, value } => dump_record(
                "PatternTarget.Assign",
                [
                    ("pattern", dump_pattern(*pattern, context)),
                    ("value", value.dump(context)),
                ],
            ),
            Self::BorrowOf {
                mutability,
                pattern,
            } => dump_record(
                "PatternTarget.BorrowOf",
                [
                    ("mutability", dump_mutability(*mutability)),
                    ("pattern", dump_pattern(*pattern, context)),
                ],
            ),
            Self::MoveOf {
                mutability,
                pattern,
            } => dump_record(
                "PatternTarget.MoveOf",
                [
                    ("mutability", dump_mutability(*mutability)),
                    ("pattern", dump_pattern(*pattern, context)),
                ],
            ),
            Self::DereferenceOf { pattern } => dump_record(
                "PatternTarget.DereferenceOf",
                [("pattern", dump_pattern(*pattern, context))],
            ),
            Self::Binding { symbol, pattern } => dump_record(
                "PatternTarget.Binding",
                [
                    ("symbol", symbol.dump(context)),
                    ("pattern", dump_optional_pattern(*pattern, context)),
                ],
            ),
            Self::Expression { value } => {
                dump_record("PatternTarget.Expression", [("value", value.dump(context))])
            }
            Self::Range {
                start,
                end,
                end_kind,
            } => dump_record(
                "PatternTarget.Range",
                [
                    ("start", start.dump(context)),
                    ("end", end.dump(context)),
                    ("end_kind", dump_range_end(*end_kind)),
                ],
            ),
            Self::Type { ty } => dump_record("PatternTarget.Type", [("ty", ty.dump(context))]),
            Self::Tuple { fields } => dump_record(
                "PatternTarget.Tuple",
                [("fields", dump_pattern_fields(fields, context))],
            ),
            Self::Newtype { ty, fields } => dump_record(
                "PatternTarget.Newtype",
                [
                    ("ty", ty.dump(context)),
                    ("fields", dump_pattern_fields(fields, context)),
                ],
            ),
            Self::Sequence { fields } => dump_record(
                "PatternTarget.Sequence",
                [("fields", dump_pattern_fields(fields, context))],
            ),
            Self::Object { fields } => dump_record(
                "PatternTarget.Object",
                [("fields", dump_pattern_fields(fields, context))],
            ),
            Self::NominalObject { ty, fields } => dump_record(
                "PatternTarget.NominalObject",
                [
                    ("ty", ty.dump(context)),
                    ("fields", dump_pattern_fields(fields, context)),
                ],
            ),
            Self::Union { patterns } => dump_record(
                "PatternTarget.Union",
                [("patterns", dump_patterns(patterns, context))],
            ),
        }
    }
}

impl Dump for PatternField {
    /// Render one pattern field.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Named {
                source,
                key,
                binding,
                value,
                pattern,
            } => dump_record(
                "PatternField.Named",
                [
                    ("source", source.dump(context)),
                    ("key", key.dump(context)),
                    ("binding", binding.dump(context)),
                    ("value", value.dump(context)),
                    ("pattern", dump_optional_pattern(*pattern, context)),
                ],
            ),
            Self::Computed {
                source,
                key,
                pattern,
            } => dump_record(
                "PatternField.Computed",
                [
                    ("source", source.dump(context)),
                    ("key", key.dump(context)),
                    ("pattern", dump_pattern(*pattern, context)),
                ],
            ),
            Self::Positional { source, pattern } => dump_record(
                "PatternField.Positional",
                [
                    ("source", source.dump(context)),
                    ("pattern", dump_pattern(*pattern, context)),
                ],
            ),
            Self::Spread { source, pattern } => dump_record(
                "PatternField.Spread",
                [
                    ("source", source.dump(context)),
                    ("pattern", dump_optional_pattern(*pattern, context)),
                ],
            ),
            Self::Elision { source } => {
                dump_record("PatternField.Elision", [("source", source.dump(context))])
            }
        }
    }
}

impl Dump for AssignPatternTerm {
    /// Render one assignment pattern term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Expression { target } => dump_record(
                "AssignPatternTerm.Expression",
                [("target", target.dump(context))],
            ),
            Self::Assign { pattern, value } => dump_record(
                "AssignPatternTerm.Assign",
                [
                    ("pattern", dump_assign_pattern(*pattern, context)),
                    ("value", value.dump(context)),
                ],
            ),
            Self::Sequence { fields } => dump_record(
                "AssignPatternTerm.Sequence",
                [("fields", dump_assign_pattern_fields(fields, context))],
            ),
            Self::Object { fields } => dump_record(
                "AssignPatternTerm.Object",
                [("fields", dump_assign_pattern_fields(fields, context))],
            ),
        }
    }
}

impl Dump for AssignPatternField {
    /// Render one assignment pattern field.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Named {
                key,
                value,
                pattern,
            } => dump_record(
                "AssignPatternField.Named",
                [
                    ("key", key.dump(context)),
                    ("value", value.dump(context)),
                    ("pattern", dump_optional_assign_pattern(*pattern, context)),
                ],
            ),
            Self::Computed { key, pattern } => dump_record(
                "AssignPatternField.Computed",
                [
                    ("key", key.dump(context)),
                    ("pattern", dump_assign_pattern(*pattern, context)),
                ],
            ),
            Self::Positional { pattern } => dump_record(
                "AssignPatternField.Positional",
                [("pattern", dump_assign_pattern(*pattern, context))],
            ),
            Self::Spread { pattern } => dump_record(
                "AssignPatternField.Spread",
                [("pattern", dump_optional_assign_pattern(*pattern, context))],
            ),
            Self::Elision => dump_record("AssignPatternField.Elision", []),
        }
    }
}

/// Render one nested pattern term.
fn dump_pattern(pattern: TermId<PatternTerm>, context: &DumpContext<'_, '_>) -> String {
    context.check.inference.term(pattern).dump(context)
}

/// Render one optional nested pattern term.
fn dump_optional_pattern(
    pattern: Option<TermId<PatternTerm>>,
    context: &DumpContext<'_, '_>,
) -> String {
    pattern
        .map(|pattern| dump_pattern(pattern, context))
        .unwrap_or_else(|| "none".to_string())
}

/// Render pattern terms as one list.
fn dump_patterns(patterns: &[TermId<PatternTerm>], context: &DumpContext<'_, '_>) -> String {
    let patterns = patterns
        .iter()
        .map(|pattern| dump_pattern(*pattern, context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(patterns)
}

/// Render pattern fields as one list.
fn dump_pattern_fields(fields: &[PatternField], context: &DumpContext<'_, '_>) -> String {
    let fields = fields
        .iter()
        .map(|field| field.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(fields)
}

/// Render one nested assignment pattern term.
fn dump_assign_pattern(
    pattern: TermId<AssignPatternTerm>,
    context: &DumpContext<'_, '_>,
) -> String {
    context.check.inference.term(pattern).dump(context)
}

/// Render one optional nested assignment pattern term.
fn dump_optional_assign_pattern(
    pattern: Option<TermId<AssignPatternTerm>>,
    context: &DumpContext<'_, '_>,
) -> String {
    pattern
        .map(|pattern| dump_assign_pattern(pattern, context))
        .unwrap_or_else(|| "none".to_string())
}

/// Render assignment pattern fields as one list.
fn dump_assign_pattern_fields(
    fields: &[AssignPatternField],
    context: &DumpContext<'_, '_>,
) -> String {
    let fields = fields
        .iter()
        .map(|field| field.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(fields)
}

/// Render one mutability label.
fn dump_mutability(mutability: Option<dir::Mutability>) -> String {
    match mutability {
        Some(dir::Mutability::Immutable) => "immutable".to_string(),
        Some(dir::Mutability::Mutable) => "mutable".to_string(),
        Some(dir::Mutability::Exclusive) => "exclusive".to_string(),
        None => "readonly".to_string(),
    }
}

/// Render one range end label.
fn dump_range_end(end: dir::RangeEnd) -> String {
    match end {
        dir::RangeEnd::Open => "open".to_string(),
        dir::RangeEnd::Inclusive => "inclusive".to_string(),
    }
}
