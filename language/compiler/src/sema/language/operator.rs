use smallvec::{SmallVec, smallvec};
use tspp_core::StringPool;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin, Protocol, Relation, Verdict};

/// The result one operator expression protocol produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum OperatorExpressionResult {
    /// Use the selected method return.
    MethodReturn,
    /// Use the place behind the selected method's returned borrow.
    Pointee,
    /// Use the builtin boolean result.
    Boolean,
}

/// The static generic argument one operator protocol requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum OperatorProtocolArgument {
    /// Static access argument.
    Access(dir::Access),
}

/// The method one operator protocol requires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum OperatorMethod {
    /// Addition method.
    Add,
    /// Subtraction method.
    Subtract,
    /// Multiplication method.
    Multiply,
    /// Division method.
    Divide,
    /// Remainder method.
    Remainder,
    /// Exponentiation method.
    Power,
    /// Bitwise and method.
    And,
    /// Bitwise or method.
    Or,
    /// Bitwise xor method.
    Xor,
    /// Bitwise not method.
    Not,
    /// Unary plus method.
    Plus,
    /// Unary negation method.
    Negate,
    /// Left shift method.
    ShiftLeft,
    /// Right shift method.
    ShiftRight,
    /// Unsigned right shift method.
    ShiftRightUnsigned,
    /// Dereference method.
    Dereference,
    /// Equality method.
    Equal,
    /// Comparison method.
    Compare,
    /// Partial comparison method.
    PartialCompare,
}

impl OperatorMethod {
    /// Return the source member key for this protocol method.
    pub(in crate::sema) fn key(self, strings: &StringPool) -> dir::StaticKey {
        dir::StaticKey::Name(strings.intern(self.name()))
    }

    /// Return the source member name for this protocol method.
    fn name(self) -> &'static str {
        // name each protocol method
        match self {
            Self::Add => "add",
            Self::Subtract => "subtract",
            Self::Multiply => "multiply",
            Self::Divide => "divide",
            Self::Remainder => "remainder",
            Self::Power => "power",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::Not => "not",
            Self::Plus => "plus",
            Self::Negate => "negate",
            Self::ShiftLeft => "shiftLeft",
            Self::ShiftRight => "shiftRight",
            Self::ShiftRightUnsigned => "shiftRightUnsigned",
            Self::Dereference => "dereference",
            Self::Equal => "equal",
            Self::Compare => "compare",
            Self::PartialCompare => "partialCompare",
        }
    }
}

/// The language item protocol one operator uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct OperatorProtocol {
    /// The operator interface language item.
    pub(in crate::sema) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::sema) arguments: SmallVec<[OperatorProtocolArgument; 2]>,
    /// The required operator method.
    pub(in crate::sema) method: OperatorMethod,
    /// The result the operator expression produces.
    pub(in crate::sema) expression_result: OperatorExpressionResult,
}

impl OperatorProtocol {
    /// Return one operator protocol.
    fn new(
        item: dir::LanguageItem,
        method: OperatorMethod,
        expression_result: OperatorExpressionResult,
    ) -> Self {
        Self {
            item,
            arguments: SmallVec::new(),
            method,
            expression_result,
        }
    }

    /// Return this operator protocol with static generic arguments.
    fn with_arguments(mut self, arguments: SmallVec<[OperatorProtocolArgument; 2]>) -> Self {
        self.arguments = arguments;

        self
    }
}

