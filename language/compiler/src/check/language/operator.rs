use destack_dir as dir;

use destack_core::StringPool;
use smallvec::{SmallVec, smallvec};

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, Protocol, answer};

/// Result produced by one operator expression protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorExpressionResult {
    /// Use the selected method return.
    MethodReturn,
    /// Use the place behind the selected method's returned borrow.
    Pointee,
    /// Use the builtin boolean result.
    Boolean,
}

/// Static generic argument required by one operator protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorProtocolArgument {
    /// Static access argument.
    Access(dir::Access),
}

/// Method required by one operator protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum OperatorMethod {
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
    pub(in crate::check) fn key(self, strings: &StringPool) -> dir::StaticKey {
        dir::StaticKey::Name(strings.intern(self.name()))
    }

    /// Return the source member name for this protocol method.
    fn name(self) -> &'static str {
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

/// Language item protocol used by one operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct OperatorProtocol {
    /// The operator interface language item.
    pub(in crate::check) item: dir::LanguageItem,
    /// The required protocol arguments.
    pub(in crate::check) arguments: SmallVec<[OperatorProtocolArgument; 2]>,
    /// The required operator method.
    pub(in crate::check) method: OperatorMethod,
    /// The result produced by the operator expression.
    pub(in crate::check) expression_result: OperatorExpressionResult,
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
    /// Return whether builtin strict equality accepts two operand types.
    pub(in crate::check) fn supports_builtin_strict_equality(
        &mut self,
        origin: Origin,
        left: dir::GlobalTypeId,
        right: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<bool>> {
        let left = answer!(self.reduce_type_head(origin, left)?);
        let right = answer!(self.reduce_type_head(origin, right)?);

        // compare transparent newtypes through their backing representations
        if let Some(instance) = self.decompose_newtype(origin, left)? {
            return self.supports_builtin_strict_equality(origin, instance.backing, right);
        }
        if let Some(instance) = self.decompose_newtype(origin, right)? {
            return self.supports_builtin_strict_equality(origin, left, instance.backing);
        }

        // accept disjoint types, the caller reports their empty overlap
        if !answer!(self.types_may_overlap(origin, left, right)?) {
            return Ok(Answer::Ready(true));
        }

        // require builtin equality for every overlapping union pairing
        if let dir::Type::Union(union) = self.ty(left)? {
            let elements = self.type_ids(left.module_id, union.elements)?.to_vec();
            for element in elements {
                if !answer!(self.supports_builtin_strict_equality(origin, element, right)?) {
                    return Ok(Answer::Ready(false));
                }
            }

            return Ok(Answer::Ready(true));
        }
        if let dir::Type::Union(union) = self.ty(right)? {
            let elements = self.type_ids(right.module_id, union.elements)?.to_vec();
            for element in elements {
                if !answer!(self.supports_builtin_strict_equality(origin, left, element)?) {
                    return Ok(Answer::Ready(false));
                }
            }

            return Ok(Answer::Ready(true));
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
            return Ok(Answer::Ready(true));
        }

        // scalar values compare by value
        let left_is_scalar = answer!(self.scalar_families(origin, left)?).is_some();
        let right_is_scalar = answer!(self.scalar_families(origin, right)?).is_some();
        if left_is_scalar && right_is_scalar {
            return Ok(Answer::Ready(true));
        }

        // reference values compare by address
        let left_is_reference = answer!(self.type_is_reference(origin, left)?);
        let right_is_reference = answer!(self.type_is_reference(origin, right)?);

        Ok(Answer::Ready(left_is_reference && right_is_reference))
    }

    /// Return one operator protocol interface instance.
    pub(in crate::check) fn operator_protocol(
        &mut self,
        origin: Origin,
        protocol: &OperatorProtocol,
        type_arguments: &[dir::GlobalTypeId],
    ) -> CompilerResult<Protocol> {
        let module = origin.module();
        let mut arguments = Vec::with_capacity(type_arguments.len() + protocol.arguments.len());
        arguments.extend_from_slice(type_arguments);

        // append static protocol arguments from the operator form
        for argument in &protocol.arguments {
            match argument {
                OperatorProtocolArgument::Access(access) => {
                    let ty = dir::Type::Memory(dir::MemoryLiteral::Access(*access));
                    arguments.push(self.intern_type(module, ty)?);
                }
            }
        }

        self.language_protocol(module, protocol.item, arguments)
    }
}

/// Return the ordered protocol candidates for one unary operator.
pub(in crate::check) fn unary_operator_protocols(
    operator: dir::UnaryOperator,
    access: dir::Access,
) -> SmallVec<[OperatorProtocol; 2]> {
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
        | dir::UnaryOperator::Typeof
        | dir::UnaryOperator::Void
        | dir::UnaryOperator::Spread => return SmallVec::new(),
    };

    smallvec![protocol]
}

/// Return the ordered protocol candidates for one binary operator.
pub(in crate::check) fn binary_operator_protocols(
    operator: dir::BinaryOperator,
) -> SmallVec<[OperatorProtocol; 2]> {
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
