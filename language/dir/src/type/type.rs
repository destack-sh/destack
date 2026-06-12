use destack_source::ModuleId;
use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, BinaryOperator, GlobalGenericParameterId, GlobalStaticId, GlobalSymbolId,
    MappedTypeModifier, ScalarLiteral, StaticKey, StringId, TypeLiteral, UnaryOperator,
};

use super::{FloatType, PrimitiveType};

/// Compiler-provided string mapping.
///
/// Examples:
/// ```ds
/// Uppercase<"id">      // "ID"
/// Capitalize<"name">   // "Name"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringMapping {
    /// Uppercase string mapping, like `Uppercase<"id">` reducing to `"ID"`.
    Uppercase,
    /// Lowercase string mapping, like `Lowercase<"ID">` reducing to `"id"`.
    Lowercase,
    /// Capitalize string mapping, like `Capitalize<"name">` reducing to `"Name"`.
    Capitalize,
    /// Uncapitalize string mapping, like `Uncapitalize<"Name">` reducing to `"name"`.
    Uncapitalize,
}

impl StringMapping {
    /// Apply this mapping to one string.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Uppercase => text.to_uppercase(),
            Self::Lowercase => text.to_lowercase(),
            Self::Capitalize => Self::recase(text, true),
            Self::Uncapitalize => Self::recase(text, false),
        }
    }

    /// Recase the first character of one string.
    fn recase(text: &str, upper: bool) -> String {
        let mut characters = text.chars();

        match characters.next() {
            Some(first) if upper => first.to_uppercase().collect::<String>() + characters.as_str(),
            Some(first) => first.to_lowercase().collect::<String>() + characters.as_str(),
            None => String::new(),
        }
    }
}

impl TryFrom<&str> for StringMapping {
    /// The error type for string mapping parsing.
    type Error = ();

    /// Parse a string mapping from its standard name.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Uppercase" => Ok(Self::Uppercase),
            "Lowercase" => Ok(Self::Lowercase),
            "Capitalize" => Ok(Self::Capitalize),
            "Uncapitalize" => Ok(Self::Uncapitalize),
            _ => Err(()),
        }
    }
}

/// Mapped-type modifiers.
///
/// Examples:
/// ```ds
/// { [K in keyof T]?: T[K] }              // optional: Present
/// { -readonly [K in keyof T]-?: T[K] }   // readonly and optional: Remove
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedTypeModifiers {
    /// The readonly modifier.
    pub readonly: MappedTypeModifier,
    /// The optional modifier.
    pub optional: MappedTypeModifier,
}

/// A mapped-type parameter.
///
/// Examples:
/// ```ds
/// [K in keyof T]
/// [K in "name" | "age" as Uppercase<K>]
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedTypeParameter {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The binder's generic parameter.
    pub parameter: GlobalGenericParameterId,
    /// The constraint type like `keyof T`.
    pub constraint: GlobalTypeId,
    /// The optional key remap like `as Foo<K>`.
    pub key_remap: Option<GlobalTypeId>,
}

/// One declaration applied to its complete positional arguments.
/// A non-generic reference is an instance with no arguments.
///
/// Examples:
/// ```ds
/// User                  // no arguments
/// Map<string, User>     // two positional arguments
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericInstance {
    /// The referenced declaration symbol.
    pub symbol: GlobalSymbolId,
    /// The complete positional arguments in declaration order.
    pub arguments: Vec<GlobalTypeId>,
}

/// Member type selected from an owner type.
///
/// Examples:
/// ```ds
/// T.Output              // the associated type selected on T
/// Ordering.Less         // the enum member selected on Ordering
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberType {
    /// The owner type.
    pub owner: GlobalTypeId,
    /// The selected member key.
    pub key: StaticKey,
    /// The complete positional arguments applied to the member.
    pub arguments: Vec<GlobalTypeId>,
}

/// Explicit runtime `Dynamic<T>` representation.
///
/// Examples:
/// ```ds
/// Dynamic<Printable>    // a boxed value known to satisfy Printable
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicType {
    /// The `Dynamic<T>` constraint.
    pub constraint: GlobalTypeId,
}

/// Canonical memory or access form.
/// Surface sigils spell these forms: `^User` is `Owned<User>`,
/// `&exclusive User` is `Borrowed<User, L, "exclusive">`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormType {
    /// The form constructor.
    pub form: Form,
    /// The type carried by the form.
    pub value: GlobalTypeId,
}

/// Canonical memory or access form constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    /// Automatically managed runtime value, the unqualified `User`.
    Managed,
    /// Owned value, like `^User`.
    Owned,
    /// Borrowed value, like `&User`, `&readonly User`, or `&exclusive User`.
    Borrowed {
        /// The solved borrow lifetime singleton.
        lifetime: GlobalTypeId,
        /// The solved borrow access singleton.
        access: GlobalTypeId,
    },
    /// Raw pointer value, like `*User`.
    Raw,
    /// Placed value, like `local User` or `shared User`.
    Placed {
        /// The solved concrete or ambient place singleton.
        place: GlobalTypeId,
    },
    /// Readonly view, like `readonly User`.
    Readonly,
}

