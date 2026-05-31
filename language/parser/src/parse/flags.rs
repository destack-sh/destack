/// Internal parser context flags.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ParserFlags {
    /// Packed parser context and behavior flags.
    flags: u32,
}

impl Default for ParserFlags {
    fn default() -> Self {
        Self {
            flags: Self::ALLOW_SEQUENCE_EXPRESSION_FLAG,
        }
    }
}

impl ParserFlags {
    const EXPRESSION_FLAG_MASK: u32 = Self::IN_PARENTHESIS_FLAG
        | Self::IN_STATEMENT_POSITION_FLAG
        | Self::IN_TERNARY_CONDITION_FLAG
        | Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG
        | Self::DISALLOW_TYPE_CONDITIONAL_FLAG
        | Self::IN_ARROW_RETURN_TYPE_FLAG
        | Self::ALLOW_SEQUENCE_EXPRESSION_FLAG
        | Self::IN_MATCH_CASE_BODY_FLAG;
    const AMBIENT_FLAG_MASK: u32 = !Self::EXPRESSION_FLAG_MASK;

    const IN_STATIC_FLAG: u32 = 1 << 0;
    const IN_COMPTIME_FLAG: u32 = 1 << 1;
    const IN_TYPE_FLAG: u32 = 1 << 2;
    const IN_SUPER_TYPE_FLAG: u32 = 1 << 3;
    const IN_VARIANT_FLAG: u32 = 1 << 4;
    const IN_BEFORE_TYPE_FLAG: u32 = 1 << 5;
    const IN_MATCH_CASE_FLAG: u32 = 1 << 6;
    const IN_UNION_PATTERN_FLAG: u32 = 1 << 7;
    const IN_DECLARE_CONTEXT_FLAG: u32 = 1 << 8;
    const IN_PARENTHESIS_FLAG: u32 = 1 << 9;
    const IN_STATEMENT_POSITION_FLAG: u32 = 1 << 10;
    const IN_STATEMENT_CONTEXT_FLAG: u32 = 1 << 11;
    const IN_BEFORE_BLOCK_FLAG: u32 = 1 << 12;
    const IN_TREE_LITERAL_FLAG: u32 = 1 << 13;
    const IN_DECORATOR_FLAG: u32 = 1 << 14;
    const IN_TERNARY_CONDITION_FLAG: u32 = 1 << 15;
    const IN_TYPE_CONDITIONAL_RIGHT_FLAG: u32 = 1 << 16;
    const IN_ARROW_RETURN_TYPE_FLAG: u32 = 1 << 17;
    const IN_TYPE_MAPPED_CONSTRAINT_FLAG: u32 = 1 << 19;
    const IN_FOR_EACH_FLAG: u32 = 1 << 20;
    const IN_NEW_RECEIVER_FLAG: u32 = 1 << 21;
    const IN_TYPEOF_QUERY_FLAG: u32 = 1 << 22;
    const IN_GENERATOR_FLAG: u32 = 1 << 23;
    const FORBID_YIELD_FLAG: u32 = 1 << 24;
    const FORBID_AWAIT_FLAG: u32 = 1 << 25;
    const ALLOW_SEQUENCE_EXPRESSION_FLAG: u32 = 1 << 26;
    const ALLOW_PRIVATE_HASH_KEY_FLAG: u32 = 1 << 27;
    const DISALLOW_AMBIGUOUS_TREE_LITERAL_FLAG: u32 = 1 << 28;
    const DISALLOW_TYPE_CONDITIONAL_FLAG: u32 = 1 << 29;
    const IN_MATCH_CASE_BODY_FLAG: u32 = 1 << 30;
    const IN_DECORATOR_HEAD_FLAG: u32 = 1 << 31;

    #[inline]
    const fn has_flag(self, flag: u32) -> bool {
        (self.flags & flag) != 0
    }

