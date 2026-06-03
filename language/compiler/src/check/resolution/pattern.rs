use destack_dir as dir;

use crate::check::{CheckState, GenericInstance, StaticOperand, TypeOperand};

/// Pattern decision resolved by check before commit.
///
/// Examples:
/// ```ds
/// match (value) { Some(item) => item }
/// const { name } = user
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternDecision {
    /// One pattern target resolved.
    ///
    /// Examples:
    /// ```ds
    /// Some(item)
    /// ```
    Resolved(PatternResolution),
    /// Pattern resolution failed.
    ///
    /// Examples:
    /// ```ds
    /// Missing(item)
    /// ```
    Rejected(PatternFailure),
}

/// Pattern failure resolved by check.
///
/// Examples:
/// ```ds
/// Missing(item)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PatternFailure {
    /// No matching pattern target exists.
    ///
    /// Examples:
    /// ```ds
    /// Missing(item)
    /// ```
    Missing,
}

/// Pattern selected by check before commit.
///
/// Examples:
/// ```ds
/// Some(item)
/// { name }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternResolution {
    /// The source pattern node.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected pattern target.
    pub(in crate::check) target: PatternTargetResolution,
}

/// Pattern target selected by check before commit.
///
/// Examples:
/// ```ds
/// _
/// Some(item)
/// { name }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum PatternTargetResolution {
    /// Pattern that accepts the input without binding.
    ///
    /// Examples:
    /// ```ds
    /// _
    /// ```
    Wildcard,
    /// Pattern that binds a symbol and may refine with a nested pattern.
    ///
    /// Examples:
    /// ```ds
    /// name
    /// name: Some(item)
    /// ```
    Binding(PatternBindingResolution),
    /// Pattern that accepts one static literal value.
    ///
    /// Examples:
    /// ```ds
    /// 1
    /// "ready"
    /// ```
    Literal(PatternLiteralResolution),
    /// Pattern that accepts one scalar interval.
    ///
    /// Examples:
    /// ```ds
    /// 0..10
    /// 0..=10
    /// ```
    Range(PatternRangeResolution),
    /// Pattern that destructures a tuple-shaped input.
    ///
    /// Examples:
    /// ```ds
    /// [left, right]
    /// ```
    Tuple(PatternTupleResolution),
    /// Pattern that destructures an ordered collection.
    ///
    /// Examples:
    /// ```ds
    /// [head, ...tail]
    /// ```
    Sequence(PatternSequenceResolution),
    /// Pattern that destructures a structural input.
    ///
    /// Examples:
    /// ```ds
    /// { name }
    /// ```
    Shape(PatternShapeResolution),
    /// Pattern that destructures a symbol-backed nominal input.
    ///
    /// Examples:
    /// ```ds
    /// Point { x, y }
    /// ```
    Nominal(PatternNominalResolution),
    /// Pattern that unwraps a symbol-backed newtype input.
    ///
    /// Examples:
    /// ```ds
    /// UserId(raw)
    /// ```
    Newtype(PatternNewtypeResolution),
    /// Pattern that selects a symbol-backed variant input.
    ///
    /// Examples:
    /// ```ds
    /// Result.Ok(value)
    /// ```
    Variant(PatternVariantResolution),
    /// Pattern that accepts one of several alternatives.
    ///
    /// Examples:
    /// ```ds
    /// Ok(value) | Error(value)
    /// ```
    Union(PatternUnionResolution),
    /// Pattern that borrows the input before matching.
    ///
    /// Examples:
    /// ```ds
    /// &value
    /// ```
    Borrow(PatternBorrowResolution),
    /// Pattern that moves the input before matching.
    ///
    /// Examples:
    /// ```ds
    /// ^value
    /// ```
    Move(PatternMoveResolution),
    /// Pattern that dereferences the input before matching.
    ///
    /// Examples:
    /// ```ds
    /// *value
    /// ```
    Dereference(PatternDereferenceResolution),
}

/// Symbol binding selected by one pattern.
///
/// Examples:
/// ```ds
/// name
/// name: Some(item)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct PatternBindingResolution {
    /// The bound symbol, when the binding has a user-visible name.
    pub(in crate::check) symbol: Option<dir::GlobalSymbolId>,
    /// The nested pattern matched after binding.
    pub(in crate::check) pattern: Option<dir::GlobalNodeIdAny>,
}

/// Static literal selected by one pattern.
///
/// Examples:
/// ```ds
/// 1
/// "ready"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternLiteralResolution {
    /// The selected literal value.
    pub(in crate::check) value: StaticOperand,
}

/// Scalar range selected by one pattern.
///
/// Examples:
/// ```ds
/// 0..10
/// 0..=10
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternRangeResolution {
    /// The scalar domain constrained by the range.
    pub(in crate::check) domain: TypeOperand,
    /// The selected lower bound.
    pub(in crate::check) start: Option<StaticOperand>,
    /// The selected upper bound.
    pub(in crate::check) end: Option<StaticOperand>,
    /// Whether the upper bound is inclusive.
    pub(in crate::check) end_bound: dir::RangeEnd,
}

/// Tuple fields selected by one pattern.
///
/// Examples:
/// ```ds
/// [left, right]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct PatternTupleResolution {
    /// The tuple field mapping in source order.
    pub(in crate::check) fields: Vec<PatternFieldResolution>,
}