/// Singleton type of one normalized memory value.
/// Literal spellings at language-item-typed positions normalize here:
/// the `"exclusive"` in `Borrowed<User, L, "exclusive">` commits as
/// `MemoryLiteral::Access(Access::Exclusive)`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryLiteral {
    /// Memory access singleton, like `"readonly"` or `"exclusive"`.
    Access(Access),
    /// Storage space singleton, like `"local"` or `"shared"`.
    Space(Space),
    /// Placement singleton, like `"ambient"` or a concrete space.
    Place(Place),
    /// Lifetime singleton, like `"static"` or a lifetime parameter.
    Lifetime(Lifetime),
}

impl MemoryLiteral {
    /// Return the canonical source text of one memory literal.
    pub fn text(&self) -> &'static str {
        match self {
            Self::Access(Access::Readonly) => "readonly",
            Self::Access(Access::Mutable) => "mutable",
            Self::Access(Access::Exclusive) => "exclusive",
            Self::Space(Space::Local) | Self::Place(Place::Space(Space::Local)) => "local",
            Self::Space(Space::Shared) | Self::Place(Place::Space(Space::Shared)) => "shared",
            Self::Space(Space::Static) | Self::Place(Place::Space(Space::Static)) => "static",
            Self::Space(Space::Frame) | Self::Place(Place::Space(Space::Frame)) => "frame",
            Self::Place(Place::Ambient) => "ambient",
            Self::Lifetime(Lifetime::Frame) => "frame",
            Self::Lifetime(_) => "static",
        }
    }
}

/// Normalized memory access value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Access {
    /// Shared readonly access.
    Readonly,
    /// Mutable access.
    Mutable,
    /// Exclusive access.
    Exclusive,
}

/// Normalized storage space value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    /// Local storage.
    Local,
    /// Shared storage.
    Shared,
    /// Static storage.
    Static,
    /// Frame storage.
    Frame,
}

/// Normalized place value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    /// Ambient placement.
    Ambient,
    /// Concrete storage space.
    Space(Space),
}

/// Normalized lifetime value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lifetime {
    /// Static lifetime.
    Static,
    /// The enclosing frame's lifetime.
    Frame,
    /// Symbolic lifetime parameter or associated constant.
    Symbol(GlobalSymbolId),
}

/// An index signature in an object type.
///
/// Examples:
/// ```ds
/// { [key: string]: int32 }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeIndexSignature {
    /// The parameter name like `K`.
    pub name: StringId,
    /// The key type.
    pub key_type: GlobalTypeId,
    /// The value type.
    pub value_type: GlobalTypeId,
    /// Whether the index signature is optional.
    pub is_optional: bool,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// A conditional type.
///
/// Examples:
/// ```ds
/// T extends string ? Text : Raw
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionalType {
    /// The left operand.
    pub left: GlobalTypeId,
    /// The right operand.
    pub right: GlobalTypeId,
    /// The type selected when the condition holds.
    pub then_type: GlobalTypeId,
    /// The type selected when the condition does not hold.
    pub else_type: GlobalTypeId,
    /// Whether the conditional distributes over union-valued left operands.
    pub is_distributive: bool,
}

/// A mapped type.
///
/// Examples:
/// ```ds
/// { [K in keyof T]: T[K] }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MappedType {
    /// The mapped parameter.
    pub parameter: MappedTypeParameter,
    /// The mapped modifiers.
    pub modifiers: MappedTypeModifiers,
    /// The mapped value type.
    pub value: GlobalTypeId,
}

/// Indexed access type.
///
/// Examples:
/// ```ds
/// User["name"]
/// Pair[0]
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexType {
    /// The indexed type.
    pub left: GlobalTypeId,
    /// The index type.
    pub index: GlobalTypeId,
}

/// A template literal type.
///
/// Examples:
/// ```ds
/// `get${Name}`
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateLiteralType {
    /// The literal string segments.
    pub strings: Vec<StringId>,
    /// The interpolated type spans.
    pub spans: Vec<GlobalTypeId>,
}

/// An infer binding inside a conditional type pattern.
///
/// Examples:
/// ```ds
/// T extends Array<infer E> ? E : never
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferType {
    /// The inferred binding name.
    pub name: Option<StringId>,
    /// The optional inferred constraint.
    pub constraint: Option<GlobalTypeId>,
}

/// A unary type operator.
///
/// Examples:
/// ```ds
/// keyof User
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnaryType {
    /// The target type.
    pub target: GlobalTypeId,
}

/// Homogeneous array type.
///
/// Examples:
/// ```ds
/// int32[]
/// Array<int32>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArrayType {
    /// The element type.
    pub element: GlobalTypeId,
}

/// A fixed-length array type.
///
/// Examples:
/// ```ds
/// [uint8; 4]
/// FixedArray<uint8, 4>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixedArrayType {
    /// The element type.
    pub element: GlobalTypeId,
    /// The static array length singleton.
    pub count: GlobalTypeId,
}

/// Compact scalar interval type.
///
/// Examples:
/// ```ds
/// 0..10
/// 0..=255
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RangeType {
    /// The inclusive lower bound.
    pub start: Option<ScalarLiteral>,
    /// The upper bound.
    pub end: Option<ScalarLiteral>,
    /// Whether the upper bound is included.
    pub is_inclusive: bool,
}