impl CheckState<'_> {
    /// Return whether every overlapping operand pair has strict equality.
    pub(in crate::sema) fn has_strict_equality(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // decide the exact capability through the ordinary interface relation
        let verdict = self.strict_equality_witness_verdict(origin, left, right)?;
        if verdict == Verdict::Holds {
            return Ok(true);
        }

        // require a parameter operand's witness, its instantiations comparing through it
        let left = self.normalize(origin, left)?;
        let right = self.normalize(origin, right)?;
        if matches!(self.ty(left)?, dir::Type::Parameter(_))
            || matches!(self.ty(right)?, dir::Type::Parameter(_))
        {
            return Ok(false);
        }
        if verdict == Verdict::Ambiguous {
            return Ok(true);
        }

        self.has_builtin_strict_equality(origin, left, right)
    }

    /// Return whether one operand's declared `StrictEqual<R>` witness covers the other.
    pub(in crate::sema) fn has_strict_equality_witness(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        Ok(self.strict_equality_witness_verdict(origin, left, right)? == Verdict::Holds)
    }

    /// Decide whether one operand conforms to `StrictEqual<R>` over the other.
    fn strict_equality_witness_verdict(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Verdict> {
        let target = self.language_type(dir::LanguageItem::StrictEqual, &[right])?;

        self.decide_relation(origin, Relation::Subtype, left, target)
    }

    /// Return whether every overlapping operand pair has builtin strict equality.
    pub(in crate::sema) fn has_builtin_strict_equality(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        // expose aliased operands before comparing their shapes
        let left = self.normalize(origin, left)?;
        let right = self.normalize(origin, right)?;

        // compare transparent newtypes through their backing representations
        if let Some(instance) = self.decompose_newtype(origin, left)? {
            return self.has_builtin_strict_equality(origin, instance.backing, right);
        }
        if let Some(instance) = self.decompose_newtype(origin, right)? {
            return self.has_builtin_strict_equality(origin, left, instance.backing);
        }

        // accept disjoint types and leave their empty overlap to the caller
        if !self.types_may_overlap(origin, left, right)? {
            return Ok(true);
        }

        // require builtin equality for every overlapping union pairing
        if let dir::Type::Union(union) = self.ty(left)? {
            let elements = self.type_ids(left.module_id, union.elements)?;
            for element in elements {
                if !self.has_builtin_strict_equality(origin, *element, right)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }
        if let dir::Type::Union(union) = self.ty(right)? {
            let elements = self.type_ids(right.module_id, union.elements)?;
            for element in elements {
                if !self.has_builtin_strict_equality(origin, left, *element)? {
                    return Ok(false);
                }
            }

            return Ok(true);
        }

        // accept a nullish operand without inspecting the other one
        let left_is_nullish = matches!(
            self.ty(left)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        );
        let right_is_nullish = matches!(
            self.ty(right)?,
            dir::Type::Null | dir::Type::Undefined | dir::Type::Never
        );
        if left_is_nullish || right_is_nullish {
            return Ok(true);
        }

        // scalar values compare by value
        let left_is_scalar = self.scalar_families(origin, left)?.is_some();
        let right_is_scalar = self.scalar_families(origin, right)?.is_some();
        if left_is_scalar && right_is_scalar {
            return Ok(true);
        }

        // reference values compare by address
        let left_is_reference = self.type_is_reference(origin, left)?;
        let right_is_reference = self.type_is_reference(origin, right)?;

        Ok(left_is_reference && right_is_reference)
    }

    /// Return one operator protocol interface instance.
    pub(in crate::sema) fn operator_protocol(
        &mut self,
        origin: Origin,
        protocol: &OperatorProtocol,
        type_arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Protocol> {
        // collect the arguments the protocol instance binds
        let mut arguments = Vec::with_capacity(type_arguments.len() + protocol.arguments.len());

        // start from the written type arguments, each asked as the implemented header binds it
        for argument in type_arguments {
            arguments.push(self.ask_argument(origin, *argument)?);
        }

        // append static protocol arguments from the operator form
        for argument in &protocol.arguments {
            match argument {
                OperatorProtocolArgument::Access(access) => {
                    let ty = self.access_literal(*access)?;
                    arguments.push(ty);
                }
            }
        }

        self.language_protocol(protocol.item, arguments)
    }
}

/// Return the ordered protocol candidates for one unary operator.
pub(in crate::sema) fn unary_operator_protocols(
    operator: dir::UnaryOperator,
    access: dir::Access,
) -> SmallVec<[OperatorProtocol; 2]> {
    // name the protocol each unary operator uses
    let protocol = match operator {
        dir::UnaryOperator::Negate => OperatorProtocol::new(
            dir::LanguageItem::Negate,
            OperatorMethod::Negate,
            OperatorExpressionResult::MethodReturn,
        ),
        dir::UnaryOperator::Plus => OperatorProtocol::new(
            dir::LanguageItem::Plus,
            OperatorMethod::Plus,
            OperatorExpressionResult::MethodReturn,
        ),
        dir::UnaryOperator::ElementwiseNot => OperatorProtocol::new(
            dir::LanguageItem::Not,
            OperatorMethod::Not,
            OperatorExpressionResult::MethodReturn,
        ),
        dir::UnaryOperator::Dereference => OperatorProtocol::new(
            dir::LanguageItem::Dereference,
            OperatorMethod::Dereference,
            OperatorExpressionResult::Pointee,
        )
        .with_arguments(smallvec![OperatorProtocolArgument::Access(access)]),
        dir::UnaryOperator::PostIncrement
        | dir::UnaryOperator::PostDecrement
        | dir::UnaryOperator::PreIncrement
        | dir::UnaryOperator::PreDecrement
        | dir::UnaryOperator::Not
        | dir::UnaryOperator::Spread => return SmallVec::new(),
    };

    smallvec![protocol]
}

/// Return the ordered protocol candidates for one binary operator.
pub(in crate::sema) fn binary_operator_protocols(
    operator: dir::BinaryOperator,
) -> SmallVec<[OperatorProtocol; 2]> {
    // name the protocols each binary operator uses
    match operator {
        dir::BinaryOperator::Add => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::Add,
                OperatorMethod::Add,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::Subtract => smallvec![OperatorProtocol::new(
            dir::LanguageItem::Subtract,
            OperatorMethod::Subtract,
            OperatorExpressionResult::MethodReturn,
        )],
        dir::BinaryOperator::Multiply => smallvec![OperatorProtocol::new(
            dir::LanguageItem::Multiply,
            OperatorMethod::Multiply,
            OperatorExpressionResult::MethodReturn,
        )],
        dir::BinaryOperator::Divide => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::Divide,
                OperatorMethod::Divide,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::Remainder => smallvec![OperatorProtocol::new(
            dir::LanguageItem::Remainder,
            OperatorMethod::Remainder,
            OperatorExpressionResult::MethodReturn,
        )],
        dir::BinaryOperator::Exponent => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::Power,
                OperatorMethod::Power,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::ShiftLeft => smallvec![OperatorProtocol::new(
            dir::LanguageItem::ShiftLeft,
            OperatorMethod::ShiftLeft,
            OperatorExpressionResult::MethodReturn,
        )],
        dir::BinaryOperator::ShiftRight => smallvec![OperatorProtocol::new(
            dir::LanguageItem::ShiftRight,
            OperatorMethod::ShiftRight,
            OperatorExpressionResult::MethodReturn,
        )],
        dir::BinaryOperator::UnsignedShiftRight => smallvec![OperatorProtocol::new(
            dir::LanguageItem::ShiftRightUnsigned,
            OperatorMethod::ShiftRightUnsigned,
            OperatorExpressionResult::MethodReturn,
        )],
        dir::BinaryOperator::ElementwiseAnd => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::And,
                OperatorMethod::And,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::ElementwiseXor => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::Xor,
                OperatorMethod::Xor,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::ElementwiseOr => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::Or,
                OperatorMethod::Or,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual => {
            smallvec![OperatorProtocol::new(
                dir::LanguageItem::PartialEqual,
                OperatorMethod::Equal,
                OperatorExpressionResult::MethodReturn,
            )]
        }
        dir::BinaryOperator::LessThan
        | dir::BinaryOperator::LessThanOrEqual
        | dir::BinaryOperator::GreaterThan
        | dir::BinaryOperator::GreaterThanOrEqual => smallvec![
            OperatorProtocol::new(
                dir::LanguageItem::Compare,
                OperatorMethod::Compare,
                OperatorExpressionResult::Boolean,
            ),
            OperatorProtocol::new(
                dir::LanguageItem::PartialCompare,
                OperatorMethod::PartialCompare,
                OperatorExpressionResult::Boolean,
            ),
        ],
        dir::BinaryOperator::EqualStrict
        | dir::BinaryOperator::NotEqualStrict
        | dir::BinaryOperator::And
        | dir::BinaryOperator::Or
        | dir::BinaryOperator::Coalesce
        | dir::BinaryOperator::In => SmallVec::new(),
    }
}
