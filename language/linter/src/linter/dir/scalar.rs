use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

/// One builtin integer comparison with its constant on the right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IntegerComparison {
    /// The nonconstant expression.
    pub(crate) value: dir::LocalNodeId<dir::Expression>,
    /// The normalized comparison operator.
    pub(crate) operator: dir::BinaryOperator,
}

/// One builtin integral expression offset by exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerStep {
    /// One value incremented by one.
    Increment(dir::LocalNodeId<dir::Expression>),
    /// One value decremented by one.
    Decrement(dir::LocalNodeId<dir::Expression>),
}

impl DirModule<'_> {
    /// Return one builtin integer comparison against an exact constant.
    pub(crate) fn integer_comparison(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        constant: i64,
    ) -> Result<Option<IntegerComparison>, ProviderError> {
        let Some((operator, [left, right])) = self.builtin_binary(expression)? else {
            return Ok(None);
        };
        let Some(swapped) = operator.swapped() else {
            return Ok(None);
        };
        let left_constant = self.integral_constant(left.source.local_id)? == Some(constant);
        let right_constant = self.integral_constant(right.source.local_id)? == Some(constant);

        // normalize the sole matching constant onto the right
        let comparison = match (left_constant, right_constant) {
            (false, true) => IntegerComparison {
                value: left.source.local_id,
                operator,
            },
            (true, false) => IntegerComparison {
                value: right.source.local_id,
                operator: swapped,
            },
            _ => return Ok(None),
        };

        Ok(Some(comparison))
    }

    /// Return the authored integral value carried by one represented newtype construction.
    pub(crate) fn newtype_integral(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
        item: dir::LanguageItem,
    ) -> Result<Option<(i64, dir::LocalNodeId<dir::Expression>)>, ProviderError> {
        let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            is_optional: false,
            ..
        } = self.view().get(expression)
        else {
            return Ok(None);
        };
        if !generic_arguments.is_empty() || self.language_item(*left)? != Some(item) {
            return Ok(None);
        }
        if self.representation_item(expression.into_any())? != Some(item) {
            return Err(ProviderError::internal(format!(
                "language item constructor {item:?} has another representation"
            )));
        }
        let [argument] = arguments.as_slice() else {
            return Ok(None);
        };
        let Some(value) = self.view().get(*argument).value() else {
            return Ok(None);
        };
        let Some(number) = self.integral_constant(value)? else {
            return Ok(None);
        };

        Ok(Some((number, value)))
    }

    /// Return the concrete primitive type selected for one node.
    pub fn primitive_type(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Result<Option<dir::PrimitiveType>, ProviderError> {
        // inspect the represented value beneath placement forms
        let type_id = self.node_type_id(node)?;
        let type_id = self.dir.strip_form(type_id)?;
        let ty = self.dir.get_type(type_id)?;

        let primitive = match ty {
            dir::Type::Primitive(primitive) => Some(primitive),
            _ => None,
        };

        Ok(primitive)
    }

    /// Return the exact scalar constant selected by one expression.
    pub fn scalar_constant(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::Literal>, ProviderError> {
        // read authored scalar literals directly
        if let Some(value) = self.view().get(node).as_scalar() {
            return Ok(Some(value));
        }

        // select one symbol-backed constant expression
        if let Some(symbol) = self.selected_symbol(node)?
            && let Some(value) = self.dir.symbol_static(symbol)?
        {
            let value = match value {
                dir::StaticTerm::Literal { value } => Some(value),
                _ => None,
            };
            if value.is_some() {
                return Ok(value);
            }
        }

        // read exact literal expression types directly
        let value = match self.node_type(node.into_any())? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };

        Ok(value)
    }

    /// Return the exact integral constant selected by one expression.
    pub fn integral_constant(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<i64>, ProviderError> {
        let value = self
            .scalar_constant(node)?
            .and_then(|value| value.as_integral());

        Ok(value)
    }

    /// Return whether one integral expression is an exact all-ones constant.
    pub(crate) fn is_all_ones_constant(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        let Some(value) = self.integral_constant(expression)? else {
            return Ok(false);
        };
        let type_id = self.adjusted_type_id(expression.into_any())?;
        let type_id = self.dir.strip_form(type_id)?;
        let ty = self.dir.get_type(type_id)?;
        if ty.scalar_domain() == Some(dir::ScalarDomain::Bigint) {
            return Ok(value == -1);
        }
        let integer = match ty {
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => integer,
            _ => return Ok(false),
        };

        // signed integers represent all bits set as negative one
        if integer.is_signed() {
            return Ok(value == -1);
        }

        // compare the maximum representable unsigned value
        let maximum = integer
            .finite_interval()
            .and_then(|interval| interval.end)
            .and_then(|literal| literal.as_integral());

        Ok(maximum == Some(value))
    }

    /// Return whether one expression denotes an exact numeric constant.
    pub(crate) fn is_numeric_constant(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
        expected: f64,
    ) -> Result<bool, ProviderError> {
        let is_expected = match self.scalar_constant(node)? {
            Some(dir::Literal::Integer(value) | dir::Literal::Bigint(value)) => {
                let value = value as i128;

                value as f64 == expected && expected as i128 == value
            }
            Some(dir::Literal::Float(value)) => value == expected,
            _ => false,
        };

        Ok(is_expected)
    }

    /// Select one builtin or standard-library bigint binary operation.
    pub(crate) fn integral_binary(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<(dir::BinaryOperator, [dir::LocalNodeId<dir::Expression>; 2])>, ProviderError>
    {
        let dir::Expression::Binary {
            left,
            operator,
            right,
        } = self.view().get(expression)
        else {
            return Ok(None);
        };

        // recognize compiler-defined machine-integer operations
        if let Some((operator, operands)) = self.builtin_binary(expression)?
            && operands.iter().all(dir::BuiltinOperand::is_integral)
        {
            let operands = [operands[0].source.local_id, operands[1].source.local_id];

            return Ok(Some((operator, operands)));
        }

        // require canonical bigint operands and their standard-library operation
        let left_domain = self.node_type(left.into_any())?.scalar_domain();
        let right_domain = self.node_type(right.into_any())?.scalar_domain();
        if left_domain != Some(dir::ScalarDomain::Bigint)
            || right_domain != Some(dir::ScalarDomain::Bigint)
        {
            return Ok(None);
        }
        if matches!(
            operator,
            dir::BinaryOperator::UnsignedShiftRight
                | dir::BinaryOperator::EqualStrict
                | dir::BinaryOperator::NotEqualStrict
                | dir::BinaryOperator::And
                | dir::BinaryOperator::Or
                | dir::BinaryOperator::Coalesce
                | dir::BinaryOperator::In
        ) {
            return Ok(None);
        }
        let Some(member) = self.language_member(expression)? else {
            return Ok(None);
        };
        if member.owner != dir::LanguageItem::BigInt {
            return Ok(None);
        }

        Ok(Some((*operator, [*left, *right])))
    }

    /// Select a builtin integral expression offset by exactly one.
    pub(crate) fn integer_step(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<IntegerStep>, ProviderError> {
        let Some((operator, [left, right])) = self.integral_binary(expression)? else {
            return Ok(None);
        };

        // classify exact additive unit constants on either side
        let left_constant = self.integral_constant(left)?;
        let right_constant = self.integral_constant(right)?;
        let step = match (operator, left_constant, right_constant) {
            (dir::BinaryOperator::Add, Some(1), _) => IntegerStep::Increment(right),
            (dir::BinaryOperator::Add, _, Some(1)) => IntegerStep::Increment(left),
            (dir::BinaryOperator::Add, Some(-1), _) => IntegerStep::Decrement(right),
            (dir::BinaryOperator::Add, _, Some(-1)) => IntegerStep::Decrement(left),
            (dir::BinaryOperator::Subtract, _, Some(1)) => IntegerStep::Decrement(left),
            (dir::BinaryOperator::Subtract, _, Some(-1)) => IntegerStep::Increment(left),
            _ => return Ok(None),
        };

        Ok(Some(step))
    }

    /// Return whether one expression is an exact negative-zero constant.
    pub fn is_negative_zero(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        let is_negative_zero = self
            .scalar_constant(node)?
            .is_some_and(|value| value.is_negative_zero());

        Ok(is_negative_zero)
    }

    /// Return whether one expression denotes positive or negative infinity.
    pub fn is_infinite(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        Ok(self.infinity(node)?.is_some())
    }

    /// Return the exact infinite value denoted by one expression.
    pub fn infinity(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<f64>, ProviderError> {
        // recognize exact scalar constants
        if let Some(dir::Literal::Float(value)) = self.scalar_constant(node)?
            && value.is_infinite()
        {
            return Ok(Some(value));
        }

        // recognize the canonical standard-library constants by selected symbol
        let item = self.language_item(node)?;
        let value = match item {
            Some(dir::LanguageItem::Infinity | dir::LanguageItem::NumberPositiveInfinity) => {
                Some(f64::INFINITY)
            }
            Some(dir::LanguageItem::NumberNegativeInfinity) => Some(f64::NEG_INFINITY),
            _ => None,
        };
        if value.is_some() {
            return Ok(value);
        }

        // preserve infinity through compiler-defined unary signs
        let dir::Expression::Unary { right, .. } = self.view().get(node) else {
            return Ok(None);
        };
        let Some((operator, _)) = self.builtin_unary(node)? else {
            return Ok(None);
        };
        let value = match operator {
            dir::UnaryOperator::Plus => self.infinity(*right)?,
            dir::UnaryOperator::Negate => self.infinity(*right)?.map(|value| -value),
            _ => None,
        };

        Ok(value)
    }

    /// Return whether one expression denotes NaN.
    pub fn is_nan(&self, node: dir::LocalNodeId<dir::Expression>) -> Result<bool, ProviderError> {
        // recognize exact scalar constants
        if self
            .scalar_constant(node)?
            .is_some_and(|value| value.is_nan())
        {
            return Ok(true);
        }

        // recognize the canonical standard-library constants by selected symbol
        let item = self.language_item(node)?;
        if matches!(
            item,
            Some(dir::LanguageItem::NaN | dir::LanguageItem::NumberNaN)
        ) {
            return Ok(true);
        }

        // preserve NaN through compiler-defined unary signs
        let dir::Expression::Unary { right, .. } = self.view().get(node) else {
            return Ok(false);
        };
        let Some((operator, _)) = self.builtin_unary(node)? else {
            return Ok(false);
        };
        let is_nan = matches!(
            operator,
            dir::UnaryOperator::Plus | dir::UnaryOperator::Negate
        ) && self.is_nan(*right)?;

        Ok(is_nan)
    }
}