impl RangeType {
    /// Return whether every interval inhabitant fits one primitive type.
    pub fn fits_primitive(&self, primitive: PrimitiveType) -> bool {
        match primitive {
            // integer intervals fit when both bounds fit
            PrimitiveType::Integer(integer) => {
                let start_fits = match &self.start {
                    Some(ScalarLiteral::Integer(start)) => integer.fits_literal(*start),
                    Some(_) | None => false,
                };
                let end_fits = match &self.end {
                    Some(ScalarLiteral::Integer(end)) => integer.fits_literal(*end),
                    Some(_) | None => false,
                };

                start_fits && end_fits
            }
            // character intervals fit the character primitive
            PrimitiveType::Character => matches!(
                (&self.start, &self.end),
                (
                    Some(ScalarLiteral::Character(_)) | None,
                    Some(ScalarLiteral::Character(_)) | None,
                )
            ),
            _ => false,
        }
    }

    /// Return whether this interval contains another interval.
    pub fn contains(&self, inner: &RangeType) -> bool {
        // the outer start must not exceed the inner start
        let start_holds = match (&self.start, &inner.start) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(ScalarLiteral::Integer(outer)), Some(ScalarLiteral::Integer(inner))) => {
                outer <= inner
            }
            (Some(ScalarLiteral::Character(outer)), Some(ScalarLiteral::Character(inner))) => {
                outer <= inner
            }
            _ => false,
        };
        if !start_holds {
            return false;
        }

        // the outer end must not fall below the inner end
        match (&self.end, &inner.end) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(ScalarLiteral::Integer(outer_end)), Some(ScalarLiteral::Integer(inner_end))) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            (
                Some(ScalarLiteral::Character(outer_end)),
                Some(ScalarLiteral::Character(inner_end)),
            ) => {
                inner_end < outer_end
                    || (inner_end == outer_end && (self.is_inclusive || !inner.is_inclusive))
            }
            _ => false,
        }
    }
}

/// Runtime-length homogeneous view type.
///
/// Examples:
/// ```ds
/// [uint8]
/// Slice<uint8>
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceType {
    /// The element type.
    pub element: GlobalTypeId,
}

/// A tuple type.
///
/// Examples:
/// ```ds
/// (string, int32)
/// ["id", 42]
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleType {
    /// The tuple source form.
    pub form: TupleForm,
    /// The tuple elements.
    pub elements: Vec<TypeElement>,
}

/// The source form of a tuple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TupleForm {
    /// Parenthesized tuple form, like `(string, int32)`.
    Tuple,
    /// Bracket tuple form, like `["id", 42]`.
    Array,
}

/// A structural object shape type.
///
/// Examples:
/// ```ds
/// { name: string; age?: int32 }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeType {
    /// The shape fields.
    pub fields: Vec<TypeField>,
    /// The call signatures.
    pub call_signatures: Vec<GlobalTypeId>,
    /// The construct signatures.
    pub construct_signatures: Vec<GlobalTypeId>,
    /// The index signatures.
    pub index_signatures: Vec<TypeIndexSignature>,
}

/// A function type.
///
/// Examples:
/// ```ds
/// (value: int32) => string
/// async <T>(input: T) => Promise<T>
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    /// The function asynchrony.
    pub asynchrony: Asynchrony,
    /// The generic parameter types.
    pub generic_parameters: Vec<GlobalTypeId>,
    /// The optional `this` parameter type.
    pub this_parameter: Option<GlobalTypeId>,
    /// The runtime parameters.
    pub parameters: Vec<FunctionParameterType>,
    /// The optional return type.
    pub return_type: Option<GlobalTypeId>,
    /// Whether this is a generator function.
    pub is_generator: bool,
}

/// A runtime parameter in a function type.
///
/// Examples:
/// ```ds
/// (value?: int32, ...rest: string[]) => void
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionParameterType {
    /// The parameter type.
    pub ty: GlobalTypeId,
    /// The static generic parameter supplied by this runtime argument.
    pub static_parameter: Option<GlobalGenericParameterId>,
    /// Whether the parameter may be omitted at the call site.
    pub is_optional: bool,
    /// Whether the parameter captures remaining call arguments.
    pub is_rest: bool,
}

/// A closure type with its function contract and captured environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClosureType {
    /// The function contract.
    pub function: GlobalTypeId,
    /// The captured environment type.
    pub environment: GlobalTypeId,
}

/// A union type.
///
/// Examples:
/// ```ds
/// string | int32
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnionType {
    /// The union elements.
    pub elements: Vec<GlobalTypeId>,
}

/// An intersection type.
///
/// Examples:
/// ```ds
/// Named & Aged
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntersectionType {
    /// The intersection elements.
    pub elements: Vec<GlobalTypeId>,
}

/// One static binary operation over singleton operands.
///
/// Examples:
/// ```ds
/// N * 2
/// Mode == "inline"
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticBinaryType {
    /// The applied operator.
    pub operator: StaticBinaryOperator,
    /// The left operand.
    pub left: GlobalTypeId,
    /// The right operand.
    pub right: GlobalTypeId,
}