/// Ordered collection selected by one pattern.
///
/// Examples:
/// ```ds
/// [head, ...tail]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum PatternSequenceResolution {
    /// Dynamically sized array pattern.
    ///
    /// Examples:
    /// ```ds
    /// [head, ...tail]
    /// ```
    Array {
        /// The fixed prefix and suffix fields.
        fields: Vec<PatternFieldResolution>,
        /// The rest field, when present.
        rest: Option<PatternRestResolution>,
    },
    /// Borrowed slice pattern.
    ///
    /// Examples:
    /// ```ds
    /// &[head, ...tail]
    /// ```
    Slice {
        /// The fixed prefix and suffix fields.
        fields: Vec<PatternFieldResolution>,
        /// The rest field, when present.
        rest: Option<PatternRestResolution>,
    },
    /// Fixed-size array pattern.
    ///
    /// Examples:
    /// ```ds
    /// [left, right]
    /// ```
    FixedArray {
        /// The fixed element fields.
        fields: Vec<PatternFieldResolution>,
        /// The selected array length.
        length: StaticOperand,
    },
}

/// Structural fields selected by one pattern.
///
/// Examples:
/// ```ds
/// { name }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct PatternShapeResolution {
    /// The structural field mapping in source order.
    pub(in crate::check) fields: Vec<PatternFieldResolution>,
}

/// Symbol-backed nominal pattern selected by check.
///
/// Examples:
/// ```ds
/// Point { x, y }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternNominalResolution {
    /// The selected nominal symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The selected generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The nominal field mapping in source order.
    pub(in crate::check) fields: Vec<PatternFieldResolution>,
}

/// Symbol-backed newtype pattern selected by check.
///
/// Examples:
/// ```ds
/// UserId(raw)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternNewtypeResolution {
    /// The selected newtype symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The selected generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The wrapped value pattern.
    pub(in crate::check) value: Option<dir::GlobalNodeIdAny>,
}

/// Symbol-backed variant pattern selected by check.
///
/// Examples:
/// ```ds
/// Result.Ok(value)
/// ```
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) struct PatternVariantResolution {
    /// The selected variant symbol.
    pub(in crate::check) symbol: dir::GlobalSymbolId,
    /// The selected generic instance.
    pub(in crate::check) instance: Option<GenericInstance>,
    /// The selected static discriminant value.
    pub(in crate::check) discriminant: Option<StaticOperand>,
    /// The variant field mapping in source order.
    pub(in crate::check) fields: Vec<PatternFieldResolution>,
}

/// Alternative patterns selected by check.
///
/// Examples:
/// ```ds
/// Ok(value) | Error(value)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct PatternUnionResolution {
    /// The alternative pattern nodes.
    pub(in crate::check) alternatives: Vec<dir::GlobalNodeIdAny>,
}

/// Borrow operation selected by one pattern.
///
/// Examples:
/// ```ds
/// &value
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternBorrowResolution {
    /// The requested borrow access, if source explicit.
    pub(in crate::check) access: Option<dir::Access>,
    /// The pattern matched through the borrow.
    pub(in crate::check) pattern: dir::GlobalNodeIdAny,
}

/// Move operation selected by one pattern.
///
/// Examples:
/// ```ds
/// ^value
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternMoveResolution {
    /// The requested move access, if source explicit.
    pub(in crate::check) access: Option<dir::Access>,
    /// The pattern matched after moving.
    pub(in crate::check) pattern: dir::GlobalNodeIdAny,
}

/// Dereference operation selected by one pattern.
///
/// Examples:
/// ```ds
/// *value
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternDereferenceResolution {
    /// The pattern matched through the dereference.
    pub(in crate::check) pattern: dir::GlobalNodeIdAny,
}

/// One destructured pattern field selected by check.
///
/// Examples:
/// ```ds
/// { name }
/// [head]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternFieldResolution {
    /// The source node that introduces the field.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The selected field target.
    pub(in crate::check) target: PatternFieldTargetResolution,
    /// The nested pattern matched for the field.
    pub(in crate::check) pattern: Option<dir::GlobalNodeIdAny>,
}

/// Field target selected by one destructuring pattern.
///
/// Examples:
/// ```ds
/// { name }
/// [head]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum PatternFieldTargetResolution {
    /// Named or symbolic field target.
    ///
    /// Examples:
    /// ```ds
    /// { name }
    /// ```
    Key(dir::StaticKey),
    /// Positional field target.
    ///
    /// Examples:
    /// ```ds
    /// [head]
    /// ```
    Index(usize),
}

/// Rest field selected by one ordered pattern.
///
/// Examples:
/// ```ds
/// [...rest]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct PatternRestResolution {
    /// The source node that introduces the rest field.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The nested pattern matched for the rest field.
    pub(in crate::check) pattern: Option<dir::GlobalNodeIdAny>,
}

impl CheckState<'_> {
    /// Select one pattern decision.
    pub(in crate::check) fn select_pattern(
        &mut self,
        source: dir::GlobalNodeIdAny,
        decision: PatternDecision,
    ) {
        self.inference.select_pattern(source, decision);
    }
}
