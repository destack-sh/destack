use crate::{Lexer, LexerSnapshot, is_semantic, keyword_from_identifier};
use core::fmt;
use destack_core::StringPool;
use destack_dir::{
    BlockForm, Expression, Keyword, LocalNodeId, Node, NodeType, StringId, Token, TokenLiteral,
    TokenSpan, TokenType, Tree, TreeMark, TreeStore, TypeExpression,
};
use destack_source::{
    DiagnosticCollector, EnclosingSpan, File, FileId, LanguageType, ModuleId, MultiSpan,
    NodeSearchMode, NodeSpanBoundary, NodeSpanType, PackageId, Span,
};
use std::fmt::Debug;
use std::sync::Arc;

use crate::{ParseError, ParseResult};

use super::state::ParserState;

const ESTIMATED_TOKEN_BYTES: usize = 6;

/// Cached string ids for type literal identifiers.
#[allow(dead_code)]
#[derive(Debug)]
pub(crate) struct TypeLiteralIdentifiers {
    pub(crate) undefined: StringId,
    pub(crate) unknown: StringId,
    pub(crate) object: StringId,
    pub(crate) null_: StringId,
    pub(crate) any: StringId,
    pub(crate) never: StringId,
    pub(crate) boolean: StringId,
    pub(crate) void: StringId,
    pub(crate) char_: StringId,
    pub(crate) string: StringId,
    pub(crate) bigint: StringId,
    pub(crate) number: StringId,
    pub(crate) int: StringId,
    pub(crate) isize: StringId,
    pub(crate) uint: StringId,
    pub(crate) usize: StringId,
    pub(crate) float: StringId,
    pub(crate) symbol: StringId,
    pub(crate) unique: StringId,
}

impl TypeLiteralIdentifiers {
    /// Create cached ids for the current string pool.
    fn new(strings: &StringPool) -> Self {
        Self {
            undefined: strings.intern("undefined"),
            unknown: strings.intern("unknown"),
            object: strings.intern("object"),
            null_: strings.intern("null"),
            any: strings.intern("any"),
            never: strings.intern("never"),
            boolean: strings.intern("boolean"),
            void: strings.intern("void"),
            char_: strings.intern("char"),
            string: strings.intern("string"),
            bigint: strings.intern("bigint"),
            number: strings.intern("number"),
            int: strings.intern("int"),
            isize: strings.intern("isize"),
            uint: strings.intern("uint"),
            usize: strings.intern("usize"),
            float: strings.intern("float"),
            symbol: strings.intern("symbol"),
            unique: strings.intern("unique"),
        }
    }
}

/// Internal parser context flags.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ParserFlags {
    /// Packed parser context and behavior flags.
    flags: u32,
    /// The left precedence preceding (i.e. before) the expression.
    /// Determines expression operator lifting and grouping.
    pub(crate) left_precedence: Option<u16>,
}

impl Default for ParserFlags {
    fn default() -> Self {
        Self {
            flags: Self::ALLOW_SEQUENCE_EXPRESSION_FLAG,
            left_precedence: None,
        }
    }
}

/// Parser options that can be configured externally.
#[derive(Debug, Copy, Clone)]
pub struct ParserOptions {
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool,
    /// Whether token side tokens should be retained for formatter and comment output.
    pub retain_trivia_tokens: bool,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    pub preserve_parenthesized_wrappers: bool,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            disallow_ambiguous_tree_literal: false,
            retain_trivia_tokens: true,
            preserve_parenthesized_wrappers: true,
        }
    }
}

