use crate::{Lexer, LexerSnapshot, is_semantic};
use core::fmt;
use destack_ast::{
    BlockFormat, Expression, Keyword, LocalNodeId, Node, NodeTree, NodeTreeImpl, NodeTreeMark,
    NodeType, StringId, Token, TokenSpan, TokenType, TypeExpression,
};
use destack_core::LocalStringPool;
use destack_source::{
    DiagnosticCollector, EnclosingSpan, File, FileId, LanguageType, MultiSpan, NodeSearchMode,
    NodeSpanType, Span,
};
use std::fmt::Debug;
#[cfg(feature = "timings")]
use std::ptr::NonNull;
#[cfg(feature = "timings")]
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "timings")]
use crate::parse::timing::ParserTimings;
use crate::parse::timing::{ParserTimingEntry, ParserTimingScope, ParserTimingTag, tags};
use crate::{ParseError, ParseResult};

use super::state::ParserState;
#[cfg(feature = "timings")]
use super::stats::ParserSpeculationStats;
use super::stats::ParserStats;

const ESTIMATED_TOKEN_BYTES: usize = 6;
const ESTIMATED_STRING_COUNT_DENOMINATOR: usize = 16;
const ESTIMATED_STRING_BYTES_DENOMINATOR: usize = 8;

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
    pub(crate) character: StringId,
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
    fn new(strings: &mut LocalStringPool) -> Self {
        Self {
            undefined: strings.intern("undefined"),
            unknown: strings.intern("unknown"),
            object: strings.intern("object"),
            null_: strings.intern("null"),
            any: strings.intern("any"),
            never: strings.intern("never"),
            boolean: strings.intern("boolean"),
            void: strings.intern("void"),
            character: strings.intern("character"),
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

/// Configure Parser behavior.
/// Useful for enabling/disabling features in some AST subtrees.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ParserOptions {
    /// Packed parser context and behavior flags.
    flags: u32,
    /// The left precedence preceding (i.e. before) the expression.
    /// Determines expression operator lifting and grouping.
    pub left_precedence: Option<u16>,
}

impl Default for ParserOptions {
    fn default() -> Self {
        Self {
            flags: Self::ALLOW_SEQUENCE_EXPRESSION_FLAG,
            left_precedence: None,
        }
    }
}

/// Parser settings that can be configured externally.
/// NOTE #Cleanup: ParserSettings living separately from Parser and ParserOptions feels awkward.
#[derive(Debug, Copy, Clone)]
pub struct ParserSettings {
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool,
    /// Whether token side tokens should be retained for formatter and comment output.
    pub retain_trivia_tokens: bool,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    pub preserve_parenthesized_wrappers: bool,
}

impl Default for ParserSettings {
    fn default() -> Self {
        Self {
            disallow_ambiguous_tree_literal: false,
            retain_trivia_tokens: true,
            preserve_parenthesized_wrappers: true,
        }
    }
}

#[allow(unused)]
impl ParserOptions {
    const EXPRESSION_FLAG_MASK: u32 = Self::IN_PARENTHESIS_FLAG
        | Self::IN_STATEMENT_POSITION_FLAG
        | Self::IN_TERNARY_CONDITION_FLAG
        | Self::IN_TYPE_CONDITIONAL_RIGHT_FLAG
        | Self::DISALLOW_TYPE_CONDITIONAL_FLAG
        | Self::IN_ARROW_RETURN_TYPE_FLAG
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
    const IN_TYPE_MAPPED_CONSTRAINT_FLAG: u32 = 1 << 18;
    const IN_FOR_EACH_FLAG: u32 = 1 << 19;
    const IN_NEW_RECEIVER_FLAG: u32 = 1 << 20;
    const IN_TYPEOF_QUERY_FLAG: u32 = 1 << 21;
    const IN_GENERATOR_FLAG: u32 = 1 << 22;
    const FORBID_YIELD_FLAG: u32 = 1 << 23;
    const FORBID_AWAIT_FLAG: u32 = 1 << 24;
    const ALLOW_SEQUENCE_EXPRESSION_FLAG: u32 = 1 << 25;
    const ALLOW_PRIVATE_HASH_KEY_FLAG: u32 = 1 << 26;
    const DISALLOW_AMBIGUOUS_TREE_LITERAL_FLAG: u32 = 1 << 27;
    const DISALLOW_TYPE_CONDITIONAL_FLAG: u32 = 1 << 28;

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

    /// Replace the hot expression-local portion of these options.
    #[inline]
    pub(crate) fn with_expression_context(mut self, context: ParserOptions) -> Self {
        self.flags = (self.flags & !Self::EXPRESSION_FLAG_MASK)
            | (context.flags & Self::EXPRESSION_FLAG_MASK);
        if context.is_in_statement_position() {
            self.set_in_statement_context(true);
        }
        self.left_precedence = context.left_precedence;
        self
    }

    /// Replace the ambient parser portion of these options.
    #[inline]
    pub(crate) fn with_ambient_context(mut self, context: ParserOptions) -> Self {
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
        let options = self.with_flag(Self::IN_SUPER_TYPE_FLAG, enabled);
        if enabled {
            options.with_type(true)
        } else {
            options
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
        let options = self.with_flag(Self::IN_STATEMENT_POSITION_FLAG, enabled);
        if enabled {
            options.with_flag(Self::IN_STATEMENT_CONTEXT_FLAG, true)
        } else {
            options
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

    /// Reset position-related options but preserve context options like `in_generator`.
    pub(crate) fn nested(self) -> Self {
        let mut options = Self::default();
        options.set_in_generator(self.is_in_generator());
        options.set_in_comptime(self.is_in_comptime());
        options.set_forbid_yield(self.is_forbid_yield());
        options.set_forbid_await(self.is_forbid_await());
        options.set_allow_sequence_expression(self.allows_sequence_expression());
        options.set_in_decorator(self.is_in_decorator());
        options.set_disallow_ambiguous_tree_literal(self.is_disallow_ambiguous_tree_literal());
        options.set_in_declare_context(self.is_in_declare_context());
        options.set_in_statement_context(self.is_in_statement_context());
        options
    }
}

/// A parser for a single source file AST.
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

    /// The current parser cursor index in the semantic token stream.
    pos: usize,
    /// The current visible token at the parser cursor.
    current_token: TokenSpan,
    /// The previous semantic token end.
    previous_token_end: u32,
    /// The last consumed visible token.
    last_consumed_token: TokenSpan,
    /// Whether the parser is finished.
    is_finished: bool,
    /// The parser options.
    pub(crate) options: ParserOptions,
    /// Whether transparent parenthesized wrappers should be preserved in the tree.
    preserve_parenthesized_wrappers: bool,

    /// The Node AST tree.
    pub tree: NodeTree,
    /// The string pool (lockless for single-threaded parsing).
    pub strings: LocalStringPool,

    /// The language type for parsing behavior.
    pub language: LanguageType,
    /// The diagnostic collector.
    pub diagnostics: DiagnosticCollector,
    /// The errors encountered so far (for deduplication).
    pub errors: Vec<ParseError>,
    /// Optional parser timing collector.
    #[cfg(feature = "timings")]
    pub(crate) timings: Option<Rc<ParserTimings>>,
    /// Optional parser stats.
    pub(crate) stats: ParserStats,
    /// Semantic parser bookkeeping.
    pub(crate) state: ParserState,
}

/// Cursor information for the next non-newline token.
#[derive(Debug, Copy, Clone)]
pub(crate) struct NonNewlineTokenCursor {
    /// The non-newline token index in the semantic stream.
    pub index: usize,
    /// The non-newline token type at `index`.
    pub token_type: TokenType,
    /// The number of leading newline tokens skipped before `index`.
    pub skipped_newline_count: usize,
    /// Whether this cursor position is preceded by a line break.
    pub has_line_break_before: bool,
}

impl NonNewlineTokenCursor {
    /// Return true when this cursor starts after a statement boundary.
    #[inline]
    pub(crate) const fn starts_after_statement_boundary(self) -> bool {
        self.has_line_break_before
            || matches!(
                self.token_type,
                TokenType::Semicolon | TokenType::End | TokenType::CloseBrace
            )
    }

    /// Return true when this cursor cannot start an immediate operand.
    #[inline]
    pub(crate) const fn omits_restricted_operand(self) -> bool {
        self.starts_after_statement_boundary()
            || matches!(
                self.token_type,
                TokenType::CloseParenthesis
                    | TokenType::CloseBracket
                    | TokenType::Comma
                    | TokenType::Colon
            )
    }
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl Parser {
    /// Return true when raw input before one token stream index contains a line terminator.
    #[inline]
    pub(crate) fn input_has_line_terminator_before_index(&mut self, index: usize) -> bool {
        if self.lexer.materialized_line_terminator_before(index) {
            return true;
        }

        self.tokens()
            .get(index.saturating_sub(1))
            .is_some_and(|token| token.token.ty == TokenType::Newline)
    }

    /// Return true when raw input before the current token contains a line terminator.
    #[inline]
    pub(crate) fn input_has_line_terminator_before_current_token(&mut self) -> bool {
        self.input_has_line_terminator_before_index(self.pos_index())
    }

    /// Return true when transparent parenthesized wrappers stay in the parsed tree.
    #[inline]
    pub(crate) fn preserves_parenthesized_wrappers(&self) -> bool {
        self.preserve_parenthesized_wrappers
    }

    /// Create one parser for a file before lexing begins.
    fn parser_for_file(file: Arc<File>, language: LanguageType) -> Self {
        // initialize the lexer for lazy lexing
        let lexer = Lexer::new(file.clone(), language);

        // size the hot buffers from source bytes up front
        let source_len = file.text().len();
        let estimated_tokens = source_len / ESTIMATED_TOKEN_BYTES;
        let estimated_nodes = estimated_tokens;
        let estimated_string_count = estimated_tokens / ESTIMATED_STRING_COUNT_DENOMINATOR;
        let estimated_string_bytes = source_len / ESTIMATED_STRING_BYTES_DENOMINATOR;
        let file_id = file.id;
        let mut strings =
            LocalStringPool::with_capacity(estimated_string_count, estimated_string_bytes);
        let type_literal_identifiers = TypeLiteralIdentifiers::new(&mut strings);
        Self {
            file,
            file_id,
            lexer,
            pos: 0,
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
            options: ParserOptions::default(),
            preserve_parenthesized_wrappers: true,
            language,
            tree: NodeTree::with_capacity(estimated_nodes),
            strings,
            diagnostics: DiagnosticCollector::new(),
            errors: Vec::new(),
            #[cfg(feature = "timings")]
            timings: timings_from_env(),
            stats: ParserStats::new(speculation_stats_enabled_from_env()),
            state: ParserState::new(estimated_tokens, type_literal_identifiers),
        }
    }

    /// Create a new parser from a text File and tokenize it.
    #[tracing::instrument(name = "parser.lex", level = "trace", skip_all, fields(file_id = ?file.id))]
    pub fn lex_file(file: Arc<File>, language: LanguageType) -> Self {
        let mut parser = Self::parser_for_file(file, language);

        // reset parser state to start
        parser.reset();
        parser
    }

    /// Lex a file and apply parser settings.
    #[tracing::instrument(
        name = "parser.lex",
        level = "trace",
        skip_all,
        fields(file_id = ?file.id)
    )]
    pub fn lex_file_with_settings(
        file: Arc<File>,
        language: LanguageType,
        settings: ParserSettings,
    ) -> Self {
        let mut parser = Self::parser_for_file(file, language);
        parser
            .lexer
            .set_retain_trivia_tokens(settings.retain_trivia_tokens);
        parser.reset();
        parser.apply_settings(settings);
        parser
    }

    /// Apply externally provided parser settings.
    #[inline]
    pub fn apply_settings(&mut self, settings: ParserSettings) {
        self.options
            .set_disallow_ambiguous_tree_literal(settings.disallow_ambiguous_tree_literal);
        self.preserve_parenthesized_wrappers = settings.preserve_parenthesized_wrappers;
        if self.lexer.tokens().is_empty() && self.lexer.side_tokens().is_empty() {
            self.lexer
                .set_retain_trivia_tokens(settings.retain_trivia_tokens);
        } else {
            debug_assert!(
                self.lexer.retains_trivia_tokens() == settings.retain_trivia_tokens,
                "trivia retention must be configured before lexing starts"
            );
        }
    }

    /// Enable or disable parser speculation counters.
    #[inline]
    #[cfg(feature = "timings")]
    pub fn set_collect_speculation_stats(&mut self, enabled: bool) {
        self.stats.set_collect_speculation_stats(enabled);
    }

    /// Get the span of all side annotations.
    #[inline]
    pub fn compute_side_span(&self) -> MultiSpan {
        Self::compute_side_span_from_tree(&self.tree)
    }

    /// Get the span of all side decorators from a tree.
    #[inline]
    pub fn compute_side_span_from_tree(tree: &NodeTree) -> MultiSpan {
        MultiSpan::new(tree.get_side_decorator_spans())
    }

    /// Reset the parser.
    pub(crate) fn reset(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.pos = 0;
        self.previous_token_end = 0;
        let mut options = ParserOptions::default();
        options.set_disallow_ambiguous_tree_literal(
            self.language.supports_jsx() && self.language.is_typescript(),
        );
        self.options = options;
        self.errors.clear();
        self.stats.reset();
        self.state.reset();

        self.refresh_current_token();
        self.previous_token_end = 0;
        self.last_consumed_token = TokenSpan {
            token: Token::end(),
            span: Span::new(self.file_id, 0, 0),
        };
    }

    /// Start a parser timing scope.
    pub(crate) fn timing_scope(&self, tag: ParserTimingTag) -> ParserTimingScope {
        #[cfg(not(feature = "timings"))]
        {
            let _ = tag;
            ParserTimingScope::disabled()
        }

        #[cfg(feature = "timings")]
        {
            let Some(timings) = self.timings.as_ref() else {
                return ParserTimingScope::disabled();
            };
            let timings_ptr = NonNull::from(timings.as_ref());
            ParserTimingScope::new(Some(timings_ptr), tag)
        }
    }

    /// Snapshot timing entries recorded by the parser.
    pub fn timing_snapshot(&self) -> Option<Vec<ParserTimingEntry>> {
        #[cfg(not(feature = "timings"))]
        {
            None
        }

        #[cfg(feature = "timings")]
        {
            self.timings.as_ref().map(|timings| timings.snapshot())
        }
    }

    /// Record a parser timing sample directly.
    #[cfg(feature = "timings")]
    #[inline]
    pub(crate) fn record_timing(&self, tag: ParserTimingTag, duration: std::time::Duration) {
        if let Some(timings) = self.timings.as_ref() {
            timings.record(tag, duration);
        }
    }

    /// Snapshot speculative parser dispatch counters.
    #[cfg(feature = "timings")]
    pub fn speculation_snapshot(&self) -> Option<ParserSpeculationStats> {
        self.stats.speculation_snapshot()
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

    /// Ensure a token exists at the given index.
    #[inline]
    pub(crate) fn ensure_token(&mut self, index: usize) {
        let _timing = self.timing_scope(tags::PARSE_LEX_ENSURE_TOKEN);
        self.lexer.ensure_token(index);
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
                Some(destack_ast::LiteralType::RegexString { .. })
            )
        {
            return true;
        }

        if !matches!(token_type, TokenType::Divide | TokenType::DivideAssign) {
            return false;
        }

        let token = self.lexer.re_lex_as_regex(self.current_token);
        self.lexer.commit_current_token_re_lex(self.pos, token);
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

    /// Return the next non newline token index from a start index.
    #[inline]
    pub(crate) fn next_non_newline_index_from_stream(&mut self, start: usize) -> usize {
        let _timing = self.timing_scope(tags::PARSE_LEX_NEXT_NON_NEWLINE);
        self.first_non_newline_index_from(start)
    }

    /// Return the first non-newline token index from a start index.
    #[inline]
    pub(crate) fn first_non_newline_index_from(&mut self, start: usize) -> usize {
        let mut index = start;
        loop {
            let tokens = self.tokens();
            while let Some(token) = tokens.get(index) {
                if token.token.ty != TokenType::Newline {
                    return index;
                }

                index += 1;
            }

            if self.lexer.is_lexed_to_end() {
                return index;
            }

            self.ensure_token(index);
        }
    }

    /// Return cursor information for the first non-newline token from a start index.
    #[inline]
    pub(crate) fn scanner_cursor_from(&mut self, start: usize) -> NonNewlineTokenCursor {
        let index = self.first_non_newline_index_from(start);
        let token_type = self
            .tokens()
            .get(index)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End);
        let skipped_newline_count = index.saturating_sub(start);
        let has_line_break_before = if skipped_newline_count > 0 {
            true
        } else {
            self.lexer.materialized_line_terminator_before(index)
        };

        NonNewlineTokenCursor {
            index,
            token_type,
            skipped_newline_count,
            has_line_break_before,
        }
    }

    /// Return the matching pair index for an opening token index, lexing ahead if needed.
    #[inline]
    pub(crate) fn matching_pair_or_lex(&mut self, index: usize) -> Option<usize> {
        let _timing = self.timing_scope(tags::PARSE_LEX_MATCHING_PAIR);

        // fully materialized streams can serve pair lookups without incremental lex checks
        if self.lexer.is_lexed_to_end() {
            return self.lexer.matching_pair(index);
        }

        // tree literal lexing needs parser driven mode switches before aggressive lookahead
        if self.allow_tree_literals() && !self.options.is_in_type() {
            return self.lexer.matching_pair(index);
        }

        self.ensure_token(index);

        let token = self.tokens().get(index)?;
        if !matches!(
            token.token.ty,
            TokenType::OpenParenthesis | TokenType::OpenBrace | TokenType::OpenBracket
        ) {
            return None;
        }

        loop {
            let value = self.lexer.matching_pair(index);
            if value.is_some() {
                break value;
            }

            if self.lexer.is_lexed_to_end() {
                break None;
            }

            self.ensure_token(self.tokens().len());
        }
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

    /// Ensure a token exists at the given index and return it.
    #[inline]
    pub(crate) fn token_at(&mut self, index: usize) -> Option<TokenSpan> {
        let _timing = self.timing_scope(tags::PARSE_LEX_TOKEN_AT);

        if let Some(token) = self.tokens().get(index).copied() {
            return Some(token);
        }

        self.lexer.token(index)
    }

    /// Ensure a token exists at the given index and return a reference.
    #[inline]
    pub(crate) fn token_ref_at(&mut self, index: usize) -> Option<&TokenSpan> {
        if index < self.tokens().len() {
            return self.tokens().get(index);
        }

        self.ensure_token(index);
        self.tokens().get(index)
    }

    /// Look up the token type at a given index.
    #[inline]
    pub(crate) fn token_type_at(&mut self, index: usize) -> TokenType {
        if let Some(token) = self.tokens().get(index) {
            return token.token.ty;
        }

        self.ensure_token(index);
        self.tokens()
            .get(index)
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
    }

    /// Return true when two tokens touch in the source with no whitespace.
    #[inline]
    pub(crate) fn tokens_are_adjacent(&mut self, left_index: usize, right_index: usize) -> bool {
        let Some(left) = self.token_at(left_index) else {
            return false;
        };
        let Some(right) = self.token_at(right_index) else {
            return false;
        };

        left.span.end == right.span.start
    }

    /// Check that two tokens are adjacent (no whitespace between them).
    #[inline]
    pub(crate) fn check_tokens_are_adjacent(
        &mut self,
        left_index: usize,
        right_index: usize,
    ) -> ParseResult<()> {
        if self.tokens_are_adjacent(left_index, right_index) {
            return Ok(());
        }

        let span = self
            .token_ref_at(right_index)
            .map(|token| token.span)
            .unwrap_or(self.eof_span());
        Err(ParseError::unexpected(span))
    }

    /// Truncate identifier caches to match the current token count.
    fn truncate_identifier_caches(&mut self) {
        let len = self.tokens().len();
        self.state.truncate_identifier_caches(len);
    }

    /// Ensure identifier caches can index at least `index`.
    #[inline]
    pub(crate) fn ensure_identifier_cache_capacity(&mut self, index: usize) {
        self.state.ensure_identifier_cache_capacity(index);
    }

    /// Refresh the parser owned current token from the backing lexer.
    #[inline]
    fn refresh_current_token(&mut self) {
        self.current_token = self.lexer.token(self.pos).unwrap_or(TokenSpan {
            token: Token::end(),
            span: self.eof_span(),
        });
    }

    /// Synchronize the parser owned cursor state to the current token index.
    #[inline]
    fn sync_cursor_to_pos(&mut self) {
        self.refresh_current_token();

        if self.pos == 0 {
            self.previous_token_end = 0;
            self.last_consumed_token = TokenSpan {
                token: Token::end(),
                span: Span::new(self.file_id, 0, 0),
            };
            return;
        }

        let previous_index = self.pos - 1;
        let previous_token = self.token_at(previous_index).unwrap_or(TokenSpan {
            token: Token::end(),
            span: self.eof_span(),
        });
        self.previous_token_end = previous_token.span.end;
        self.last_consumed_token = previous_token;
    }

    /// Get the current token index.
    #[inline]
    pub(crate) fn pos_index(&self) -> usize {
        self.pos
    }

    /// Look up a keyword at a token index.
    #[inline]
    pub(crate) fn keyword_for_index(&mut self, index: usize) -> Option<Keyword> {
        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        if !self
            .tokens()
            .get(index)
            .is_some_and(|token| token.token.ty == TokenType::Identifier)
        {
            return None;
        }

        let _timing = self.timing_scope(tags::PARSE_LEX_KEYWORD);
        self.lexer.materialized_keyword(index)
    }

    /// Return whether an identifier token contains escape syntax.
    #[inline]
    pub(crate) fn identifier_has_escape_for_index(&mut self, index: usize) -> bool {
        self.lexer.identifier_has_escape(index)
    }

    /// Look up a pre interned identifier at a token index.
    #[inline]
    pub(crate) fn identifier_for_index(&mut self, index: usize) -> Option<StringId> {
        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        self.ensure_identifier_cache_capacity(index);

        if self.state.token_identifiers_cached[index] {
            return self.state.token_identifiers[index];
        }

        let token = self.tokens().get(index)?;
        let identifier = if token.token.ty == TokenType::Identifier {
            #[cfg(feature = "timings")]
            let started_at = Instant::now();

            let span_str = self.file.span_str(token.span);
            let identifier = self.strings.intern(span_str);

            #[cfg(feature = "timings")]
            self.record_timing(tags::PARSE_ALLOC_IDENTIFIER_INTERN, started_at.elapsed());

            Some(identifier)
        } else {
            None
        };
        self.state.token_identifiers[index] = identifier;
        self.state.token_identifiers_cached[index] = true;
        identifier
    }

    /// Return true when the identifier token at index matches the expected string.
    #[inline]
    pub(crate) fn identifier_equals_at(&mut self, index: usize, expected: &str) -> bool {
        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        let Some(token) = self.tokens().get(index) else {
            return false;
        };
        if token.token.ty != TokenType::Identifier {
            return false;
        }

        let expected = self.strings.intern(expected);

        self.identifier_for_index(index) == Some(expected)
    }

    /// Return true when the identifier token at index is `global`.
    #[inline]
    pub(crate) fn is_global_identifier_at(&mut self, index: usize) -> bool {
        self.identifier_equals_at(index, "global")
    }

    /// Return true when the identifier token at index is `module`.
    #[inline]
    pub(crate) fn is_module_identifier_at(&mut self, index: usize) -> bool {
        self.language.supports_module_declaration() && self.identifier_equals_at(index, "module")
    }

    /// Return a lookahead index from the current parser position.
    #[inline]
    fn lookahead_index(&self, delta: usize) -> usize {
        self.pos + delta
    }

    /// Get the token index used by peek_next.
    #[inline]
    pub(crate) fn index_for_next(&self) -> usize {
        self.lookahead_index(1)
    }

    /// Get the token index used by peek_next_next.
    #[inline]
    pub(crate) fn index_for_next_next(&self) -> usize {
        self.lookahead_index(2)
    }

    /// Get the token index used by peek_next_next_next.
    #[inline]
    pub(crate) fn index_for_next_next_next(&self) -> usize {
        self.lookahead_index(3)
    }

    /// Ensure identifier caches align with the current token count after a rewind.
    fn reset_identifier_caches_after_rewind(&mut self) {
        self.truncate_identifier_caches();
    }

    /// Parse everything as an implicit namespace with optional trivia attachment.
    fn parse_root_expressions(&mut self, attach_trivia: bool) -> Vec<LocalNodeId<Expression>> {
        // parse leading triple-slash reference path directives
        let (mut expressions, consumed_to_end) =
            self.parse_leading_triple_slash_reference_imports();

        // parse the root block body with recovery when source has non-directive content
        if !consumed_to_end {
            let start = self.mark_span();
            let mut body_expressions = self.with_recovery(
                &start,
                |parser| parser.eat_block_body(BlockFormat::Implicit),
                Vec::new(),
                TokenType::End,
            );
            expressions.append(&mut body_expressions);
        }

        // ensure one stable owner for trivia-only files
        self.ensure_trivia_anchor_maybe(&mut expressions, consumed_to_end);

        // attach comments only in the full parse pipeline
        if attach_trivia {
            self.attach_comments();
            self.is_finished = true;
        }

        expressions
    }

    /// Parse everything as an implicit namespace.
    #[tracing::instrument(name = "parser.parse", level = "trace", skip_all, fields(file_id = ?self.file_id))]
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

        let stub_span = Span::new(self.file_id, 0, self.file.len);

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

        // directive-only files with attachable semantic tokens already have stable owners
        if self.lexer.has_attachable_semantic_tokens() {
            return;
        }

        // insert one internal anchor so trivia can attach without parse errors
        let _ = self.insert_node(Expression::Stub, stub_span);
    }

    /// Get the current position in the tokens.
    #[inline]
    pub fn pos(&self) -> u32 {
        self.pos as u32
    }

    /// Attach retained comments after parsing when needed.
    pub fn attach_comments(&mut self) {
        // skip comment output when trivia retention is disabled
        if !self.lexer.retains_trivia_tokens() {
            return;
        }

        let _timing = self.timing_scope(tags::PARSE_COMMENTS);

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

    /// Swap parser options and return the previous value.
    #[inline(always)]
    pub(crate) fn swap_options(&mut self, options: ParserOptions) -> ParserOptions {
        let old_options = self.options;
        self.options = options;
        old_options
    }

    /// Restore parser options from a previous swap.
    #[inline(always)]
    pub(crate) fn restore_options(&mut self, old_options: ParserOptions) {
        self.options = old_options;
    }

    /// Execute a function with new parser options.
    /// The previous options are restored after the function returns.
    #[inline(always)]
    pub(crate) fn with_options<T>(
        &mut self,
        options: ParserOptions,
        func: impl FnOnce(&mut Self) -> T,
    ) -> T {
        if self.options == options {
            return func(self);
        }

        self.stats.record_with_options_call();

        let old_options = self.swap_options(options);
        let result = func(self);
        self.restore_options(old_options);
        result
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

    /// Gets a mark of the current position.
    #[inline(always)]
    pub fn mark(&self) -> ParserMark {
        let _timing = self.timing_scope(tags::PARSE_ALLOC_MARK);

        // snapshot lexer and cache state for any speculative parse
        let lexer_mark = Some(self.lexer.snapshot());
        ParserMark::new(
            self.pos,
            self.current_token,
            self.previous_token_end,
            self.last_consumed_token,
            self.tree.mark(),
            lexer_mark,
            self.errors.len(),
            self.diagnostics.len(),
        )
    }

    /// Get a mark for token rewinds without tree allocation snapshots.
    #[inline(always)]
    pub fn mark_rewind(&self) -> ParserMark {
        // snapshot lexer and cache state for any speculative parse
        let lexer_mark = Some(self.lexer.snapshot());
        ParserMark {
            pos: self.pos,
            current_token: self.current_token,
            previous_token_end: self.previous_token_end,
            last_consumed_token: self.last_consumed_token,
            tree_mark: None,
            lexer_mark,
            error_count: None,
            diagnostic_count: None,
            span_override: None,
        }
    }

    /// Get a lightweight mark used for span calculations without rewind support.
    #[inline(always)]
    pub fn mark_span(&self) -> ParserMark {
        ParserMark {
            pos: self.pos,
            current_token: self.current_token,
            previous_token_end: self.previous_token_end,
            last_consumed_token: self.last_consumed_token,
            tree_mark: None,
            lexer_mark: None,
            error_count: None,
            diagnostic_count: None,
            span_override: None,
        }
    }

    /// Run a closure at a temporary token position and restore parser state afterward.
    pub(crate) fn with_pos<T>(&mut self, pos: usize, func: impl FnOnce(&mut Self) -> T) -> T {
        let mark = self.mark_rewind();
        self.pos = pos;
        self.sync_cursor_to_pos();
        let result = func(self);
        self.rewind(mark);
        result
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn rewind(&mut self, mark: ParserMark) {
        self.stats.record_rewind();
        self.pos = mark.pos;
        self.current_token = mark.current_token;
        self.previous_token_end = mark.previous_token_end;
        self.last_consumed_token = mark.last_consumed_token;
        if let Some(lexer_mark) = mark.lexer_mark {
            self.lexer.restore(lexer_mark);
            self.reset_identifier_caches_after_rewind();
        }
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn restore(&mut self, mark: ParserMark, idx: u32) {
        let _timing = self.timing_scope(tags::PARSE_ALLOC_RESTORE);

        self.stats.record_restore();
        self.pos = mark.pos;
        self.current_token = mark.current_token;
        self.previous_token_end = mark.previous_token_end;
        self.last_consumed_token = mark.last_consumed_token;
        if let Some(tree_mark) = mark.tree_mark {
            debug_assert_eq!(tree_mark.next_global_id(), idx);
            self.tree.restore_to_mark(tree_mark);
        } else {
            self.tree.reset_to(idx);
        }
        if let Some(error_count) = mark.error_count {
            self.errors.truncate(error_count);
        }
        if let Some(diagnostic_count) = mark.diagnostic_count {
            self.diagnostics.truncate(diagnostic_count);
        }
        if let Some(lexer_mark) = mark.lexer_mark {
            self.lexer.restore(lexer_mark);
            self.reset_identifier_caches_after_rewind();
        }
    }

    /// Insert a node into the AST tree.
    #[inline]
    pub(crate) fn insert_node<T>(&mut self, node: T, span: Span) -> LocalNodeId<T>
    where
        T: Node,
        NodeTree: NodeTreeImpl<T>,
    {
        let _timing = self.timing_scope(tags::PARSE_ALLOC_NODE);
        self.tree.insert_during_parse(node, span)
    }

    /// Attach one child-owned leading boundary span.
    pub(crate) fn set_node_leading_span<T>(&mut self, node_id: LocalNodeId<T>, boundary_start: u32)
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_span = self.tree.get_span(node_id);
        if boundary_start >= node_span.start {
            return;
        }

        let leading_span = Span::new(node_span.file, boundary_start, node_span.start);
        self.tree
            .set_side_span(node_id, NodeSpanType::Leading, leading_span);
    }

    /// Attach one child-owned trailing boundary span.
    pub(crate) fn set_node_trailing_span<T>(&mut self, node_id: LocalNodeId<T>, boundary_end: u32)
    where
        T: Node + Clone,
        NodeTree: NodeTreeImpl<T>,
    {
        let node_span = self.tree.get_span(node_id);
        if boundary_end <= node_span.end {
            return;
        }

        let trailing_span = Span::new(node_span.file, node_span.end, boundary_end);
        self.tree
            .set_side_span(node_id, NodeSpanType::Trailing, trailing_span);
    }

    /// Get a mark and return the span of the current position.
    #[inline(always)]
    pub fn get_span_from(&self, mark: &ParserMark) -> Span {
        if let Some(span) = mark.span_override {
            return span;
        }

        let tokens = self.tokens();
        let token_count = tokens.len();
        if token_count == 0 || mark.pos >= token_count {
            return self.eof_span();
        }

        let pos = self.pos;
        let end_index = if pos > 0 { pos - 1 } else { 0 }.min(token_count - 1);
        let start_token = unsafe { tokens.get_unchecked(mark.pos) };
        let end_token = unsafe { tokens.get_unchecked(end_index) };
        Span {
            file: self.file_id,
            start: start_token.span.start,
            end: end_token.span.end,
        }
    }

    /// Get the span between two marks.
    #[inline(always)]
    pub fn get_span_between(&self, start: &ParserMark, end: &ParserMark) -> Span {
        let tokens = self.tokens();
        let start_token = unsafe { tokens.get_unchecked(start.pos) };
        let end_token = unsafe { tokens.get_unchecked(end.pos) };
        Span::new(self.file_id, start_token.span.start, end_token.span.end)
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

    /// Peek the next token type.
    #[inline]
    pub fn peek_next_token_type(&mut self) -> TokenType {
        self.token_type_at(self.index_for_next())
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

    /// Return true when the next-next token matches the given type.
    #[inline]
    pub fn peek_next_is(&mut self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_is requires semantic token type"
        );

        self.peek_next_token_type() == token_type
    }

    /// Return true when the next next token matches the given type.
    #[inline]
    pub fn peek_next_next_is(&mut self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_next_is requires semantic token type"
        );
        self.token_type_at(self.index_for_next_next()) == token_type
    }

    /// Return true when the next next next token matches the given type.
    #[inline]
    pub fn peek_next_next_next_is(&mut self, token_type: TokenType) -> bool {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_next_next_is requires semantic token type"
        );
        self.token_type_at(self.index_for_next_next_next()) == token_type
    }

    /// Return true when more tokens remain before End.
    #[inline]
    pub fn has_more_tokens(&mut self) -> bool {
        self.peek_token_type() != TokenType::End
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next(&mut self) -> ParseResult<&TokenSpan> {
        let index = self.lookahead_index(1);

        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        self.tokens()
            .get(index)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next next Token or error.
    #[inline]
    pub fn peek_next_next(&mut self) -> ParseResult<&TokenSpan> {
        let index = self.lookahead_index(2);

        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        self.tokens()
            .get(index)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next next next Token or error.
    #[inline]
    pub fn peek_next_next_next(&mut self) -> ParseResult<&TokenSpan> {
        let index = self.lookahead_index(3);

        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        self.tokens()
            .get(index)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Eat the next Token or error.
    #[inline]
    pub fn eat(&mut self) -> ParseResult<&TokenSpan> {
        let consumed = self.current_token;
        self.last_consumed_token = consumed;
        self.previous_token_end = consumed.span.end;
        self.pos += 1;
        self.refresh_current_token();

        Ok(&self.last_consumed_token)
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");

        self.last_consumed_token = self.current_token;
        self.previous_token_end = self.current_token.span.end;
        self.pos += 1;
        self.refresh_current_token();
    }

    /// Advance the next token as a tree child token.
    #[inline]
    pub(crate) fn bump_tree_child(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        debug_assert_eq!(
            self.tokens().len(),
            self.pos + 1,
            "tree child advancement requires no materialized lookahead"
        );

        self.last_consumed_token = self.current_token;
        self.previous_token_end = self.current_token.span.end;
        self.pos += 1;
        self.current_token = self.lexer.next_tree_child();
    }

    /// Bump the Token position by a given distance.
    #[inline]
    pub fn bump_by(&mut self, distance: u8) {
        debug_assert!(!self.is_finished, "parser is already finished");

        let next_pos = self.pos + (distance as usize);
        self.ensure_token(next_pos);
        self.last_consumed_token = self.current_token;
        self.pos += distance as usize;
        self.previous_token_end = self
            .token_at(self.pos.saturating_sub(1))
            .map(|token| token.span.end)
            .unwrap_or(self.previous_token_end);
        self.refresh_current_token();
    }

    /// Advance the token position to a specific index.
    #[inline]
    pub(crate) fn advance_to(&mut self, pos: usize) {
        debug_assert!(!self.is_finished, "parser is already finished");

        if pos >= self.tokens().len() {
            self.ensure_token(pos);
        }

        debug_assert!(pos <= self.tokens().len(), "advance past end of tokens");
        self.pos = pos;
        self.sync_cursor_to_pos();
    }

    /// Peek a token at a position.
    #[inline]
    pub fn peek_token_ahead(
        &mut self,
        delta: u32,
        token_type: TokenType,
    ) -> ParseResult<&TokenSpan> {
        let index = self.pos + (delta as usize);
        self.ensure_token(index);
        self.tokens()
            .get(index)
            .filter(|token| token.token.ty == token_type)
            .ok_or(ParseError::unexpected(self.eof_span()))
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

    /// Peek the next next token.
    #[inline]
    pub fn peek_next_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        debug_assert!(
            is_semantic(token_type),
            "peek_next_token requires semantic token type"
        );
        let next = self.peek_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next token in a list of token types.
    #[inline]
    pub fn peek_next_token_in(&mut self, token_types: &[TokenType]) -> ParseResult<&TokenSpan> {
        let next = self.peek_next()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next token in a list of token types.
    #[inline]
    pub fn peek_next_next_token_in(
        &mut self,
        token_types: &[TokenType],
    ) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next()?;
        if token_types.contains(&next.token.ty) {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token.
    #[inline]
    pub fn peek_next_next_next_token(&mut self, token_type: TokenType) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next_next()?;
        if next.token.ty == token_type {
            Ok(next)
        } else {
            Err(ParseError::unexpected(next.span))
        }
    }

    /// Peek the next next next token in a list of token types.
    #[inline]
    pub fn peek_next_next_next_token_in(
        &mut self,
        token_types: &[TokenType],
    ) -> ParseResult<&TokenSpan> {
        let next = self.peek_next_next_next()?;
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

    /// Attempt a function with recovery.
    pub fn with_recovery<T>(
        &mut self,
        start: &ParserMark,
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

    /// Recover until the expected token.
    /// Everything from start to then is an error.
    pub fn try_recover(
        &mut self,
        start: &ParserMark,
        recover: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        // let error = error.unwrap_or_else(|| ParseError::unexpected(self.get_span_from(start)));
        while let Ok(token) = self.peek() {
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
        start: &ParserMark,
        terminator: TokenType,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            // recover from here and keep the separator or terminator for the caller
            if self.token_matches_terminator(token_type, terminator)
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
        start: &ParserMark,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            // recover from here and keep the boundary token for the caller
            if Self::is_statement_stop_token(token_type) || token_type == TokenType::CloseBrace {
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
        start: &ParserMark,
        error: Option<ParseError>,
    ) -> ParseResult<()> {
        while let Ok(token) = self.peek() {
            let token_type = token.token.ty;

            // recover from here and keep the boundary token for the caller
            if token_type == TokenType::CloseBrace || Self::is_any_stop_token(token_type) {
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
        let start = self.mark_span();
        while let Ok(token) = self.peek()
            && token.token.ty != bail
        {
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
        options: ParserOptions,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        if self.is_type_expression_boundary() {
            return Ok(self.recover_missing_type_expression_here(owner));
        }

        self.with_options(options, |parser| parser.eat_type_expression())
    }

    /// Eat one committed type expression node or recover one missing child at a type boundary.
    pub(crate) fn eat_type_expression_node_or_recover_missing(
        &mut self,
        options: ParserOptions,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<TypeExpression>> {
        self.eat_type_expression_or_recover_missing(options, owner)
    }

    /// Eat one committed expression or recover one missing child at an expression boundary.
    pub(crate) fn eat_expression_or_recover_missing(
        &mut self,
        options: ParserOptions,
        owner: NodeType,
    ) -> ParseResult<LocalNodeId<Expression>> {
        if Self::is_expression_slot_boundary_token(self.peek_token_type()) {
            return Ok(self.recover_missing_expression_here(owner));
        }

        self.eat_expression(options)
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

#[cfg(feature = "timings")]
fn timings_enabled_from_env() -> bool {
    std::env::var("DESTACK_PARSER_TIMINGS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value > 0)
        .or_else(|| {
            std::env::var("DESTACK_TIMINGS")
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
                .map(|value| value > 0)
        })
        .unwrap_or(false)
}

#[cfg(feature = "timings")]
fn timings_from_env() -> Option<Rc<ParserTimings>> {
    timings_enabled_from_env().then(|| Rc::new(ParserTimings::default()))
}

#[cfg(feature = "timings")]
fn speculation_stats_enabled_from_env() -> bool {
    std::env::var("DESTACK_PARSER_SPECULATION_STATS")
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value > 0)
        .or_else(|| {
            std::env::var("DESTACK_PARSER_DISPATCH_STATS")
                .ok()
                .and_then(|value| value.parse::<u8>().ok())
                .map(|value| value > 0)
        })
        .unwrap_or(false)
}

#[cfg(not(feature = "timings"))]
fn speculation_stats_enabled_from_env() -> bool {
    false
}
#[derive(Debug, Clone)]
pub struct ParserMark {
    /// The token position.
    pos: usize,
    /// The parser owned current token at mark time.
    current_token: TokenSpan,
    /// The previous semantic token end at mark time.
    previous_token_end: u32,
    /// The last consumed visible token at mark time.
    last_consumed_token: TokenSpan,
    /// Tree allocation snapshot at mark time.
    tree_mark: Option<NodeTreeMark>,
    /// The lexer mark for speculative parsing.
    lexer_mark: Option<LexerSnapshot>,
    /// The parser error count at mark time.
    error_count: Option<usize>,
    /// The parser diagnostic count at mark time.
    diagnostic_count: Option<usize>,
    /// Optional override span for synthetic marks.
    span_override: Option<Span>,
}

impl ParserMark {
    /// Create a new ParserMark.
    #[inline]
    pub(crate) fn new(
        pos: usize,
        current_token: TokenSpan,
        previous_token_end: u32,
        last_consumed_token: TokenSpan,
        tree_mark: NodeTreeMark,
        lexer_mark: Option<LexerSnapshot>,
        error_count: usize,
        diagnostic_count: usize,
    ) -> Self {
        Self {
            pos,
            current_token,
            previous_token_end,
            last_consumed_token,
            tree_mark: Some(tree_mark),
            lexer_mark,
            error_count: Some(error_count),
            diagnostic_count: Some(diagnostic_count),
            span_override: None,
        }
    }

    /// Create a synthetic mark from a span without capturing lexer state.
    #[inline]
    pub(crate) fn from_span(span: Span) -> Self {
        Self {
            pos: 0,
            current_token: TokenSpan {
                token: Token::end(),
                span,
            },
            previous_token_end: span.start,
            last_consumed_token: TokenSpan {
                token: Token::end(),
                span,
            },
            tree_mark: None,
            lexer_mark: None,
            error_count: None,
            diagnostic_count: None,
            span_override: Some(span),
        }
    }
}