/// One static binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StaticBinaryOperator {
    /// `left + right`.
    Add,
    /// `left - right`.
    Subtract,
    /// `left * right`.
    Multiply,
    /// `left / right`.
    Divide,
    /// `left % right`.
    Remainder,
    /// `left ** right`.
    Exponent,
    /// `left << right`.
    ShiftLeft,
    /// `left >> right`.
    ShiftRight,
    /// `left >>> right`.
    UnsignedShiftRight,
    /// `left & right`.
    BitwiseAnd,
    /// `left ^ right`.
    BitwiseXor,
    /// `left | right`.
    BitwiseOr,
    /// `left == right`.
    Equal,
    /// `left === right`.
    EqualStrict,
    /// `left != right`.
    NotEqual,
    /// `left !== right`.
    NotEqualStrict,
    /// `left < right`.
    LessThan,
    /// `left <= right`.
    LessThanOrEqual,
    /// `left > right`.
    GreaterThan,
    /// `left >= right`.
    GreaterThanOrEqual,
    /// `left && right`.
    And,
    /// `left || right`.
    Or,
}

impl StaticBinaryOperator {
    /// Evaluate this operator over two scalar literals.
    pub fn apply(
        self,
        left: ScalarLiteral,
        right: ScalarLiteral,
    ) -> Result<ScalarLiteral, &'static str> {
        use ScalarLiteral as Literal;
        use StaticBinaryOperator as Operator;

        let literal = match (self, left, right) {
            // integer arithmetic is checked
            (Operator::Add, Literal::Integer(left), Literal::Integer(right)) => Literal::Integer(
                left.checked_add(right)
                    .ok_or("integer addition overflows")?,
            ),
            (Operator::Subtract, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(
                    left.checked_sub(right)
                        .ok_or("integer subtraction overflows")?,
                )
            }
            (Operator::Multiply, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(
                    left.checked_mul(right)
                        .ok_or("integer multiplication overflows")?,
                )
            }
            (Operator::Divide, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left.checked_div(right).ok_or("integer division by zero")?)
            }
            (Operator::Remainder, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left.checked_rem(right).ok_or("integer remainder by zero")?)
            }
            (Operator::Exponent, Literal::Integer(left), Literal::Integer(right)) => {
                let exponent =
                    u32::try_from(right).map_err(|_| "integer exponent must be non-negative")?;

                Literal::Integer(
                    left.checked_pow(exponent)
                        .ok_or("integer exponentiation overflows")?,
                )
            }

            // shifts and bitwise operations stay in integer space
            (Operator::ShiftLeft, Literal::Integer(left), Literal::Integer(right)) => {
                let amount =
                    u32::try_from(right).map_err(|_| "shift amount must be non-negative")?;

                Literal::Integer(left.checked_shl(amount).ok_or("shift amount too large")?)
            }
            (Operator::ShiftRight, Literal::Integer(left), Literal::Integer(right)) => {
                let amount =
                    u32::try_from(right).map_err(|_| "shift amount must be non-negative")?;

                Literal::Integer(left.checked_shr(amount).ok_or("shift amount too large")?)
            }
            (Operator::UnsignedShiftRight, Literal::Integer(left), Literal::Integer(right)) => {
                let amount =
                    u32::try_from(right).map_err(|_| "shift amount must be non-negative")?;
                let shifted = (left as u64)
                    .checked_shr(amount)
                    .ok_or("shift amount too large")?;

                Literal::Integer(shifted as i64)
            }
            (Operator::BitwiseAnd, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left & right)
            }
            (Operator::BitwiseXor, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left ^ right)
            }
            (Operator::BitwiseOr, Literal::Integer(left), Literal::Integer(right)) => {
                Literal::Integer(left | right)
            }

            // float arithmetic follows ieee semantics
            (Operator::Add, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left + right)
            }
            (Operator::Subtract, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left - right)
            }
            (Operator::Multiply, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left * right)
            }
            (Operator::Divide, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left / right)
            }
            (Operator::Remainder, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left % right)
            }
            (Operator::Exponent, Literal::Float(left), Literal::Float(right)) => {
                Literal::Float(left.powf(right))
            }

            // ordering compares within one operand kind
            (
                Operator::LessThan
                | Operator::LessThanOrEqual
                | Operator::GreaterThan
                | Operator::GreaterThanOrEqual,
                left,
                right,
            ) => {
                let ordering = match (left, right) {
                    (Literal::Integer(left), Literal::Integer(right)) => left.cmp(&right),
                    (Literal::Float(left), Literal::Float(right)) => left
                        .partial_cmp(&right)
                        .ok_or("float comparison is undefined for nan")?,
                    (Literal::Character(left), Literal::Character(right)) => left.cmp(&right),
                    _ => return Err("static comparison requires matching operand kinds"),
                };

                Literal::Boolean(match self {
                    Operator::LessThan => ordering.is_lt(),
                    Operator::LessThanOrEqual => ordering.is_le(),
                    Operator::GreaterThan => ordering.is_gt(),
                    _ => ordering.is_ge(),
                })
            }

            // equality compares across kinds, distinct kinds compare unequal
            (
                Operator::Equal
                | Operator::EqualStrict
                | Operator::NotEqual
                | Operator::NotEqualStrict,
                left,
                right,
            ) => {
                let equal = match (left, right) {
                    (Literal::Integer(left), Literal::Integer(right)) => left == right,
                    (Literal::Float(left), Literal::Float(right)) => left == right,
                    (Literal::Boolean(left), Literal::Boolean(right)) => left == right,
                    (Literal::String(left), Literal::String(right)) => left == right,
                    (Literal::Character(left), Literal::Character(right)) => left == right,
                    (Literal::Null, Literal::Null) => true,
                    (Literal::Undefined, Literal::Undefined) => true,
                    _ => false,
                };

                let negated = matches!(self, Operator::NotEqual | Operator::NotEqualStrict);

                Literal::Boolean(equal != negated)
            }

            // logical joins reach here only with non-boolean operands
            (Operator::And | Operator::Or, _, _) => {
                return Err("logical operator requires boolean operands");
            }

            _ => return Err("static operator does not apply to its operand kinds"),
        };

        Ok(literal)
    }
}

