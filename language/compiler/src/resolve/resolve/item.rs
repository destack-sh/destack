use destack_dir as dir;

use crate::resolve::state::ResolveState;

impl ResolveState<'_> {
    /// Record language items implied by one function signature.
    ///
    /// Example:
    /// ```ds
    /// async function load() {}
    /// function* ids() {}
    /// async function* events() {}
    /// ```
    pub(in crate::resolve) fn record_function_language_items(
        &mut self,
        signature: &dir::FunctionSignature,
    ) {
        match (signature.asynchrony, signature.is_generator) {
            // async function f() {}
            (dir::Asynchrony::Async, false) => {
                self.record_language_item(dir::LanguageItem::Promise);
            }
            // function* f() {}
            (dir::Asynchrony::Sync, true) => {
                self.record_language_item(dir::LanguageItem::Generator);
            }
            // async function* f() {}
            (dir::Asynchrony::Async, true) => {
                self.record_language_item(dir::LanguageItem::AsyncGenerator);
            }
            // function f() {}
            (dir::Asynchrony::Sync, false) => {}
        }
    }

    /// Record language items implied by try propagation syntax.
    ///
    /// Example:
    /// ```ds
    /// const value = parse()?;
    /// ```
    pub(in crate::resolve) fn record_try_language_items(&mut self) {
        self.record_language_item(dir::LanguageItem::Try);
        self.record_language_item(dir::LanguageItem::FromResidual);
    }

    /// Record language item owners that member lookup can project through.
    ///
    /// Check projects scalar and collection receivers onto these declarations.
    /// Resolve records them before component partitioning.
    pub(in crate::resolve) fn record_member_owner_language_items(&mut self) {
        self.record_language_item(dir::LanguageItem::String);
        self.record_language_item(dir::LanguageItem::Number);
        self.record_language_item(dir::LanguageItem::Array);
        self.record_language_item(dir::LanguageItem::Slice);
        self.record_language_item(dir::LanguageItem::FixedArray);
    }

    /// Record the iterable item implied by yield delegation.
    ///
    /// Example:
    /// ```ds
    /// function* ids() {
    ///     yield* values;
    /// }
    /// ```
    pub(in crate::resolve) fn record_yield_star_language_item(&mut self) {
        let Some(function) = self.current_function() else {
            return;
        };
        if !function.is_generator {
            return;
        }
        let item = match function.asynchrony {
            // yield* value
            dir::Asynchrony::Sync => dir::LanguageItem::Iterable,
            // await yield* value
            dir::Asynchrony::Async => dir::LanguageItem::AsyncIterable,
        };

        self.record_language_item(item);
    }

    /// Record the range item implied by one range expression.
    ///
    /// Example:
    /// ```ds
    /// const open = start..end;
    /// const closed = start..=end;
    /// ```
    pub(in crate::resolve) fn record_range_language_item(
        &mut self,
        has_start: bool,
        has_end: bool,
        end_kind: dir::RangeEnd,
    ) {
        let item = match (has_start, has_end, end_kind) {
            // start..end
            (true, true, dir::RangeEnd::Open) => dir::LanguageItem::Range,
            // start..=end
            (true, true, dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeInclusive,
            // start..
            (true, false, _) => dir::LanguageItem::RangeFrom,
            // ..end
            (false, true, dir::RangeEnd::Open) => dir::LanguageItem::RangeTo,
            // ..=end
            (false, true, dir::RangeEnd::Inclusive) => dir::LanguageItem::RangeToInclusive,
            // ..
            (false, false, _) => dir::LanguageItem::RangeFull,
        };

        self.record_language_item(item);
    }

    /// Record language items implied by one unary operator.
    ///
    /// Example:
    /// ```ds
    /// const negated = -value;
    /// const dereferenced = *pointer;
    /// ```
    pub(in crate::resolve) fn record_unary_operator_language_items(
        &mut self,
        operator: dir::UnaryOperator,
    ) {
        let item = match operator {
            // -value
            dir::UnaryOperator::Negate => dir::LanguageItem::Negate,
            // +value
            dir::UnaryOperator::Plus => dir::LanguageItem::Plus,
            // ~value
            dir::UnaryOperator::ElementwiseNot => dir::LanguageItem::Not,
            // *value
            dir::UnaryOperator::Dereference => dir::LanguageItem::Dereference,
            // builtin unary operators
            dir::UnaryOperator::PostIncrement
            | dir::UnaryOperator::PostDecrement
            | dir::UnaryOperator::PreIncrement
            | dir::UnaryOperator::PreDecrement
            | dir::UnaryOperator::Not
            | dir::UnaryOperator::Typeof
            | dir::UnaryOperator::Void
            | dir::UnaryOperator::Spread => return,
        };

        self.record_language_item(item);
    }

    /// Record language items implied by one binary operator.
    ///
    /// Example:
    /// ```ds
    /// const total = left + right;
    /// const is_less = left < right;
    /// ```
    pub(in crate::resolve) fn record_binary_operator_language_items(
        &mut self,
        operator: dir::BinaryOperator,
    ) {
        let item = match operator {
            // left + right
            dir::BinaryOperator::Add => dir::LanguageItem::Add,
            // left - right
            dir::BinaryOperator::Subtract => dir::LanguageItem::Subtract,
            // left * right
            dir::BinaryOperator::Multiply => dir::LanguageItem::Multiply,
            // left / right
            dir::BinaryOperator::Divide => dir::LanguageItem::Divide,
            // left % right
            dir::BinaryOperator::Remainder => dir::LanguageItem::Remainder,
            // left ** right
            dir::BinaryOperator::Exponent => dir::LanguageItem::Power,
            // left << right
            dir::BinaryOperator::ShiftLeft => dir::LanguageItem::ShiftLeft,
            // left >> right
            dir::BinaryOperator::ShiftRight => dir::LanguageItem::ShiftRight,
            // left >>> right
            dir::BinaryOperator::UnsignedShiftRight => dir::LanguageItem::ShiftRightUnsigned,
            // left & right
            dir::BinaryOperator::ElementwiseAnd => dir::LanguageItem::And,
            // left ^ right
            dir::BinaryOperator::ElementwiseXor => dir::LanguageItem::Xor,
            // left | right
            dir::BinaryOperator::ElementwiseOr => dir::LanguageItem::Or,
            // left == right
            dir::BinaryOperator::Equal | dir::BinaryOperator::NotEqual => {
                dir::LanguageItem::PartialEqual
            }
            // ordered comparisons
            dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual => {
                self.record_language_item(dir::LanguageItem::Compare);
                self.record_language_item(dir::LanguageItem::Ordering);
                self.record_language_item(dir::LanguageItem::PartialCompare);

                return;
            }
            // builtin binary operators
            dir::BinaryOperator::EqualStrict
            | dir::BinaryOperator::NotEqualStrict
            | dir::BinaryOperator::And
            | dir::BinaryOperator::Or
            | dir::BinaryOperator::Coalesce
            | dir::BinaryOperator::In => return,
        };

        self.record_language_item(item);
    }
}
