use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

/// One builtin integral expression offset by exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IntegerStep {
    /// One value incremented by one.
    Increment(dir::LocalNodeId<dir::Expression>),
    /// One value decremented by one.
    Decrement(dir::LocalNodeId<dir::Expression>),
}

impl DirModule<'_> {
    /// Return the concrete primitive type selected for one checked node.
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

    /// Return the exact scalar constant selected by one checked expression.
    pub fn scalar_constant(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<dir::ScalarLiteral>, ProviderError> {
        // select one symbol-backed constant expression
        if let Some(symbol) = self.selected_symbol(node)?
            && let Some(value) = self.dir.symbol_static(symbol)?
        {
            let value = match value {
                dir::StaticTerm::ScalarLiteral { value } => Some(value),
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

    /// Return the exact integral constant selected by one checked expression.
    pub fn integral_constant(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<i64>, ProviderError> {
        let value = self
            .scalar_constant(node)?
            .and_then(|value| value.as_integral());

        Ok(value)
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

    /// Return whether one checked expression denotes positive or negative infinity.
    pub fn is_infinite(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<bool, ProviderError> {
        Ok(self.infinity(node)?.is_some())
    }

    /// Return the exact infinite value denoted by one checked expression.
    pub fn infinity(
        &self,
        node: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<f64>, ProviderError> {
        // recognize exact checked scalar constants
        if let Some(dir::ScalarLiteral::Float(value)) = self.scalar_constant(node)?
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

    /// Return whether one checked expression denotes NaN.
    pub fn is_nan(&self, node: dir::LocalNodeId<dir::Expression>) -> Result<bool, ProviderError> {
        // recognize exact checked scalar constants
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
