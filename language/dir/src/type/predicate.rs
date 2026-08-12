use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    GlobalTypeId, PrimitiveType, Projection, RangeEnd, RangeType, ScalarLiteral, StaticKey,
    TypeFold,
};

/// Executable predicate selected during checking.
///
/// Examples:
/// ```ds
/// value is string       // Unary
/// value is Shape.Circle // Unary over VariantTag, projection: VariantPayload
/// "name" in value       // Membership
/// value is "a" | "b"    // Any
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct Predicate {
    /// The test to execute.
    pub test: PredicateTest,
    /// The narrowed value type after the predicate succeeds.
    pub narrowed: Option<GlobalTypeId>,
    /// The projected value available after the predicate succeeds.
    pub projection: Option<Box<Projection>>,
}

impl Predicate {
    /// Create a predicate without a success narrowing.
    pub fn new(test: PredicateTest) -> Self {
        Self {
            test,
            narrowed: None,
            projection: None,
        }
    }

    /// Create a unary predicate.
    pub fn unary(input: PredicateOperand, condition: PredicateCondition) -> Self {
        Self::new(PredicateTest::Unary(Box::new(PredicateUnaryTest {
            input,
            condition,
        })))
    }

    /// Set the narrowed type available after this predicate succeeds.
    pub fn with_narrowed(mut self, narrowed: GlobalTypeId) -> Self {
        self.narrowed = Some(narrowed);

        self
    }

    /// Set the projected value available after this predicate succeeds.
    pub fn with_projection(mut self, projection: Projection) -> Self {
        self.projection = Some(Box::new(projection));

        self
    }

    /// Return whether this predicate always rejects.
    pub fn is_never(&self) -> bool {
        matches!(
            &self.test,
            PredicateTest::Unary(test) if matches!(test.condition, PredicateCondition::Never)
        )
    }
}

/// Value tested by one executable predicate.
///
/// Examples:
/// ```ds
/// value          // projection: none
/// dynamic.type   // projection: DynamicType
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum PredicateOperand {
    /// Direct input value.
    Direct(GlobalTypeId),
    /// Value computed by one projection.
    Projected(Box<Projection>),
}

impl PredicateOperand {
    /// Create a direct predicate operand.
    pub fn direct(ty: GlobalTypeId) -> Self {
        Self::Direct(ty)
    }

    /// Create a predicate operand from a selected projection.
    pub fn projected(projection: Projection) -> Self {
        Self::Projected(Box::new(projection))
    }

    /// Return the tested value type.
    pub fn ty(&self) -> GlobalTypeId {
        match self {
            Self::Direct(ty) => *ty,
            Self::Projected(projection) => projection.ty(),
        }
    }
}

/// Predicate test selected during checking.
///
/// Examples:
/// ```ds
/// value is string       // Unary
/// "name" in value       // Membership
/// value is "a" | "b"    // Any
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum PredicateTest {
    /// Unary test over one input value.
    ///
    /// Examples:
    /// ```ds
    /// value is string
    /// match value { "ready" => true }
    /// ```
    Unary(Box<PredicateUnaryTest>),
    /// Structural membership test over a receiver and key.
    ///
    /// Examples:
    /// ```ds
    /// "name" in value
    /// ```
    Membership(Box<PredicateMembershipTest>),
    /// Predicate that accepts when any alternative accepts.
    ///
    /// Examples:
    /// ```ds
    /// value is "yes" | "no"
    /// ```
    Any(Vec<Predicate>),
}

/// Unary predicate over one input value.
///
/// Examples:
/// ```ds
/// value is string       // input: value, condition: Primitive
/// value is Shape.Circle // input: VariantTag, condition: Literal
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct PredicateUnaryTest {
    /// The value tested by this predicate.
    pub input: PredicateOperand,
    /// The condition applied to the projected input.
    pub condition: PredicateCondition,
}

/// Condition applied to one predicate input.
///
/// Examples:
/// ```ds
/// _                // Always
/// never            // Never
/// "ready"          // Literal
/// 0..=255          // Range
/// value is string  // Primitive
/// value is UserId  // Type
/// value is Animal  // Subtype
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum PredicateCondition {
    /// Condition that accepts any projected input.
    ///
    /// Examples:
    /// ```ds
    /// match value { _ => true }
    /// ```
    Always,
    /// Condition that rejects every projected input.
    ///
    /// Examples:
    /// ```ds
    /// match value { never => false }
    /// ```
    Never,
    /// Scalar literal condition, like `"ready"` or `0`.
    ///
    /// Examples:
    /// ```ds
    /// match status { "ready" => true }
    /// ```
    Literal(ScalarLiteral),
    /// Scalar interval condition, like `0..=255`.
    ///
    /// Examples:
    /// ```ds
    /// match code { 200..=299 => true }
    /// ```
    Range(PredicateRange),
    /// Primitive tag condition, like `string` or `int32`.
    ///
    /// Examples:
    /// ```ds
    /// if (value is string) {}
    /// ```
    Primitive(PrimitiveType),
    /// Exact runtime type descriptor condition.
    ///
    /// Examples:
    /// ```ds
    /// if (value is UserId) {}
    /// ```
    Type(GlobalTypeId),
    /// Runtime subtype descriptor condition.
    ///
    /// Examples:
    /// ```ds
    /// if (value is Animal) {}
    /// ```
    Subtype(GlobalTypeId),
}

/// Structural membership predicate.
///
/// Examples:
/// ```ds
/// "name" in value
/// key in value
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct PredicateMembershipTest {
    /// The receiver value.
    pub receiver: PredicateOperand,
    /// The tested key.
    pub key: PredicateKey,
}

/// Key tested by one structural membership predicate.
///
/// Examples:
/// ```ds
/// "name" in value // Static
/// key in value    // Dynamic
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub enum PredicateKey {
    /// Statically known property key.
    ///
    /// Examples:
    /// ```ds
    /// "name" in value
    /// ```
    Static(StaticKey),
    /// Runtime-computed property key.
    ///
    /// Examples:
    /// ```ds
    /// key in value
    /// ```
    Dynamic(PredicateOperand),
}

/// Scalar interval condition.
///
/// Examples:
/// ```ds
/// 0..=255
/// "a".."z"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect, TypeFold)]
pub struct PredicateRange {
    /// The scalar domain constrained by the range.
    pub domain: GlobalTypeId,
    /// The optional committed lower bound.
    pub start: Option<ScalarLiteral>,
    /// The optional committed upper bound.
    pub end: Option<ScalarLiteral>,
    /// Whether the upper bound is inclusive.
    pub end_bound: RangeEnd,
}

impl PredicateRange {
    /// Return whether this predicate range contains one scalar literal.
    pub fn contains_literal(&self, literal: ScalarLiteral) -> bool {
        let range = RangeType::new(self.start, self.end, self.end_bound);

        range.contains_literal(literal)
    }
}