impl TryFrom<BinaryOperator> for StaticBinaryOperator {
    type Error = ();

    /// Map one source binary operator onto its static operator.
    fn try_from(operator: BinaryOperator) -> Result<Self, ()> {
        let operator = match operator {
            BinaryOperator::Add => Self::Add,
            BinaryOperator::Subtract => Self::Subtract,
            BinaryOperator::Multiply => Self::Multiply,
            BinaryOperator::Divide => Self::Divide,
            BinaryOperator::Remainder => Self::Remainder,
            BinaryOperator::Exponent => Self::Exponent,
            BinaryOperator::ShiftLeft => Self::ShiftLeft,
            BinaryOperator::ShiftRight => Self::ShiftRight,
            BinaryOperator::UnsignedShiftRight => Self::UnsignedShiftRight,
            BinaryOperator::ElementwiseAnd => Self::BitwiseAnd,
            BinaryOperator::ElementwiseXor => Self::BitwiseXor,
            BinaryOperator::ElementwiseOr => Self::BitwiseOr,
            BinaryOperator::Equal => Self::Equal,
            BinaryOperator::EqualStrict => Self::EqualStrict,
            BinaryOperator::NotEqual => Self::NotEqual,
            BinaryOperator::NotEqualStrict => Self::NotEqualStrict,
            BinaryOperator::LessThan => Self::LessThan,
            BinaryOperator::LessThanOrEqual => Self::LessThanOrEqual,
            BinaryOperator::GreaterThan => Self::GreaterThan,
            BinaryOperator::GreaterThanOrEqual => Self::GreaterThanOrEqual,
            BinaryOperator::And => Self::And,
            BinaryOperator::Or => Self::Or,
            _ => return Err(()),
        };

        Ok(operator)
    }
}

/// One static unary operation over one singleton operand.
///
/// Examples:
/// ```ds
/// !Wide
/// -Offset
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticUnaryType {
    /// The applied operator.
    pub operator: StaticUnaryOperator,
    /// The operand.
    pub target: GlobalTypeId,
}

/// One static unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StaticUnaryOperator {
    /// `!target`.
    Not,
    /// `-target`.
    Negate,
    /// `~target`.
    BitwiseNot,
}

impl TryFrom<UnaryOperator> for StaticUnaryOperator {
    type Error = ();

    /// Map one source unary operator onto its static operator.
    fn try_from(operator: UnaryOperator) -> Result<Self, ()> {
        let operator = match operator {
            UnaryOperator::Not => Self::Not,
            UnaryOperator::Negate => Self::Negate,
            UnaryOperator::ElementwiseNot => Self::BitwiseNot,
            _ => return Err(()),
        };

        Ok(operator)
    }
}

/// Type-level operation preserved by check.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeOperation {
    /// Compiler-known string mapping type, like `Uppercase<S>`.
    StringMapping {
        /// The string mapping operation.
        mapping: StringMapping,
        /// The mapped string type.
        target: GlobalTypeId,
    },
    /// Conditional type expression, like `T extends string ? A : B`.
    Conditional(ConditionalType),
    /// Mapped type expression, like `{ [K in keyof T]: T[K] }`.
    Mapped(MappedType),
    /// Indexed access type expression, like `User["name"]`.
    Index(IndexType),
    /// Template literal type expression, like `` `get${Name}` ``.
    TemplateLiteral(TemplateLiteralType),
    /// Type infer binding in a conditional type pattern, like `infer E`.
    Infer(InferType),
    /// `keyof T`.
    KeyOf(UnaryType),
    /// Try success projection like `value?` continuing evaluation.
    TryOutput {
        /// The tried value type.
        value: GlobalTypeId,
    },
    /// Try failure projection like `value?` propagating its residual.
    TryResidual {
        /// The tried value type.
        value: GlobalTypeId,
    },
    /// Static binary operation like `N * 2` or `Mode == "inline"`.
    StaticBinary(StaticBinaryType),
    /// Static unary operation like `!Wide`.
    StaticUnary(StaticUnaryType),
}