#[allow(unused)]
impl ParserFlags {
    const EXPRESSION_FLAG_MASK: u32 = Self::IN_PARENTHESIS_FLAG
        | Self::IN_STATEMENT_POSITION_FLAG
        | Self::IN_TERNARY_CONDITION_FLAG
        | Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG
        | Self::DISALLOW_TYPE_CONDITIONAL_FLAG
        | Self::IN_ARROW_RETURN_TYPE_FLAG
        | Self::ALLOW_TYPE_PREDICATE_FLAG
        | Self::ALLOW_SEQUENCE_EXPRESSION_FLAG;
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
    const ALLOW_TYPE_PREDICATE_FLAG: u32 = 1 << 18;
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
    const IN_MODULE_DIRECTIVE_FLAG: u32 = 1 << 30;

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
    pub(crate) const fn is_in_union_pattern(self) -> bool {
        self.has_flag(Self::IN_UNION_PATTERN_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_declare_context(self) -> bool {
        self.has_flag(Self::IN_DECLARE_CONTEXT_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_parenthesis(self) -> bool {
        self.has_flag(Self::IN_PARENTHESIS_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_statement_position(self) -> bool {
        self.has_flag(Self::IN_STATEMENT_POSITION_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_statement_context(self) -> bool {
        self.has_flag(Self::IN_STATEMENT_CONTEXT_FLAG)
    }

    /// Return whether the parser is inside a module directive body.
    #[inline]
    pub(crate) const fn is_in_module_directive(self) -> bool {
        self.has_flag(Self::IN_MODULE_DIRECTIVE_FLAG)
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
    pub(crate) const fn allows_type_predicate(self) -> bool {
        self.has_flag(Self::ALLOW_TYPE_PREDICATE_FLAG)
    }

    #[inline]
    pub(crate) const fn is_in_type_mapped_constraint(self) -> bool {
        self.has_flag(Self::IN_TYPE_MAPPED_CONSTRAINT_FLAG)
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
        self.left_precedence = context.left_precedence;
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
    pub(crate) fn set_in_variant(&mut self, enabled: bool) {
        self.set_flag(Self::IN_VARIANT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_before_type(&mut self, enabled: bool) {
        self.set_flag(Self::IN_BEFORE_TYPE_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_match_case(&mut self, enabled: bool) {
        self.set_flag(Self::IN_MATCH_CASE_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_union_pattern(&mut self, enabled: bool) {
        self.set_flag(Self::IN_UNION_PATTERN_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_declare_context(&mut self, enabled: bool) {
        self.set_flag(Self::IN_DECLARE_CONTEXT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_parenthesis(&mut self, enabled: bool) {
        self.set_flag(Self::IN_PARENTHESIS_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_statement_position(&mut self, enabled: bool) {
        self.set_flag(Self::IN_STATEMENT_POSITION_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_statement_context(&mut self, enabled: bool) {
        self.set_flag(Self::IN_STATEMENT_CONTEXT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_before_block(&mut self, enabled: bool) {
        self.set_flag(Self::IN_BEFORE_BLOCK_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_tree_literal(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TREE_LITERAL_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_decorator(&mut self, enabled: bool) {
        self.set_flag(Self::IN_DECORATOR_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_ternary_condition(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TERNARY_CONDITION_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_type_conditional_right(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_disallow_type_conditional(&mut self, enabled: bool) {
        self.set_flag(Self::DISALLOW_TYPE_CONDITIONAL_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_arrow_return_type(&mut self, enabled: bool) {
        self.set_flag(Self::IN_ARROW_RETURN_TYPE_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_type_mapped_constraint(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TYPE_MAPPED_CONSTRAINT_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_for_each(&mut self, enabled: bool) {
        self.set_flag(Self::IN_FOR_EACH_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_new_receiver(&mut self, enabled: bool) {
        self.set_flag(Self::IN_NEW_RECEIVER_FLAG, enabled);
    }

    #[inline]
    pub(crate) fn set_in_typeof_query(&mut self, enabled: bool) {
        self.set_flag(Self::IN_TYPEOF_QUERY_FLAG, enabled);
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
    pub(crate) fn set_allow_private_hash_key(&mut self, enabled: bool) {
        self.set_flag(Self::ALLOW_PRIVATE_HASH_KEY_FLAG, enabled);
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

    /// Set `in_super_type` to the given value.
    #[inline]
    pub(crate) fn with_super_type(self, enabled: bool) -> Self {
        let flags = self.with_flag(Self::IN_SUPER_TYPE_FLAG, enabled);
        if enabled {
            flags.with_type(true)
        } else {
            flags
        }
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

    /// Set `in_union_pattern` to the given value.
    #[inline]
    pub(crate) fn with_union_pattern(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_UNION_PATTERN_FLAG, enabled)
    }

    /// Set `in_declare_context` to the given value.
    #[inline]
    pub(crate) fn with_declare_context(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_DECLARE_CONTEXT_FLAG, enabled)
    }

    /// Set `in_parenthesis` to the given value.
    #[inline]
    pub(crate) fn with_parenthesis(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_PARENTHESIS_FLAG, enabled)
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

    /// Set `in_module_directive` to the given value.
    #[inline]
    pub(crate) fn with_module_directive(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_MODULE_DIRECTIVE_FLAG, enabled)
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

    /// Set `in_ternary_condition` to the given value.
    #[inline]
    pub(crate) fn with_ternary_condition(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_TERNARY_CONDITION_FLAG, enabled)
    }

    /// Set `in_type_conditional_right` to the given value.
    #[inline]
    pub(crate) fn with_type_conditional_right(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG, enabled)
    }

    /// Set `in_type_conditional_right=false`.
    #[inline]
    pub(crate) fn not_in_type_conditional_right(self) -> Self {
        self.with_type_conditional_right(false)
    }

    /// Set `disallow_type_conditional` to the given value.
    #[inline]
    pub(crate) fn with_disallow_type_conditional(self, enabled: bool) -> Self {
        self.with_flag(Self::DISALLOW_TYPE_CONDITIONAL_FLAG, enabled)
    }

    /// Set `in_arrow_return_type` to the given value.
    #[inline]
    pub(crate) fn with_arrow_return_type(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_ARROW_RETURN_TYPE_FLAG, enabled)
    }

    /// Set `in_type_mapped_constraint` to the given value.
    #[inline]
    pub(crate) fn with_type_mapped_constraint(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_TYPE_MAPPED_CONSTRAINT_FLAG, enabled)
    }

    /// Set `in_for_each` to the given value.
    #[inline]
    pub(crate) fn with_for_each(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_FOR_EACH_FLAG, enabled)
    }

    /// Set `in_new_receiver` to the given value.
    #[inline]
    pub(crate) fn with_new_receiver(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_NEW_RECEIVER_FLAG, enabled)
    }

    /// Set `in_typeof_query` to the given value.
    #[inline]
    pub(crate) fn with_typeof_query(self, enabled: bool) -> Self {
        self.with_flag(Self::IN_TYPEOF_QUERY_FLAG, enabled)
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

    /// Set `in_comptime=true`.
    #[inline]
    pub(crate) fn in_comptime(self) -> Self {
        self.with_flag(Self::IN_COMPTIME_FLAG, true)
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

    /// Set `forbid_await=true`.
    #[inline]
    pub(crate) fn forbid_await(self) -> Self {
        self.with_flag(Self::FORBID_AWAIT_FLAG, true)
    }

    /// Set `in_variant=true`.
    #[inline]
    pub(crate) fn in_variant(self) -> Self {
        self.with_flag(Self::IN_VARIANT_FLAG, true)
    }

    /// Set `in_variant=false`.
    #[inline]
    pub(crate) fn not_in_variant(self) -> Self {
        self.with_flag(Self::IN_VARIANT_FLAG, false)
    }

    /// Set `in_before_type=true`.
    #[inline]
    pub(crate) fn in_before_type(self) -> Self {
        self.with_flag(Self::IN_BEFORE_TYPE_FLAG, true)
    }

    /// Set `in_match_case=true`.
    #[inline]
    pub(crate) fn in_match_case(self) -> Self {
        self.with_flag(Self::IN_MATCH_CASE_FLAG, true)
    }

    /// Set `in_union_pattern=true`.
    #[inline]
    pub(crate) fn in_union_pattern(self) -> Self {
        self.with_flag(Self::IN_UNION_PATTERN_FLAG, true)
    }

    /// Set `in_declare_context=true`.
    #[inline]
    pub(crate) fn in_declare_context(self) -> Self {
        self.with_flag(Self::IN_DECLARE_CONTEXT_FLAG, true)
    }

    /// Set `in_parenthesis=true`.
    #[inline]
    pub(crate) fn in_parenthesis(self) -> Self {
        self.with_flag(Self::IN_PARENTHESIS_FLAG, true)
    }

    /// Set `in_parenthesis=false`.
    #[inline]
    pub(crate) fn not_in_parenthesis(self) -> Self {
        self.with_flag(Self::IN_PARENTHESIS_FLAG, false)
    }

    /// Set `in_statement_position=true`.
    #[inline]
    pub(crate) fn in_statement_position(self) -> Self {
        self.with_flag(Self::IN_STATEMENT_POSITION_FLAG, true)
            .with_flag(Self::IN_STATEMENT_CONTEXT_FLAG, true)
    }

    /// Set `in_statement_position=false`.
    #[inline]
    pub(crate) fn not_in_statement_position(self) -> Self {
        self.with_flag(Self::IN_STATEMENT_POSITION_FLAG, false)
    }

    /// Set `in_before_block=true`.
    #[inline]
    pub(crate) fn in_before_block(self) -> Self {
        self.with_flag(Self::IN_BEFORE_BLOCK_FLAG, true)
    }

    /// Set `in_tree_literal=true`.
    #[inline]
    pub(crate) fn in_tree_literal(self) -> Self {
        self.with_flag(Self::IN_TREE_LITERAL_FLAG, true)
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

    /// Set `allow_type_predicate=true`.
    #[inline]
    pub(crate) fn allow_type_predicate(self) -> Self {
        self.with_flag(Self::ALLOW_TYPE_PREDICATE_FLAG, true)
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

    /// Set `allow_private_hash_key=true`.
    #[inline]
    pub(crate) fn allow_private_hash_key(self) -> Self {
        self.with_flag(Self::ALLOW_PRIVATE_HASH_KEY_FLAG, true)
    }

    /// Set `in_generator=true`.
    #[inline]
    pub(crate) fn in_generator(self) -> Self {
        self.with_flag(Self::IN_GENERATOR_FLAG, true)
    }

    /// Set `in_decorator=true`.
    #[inline]
    pub(crate) fn in_decorator(self) -> Self {
        self.with_flag(Self::IN_DECORATOR_FLAG, true)
    }

    /// Set `in_decorator=false`.
    #[inline]
    pub(crate) fn not_in_decorator(self) -> Self {
        self.with_flag(Self::IN_DECORATOR_FLAG, false)
    }

    /// Set `left_precedence=precedence`.
    #[inline]
    pub(crate) fn in_left_precedence(self, precedence: u16) -> Self {
        Self {
            left_precedence: Some(precedence),
            ..self
        }
    }

    /// Clear `left_precedence` to allow all operators.
    #[inline]
    pub(crate) fn not_in_left_precedence(self) -> Self {
        Self {
            left_precedence: None,
            ..self
        }
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
        flags.set_disallow_ambiguous_tree_literal(self.is_disallow_ambiguous_tree_literal());
        flags.set_in_declare_context(self.is_in_declare_context());
        flags.set_in_statement_context(self.is_in_statement_context());
        flags.set_flag(
            Self::IN_MODULE_DIRECTIVE_FLAG,
            self.is_in_module_directive(),
        );
        flags
    }
}

/// A parser for a single source file.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser {
    /// The source we're parsing.
    pub file: Arc<File>,
    /// The source ID.
    pub file_id: FileId,
    /// The lexer backing the parser.
    pub(crate) lexer: Lexer,

    /// The current visible token at the parser cursor.
    current_token: TokenSpan,
    /// The previous semantic token end.
    previous_token_end: u32,
    /// The last consumed visible token.
    last_consumed_token: TokenSpan,
    /// Whether the parser is finished.
    is_finished: bool,
    /// The parser context flags.
    pub(crate) flags: ParserFlags,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    preserve_parenthesized_wrappers: bool,

    /// The Node DIR tree.
    pub tree: Tree,
    /// The shared string pool.
    pub strings: Arc<StringPool>,

    /// The language type for parsing behavior.
    pub language: LanguageType,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The errors encountered so far (for deduplication).
    pub errors: Vec<ParseError>,
    /// Semantic parser bookkeeping.
    pub(crate) state: ParserState,
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl Parser {
    /// Return true when the current token starts after a line break.
    #[inline]
    pub(crate) fn current_token_is_on_new_line(&self) -> bool {
        self.current_token.token.is_on_new_line
    }

    /// Return true when the previous token started after a line break.
    #[inline]
    pub(crate) fn previous_token_is_on_new_line(&self) -> bool {
        self.last_consumed_token.token.is_on_new_line
    }

    /// Return true when comments appear between the previous token and current token.
    pub(crate) fn current_token_has_leading_comment(&self) -> bool {
        let start = self.previous_token_end;
        let end = self.current_token.span.start;

        self.lexer.side_tokens().iter().any(|token| {
            token.span.start >= start
                && token.span.end <= end
                && matches!(
                    token.token.ty,
                    TokenType::LineComment
                        | TokenType::BlockComment
                        | TokenType::DocLineComment
                        | TokenType::DocBlockComment
                )
        })
    }

    /// Return true when transparent parenthesized wrappers stay in the parsed tree.
    #[inline]
    pub(crate) fn preserves_parenthesized_wrappers(&self) -> bool {
        self.preserve_parenthesized_wrappers
    }

    /// Create one parser for a file before lexing begins.
    fn parser_for_file(file: Arc<File>, language: LanguageType, strings: Arc<StringPool>) -> Self {
        let module_id = ModuleId::new(PackageId::new(0), file.id.0);

        Self::parser_for_module(file, language, module_id, strings)
    }

    /// Create one parser for a source module before lexing begins.
    fn parser_for_module(
        file: Arc<File>,
        language: LanguageType,
        module_id: ModuleId,
        strings: Arc<StringPool>,
    ) -> Self {
        let source_len = file.text().len();
        let estimated_tokens = source_len / ESTIMATED_TOKEN_BYTES;
        let estimated_nodes = estimated_tokens;
        let tree = Tree::with_capacity(module_id, estimated_nodes);

        Self::parser_for_module_tree(file, language, strings, tree)
    }

    /// Create one parser that appends into an existing DIR tree.
    fn parser_for_module_tree(
        file: Arc<File>,
        language: LanguageType,
        strings: Arc<StringPool>,
        tree: Tree,
    ) -> Self {
        // initialize the lexer for lazy lexing
        let lexer = Lexer::new(file.clone(), language);

        // initialize source-local parser state
        let file_id = file.id;
        let type_literal_identifiers = TypeLiteralIdentifiers::new(strings.as_ref());
        Self {
            file,
            file_id,
            lexer,
            current_token: TokenSpan {
                token: Token::end(),
                span: Span::new(file_id, 0, 0),
            },
            previous_token_end: 0,
            last_consumed_token: TokenSpan {
                token: Token::end(),
                span: Span::new(file_id, 0, 0),
            },
            is_finished: false,
            flags: ParserFlags::default(),
            preserve_parenthesized_wrappers: true,
            language,
            tree,
            strings,
            diagnostics: DiagnosticCollector::new(),
            errors: Vec::new(),
            state: ParserState::new(type_literal_identifiers),
        }
    }

    /// Create a new parser from a text File and tokenize it.
    pub fn lex_file(file: Arc<File>, language: LanguageType, strings: Arc<StringPool>) -> Self {
        let mut parser = Self::parser_for_file(file, language, strings);

        // reset parser state to start
        parser.reset();
        parser
    }

    /// Create a new parser from a module text File and tokenize it.
    pub fn lex_module(
        module_id: ModuleId,
        file: Arc<File>,
        language: LanguageType,
        strings: Arc<StringPool>,
    ) -> Self {
        let mut parser = Self::parser_for_module(file, language, module_id, strings);

        // reset parser state to start
        parser.reset();
        parser
    }

    /// Lex a file and apply parser options.
    pub fn lex_file_with_options(
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
    ) -> Self {
        let mut parser = Self::parser_for_file(file, language, strings);
        parser
            .lexer
            .set_retain_trivia_tokens(options.retain_trivia_tokens);
        parser.reset();
        parser.apply_options(options);
        parser
    }

    /// Lex a module text File and apply parser options.
    pub fn lex_module_with_options(
        module_id: ModuleId,
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
    ) -> Self {
        let mut parser = Self::parser_for_module(file, language, module_id, strings);
        parser
            .lexer
            .set_retain_trivia_tokens(options.retain_trivia_tokens);
        parser.reset();
        parser.apply_options(options);
        parser
    }

    /// Lex a module text File into an existing DIR tree and apply parser options.
    pub fn lex_module_tree_with_options(
        file: Arc<File>,
        language: LanguageType,
        options: ParserOptions,
        strings: Arc<StringPool>,
        tree: Tree,
    ) -> Self {
        let mut parser = Self::parser_for_module_tree(file, language, strings, tree);
        parser
            .lexer
            .set_retain_trivia_tokens(options.retain_trivia_tokens);
        parser.reset();
        parser.apply_options(options);
        parser
    }

    /// Apply externally provided parser options.
    #[inline]
    pub fn apply_options(&mut self, options: ParserOptions) {
        self.flags
            .set_disallow_ambiguous_tree_literal(options.disallow_ambiguous_tree_literal);
        self.preserve_parenthesized_wrappers = options.preserve_parenthesized_wrappers;
        if self.lexer.tokens().is_empty() && self.lexer.side_tokens().is_empty() {
            self.lexer
                .set_retain_trivia_tokens(options.retain_trivia_tokens);
        } else {
            debug_assert!(
                self.lexer.retains_trivia_tokens() == options.retain_trivia_tokens,
                "trivia retention must be configured before lexing starts"
            );
        }
    }
    /// Get the span of all side annotations.
    #[inline]
    pub fn compute_side_span(&self) -> MultiSpan {
        Self::compute_side_span_from_tree(&self.tree)
    }

    /// Get the span of all side decorators from a tree.
    #[inline]
    pub fn compute_side_span_from_tree(tree: &Tree) -> MultiSpan {
        MultiSpan::new(tree.get_side_decorator_spans())
    }

    /// Reset the parser.
    pub(crate) fn reset(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.previous_token_end = 0;
        let mut flags = ParserFlags::default();
        flags.set_disallow_ambiguous_tree_literal(
            self.language.supports_jsx() && self.language.is_typescript(),
        );
        self.flags = flags;
        self.errors.clear();
        self.state.reset();

        self.read_next_token();
        self.previous_token_end = 0;
        self.last_consumed_token = TokenSpan {
            token: Token::end(),
            span: Span::new(self.file_id, 0, 0),
        };
    }

    /// Swap parser flags and return the previous value.
    #[inline(always)]
    pub(crate) fn swap_flags(&mut self, flags: ParserFlags) -> ParserFlags {
        let old_flags = self.flags;
        self.flags = flags;
        old_flags
    }

    /// Restore parser flags from a previous swap.
    #[inline(always)]
    pub(crate) fn restore_flags(&mut self, old_flags: ParserFlags) {
        self.flags = old_flags;
    }

    /// Execute a function with new parser flags.
    /// The previous flags are restored after the function returns.
    #[inline(always)]
    pub(crate) fn with_flags<T>(
        &mut self,
        flags: ParserFlags,
        func: impl FnOnce(&mut Self) -> T,
    ) -> T {
        if self.flags == flags {
            return func(self);
        }

        let old_flags = self.swap_flags(flags);
        let result = func(self);
        self.restore_flags(old_flags);
        result
    }

    /// Return the current semantic tokens.
    #[inline]
    pub(crate) fn tokens(&self) -> &[TokenSpan] {
        self.lexer.tokens()
    }

    /// Return the innermost expression after skipping parenthesized wrappers.
    #[inline]
    pub(crate) fn without_parentheses_expression(
        &self,
        mut expression_id: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        while let Expression::Parenthesized { expression } = self.tree.get(expression_id) {
            expression_id = *expression;
        }

        expression_id
    }

    /// Return true when tree literal lexing is enabled.
    #[inline]
    pub(crate) fn allow_tree_literals(&self) -> bool {
        self.lexer.allow_tree_literals()
    }

    /// Set whether tree literal lexing is enabled.
    #[inline]
    pub(crate) fn set_allow_tree_literals(&mut self, allow: bool) {
        self.lexer.set_allow_tree_literals(allow);
    }

    /// Eat a tree opening `<`.
    #[inline]
    pub(crate) fn eat_tree_opening_angle(&mut self) -> ParseResult<()> {
        if !self.peek_is(TokenType::LessThan) {
            return Err(ParseError::expected(self.peek()?.span, TokenType::LessThan));
        }

        self.bump();
        Ok(())
    }

    /// Enable or disable tree attribute value lexing for the next token.
    #[inline]
    pub(crate) fn set_tree_attribute_value(&mut self, enabled: bool) {
        self.lexer.set_tree_attribute_value(enabled);
    }

    /// Re-lex the current token as a generic `<`.
    #[inline]
    pub(crate) fn re_lex_generic_l_angle(&mut self) -> bool {
        let token_type = self.current_token.token.ty;
        if token_type == TokenType::LessThan {
            return true;
        }

        if !matches!(
            token_type,
            TokenType::ShiftLeft | TokenType::LessThanOrEqual | TokenType::ShiftLeftAssign
        ) {
            return false;
        }

        let token = self.lexer.re_lex_as_typed_l_angle(self.current_token);
        self.lexer.replace_current_token(token);
        self.current_token = token;
        true
    }

    /// Re-lex the current token as one `>`.
    #[inline]
    pub(crate) fn re_lex_r_angle(&mut self) -> bool {
        let token_type = self.current_token.token.ty;
        if token_type == TokenType::GreaterThan {
            return true;
        }

        if !matches!(
            token_type,
            TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        ) {
            return false;
        }

        let token = self.lexer.re_lex_as_r_angle(self.current_token);
        self.lexer.replace_current_token(token);
        self.current_token = token;
        true
    }

    /// Re-lex the current `/` or `/=` token as a regex literal
    #[inline]
    pub(crate) fn re_lex_regex(&mut self) -> bool {
        let token_type = self.current_token.token.ty;
        if token_type == TokenType::Literal
            && matches!(
                self.current_token.token.literal,
                Some(TokenLiteral::RegexString { .. })
            )
        {
            return true;
        }

        if !matches!(token_type, TokenType::Divide | TokenType::DivideAssign) {
            return false;
        }

        let token = self.lexer.re_lex_as_regex(self.current_token);
        self.lexer.replace_current_token(token);
        self.current_token = token;
        true
    }

    /// Eat one typed angle-close token.
    #[inline]
    pub(crate) fn eat_type_angle_close(&mut self) -> ParseResult<()> {
        if !self.re_lex_r_angle() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        self.bump();
        Ok(())
    }

    /// Eat one expression-position typed angle-close token.
    #[inline]
    pub(crate) fn eat_expression_type_angle_close(&mut self) -> ParseResult<()> {
        if !Self::starts_expression_type_angle_close(self.peek_token_type()) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        self.eat_type_angle_close()
    }

    /// Return true when one token can begin a type-angle close sequence.
    #[inline]
    pub(crate) const fn starts_type_angle_close(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::GreaterThan
                | TokenType::ShiftRight
                | TokenType::UnsignedShiftRight
                | TokenType::GreaterThanOrEqual
                | TokenType::ShiftRightAssign
                | TokenType::UnsignedShiftRightAssign
        )
    }

    /// Return true when one token can begin an expression-position type-angle close.
    #[inline]
    pub(crate) const fn starts_expression_type_angle_close(token_type: TokenType) -> bool {
        matches!(
            token_type,
            TokenType::GreaterThan | TokenType::ShiftRight | TokenType::UnsignedShiftRight
        )
    }

    /// Return true when the current token can begin a type-angle close sequence.
    #[inline]
    pub(crate) fn peek_starts_type_angle_close(&mut self) -> bool {
        Self::starts_type_angle_close(self.peek_token_type())
    }

    /// Return true when the current token can begin an expression-position type-angle close.
    #[inline]
    pub(crate) fn peek_starts_expression_type_angle_close(&mut self) -> bool {
        Self::starts_expression_type_angle_close(self.peek_token_type())
    }

    /// Return true when the current token can begin one `>` in tree tag syntax.
    #[inline]
    pub(crate) fn peek_starts_tree_tag_close(&mut self) -> bool {
        Self::starts_type_angle_close(self.peek_token_type())
    }

    /// Return owned token buffers after lexing to EOF.
    pub fn take_tokens(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        self.lexer.take_tokens()
    }

    /// Return the EOF span without forcing a full lex.
    #[inline]
    pub(crate) fn eof_span(&self) -> Span {
        Span::new(self.file_id, self.file.len, self.file.len)
    }

    /// Read the next token from the lexer cursor.
    #[inline]
    fn read_next_token(&mut self) {
        self.current_token = self.lexer.next_token();
    }

    /// Parse everything as an implicit namespace with optional trivia attachment.
    fn parse_root_expressions(&mut self, attach_trivia: bool) -> Vec<LocalNodeId<Expression>> {
        let start = self.span_start();
        let mut expressions = self.with_token_recovery(
            &start,
            |parser| parser.eat_block_body(BlockForm::Implicit),
            Vec::new(),
            TokenType::End,
        );

        // ensure one stable owner for trivia only files
        self.ensure_trivia_anchor_maybe(&mut expressions, false);

        // attach comments only in the full parse pipeline
        if attach_trivia {
            self.attach_comments();
            self.is_finished = true;
        }

        expressions
    }

    /// Parse everything as an implicit namespace.
    pub fn parse(&mut self) -> Vec<LocalNodeId<Expression>> {
        self.parse_root_expressions(true)
    }

    /// Return whether the parser finished one full parse pipeline.
    pub const fn is_finished(&self) -> bool {
        self.is_finished
    }

    /// Parse everything as an implicit namespace without attaching trivia.
    pub fn parse_without_trivia(&mut self) -> Vec<LocalNodeId<Expression>> {
        self.parse_root_expressions(false)
    }

    /// Ensure one stable owner for comment trivia in comment only files.
    fn ensure_trivia_anchor_maybe(
        &mut self,
        expressions: &mut Vec<LocalNodeId<Expression>>,
        consumed_to_end: bool,
    ) {
        if !self.lexer.retains_trivia_tokens() {
            return;
        }

        // most files already have parsed body expressions and never need a trivia anchor
        if !consumed_to_end && !expressions.is_empty() {
            return;
        }

        // materialize the full stream before trivia ownership checks
        self.lexer.lex_to_end();

        // skip files without retained comments
        if !self.lexer.has_comment_tokens() {
            return;
        }

        let stub_span = self.eof_span();

        // comment only files need one returned expression owner
        if expressions.is_empty() {
            let stub = self.insert_node(Expression::Stub, stub_span);
            expressions.push(stub);
            return;
        }

        // only directive only files need an internal trivia owner
        if !consumed_to_end {
            return;
        }

        // directive only files with attachable semantic tokens already have stable owners
        if self.lexer.has_attachable_semantic_tokens() {
            return;
        }

        // insert one internal anchor so trivia can attach without parse errors
        let _ = self.insert_node(Expression::Stub, stub_span);
    }

    /// Attach retained comments after parsing when needed.
    pub fn attach_comments(&mut self) {
        // skip comment output when trivia retention is disabled
        if !self.lexer.retains_trivia_tokens() {
            return;
        }

        // materialize the stream so comment state is complete
        self.lexer.lex_to_end();

        if !self.lexer.has_comment_tokens() {
            return;
        }

        // avoid copying comments twice when direct entrypoints attach manually
        if !self.tree.comments().is_empty() {
            return;
        }

        // finalize raw comments in parse order
        let comments = self.lexer.take_trivia_comments();
        self.tree.comments_mut().extend(comments);
    }

    /// Handle an error as a Diagnostic.
    /// Errors are deduplicated by leaf content to avoid squiggly red line noise.
    #[inline]
    pub(crate) fn error(&mut self, e: &ParseError) {
        if !self.errors.iter().any(|d| d.eq_content(e)) {
            self.errors.push(e.clone());
            let diagnostic = e.to_diagnostic(self.file.as_ref(), self.tokens());
            self.diagnostics.insert(diagnostic);
        }
    }

    /// Create a checkpoint for speculative parsing that may allocate tree nodes.
    #[inline(always)]
    pub fn checkpoint(&mut self) -> ParserCheckpoint {
        ParserCheckpoint {
            current_token: self.current_token,
            previous_token_end: self.previous_token_end,
            last_consumed_token: self.last_consumed_token,
            tree_mark: self.tree.mark(),
            lexer_checkpoint: self.lexer.snapshot(),
            error_count: self.errors.len(),
            diagnostic_count: self.diagnostics.len(),
        }
    }

    /// Create a checkpoint for speculative cursor movement without tree allocation snapshots.
    #[inline(always)]
    pub fn cursor_checkpoint(&mut self) -> ParserCursorCheckpoint {
        ParserCursorCheckpoint {
            current_token: self.current_token,
            previous_token_end: self.previous_token_end,
            last_consumed_token: self.last_consumed_token,
            lexer_checkpoint: self.lexer.snapshot(),
        }
    }

    /// Create a lightweight span start at the current parser cursor.
    #[inline(always)]
    pub fn span_start(&self) -> ParserSpanStart {
        ParserSpanStart {
            current_token: self.current_token,
        }
    }

    /// Rewind the parser cursor to one cursor checkpoint.
    pub fn rewind(&mut self, checkpoint: ParserCursorCheckpoint) {
        self.current_token = checkpoint.current_token;
        self.previous_token_end = checkpoint.previous_token_end;
        self.last_consumed_token = checkpoint.last_consumed_token;
        self.lexer.restore(checkpoint.lexer_checkpoint);
    }

    /// Restore the parser and tree to one full checkpoint.
    pub fn restore(&mut self, checkpoint: ParserCheckpoint, idx: u32) {
        self.current_token = checkpoint.current_token;
        self.previous_token_end = checkpoint.previous_token_end;
        self.last_consumed_token = checkpoint.last_consumed_token;
        debug_assert_eq!(checkpoint.tree_mark.next_global_id(), idx);
        self.tree.restore_to_mark(checkpoint.tree_mark);
        self.errors.truncate(checkpoint.error_count);
        self.diagnostics.truncate(checkpoint.diagnostic_count);
        self.lexer.restore(checkpoint.lexer_checkpoint);
    }

    /// Run a closure against a speculative parser cursor.
    pub(crate) fn lookahead<T>(&mut self, func: impl FnOnce(&mut Self) -> T) -> T {
        let checkpoint = self.cursor_checkpoint();
        let result = func(self);
        self.rewind(checkpoint);
        result
    }

    /// Return the next parser token without consuming it.
    #[inline]
    pub(crate) fn next_token(&mut self) -> TokenSpan {
        self.lookahead(|parser| {
            parser.bump();
            parser.current_token()
        })
    }

    /// Return the next parser token type without consuming it.
    #[inline]
    pub(crate) fn next_token_type(&mut self) -> TokenType {
        self.lookahead(|parser| {
            parser.bump();
            parser.peek_token_type()
        })
    }

    /// Return the next parser keyword without consuming it.
    #[inline]
    pub(crate) fn next_keyword(&mut self) -> Option<Keyword> {
        self.lookahead(|parser| {
            parser.bump();
            parser.current_keyword()
        })
    }

    /// Insert a node into the DIR tree.
    #[inline]
    pub(crate) fn insert_node<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        Tree: TreeStore<T>,
    {
        self.tree.insert_during_parse(node, span)
    }

    /// Attach one child-owned leading boundary span.
    pub(crate) fn set_node_leading_span<T>(&mut self, node_id: LocalNodeId<T>, boundary_start: u32)
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.tree.get_span(node_id);
        if boundary_start >= node_span.start {
            return;
        }

        let leading_span = Span::new(node_span.file, boundary_start, node_span.start);
        self.tree.set_side_span(
            node_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Leading),
            leading_span,
        );
    }

    /// Attach one child-owned trailing boundary span.
    pub(crate) fn set_node_trailing_span<T>(&mut self, node_id: LocalNodeId<T>, boundary_end: u32)
    where
        T: Node + Clone,
        Tree: TreeStore<T>,
    {
        let node_span = self.tree.get_span(node_id);
        if boundary_end <= node_span.end {
            return;
        }

        let trailing_span = Span::new(node_span.file, node_span.end, boundary_end);
        self.tree.set_side_span(
            node_id,
            NodeSpanType::Boundary(NodeSpanBoundary::Trailing),
            trailing_span,
        );
    }

    /// Return the span from one parser span start to the previous token.
    #[inline(always)]
    pub fn get_span_from(&self, start: &ParserSpanStart) -> Span {
        let start = start.current_token.span.start;
        let end = self.previous_token_end.max(start);
        Span::new(self.file_id, start, end)
    }

    /// Return the span between two parser span starts.
    #[inline(always)]
    pub fn get_span_between(&self, start: &ParserSpanStart, end: &ParserSpanStart) -> Span {
        Span::new(
            self.file_id,
            start.current_token.span.start,
            end.current_token.span.end,
        )
    }

    /// Gets the str source backing a Span.
    #[inline]
    pub fn get_span_str(&self, span: Span) -> &str {
        self.file.get_span_str(span).unwrap_or_default()
    }

    /// Gets the str source backing a TokenSpan.
    #[inline]
    pub fn get_token_str(&self, token: TokenSpan) -> &str {
        self.file.get_span_str(token.span).unwrap_or_default()
    }

    /// Get the source text for the current token.
    #[inline]
    pub(crate) fn current_token_str(&self) -> &str {
        self.get_token_str(self.current_token)
    }

    /// Get the current token.
    #[inline]
    pub(crate) fn current_token(&self) -> TokenSpan {
        self.current_token
    }

    /// Return the current token as a keyword.
    #[inline]
    pub(crate) fn current_keyword(&self) -> Option<Keyword> {
        if self.current_token.token.ty != TokenType::Identifier {
            return None;
        }

        keyword_from_identifier(self.current_token_str())
    }

    /// Return true when the current identifier has the expected source text.
    #[inline]
    pub(crate) fn current_identifier_str_is(&self, expected: &str) -> bool {
        self.current_token.token.ty == TokenType::Identifier && self.current_token_str() == expected
    }

    /// Return true when the current identifier is `global`.
    #[inline]
    pub(crate) fn is_global_identifier(&self) -> bool {
        self.current_identifier_str_is("global")
    }

    /// Return true when the current identifier is `module`.
    #[inline]
    pub(crate) fn is_module_identifier(&self) -> bool {
        self.language.supports_module_declaration() && self.current_identifier_str_is("module")
    }

    /// Get the previous Token.
    #[inline]
    pub fn prev(&self) -> Option<&TokenSpan> {
        (self.previous_token_end > 0).then_some(&self.last_consumed_token)
    }

    /// Get the previous Token type.
    #[inline]
    pub fn prev_token_type(&self) -> TokenType {
        self.prev()
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
    }

    /// Return the end offset of the previously consumed semantic token.
    #[inline]
    pub(crate) fn prev_token_end(&self) -> u32 {
        self.previous_token_end
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek(&mut self) -> ParseResult<&TokenSpan> {
        Ok(&self.current_token)
    }

    /// Peek the next token type, defaulting to End at EOF.
    #[inline]
    pub fn peek_token_type(&mut self) -> TokenType {
        self.current_token.token.ty
    }

    /// Return true when the next token matches the given type.
    #[inline]
    pub fn peek_is(&mut self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_is requires semantic token type"
        );

        self.peek_token_type() == token_type
    }

    /// Return true when more tokens remain before End.
    #[inline]
    pub fn has_more_tokens(&mut self) -> bool {
        self.peek_token_type() != TokenType::End
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat(&mut self) -> ParseResult<&TokenSpan> {
        let consumed = self.current_token;
        self.last_consumed_token = consumed;
        self.previous_token_end = consumed.span.end;
        self.read_next_token();

        Ok(&self.last_consumed_token)
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");

        self.last_consumed_token = self.current_token;
        self.previous_token_end = self.current_token.span.end;
        self.read_next_token();
    }

    /// Advance the next token as a tree child token.
    #[inline]
    pub(crate) fn bump_tree_child(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.last_consumed_token = self.current_token;
        self.previous_token_end = self.current_token.span.end;
        self.current_token = self.lexer.next_tree_child();
    }

    /// Peek the next token.
    #[inline]
    pub fn peek_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_token requires semantic token type"
        );
        let next = self.peek()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next token in a list of token types.
    #[inline]
    pub fn peek_token_in(&mut self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Eat a token.
    #[inline]
    pub fn eat_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "eat_token requires semantic token type"
        );
        let current = self.eat()?;
        if current.token.ty == token_type {
            Ok(current)
        } else {
            Err(ParseError::unexpected(current.span))
        }
    }

    /// Eat a token maybe.
    pub fn eat_token_maybe(&mut self, token_type: TokenType) -> ParseResult<bool> {
        if self.peek_is(token_type) {
            self.bump();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Expect a token and advance the next token as a tree child token.
    #[inline]
    pub(crate) fn expect_tree_child(&mut self, token_type: TokenType) -> ParseResult<()> {
        if !self.peek_is(token_type) {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        self.bump_tree_child();
        Ok(())
    }

    /// Eat one tree tag close token and advance in the requested mode.
    #[inline]
    pub(crate) fn eat_tree_tag_close(&mut self, in_tree_child: bool) -> ParseResult<()> {
        if !self.re_lex_r_angle() {
            return Err(ParseError::unexpected(self.peek()?.span));
        }

        if in_tree_child {
            self.bump_tree_child();
        } else {
            self.bump();
        }

        Ok(())
    }

    /// Eat a token in a list of tokens.
    #[inline]
    pub fn eat_token_in(&mut self, token_types: &[TokenType]) -> ParseResult<TokenType> {
        let current = self.eat()?;
        if token_types.contains(&current.token.ty) {
            Ok(current.token.ty)
        } else {
            Err(ParseError::unexpected(current.span))
        }
    }

    /// Eat a token in a list of tokens maybe.
    #[inline]
    pub fn eat_token_in_maybe(
        &mut self,
        token_types: &[TokenType],
    ) -> ParseResult<Option<TokenType>> {
        let token = *self.peek()?;
        if token_types.contains(&token.token.ty) {
            self.bump();
            Ok(Some(token.token.ty))
        } else {
            Ok(None)
        }
    }

    /// Attempt a function with token recovery.
    pub fn with_token_recovery<T>(
        &mut self,
        start: &ParserSpanStart,
        func: impl FnOnce(&mut Self) -> ParseResult<T>,
        default: T,
        bail: TokenType,
    ) -> T {
        match func(self) {
            Ok(result) => result,
            Err(err) => {
                let _ = self.try_recover(start, bail, Some(err));
                default
            }
        }
    }

    /// Attempt a function with statement recovery.
    pub fn with_statement_recovery<T>(
        &mut self,
        start: &ParserSpanStart,
        func: impl FnOnce(&mut Self) -> ParseResult<T>,
        default: T,
    ) -> T {
        match func(self) {
            Ok(result) => result,
            Err(err) => {
                let _ = self.try_recover_in_statement(start, Some(err));
                default
            }
        }
    }

    /// Recover until the expected token.
    /// Everything from start to then is an error.
    pub fn try_recover(
        &mut self,
        start: &ParserSpanStart,
        recover: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            if token.token.ty == TokenType::End {
                break;
            }

            // recover from here (but report error)
            if token.token.ty == recover {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            } else {
                // keep going
                self.bump();
            }
        }
        // error if we didn't hit the expected token
        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);
        Err(error)
    }

    /// Recover within one list item until a separator or terminator boundary.
    pub fn try_recover_in_item_list(
        &mut self,
        start: &ParserSpanStart,
        terminator: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            // recover from here and keep the separator or terminator for the caller
            if start.is_before(token.span) && token.token.is_on_new_line
                || self.token_matches_terminator(token_type, terminator)
                || Self::is_item_stop_token(token_type)
                || Self::is_close_delimiter_token(token_type)
            {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);
        Err(error)
    }

    /// Recover within one statement until a statement boundary.
    pub fn try_recover_in_statement(
        &mut self,
        start: &ParserSpanStart,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            // recover from here and keep the boundary token for the caller
            if start.is_before(token.span) && token.token.is_on_new_line
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace
            {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        // eof is also a valid statement boundary
        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);
        Ok(())
    }

    /// Recover within one statement from an existing source span.
    pub fn try_recover_in_statement_from_span(
        &mut self,
        start_span: Span,
        error: Option<ParseError>,
    ) -> ParseResult<Span> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            // recover from here and keep the boundary token for the caller
            if start_span.start < token.span.start && token.token.is_on_new_line
                || Self::is_statement_stop_token(token_type)
                || token_type == TokenType::CloseBrace
            {
                let recovered_span = self.recovered_span_from(start_span);
                let error = ParseError::from_source_maybe(recovered_span, error);
                self.error(&error);
                return Ok(recovered_span);
            }

            self.bump();
        }

        // eof is also a valid statement boundary
        let recovered_span = self.recovered_span_from(start_span);
        let error = ParseError::from_source_maybe(recovered_span, error);
        self.error(&error);

        Ok(recovered_span)
    }

    /// Return true when a recovered list item may continue parsing another item.
    pub(crate) fn can_continue_after_recovered_item(
        &mut self,
        terminator: TokenType,
        is_recovered_item: bool,
    ) -> bool {
        if !is_recovered_item {
            return false;
        }

        let token_type = self.peek_token_type();
        !self.token_matches_terminator(token_type, terminator)
            && !Self::is_close_delimiter_token(token_type)
            && token_type != TokenType::End
    }

    /// Return true when one token satisfies one recovery terminator.
    #[inline]
    fn token_matches_terminator(&self, token_type: TokenType, terminator: TokenType) -> bool {
        if terminator == TokenType::GreaterThan {
            return Self::starts_type_angle_close(token_type);
        }

        token_type == terminator
    }

    /// Recover within a property or member body until a boundary token.
    pub fn try_recover_in_body(
        &mut self,
        start: &ParserSpanStart,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            // recover from here and keep the boundary token for the caller
            if start.is_before(token.span) && token.token.is_on_new_line
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type)
            {
                let error = ParseError::from_source_maybe(self.get_span_from(start), error);
                self.error(&error);
                return Ok(());
            }

            self.bump();
        }

        let error = ParseError::from_source_maybe(self.get_span_from(start), error);
        self.error(&error);
        Err(error)
    }

    /// Recover within a property or member body from an existing source span.
    pub fn try_recover_in_body_from_span(
        &mut self,
        start_span: Span,
        error: Option<ParseError>,
    ) -> ParseResult<Span> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            if token_type == TokenType::End {
                break;
            }

            // recover from here and keep the boundary token for the caller
            if start_span.start < token.span.start && token.token.is_on_new_line
                || token_type == TokenType::CloseBrace
                || Self::is_any_stop_token(token_type)
            {
                let recovered_span = self.recovered_span_from(start_span);
                let error = ParseError::from_source_maybe(recovered_span, error);
                self.error(&error);
                return Ok(recovered_span);
            }

            self.bump();
        }

        let recovered_span = self.recovered_span_from(start_span);
        let error = ParseError::from_source_maybe(recovered_span, error);
        self.error(&error);

        Err(error)
    }

    /// Return a recovered span from one source span start to the previous token.
    #[inline]
    fn recovered_span_from(&self, start_span: Span) -> Span {
        let end = self.previous_token_end.max(start_span.start);
        Span::new(start_span.file, start_span.start, end)
    }

    /// Eat the expected token.
    /// If we don't get the token, it's an error, but:
    ///  1) If we do hit the expected token later, we recover from there.
    ///  2) Otherwise, we try to recover forward until the bail token.
    pub fn try_eat_token(&mut self, expected: TokenType, bail: TokenType) -> ParseResult<()> {
        // we're good if it's the expected token
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        // try to recover
        let start = self.span_start();
        while let Ok(token) = self.peek()
            && token.token.ty != bail
        {
            if token.token.ty == TokenType::End {
                break;
            }

            // ok with error if we finally hit the expected token
            if token.token.ty == expected {
                let error = ParseError::unexpected(self.get_span_from(&start));
                self.bump();
                self.error(&error);
                return Ok(());
            }
            // keep going
            else {
                self.bump();
            }
        }

        // error if we didn't hit the expected token, we're either at recovery or EOF
        let error = ParseError::unexpected(self.get_span_from(&start));
        self.error(&error);
        Err(error)
    }

    /// Insert one missing expression node at the current cursor position.
    pub(crate) fn insert_missing_expression_here(&mut self) -> LocalNodeId<Expression> {
        let anchor_span = self.anchor_span_here();
        let missing_span = Span::new(anchor_span.file, anchor_span.start, anchor_span.start);

        self.insert_node(Expression::Missing, missing_span)
    }

    /// Insert one missing type expression node at the current cursor position.
    pub(crate) fn insert_missing_type_expression_here(&mut self) -> LocalNodeId<TypeExpression> {
        let anchor_span = self.anchor_span_here();
        let missing_span = Span::new(anchor_span.file, anchor_span.start, anchor_span.start);

        self.insert_node(TypeExpression::Missing, missing_span)
    }

    /// Return the best local anchor span at the current cursor position.
    pub(crate) fn anchor_span_here(&mut self) -> Span {
        if let Ok(token) = self.peek() {
            token.span
        } else {
            self.eof_span()
        }
    }

    /// Report one unexpected node slot at the current cursor position.
    pub(crate) fn report_unexpected_for_here(&mut self, owner: NodeType) {
        let error = ParseError::unexpected_for(self.anchor_span_here(), owner);

        self.error(&error);
    }

    /// Recover one committed missing token at the current cursor position.
    pub(crate) fn recover_missing_token_here(
        &mut self,
        expected: TokenType,
        owner: NodeType,
        is_recoverable_boundary: bool,
    ) -> ParseResult<()> {
        if !is_recoverable_boundary {
            return Err(ParseError::expected(self.anchor_span_here(), expected));
        }

        self.report_unexpected_for_here(owner);
        Ok(())
    }

    /// Report one committed missing expression slot and insert the missing node.
    pub(crate) fn recover_missing_expression_here(
        &mut self,
        owner: NodeType,
    ) -> LocalNodeId<Expression> {
        self.report_unexpected_for_here(owner);
        self.insert_missing_expression_here()
    }

    /// Report one committed missing type expression slot and insert the missing node.
    pub(crate) fn recover_missing_type_expression_here(
        &mut self,
        owner: NodeType,
    ) -> LocalNodeId<TypeExpression> {
        self.report_unexpected_for_here(owner);
        self.insert_missing_type_expression_here()
    }

    /// Eat one committed type expression or recover one missing child at a type boundary.
    pub(crate) fn eat_type_expression_or_recover_missing(
        &mut self,
        flags: ParserFlags,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.is_type_expression_boundary() {
            return Ok(self.recover_missing_type_expression_here(owner));
        }

        self.with_flags(flags, |parser| parser.eat_type_expression())
    }

    /// Eat one committed type expression node or recover one missing child at a type boundary.
    pub(crate) fn eat_type_expression_node_or_recover_missing(
        &mut self,
        flags: ParserFlags,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.eat_type_expression_or_recover_missing(flags, owner)
    }

    /// Eat one committed expression or recover one missing child at an expression boundary.
    pub(crate) fn eat_expression_or_recover_missing(
        &mut self,
        flags: ParserFlags,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_expression_here(owner));
        }

        self.eat_expression(flags)
    }

    /// Eat one close token or recover one committed missing close delimiter.
    pub(crate) fn eat_close_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParseResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |_, token_type| {
            Self::is_close_delimiter_boundary_token(token_type)
        })
    }

    /// Eat one close token or recover one committed missing close delimiter with custom boundaries.
    pub(crate) fn eat_close_token_or_recover_missing_with(
        &mut self,
        expected: TokenType,
        owner: NodeType,
        is_recoverable_boundary: impl FnOnce(&mut Self, TokenType) -> bool,
    ) -> ParseResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        let token_type = self.peek_token_type();
        let is_recoverable_boundary = is_recoverable_boundary(self, token_type);

        self.recover_missing_token_here(expected, owner, is_recoverable_boundary)
    }

    /// Eat one committed list close token or recover one missing delimiter in place.
    pub(crate) fn eat_list_close_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParseResult<()> {
        if self.peek_is(expected) {
            self.bump();
            return Ok(());
        }

        self.report_unexpected_for_here(owner);
        Ok(())
    }

    /// Eat one committed type close token or recover one missing delimiter at a type boundary.
    pub(crate) fn eat_type_token_or_recover_missing(
        &mut self,
        expected: TokenType,
        owner: NodeType,
    ) -> ParseResult<()> {
        self.eat_close_token_or_recover_missing_with(expected, owner, |_, token_type| {
            Self::is_type_container_boundary_token(token_type)
        })
    }

    /// Get the node starting at a token.
    pub fn find_node_starting_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span_with_filter(
            span.start,
            span.end.saturating_sub(1),
            search,
            |candidate| candidate.span.start == span.start,
        )
    }

    /// Get the node ending at a token.
    pub fn find_node_ending_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span_with_filter(
            span.start,
            span.end.saturating_sub(1),
            search,
            |candidate| candidate.span.end == span.end,
        )
    }

    /// Get the node enclosing a token.
    pub fn find_node_enclosing_at(
        &self,
        span: &Span,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        self.select_enclosing_span_with_filter(
            span.start,
            span.end.saturating_sub(1),
            search,
            filter,
        )
    }

    /// Select the best enclosing span for a given span range and filter.
    fn select_enclosing_span_with_filter(
        &self,
        start: u32,
        end_inclusive: u32,
        search: NodeSearchMode,
        filter: impl Fn(&EnclosingSpan) -> bool,
    ) -> Option<EnclosingSpan> {
        let mut best = None;
        self.tree
            .source_map
            .visit_enclosing_spans(start, end_inclusive, |candidate| {
                if !filter(&candidate) {
                    return;
                }
                match best {
                    Some(current) => {
                        if self.is_better_enclosing_span(search, &candidate, &current) {
                            best = Some(candidate);
                        }
                    }
                    None => {
                        best = Some(candidate);
                    }
                }
            });
        best
    }

    /// Return true when candidate outranks current for the search mode.
    fn is_better_enclosing_span(
        &self,
        search: NodeSearchMode,
        candidate: &EnclosingSpan,
        current: &EnclosingSpan,
    ) -> bool {
        let candidate_len = candidate.length;
        let current_len = current.length;
        let candidate_idx = candidate.idx;
        let current_idx = current.idx;

        match search {
            NodeSearchMode::BiggestOutermost => {
                candidate_len > current_len
                    || (candidate_len == current_len && candidate_idx > current_idx)
            }
            NodeSearchMode::SmallestOutermost => {
                candidate_len < current_len
                    || (candidate_len == current_len && candidate_idx > current_idx)
            }
            NodeSearchMode::SmallestInnermost => {
                candidate_len < current_len
                    || (candidate_len == current_len && candidate_idx < current_idx)
            }
        }
    }

    /// Check if two spans are on the same line.
    #[inline]
    pub fn is_same_line(&self, left: Span, right: Span) -> bool {
        self.file.is_same_line(left.start, right.end)
    }
}
/// Full parser checkpoint for speculative parses that allocate nodes.
#[derive(Debug, Clone)]
pub struct ParserCheckpoint {
    /// The parser owned current token at checkpoint time.
    current_token: TokenSpan,
    /// The previous semantic token end at checkpoint time.
    previous_token_end: u32,
    /// The last consumed visible token at checkpoint time.
    last_consumed_token: TokenSpan,
    /// Tree allocation snapshot at checkpoint time.
    tree_mark: TreeMark,
    /// The lexer checkpoint for speculative parsing.
    lexer_checkpoint: LexerSnapshot,
    /// The parser error count at checkpoint time.
    error_count: usize,
    /// The parser diagnostic count at checkpoint time.
    diagnostic_count: usize,
}

/// Parser cursor checkpoint for speculative lookahead without node allocation.
#[derive(Debug, Clone)]
pub struct ParserCursorCheckpoint {
    /// The parser owned current token at checkpoint time.
    current_token: TokenSpan,
    /// The previous semantic token end at checkpoint time.
    previous_token_end: u32,
    /// The last consumed visible token at checkpoint time.
    last_consumed_token: TokenSpan,
    /// The lexer checkpoint for speculative parsing.
    lexer_checkpoint: LexerSnapshot,
}

/// Lightweight parser position used for span construction.
#[derive(Debug, Copy, Clone)]
pub struct ParserSpanStart {
    /// The parser owned current token at span start time.
    current_token: TokenSpan,
}

impl ParserSpanStart {
    /// Return whether this span start is before one token span.
    #[inline]
    pub(crate) fn is_before(&self, span: Span) -> bool {
        self.current_token.span.start < span.start
    }
}
