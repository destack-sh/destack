use tspp_dir as dir;

use crate::resolve::state::ResolveState;

impl ResolveState<'_> {
    /// Use language items implied by one function signature.
    ///
    /// Example:
    /// ```tspp
    /// async function load() {}
    /// function* ids() {}
    /// async function* events() {}
    /// ```
    pub(in crate::resolve) fn use_function_language_items(
        &mut self,
        signature: &dir::FunctionSignature,
    ) {
        match (signature.asynchrony, signature.is_generator) {
            // async function f() {}
            (dir::Asynchrony::Async, false) => {
                self.use_language_item(dir::LanguageItem::Promise);
            }
            // function* f() {}
            (dir::Asynchrony::Sync, true) => {
                self.use_language_item(dir::LanguageItem::Generator);
            }
            // async function* f() {}
            (dir::Asynchrony::Async, true) => {
                self.use_language_item(dir::LanguageItem::AsyncGenerator);
            }
            // function f() {}
            (dir::Asynchrony::Sync, false) => {}
        }
    }

    /// Use the language items `?` propagation implies.
    ///
    /// Example:
    /// ```tspp
    /// const value = parse()?;
    /// ```
    pub(in crate::resolve) fn use_try_language_items(&mut self) {
        self.use_language_item(dir::LanguageItem::Try);
        self.use_language_item(dir::LanguageItem::FromResidual);
    }

    /// Use language items needed by apparent member lookup.
    pub(in crate::resolve) fn use_apparent_member_language_items(&mut self) {
        for item in dir::Type::member_owner_items() {
            self.use_language_item(item);
        }
    }

    /// Use language items needed by sequence patterns.
    ///
    /// Example:
    /// ```tspp
    /// const [first, ...rest] = values;
    /// ```
    pub(in crate::resolve) fn use_sequence_pattern_language_items(&mut self) {
        self.use_language_item(dir::LanguageItem::Sequence);
    }

    /// Use the iterable item implied by yield delegation.
    ///
    /// Example:
    /// ```tspp
    /// function* ids() {
    ///     yield* values;
    /// }
    /// ```
    pub(in crate::resolve) fn use_yield_star_language_item(&mut self) {
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

        self.use_language_item(item);
    }

    /// Use the range item implied by one range expression.
    ///
    /// Example:
    /// ```tspp
    /// const open = start..end;
    /// const closed = start..=end;
    /// ```
    pub(in crate::resolve) fn use_range_language_item(
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

        self.use_language_item(item);
    }

    /// Use language items implied by one unary operator.
    ///
    /// Example:
    /// ```tspp
    /// const negated = -value;
    /// const dereferenced = *pointer;
    /// ```
    pub(in crate::resolve) fn use_unary_operator_language_items(
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
            | dir::UnaryOperator::Spread => return,
        };

        self.use_language_item(item);
    }

    /// Use language items implied by one binary operator.
    ///
    /// Example:
    /// ```tspp
    /// const total = left + right;
    /// const is_less = left < right;
    /// ```
    pub(in crate::resolve) fn use_binary_operator_language_items(
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
            // builtin strict equality capability
            dir::BinaryOperator::EqualStrict | dir::BinaryOperator::NotEqualStrict => {
                dir::LanguageItem::StrictEqual
            }
            // ordered comparisons
            dir::BinaryOperator::LessThan
            | dir::BinaryOperator::LessThanOrEqual
            | dir::BinaryOperator::GreaterThan
            | dir::BinaryOperator::GreaterThanOrEqual => {
                self.use_language_item(dir::LanguageItem::Compare);
                self.use_language_item(dir::LanguageItem::Ordering);
                self.use_language_item(dir::LanguageItem::PartialCompare);

                return;
            }
            // property membership may inspect any apparent built-in owner
            dir::BinaryOperator::In => {
                self.use_apparent_member_language_items();

                return;
            }
            // builtin binary operators
            dir::BinaryOperator::And | dir::BinaryOperator::Or | dir::BinaryOperator::Coalesce => {
                return;
            }
        };

        self.use_language_item(item);
    }
}