/// A canonical solved type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// One open inference variable.
    /// Only present in working types during check, never in committed tables.
    Variable(TypeVariableId),

    /// Error type that could not be resolved.
    Error,
    /// Never type `never`.
    Never,
    /// TypeScript `any` compatibility marker.
    Any,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Undefined type and value.
    Undefined,
    /// TypeScript `object` constraint.
    Object,
    /// Primitive type, like `string` or `int32`.
    Primitive(PrimitiveType),
    /// Scalar literal type, like `"id"` or `42`.
    Literal(ScalarLiteral),
    /// Singleton type of one normalized memory value.
    /// The only committed spelling: check normalizes literal spellings like
    /// `"shared"` to memory singletons at language-item-typed positions.
    Memory(MemoryLiteral),
    /// Singleton type of one committed static value.
    Static(GlobalStaticId),
    /// Compiler intrinsic type body.
    Intrinsic,

    /// Generic parameter, like the `T` in `class Box<T>`.
    Parameter(GlobalGenericParameterId),
    /// Type declaration reference, like `User` or `Map<string, User>`.
    Reference(GenericInstance),
    /// This type in a method signature, like `this` in `clone(): this`.
    This,
    /// Member type selected from an owner type, like `T.Output`.
    Member(MemberType),

    /// Canonical memory or access form, like `^User` or `&exclusive User`.
    Form(FormType),
    /// Explicit runtime `Dynamic<T>` representation, like `Dynamic<Printable>`.
    Dynamic(DynamicType),

    /// Type-level operation preserved by check.
    Operation(TypeOperation),

    /// Homogeneous array type, like `int32[]`.
    Array(ArrayType),
    /// Fixed-length array type, like `[uint8; 4]`.
    FixedArray(FixedArrayType),
    /// Compact scalar interval type, like `0..10`.
    Range(RangeType),
    /// Runtime-length homogeneous view type, like `[uint8]`.
    Slice(SliceType),
    /// Tuple type, like `(string, int32)`.
    Tuple(TupleType),
    /// Structural object shape type, like `{ name: string }`.
    Shape(ShapeType),
    /// Function type, like `(value: int32) => string`.
    Function(FunctionType),
    /// Closure type with an explicit captured environment.
    Closure(ClosureType),

    /// Union type `A | B | C`.
    Union(UnionType),
    /// Intersection type `A & B & C`.
    Intersection(IntersectionType),
}

impl From<TypeLiteral> for Type {
    /// Convert a source type literal into a type.
    fn from(value: TypeLiteral) -> Self {
        match value {
            TypeLiteral::Never => Self::Never,
            TypeLiteral::Any => Self::Any,
            TypeLiteral::Undefined => Self::Undefined,
            TypeLiteral::Unknown => Self::Unknown,
            TypeLiteral::Object => Self::Object,
            TypeLiteral::Void => Self::Void,
            TypeLiteral::Null => Self::Null,
            TypeLiteral::Boolean => Self::Primitive(PrimitiveType::Boolean),
            TypeLiteral::Character => Self::Primitive(PrimitiveType::Character),
            TypeLiteral::String => Self::Primitive(PrimitiveType::String),
            TypeLiteral::Bigint => Self::Primitive(PrimitiveType::Bigint),
            TypeLiteral::Number => Self::Primitive(PrimitiveType::Float(FloatType::Float64)),
            TypeLiteral::Integer(integer) => Self::Primitive(PrimitiveType::Integer(integer)),
            TypeLiteral::Float(float) => Self::Primitive(PrimitiveType::Float(float)),
            TypeLiteral::Symbol => Self::Primitive(PrimitiveType::Symbol),
            TypeLiteral::UniqueSymbol => Self::Primitive(PrimitiveType::UniqueSymbol),
        }
    }
}

impl From<ScalarLiteral> for Type {
    /// Convert a scalar literal expression into its fresh type.
    fn from(value: ScalarLiteral) -> Self {
        Self::from(&value)
    }
}

impl From<&ScalarLiteral> for Type {
    /// Convert a scalar literal expression into its fresh type.
    fn from(value: &ScalarLiteral) -> Self {
        match value {
            ScalarLiteral::Null => Self::Null,
            ScalarLiteral::Undefined => Self::Undefined,
            ScalarLiteral::Boolean(_) => Self::Primitive(PrimitiveType::Boolean),
            ScalarLiteral::Character(_) => Self::Primitive(PrimitiveType::Character),
            ScalarLiteral::String(_)
            | ScalarLiteral::Integer(_)
            | ScalarLiteral::Float(_)
            | ScalarLiteral::Bigint(_) => Self::Literal(*value),
            ScalarLiteral::RegexString { .. } => Self::Object,
        }
    }
}