    #[inline]
    fn with_flag(mut self, flag: u32, enabled: bool) -> Self {
        if enabled {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
        self
    }

    #[inline]
    fn set_flag(&mut self, flag: u32, enabled: bool) {
        if enabled {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    #[inline]
    pub(crate) const fn is_in_static(self) -> bool {
        self.has_flag(Self::IN_STATIC_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_comptime(self) -> bool {
        self.has_flag(Self::IN_COMPTIME_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_type(self) -> bool {
        self.has_flag(Self::IN_TYPE_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_super_type(self) -> bool {
        self.has_flag(Self::IN_SUPER_TYPE_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_variant(self) -> bool {
        self.has_flag(Self::IN_VARIANT_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_before_type(self) -> bool {
        self.has_flag(Self::IN_BEFORE_TYPE_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_match_case(self) -> bool {
        self.has_flag(Self::IN_MATCH_CASE_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_match_case_body(self) -> bool {
        self.has_flag(Self::IN_MATCH_CASE_BODY_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_union_pattern(self) -> bool {
        self.has_flag(Self::IN_UNION_PATTERN_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_declare_context(self) -> bool {
        self.has_flag(Self::IN_DECLARE_CONTEXT_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_statement_position(self) -> bool {
        self.has_flag(Self::IN_STATEMENT_POSITION_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_statement_context(self) -> bool {
        self.has_flag(Self::IN_STATEMENT_CONTEXT_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_before_block(self) -> bool {
        self.has_flag(Self::IN_BEFORE_BLOCK_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_tree_literal(self) -> bool {
        self.has_flag(Self::IN_TREE_LITERAL_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_decorator(self) -> bool {
        self.has_flag(Self::IN_DECORATOR_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_decorator_head(self) -> bool {
        self.has_flag(Self::IN_DECORATOR_HEAD_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_ternary_condition(self) -> bool {
        self.has_flag(Self::IN_TERNARY_CONDITION_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_type_conditional_right(self) -> bool {
        self.has_flag(Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG)
    }

    #[inline]
    pub(crate) const fn is_disallow_type_conditional(self) -> bool {
        self.has_flag(Self::DISALLOW_TYPE_CONDITIONAL_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_arrow_return_type(self) -> bool {
        self.has_flag(Self::IN_ARROW_RETURN_TYPE_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_for_each(self) -> bool {
        self.has_flag(Self::IN_FOR_EACH_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_new_receiver(self) -> bool {
        self.has_flag(Self::IN_NEW_RECEIVER_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_typeof_query(self) -> bool {
        self.has_flag(Self::IN_TYPEOF_QUERY_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_generator(self) -> bool {
        self.has_flag(Self::IN_GENERATOR_FLAG)
    }

    #[inline]
    pub(crate) const fn is_forbid_yield(self) -> bool {
        self.has_flag(Self::FORBID_YIELD_FLAG)
    }

    #[inline]
    pub(crate) const fn is_forbid_await(self) -> bool {
        self.has_flag(Self::FORBID_AWAIT_FLAG)
    }

    #[inline]
    pub(crate) const fn allows_sequence_expression(self) -> bool {
        self.has_flag(Self::ALLOW_SEQUENCE_EXPRESSION_FLAG)
    }

    #[inline]
    pub(crate) const fn allows_private_hash_key(self) -> bool {
        self.has_flag(Self::ALLOW_PRIVATE_HASH_KEY_FLAG)
    }

    #[inline]
    pub(crate) const fn is_disallow_ambiguous_tree_literal(self) -> bool {
        self.has_flag(Self::DISALLOW_AMBIGUOUS_TREE_LITERAL_FLAG)
    }

    /// Replace the hot expression-local portion of these flags.
    #[inline]
    pub(crate) fn with_expression_context(mut self, context: ParserFlags) -> Self {
        self.flags = (self.flags & !Self::EXPRESSION_FLAG_MASK)
            | (context.flags & Self::EXPRESSION_FLAG_MASK);
        if context.is_in_statement_position() {
            self.set_in_statement_context(true);
        }
        self
    }

    /// Replace the ambient parser portion of these flags.
    #[inline]
    pub(crate) fn with_ambient_context(mut self, context: ParserFlags) -> Self {
        self.flags =
            (self.flags & !Self::AMBIENT_FLAG_MASK) | (context.flags & Self::AMBIENT_FLAG_MASK);
        self
    }

    #[inline]
    pub(crate) fn set_in_static(&mut self, enabled: bool) {
        self.set_flag(Self::IN_STATIC_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_comptime(&mut self, enabled: bool) {
        self.set_flag(Self::IN_COMPTIME_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_type(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TYPE_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_super_type(&mut self, enabled: bool) {
        self.set_flag(Self::IN_SUPER_TYPE_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_before_type(&mut self, enabled: bool) {
        self.set_flag(Self::IN_BEFORE_TYPE_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_declare_context(&mut self, enabled: bool) {
        self.set_flag(Self::IN_DECLARE_CONTEXT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_statement_context(&mut self, enabled: bool) {
        self.set_flag(Self::IN_STATEMENT_CONTEXT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_decorator(&mut self, enabled: bool) {
        self.set_flag(Self::IN_DECORATOR_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_decorator_head(&mut self, enabled: bool) {
        self.set_flag(Self::IN_DECORATOR_HEAD_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_type_conditional_right(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_type_mapped_constraint(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TYPE_MAPPED_CONSTRAINT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_generator(&mut self, enabled: bool) {
        self.set_flag(Self::IN_GENERATOR_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_forbid_yield(&mut self, enabled: bool) {
        self.set_flag(Self::FORBID_YIELD_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_forbid_await(&mut self, enabled: bool) {
        self.set_flag(Self::FORBID_AWAIT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_allow_sequence_expression(&mut self, enabled: bool) {
        self.set_flag(Self::ALLOW_SEQUENCE_EXPRESSION_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_disallow_ambiguous_tree_literal(&mut self, enabled: bool) {
        self.set_flag(Self::DISALLOW_AMBIGUOUS_TREE_LITERAL_FLAG, enabled);
    }

    /// Set `in_static` to the given value.
    #[inline]
    pub(crate) fn with_static(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_STATIC_FLAG, enabled)
    }

    /// Set `in_comptime` to the given value.
    #[inline]
    pub(crate) fn with_comptime(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_COMPTIME_FLAG, enabled)
    }

    /// Set `in_type` to the given value.
    #[inline]
    pub(crate) fn with_type(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_TYPE_FLAG, enabled)
    }

    /// Set `in_variant` to the given value.
    #[inline]
    pub(crate) fn with_variant(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_VARIANT_FLAG, enabled)
    }

    /// Set `in_before_type` to the given value.
    #[inline]
    pub(crate) fn with_before_type(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_BEFORE_TYPE_FLAG, enabled)
    }

    /// Set `in_match_case` to the given value.
    #[inline]
    pub(crate) fn with_match_case(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_MATCH_CASE_FLAG, enabled)
    }

    /// Set `in_statement_position` to the given value.
    #[inline]
    pub(crate) fn with_statement_position(self, enabled: bool) -> Self {
        let flags = self.with_flag(Self::IN_STATEMENT_POSITION_FLAG, enabled);
        if enabled {
            flags.with_flag(Self::IN_STATEMENT_CONTEXT_FLAG, true)
        } else {
            flags
        }
    }

    /// Set `in_statement_context` to the given value.
    #[inline]
    pub(crate) fn with_statement_context(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_STATEMENT_CONTEXT_FLAG, enabled)
    }

    /// Set `in_before_block` to the given value.
    #[inline]
    pub(crate) fn with_before_block(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_BEFORE_BLOCK_FLAG, enabled)
    }

    /// Set `in_tree_literal` to the given value.
    #[inline]
    pub(crate) fn with_tree_literal(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_TREE_LITERAL_FLAG, enabled)
    }

    /// Set `in_decorator` to the given value.
    #[inline]
    pub(crate) fn with_decorator(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_DECORATOR_FLAG, enabled)
    }

    /// Set `forbid_await` to the given value.
    #[inline]
    pub(crate) fn with_forbid_await(self, enabled: bool) -> Self {
        self.with_flag(Self::FORBID_AWAIT_FLAG, enabled)
    }

    /// Set `allow_sequence_expression` to the given value.
    #[inline]
    pub(crate) fn with_sequence_expression(self, enabled: bool) -> Self {
        self.with_flag(Self::ALLOW_SEQUENCE_EXPRESSION_FLAG, enabled)
    }

    /// Set `allow_private_hash_key` to the given value.
    #[inline]
    pub(crate) fn with_allow_private_hash_key(self, enabled: bool) -> Self {
        self.with_flag(Self::ALLOW_PRIVATE_HASH_KEY_FLAG, enabled)
    }

    /// Set `in_static=true`.
    #[inline]
    pub(crate) fn in_static(self) -> Self {
        self.with_flag(Self::IN_STATIC_FLAG, true)
    }

    /// Set `in_type=true`.
    #[inline]
    pub(crate) fn in_type(self) -> Self {
        self.with_flag(Self::IN_TYPE_FLAG, true)
    }

    /// Set `in_type=false`.
    #[inline]
    pub(crate) fn not_in_type(self) -> Self {
        self.with_flag(Self::IN_TYPE_FLAG, false)
    }

    /// Set `in_super_type=true`.
    #[inline]
    pub(crate) fn in_super_type(self) -> Self {
        self.with_flag(Self::IN_TYPE_FLAG, true)
            .with_flag(Self::IN_SUPER_TYPE_FLAG, true)
    }

    /// Set `in_generator` to the given value.
    #[inline]
    pub(crate) fn with_generator(self, in_generator: bool) -> Self {
        self.with_flag(Self::IN_GENERATOR_FLAG, in_generator)
    }

    /// Set `forbid_yield` to the given value.
    #[inline]
    pub(crate) fn with_forbid_yield(self, forbid_yield: bool) -> Self {
        self.with_flag(Self::FORBID_YIELD_FLAG, forbid_yield)
    }

    /// Set `in_variant=true`.
    #[inline]
    pub(crate) fn in_variant(self) -> Self {
        self.with_flag(Self::IN_VARIANT_FLAG, true)
    }

    /// Set `in_before_type=true`.
    #[inline]
    pub(crate) fn in_before_type(self) -> Self {
        self.with_flag(Self::IN_BEFORE_TYPE_FLAG, true)
    }

    /// Set `in_match_case_body=true`.
    #[inline]
    pub(crate) fn in_match_case_body(self) -> Self {
        self.with_flag(Self::IN_MATCH_CASE_BODY_FLAG, true)
    }

    /// Set `in_union_pattern=true`.
    #[inline]
    pub(crate) fn in_union_pattern(self) -> Self {
        self.with_flag(Self::IN_UNION_PATTERN_FLAG, true)
    }

    /// Set `in_statement_position=true`.
    #[inline]
    pub(crate) fn in_statement_position(self) -> Self {
        self.with_flag(Self::IN_STATEMENT_POSITION_FLAG, true)
            .with_flag(Self::IN_STATEMENT_CONTEXT_FLAG, true)
    }

    /// Set `in_before_block=true`.
    #[inline]
    pub(crate) fn in_before_block(self) -> Self {
        self.with_flag(Self::IN_BEFORE_BLOCK_FLAG, true)
    }

    /// Set `in_tree_literal=false`.
    #[inline]
    pub(crate) fn not_in_tree_literal(self) -> Self {
        self.with_flag(Self::IN_TREE_LITERAL_FLAG, false)
    }

    /// Set `in_before_block=false`.
    #[inline]
    pub(crate) fn not_in_before_block(self) -> Self {
        self.with_flag(Self::IN_BEFORE_BLOCK_FLAG, false)
    }

    /// Set `in_ternary_condition=true`.
    #[inline]
    pub(crate) fn in_ternary_condition(self) -> Self {
        self.with_flag(Self::IN_TERNARY_CONDITION_FLAG, true)
    }

    /// Set `in_ternary_condition=false`.
    #[inline]
    pub(crate) fn not_in_ternary_condition(self) -> Self {
        self.with_flag(Self::IN_TERNARY_CONDITION_FLAG, false)
    }

    /// Set `in_type_conditional_right=true`.
    #[inline]
    pub(crate) fn in_type_conditional_right(self) -> Self {
        self.with_flag(Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG, true)
    }

    /// Set `disallow_type_conditional=true`.
    #[inline]
    pub(crate) fn disallow_type_conditional(self) -> Self {
        self.with_flag(Self::DISALLOW_TYPE_CONDITIONAL_FLAG, true)
    }

    /// Set `in_arrow_return_type=true`.
    #[inline]
    pub(crate) fn in_arrow_return_type(self) -> Self {
        self.with_flag(Self::IN_ARROW_RETURN_TYPE_FLAG, true)
    }

    /// Set `in_type_mapped_constraint=true`.
    #[inline]
    pub(crate) fn in_type_mapped_constraint(self) -> Self {
        self.with_flag(Self::IN_TYPE_MAPPED_CONSTRAINT_FLAG, true)
    }

    /// Set `in_for_each=true`.
    #[inline]
    pub(crate) fn in_for_each(self) -> Self {
        self.with_flag(Self::IN_FOR_EACH_FLAG, true)
    }

    /// Set `in_new_receiver=true`.
    #[inline]
    pub(crate) fn in_new_receiver(self) -> Self {
        self.with_flag(Self::IN_NEW_RECEIVER_FLAG, true)
    }

    /// Set `in_new_receiver=false`.
    #[inline]
    pub(crate) fn not_in_new_receiver(self) -> Self {
        self.with_flag(Self::IN_NEW_RECEIVER_FLAG, false)
    }

    /// Set `in_typeof_query=true`.
    #[inline]
    pub(crate) fn in_typeof_query(self) -> Self {
        self.with_flag(Self::IN_TYPEOF_QUERY_FLAG, true)
    }

    /// Set `in_decorator=true`.
    #[inline]
    pub(crate) fn in_decorator(self) -> Self {
        self.with_flag(Self::IN_DECORATOR_FLAG, true)
    }

    /// Set `in_decorator_head=true`.
    #[inline]
    pub(crate) fn in_decorator_head(self) -> Self {
        self.with_flag(Self::IN_DECORATOR_HEAD_FLAG, true)
    }

    /// Set `in_decorator=false`.
    #[inline]
    pub(crate) fn not_in_decorator(self) -> Self {
        self.with_flag(Self::IN_DECORATOR_FLAG, false)
    }

    /// Disallow sequence expressions (comma operator).
    #[inline]
    pub(crate) fn not_in_sequence_expression(self) -> Self {
        self.with_flag(Self::ALLOW_SEQUENCE_EXPRESSION_FLAG, false)
    }

    /// Disallow arrow return type shielding for nested expressions.
    #[inline]
    pub(crate) fn not_in_arrow_return_type(self) -> Self {
        self.with_flag(Self::IN_ARROW_RETURN_TYPE_FLAG, false)
    }

    /// Not previous position.
    #[inline]
    pub(crate) fn not_in_position(self) -> Self {
        self.with_flag(Self::IN_PARENTHESIS_FLAG, false)
            .with_flag(Self::IN_STATEMENT_POSITION_FLAG, false)
            .with_flag(Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG, false)
            .with_flag(Self::DISALLOW_TYPE_CONDITIONAL_FLAG, false)
    }

    /// Reset position-related flags but preserve context flags like `in_generator`.
    pub(crate) fn nested(self) -> Self {
        let mut flags = Self::default();
        flags.set_in_generator(self.is_in_generator());
        flags.set_in_comptime(self.is_in_comptime());
        flags.set_forbid_yield(self.is_forbid_yield());
        flags.set_forbid_await(self.is_forbid_await());
        flags.set_allow_sequence_expression(self.allows_sequence_expression());
        flags.set_in_decorator(self.is_in_decorator());
        flags.set_in_decorator_head(false);
        flags.set_disallow_ambiguous_tree_literal(self.is_disallow_ambiguous_tree_literal());
        flags.set_in_declare_context(self.is_in_declare_context());
        flags.set_in_statement_context(self.is_in_statement_context());
        flags
    }
}
