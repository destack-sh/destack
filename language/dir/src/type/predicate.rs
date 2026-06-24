use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{
    CallResolution, GlobalTypeId, PrimitiveType, Projection, RangeEnd, ScalarLiteral, StaticKey,
};

/// Executable predicate selected during checking.
///
/// Examples:
/// ```ds
/// value is string       // Unary
/// value is Shape.Circle // Unary over VariantTag, success projects VariantPayload
/// "name" in value       // Has
/// key in bag            // Call
/// value is "a" | "b"    // Any
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Predicate {
    /// The test to execute.
    pub test: PredicateTest,
    /// The value projection available after the predicate succeeds.
    pub success: Option<Projection>,
}

impl Predicate {
    /// Create a predicate without a success projection.
    pub fn new(test: PredicateTest) -> Self {
        Self {
            test,
            success: None,
        }
    }

    /// Create a unary predicate.
    pub fn unary(input: Projection, condition: PredicateCondition) -> Self {
        Self::new(PredicateTest::Unary(PredicateUnaryTest {
            input,
            condition,
        }))
    }

    /// Create a predicate with a success projection.
    pub fn with_success(mut self, success: Projection) -> Self {
        self.success = Some(success);

        self
    }

    /// Return whether this predicate always rejects.
    pub fn is_never(&self) -> bool {
        matches!(
            self.test,
            PredicateTest::Unary(PredicateUnaryTest {
                condition: PredicateCondition::Never,
                ..
            })
        )
    }
}

/// Predicate test selected during checking.
///
/// Examples:
/// ```ds
/// value is string       // Unary
/// "name" in value       // Has
/// key in bag            // Call
/// value is "a" | "b"    // Any
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum PredicateTest {
    /// Unary test over one projected input value.
    ///
    /// Examples:
    /// ```ds
    /// value is string
    /// match value { "ready" => true }
    /// ```
    Unary(PredicateUnaryTest),
    /// Structural membership test over a receiver and key.
    ///
    /// Examples:
    /// ```ds
    /// "name" in value
    /// ```
    Has(PredicateHasTest),
    /// Operator-dispatched predicate call.
    ///
    /// Examples:
    /// ```ds
    /// key in bag // bag implements Has<typeof key>
    /// ```
    Call(CallResolution),
    /// Predicate that accepts when any alternative accepts.
    ///
    /// Examples:
    /// ```ds
    /// value is "yes" | "no"
    /// ```
    Any(Vec<Predicate>),
}

/// Unary predicate over one projected input value.
///
/// Examples:
/// ```ds
/// value is string       // input: Identity, condition: Primitive
/// value is Shape.Circle // input: VariantTag, condition: Literal
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PredicateUnaryTest {
    /// The value projection tested by this predicate.
    pub input: Projection,
    /// The condition applied to the projected input.
    pub condition: PredicateCondition,
}

/// Condition applied to one projected predicate input.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct PredicateHasTest {
    /// The projected receiver value.
    pub receiver: Projection,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
    Dynamic(Projection),
}

/// Scalar interval condition.
///
/// Examples:
/// ```ds
/// 0..=255
/// "a".."z"
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