impl Type {
    /// Visit each direct child type id of this type.
    pub fn for_each_child(&self, mut visit: impl FnMut(GlobalTypeId)) {
        match self {
            // leaves without child types
            Self::Variable(_)
            | Self::Error
            | Self::Never
            | Self::Any
            | Self::Unknown
            | Self::Void
            | Self::Null
            | Self::Undefined
            | Self::Object
            | Self::Primitive(_)
            | Self::Literal(_)
            | Self::Memory(_)
            | Self::Static(_)
            | Self::Intrinsic
            | Self::Parameter(_)
            | Self::This
            | Self::Range(_) => {}

            // declaration applications
            Self::Reference(instance) => {
                for child in instance.arguments.iter().copied() {
                    visit(child);
                }
            }
            Self::Member(member) => {
                visit(member.owner);
                for child in member.arguments.iter().copied() {
                    visit(child);
                }
            }

            // memory forms
            Self::Form(form) => {
                visit(form.value);
                match &form.form {
                    Form::Borrowed { lifetime, access } => {
                        visit(*lifetime);
                        visit(*access);
                    }
                    Form::Placed { place } => visit(*place),
                    Form::Managed | Form::Owned | Form::Raw | Form::Readonly => {}
                }
            }
            Self::Dynamic(dynamic) => visit(dynamic.constraint),

            // type operations
            Self::Operation(operation) => match operation {
                TypeOperation::StringMapping { mapping: _, target } => visit(*target),
                TypeOperation::Conditional(conditional) => {
                    visit(conditional.left);
                    visit(conditional.right);
                    visit(conditional.then_type);
                    visit(conditional.else_type);
                }
                TypeOperation::Mapped(mapped) => {
                    visit(mapped.parameter.constraint);
                    if let Some(key_remap) = mapped.parameter.key_remap {
                        visit(key_remap);
                    }
                    visit(mapped.value);
                }
                TypeOperation::Index(index) => {
                    visit(index.left);
                    visit(index.index);
                }
                TypeOperation::TemplateLiteral(template) => {
                    for child in template.spans.iter().copied() {
                        visit(child);
                    }
                }
                TypeOperation::Infer(infer) => {
                    if let Some(constraint) = infer.constraint {
                        visit(constraint);
                    }
                }
                TypeOperation::KeyOf(unary) => visit(unary.target),
                TypeOperation::TryOutput { value } | TypeOperation::TryResidual { value } => {
                    visit(*value)
                }
                TypeOperation::StaticBinary(binary) => {
                    visit(binary.left);
                    visit(binary.right);
                }
                TypeOperation::StaticUnary(unary) => visit(unary.target),
            },

            // collections
            Self::Array(array) => visit(array.element),
            Self::FixedArray(array) => {
                visit(array.element);
                visit(array.count);
            }
            Self::Slice(slice) => visit(slice.element),
            Self::Tuple(tuple) => {
                for child in tuple.elements.iter().map(|element| element.ty) {
                    visit(child);
                }
            }

            // structural shapes
            Self::Shape(shape) => {
                for child in shape.fields.iter().map(|field| field.ty) {
                    visit(child);
                }
                for child in shape.call_signatures.iter().copied() {
                    visit(child);
                }
                for child in shape.construct_signatures.iter().copied() {
                    visit(child);
                }
                for signature in &shape.index_signatures {
                    visit(signature.key_type);
                    visit(signature.value_type);
                }
            }
            Self::Function(function) => {
                for child in function.generic_parameters.iter().copied() {
                    visit(child);
                }
                if let Some(this_parameter) = function.this_parameter {
                    visit(this_parameter);
                }
                for child in function.parameters.iter().map(|parameter| parameter.ty) {
                    visit(child);
                }
                if let Some(return_type) = function.return_type {
                    visit(return_type);
                }
            }
            Self::Closure(closure) => {
                visit(closure.function);
                visit(closure.environment);
            }

            // algebraic composites
            Self::Union(union) => {
                for child in union.elements.iter().copied() {
                    visit(child);
                }
            }
            Self::Intersection(intersection) => {
                for child in intersection.elements.iter().copied() {
                    visit(child);
                }
            }
        }
    }

    /// Apply one mapping to every direct child type id of this type.
    pub fn map_children(&mut self, map: &mut impl FnMut(GlobalTypeId) -> GlobalTypeId) {
        match self {
            // leaves without child types
            Self::Variable(_)
            | Self::Error
            | Self::Never
            | Self::Any
            | Self::Unknown
            | Self::Void
            | Self::Null
            | Self::Undefined
            | Self::Object
            | Self::Primitive(_)
            | Self::Literal(_)
            | Self::Memory(_)
            | Self::Static(_)
            | Self::Intrinsic
            | Self::Parameter(_)
            | Self::This
            | Self::Range(_) => {}

            // declaration applications
            Self::Reference(instance) => {
                for argument in &mut instance.arguments {
                    *argument = map(*argument);
                }
            }
            Self::Member(member) => {
                member.owner = map(member.owner);
                for argument in &mut member.arguments {
                    *argument = map(*argument);
                }
            }

            // memory forms
            Self::Form(form) => {
                form.value = map(form.value);
                match &mut form.form {
                    Form::Borrowed { lifetime, access } => {
                        *lifetime = map(*lifetime);
                        *access = map(*access);
                    }
                    Form::Placed { place } => *place = map(*place),
                    Form::Managed | Form::Owned | Form::Raw | Form::Readonly => {}
                }
            }
            Self::Dynamic(dynamic) => dynamic.constraint = map(dynamic.constraint),

            // type operations
            Self::Operation(operation) => match operation {
                TypeOperation::StringMapping { mapping: _, target } => *target = map(*target),
                TypeOperation::Conditional(conditional) => {
                    conditional.left = map(conditional.left);
                    conditional.right = map(conditional.right);
                    conditional.then_type = map(conditional.then_type);
                    conditional.else_type = map(conditional.else_type);
                }
                TypeOperation::Mapped(mapped) => {
                    mapped.parameter.constraint = map(mapped.parameter.constraint);
                    if let Some(key_remap) = &mut mapped.parameter.key_remap {
                        *key_remap = map(*key_remap);
                    }
                    mapped.value = map(mapped.value);
                }
                TypeOperation::Index(index) => {
                    index.left = map(index.left);
                    index.index = map(index.index);
                }
                TypeOperation::TemplateLiteral(template) => {
                    for span in &mut template.spans {
                        *span = map(*span);
                    }
                }
                TypeOperation::Infer(infer) => {
                    if let Some(constraint) = &mut infer.constraint {
                        *constraint = map(*constraint);
                    }
                }
                TypeOperation::KeyOf(unary) => unary.target = map(unary.target),
                TypeOperation::TryOutput { value } | TypeOperation::TryResidual { value } => {
                    *value = map(*value)
                }
                TypeOperation::StaticBinary(binary) => {
                    binary.left = map(binary.left);
                    binary.right = map(binary.right);
                }
                TypeOperation::StaticUnary(unary) => unary.target = map(unary.target),
            },

            // collections
            Self::Array(array) => array.element = map(array.element),
            Self::FixedArray(array) => {
                array.element = map(array.element);
                array.count = map(array.count);
            }
            Self::Slice(slice) => slice.element = map(slice.element),
            Self::Tuple(tuple) => {
                for element in &mut tuple.elements {
                    element.ty = map(element.ty);
                }
            }

            // structural shapes
            Self::Shape(shape) => {
                for field in &mut shape.fields {
                    field.ty = map(field.ty);
                }
                for signature in &mut shape.call_signatures {
                    *signature = map(*signature);
                }
                for signature in &mut shape.construct_signatures {
                    *signature = map(*signature);
                }
                for signature in &mut shape.index_signatures {
                    signature.key_type = map(signature.key_type);
                    signature.value_type = map(signature.value_type);
                }
            }
            Self::Function(function) => {
                for parameter in &mut function.generic_parameters {
                    *parameter = map(*parameter);
                }
                if let Some(this_parameter) = &mut function.this_parameter {
                    *this_parameter = map(*this_parameter);
                }
                for parameter in &mut function.parameters {
                    parameter.ty = map(parameter.ty);
                }
                if let Some(return_type) = &mut function.return_type {
                    *return_type = map(*return_type);
                }
            }
            Self::Closure(closure) => {
                closure.function = map(closure.function);
                closure.environment = map(closure.environment);
            }

            // algebraic composites
            Self::Union(union) => {
                for element in &mut union.elements {
                    *element = map(*element);
                }
            }
            Self::Intersection(intersection) => {
                for element in &mut intersection.elements {
                    *element = map(*element);
                }
            }
        }
    }

    /// Whether the type is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// Whether the type is unknown.
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }

    /// Whether the type is an error or unknown.
    pub fn is_error_or_unknown(&self) -> bool {
        self.is_error() || self.is_unknown()
    }

    /// Return the symbol if this type directly references one declaration.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Self::Reference(reference) => Some(reference.symbol),
            _ => None,
        }
    }
}

/// A field in an object-like type.
/// Methods are represented as fields whose `ty` is a `Type::Function`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeField {
    /// The key of the field.
    pub key: StaticKey,
    /// The type of the field.
    pub ty: GlobalTypeId,
    /// Whether the field is optional.
    pub is_optional: bool,
    /// Whether the field is readonly.
    pub is_readonly: bool,
}

/// An element in a tuple type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeElement {
    /// The optional label for the element.
    pub label: Option<StringId>,
    /// The element type.
    pub ty: GlobalTypeId,
    /// Whether the element is optional.
    pub is_optional: bool,
    /// Whether the element is readonly.
    pub is_readonly: bool,
    /// Whether the element is a rest element.
    pub is_rest: bool,
}

impl TypeElement {
    /// Create a default tuple element for a type.
    pub fn new(ty: GlobalTypeId) -> Self {
        Self {
            label: None,
            ty,
            is_optional: false,
            is_readonly: false,
            is_rest: false,
        }
    }
}

/// Identifier for one open inference variable inside a checked component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TypeVariableId {
    /// The module that allocated the variable.
    pub module_id: ModuleId,
    /// The variable index inside the module.
    pub index: u32,
}

impl TypeVariableId {
    /// Create a new type variable id.
    pub fn new(module_id: ModuleId, index: u32) -> Self {
        Self { module_id, index }
    }
}

/// Unique identifier for a local type.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalTypeId(pub u32);

impl LocalTypeId {
    /// Wrap a raw id as a LocalTypeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Convert this id into a global type id.
    pub fn into_global(self, module_id: ModuleId) -> GlobalTypeId {
        GlobalTypeId {
            module_id,
            local_id: self,
        }
    }
}

/// Global type id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GlobalTypeId {
    /// The module id of the global type.
    pub module_id: ModuleId,
    /// The local id of the global type.
    pub local_id: LocalTypeId,
}

impl GlobalTypeId {
    /// Create a new global type id.
    pub fn new(module_id: ModuleId, local_id: LocalTypeId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Convert this id into a local type id.
    pub fn into_local(self) -> LocalTypeId {
        self.local_id
    }
}

impl From<GlobalTypeId> for LocalTypeId {
    fn from(id: GlobalTypeId) -> Self {
        id.local_id
    }
}
