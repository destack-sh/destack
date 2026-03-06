use core::fmt;
use std::fmt::Debug;
#[cfg(feature = "parser_timings")]
use std::ptr::NonNull;
#[cfg(feature = "parser_timings")]
use std::rc::Rc;
use std::sync::Arc;

use super::expression::lookahead::DelimiterAnalysis;
use crate::{TokenStream, TokenStreamCursor, TokenStreamMark, is_semantic};
use destack_ast::{
    BlockFormat, Expression, Keyword, LocalNodeId, NodeTree, NodeTreeMark, StringId, Token,
    TokenSpan, TokenType,
};
use destack_base::LocalStringPool;
use destack_source::{
    DiagnosticCollector, EnclosingSpan, File, FileId, LanguageType, MultiSpan, NodeSearchMode, Span,
};

use crate::parse::timing::ParserTimingScope;
#[cfg(feature = "parser_timings")]
use crate::parse::timing::ParserTimings;
use crate::{ParseError, ParseResult};

/// Cached string ids for type literal identifiers.
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
#[derive(Debug, Copy, Clone, Default)]
pub struct ParserSettings {
    /// Whether ambiguous tree literal syntax is disallowed.
    pub disallow_ambiguous_tree_literal: bool,
}

/// Counters for speculative parser dispatch and rollback behavior.
#[derive(Debug, Copy, Clone, Default)]
pub struct ParserSpeculationStats {
    /// The number of `with_options` scope switches.
    pub with_options_calls: u64,
    /// The number of parser rewinds.
    pub rewind_calls: u64,
    /// The number of parser restores with tree rollback.
    pub restore_calls: u64,
    /// The number of statement keyword dispatch calls.
    pub statement_keyword_dispatch_calls: u64,
    /// The number of statement keyword dispatch prefilter rejections.
    pub statement_keyword_dispatch_prefilter_rejects: u64,
    /// The number of statement keyword dispatch keyword rejections.
    pub statement_keyword_dispatch_keyword_rejects: u64,
    /// The number of direct statement keyword hits.
    pub statement_keyword_dispatch_direct_hits: u64,
    /// The number of direct statement keyword misses.
    pub statement_keyword_dispatch_direct_misses: u64,
    /// The number of fallback keyword parser hits.
    pub statement_keyword_dispatch_fallback_hits: u64,
    /// The number of fallback keyword parser misses.
    pub statement_keyword_dispatch_fallback_misses: u64,
    /// The number of parenthesized expression plain-path calls.
    pub parenthesized_expression_plain_calls: u64,
    /// The number of parenthesized expression plain-path hits.
    pub parenthesized_expression_plain_hits: u64,
    /// The number of parenthesized expression plain-path misses.
    pub parenthesized_expression_plain_misses: u64,
    /// The number of plain parenthesized lambda calls.
    pub parenthesized_lambda_plain_calls: u64,
    /// The number of plain parenthesized lambda hits.
    pub parenthesized_lambda_plain_hits: u64,
    /// The number of plain parenthesized lambda misses.
    pub parenthesized_lambda_plain_misses: u64,
    /// The number of plain identifier lambda calls.
    pub identifier_lambda_plain_calls: u64,
    /// The number of plain identifier lambda hits.
    pub identifier_lambda_plain_hits: u64,
    /// The number of plain identifier lambda misses.
    pub identifier_lambda_plain_misses: u64,
    /// The number of async keyword speculative attempts.
    pub async_keyword_speculative_attempts: u64,
    /// The number of async keyword speculative successes.
    pub async_keyword_speculative_successes: u64,
    /// The number of async keyword speculative rollbacks.
    pub async_keyword_speculative_rollbacks: u64,
}

#[allow(unused)]
impl ParserOptions {
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

/// A parser for a single Destack source's AST.
///
/// The Parser works on "semantic" undifferentiated Tokens (keywords are just identifiers).
/// Whitespace and regular line comments are completely ignored; newline is significant (see ASI rules).
pub struct Parser {
    /// The source we're parsing.
    pub file: Arc<File>,
    /// The source ID.
    pub file_id: FileId,
    /// The token stream driving the parser.
    pub(crate) token_stream: TokenStream,

    /// Scanner owned parser cursor and lookahead cache state.
    scanner: ParserScannerState,
    /// Whether the parser is finished.
    is_finished: bool,
    /// Expression recursion depth for periodic stack growth checks.
    pub(crate) expression_stack_depth: u32,
    /// Statement recursion depth for periodic stack growth checks.
    pub(crate) statement_stack_depth: u32,
    /// The parser options.
    pub(crate) options: ParserOptions,

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
    #[cfg(feature = "parser_timings")]
    pub(crate) timings: Option<Rc<ParserTimings>>,
    /// Optional speculation and dispatch counter collector.
    pub(crate) speculation_stats: Option<ParserSpeculationStats>,
    /// Cached identifier lookup for identifier tokens.
    pub(crate) token_identifiers: Vec<Option<StringId>>,
    /// Cached-state bits for identifier lookup entries.
    pub(crate) token_identifiers_cached: Vec<bool>,
    /// Cached delimiter analysis metadata by token index.
    pub(crate) delimiter_analyses: Vec<DelimiterAnalysis>,
    /// Cached state bits for delimiter analysis entries.
    pub(crate) delimiter_analyses_cached: Vec<bool>,
    /// Cached identifiers used by type literal parsing.
    pub(crate) type_literal_identifiers: TypeLiteralIdentifiers,
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

/// Scanner lookahead facts at the current parser position.
#[derive(Debug, Copy, Clone)]
pub(crate) struct ScannerLookahead {
    /// The next raw token type after the current parser position.
    pub next_raw_token_type: TokenType,
    /// The normalized non-newline cursor from the next raw index.
    pub next_cursor: NonNewlineTokenCursor,
}

/// Scanner owned cursor and cached lookahead facts.
#[derive(Debug, Copy, Clone)]
struct ParserScannerState {
    /// The current parser cursor index in the semantic token stream.
    pos: usize,
    /// Cached current scanner facts keyed by parser token position.
    current_scanner_cache: Option<(usize, TokenType, NonNewlineTokenCursor)>,
    /// Cached scanner lookahead facts keyed by parser token position.
    scanner_lookahead_cache: Option<(usize, ScannerLookahead)>,
}

impl ParserScannerState {
    /// Create a scanner state at the start of the token stream.
    #[inline]
    fn new() -> Self {
        Self {
            pos: 0,
            current_scanner_cache: None,
            scanner_lookahead_cache: None,
        }
    }

    /// Reset scanner state to the first token.
    #[inline]
    fn reset(&mut self) {
        self.pos = 0;
        self.invalidate_cached_facts();
    }

    /// Return the current parser cursor index.
    #[inline]
    fn pos(&self) -> usize {
        self.pos
    }

    /// Set the parser cursor index and invalidate cached facts.
    #[inline]
    fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
        self.invalidate_cached_facts();
    }

    /// Advance the parser cursor by one token and invalidate cached facts.
    #[inline]
    fn advance_one(&mut self) {
        self.pos += 1;
        self.invalidate_cached_facts();
    }

    /// Advance the parser cursor by `distance` tokens and invalidate cached facts.
    #[inline]
    fn advance_by(&mut self, distance: usize) {
        self.pos += distance;
        self.invalidate_cached_facts();
    }

    /// Return a lookahead index adjusted for split token state.
    #[inline]
    fn lookahead_index(&self, delta: usize, has_active_split: bool) -> usize {
        let offset = if has_active_split {
            delta.saturating_sub(1)
        } else {
            delta
        };
        self.pos + offset
    }

    /// Invalidate cached scanner facts after token stream or cursor mutations.
    #[inline]
    fn invalidate_cached_facts(&mut self) {
        self.current_scanner_cache = None;
        self.scanner_lookahead_cache = None;
    }
}

impl Debug for Parser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parser")
    }
}

impl Parser {
    /// Create a new parser from a text File and tokenize it.
    #[tracing::instrument(name = "parser.lex", level = "trace", skip_all, fields(file_id = ?file.id))]
    pub fn lex_file(file: Arc<File>, language: LanguageType) -> Self {
        // initialize token stream for lazy lexing
        let token_stream = TokenStream::new(file.clone(), language);

        // make parser with estimated capacity
        // roughly 1 AST node per 3 tokens on average
        let estimated_tokens = file.text().len() / 6;
        let estimated_nodes = estimated_tokens / 3;
        let file_id = file.id;
        let mut strings = LocalStringPool::new();
        let type_literal_identifiers = TypeLiteralIdentifiers::new(&mut strings);
        let mut parser = Self {
            file,
            file_id,
            token_stream,
            scanner: ParserScannerState::new(),
            is_finished: false,
            expression_stack_depth: 0,
            statement_stack_depth: 0,
            options: ParserOptions::default(),
            language,
            tree: NodeTree::with_capacity(estimated_nodes),
            strings,
            diagnostics: DiagnosticCollector::new(),
            errors: Vec::new(),
            #[cfg(feature = "parser_timings")]
            timings: parser_timings_from_env(),
            speculation_stats: speculation_stats_enabled_from_env()
                .then(ParserSpeculationStats::default),
            token_identifiers: Vec::with_capacity(estimated_tokens),
            token_identifiers_cached: Vec::with_capacity(estimated_tokens),
            delimiter_analyses: Vec::with_capacity(estimated_tokens),
            delimiter_analyses_cached: Vec::with_capacity(estimated_tokens),
            type_literal_identifiers,
        };

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
        let mut parser = Self::lex_file(file, language);
        parser.apply_settings(settings);
        parser
    }

    /// Apply externally provided parser settings.
    #[inline]
    pub fn apply_settings(&mut self, settings: ParserSettings) {
        self.options
            .set_disallow_ambiguous_tree_literal(settings.disallow_ambiguous_tree_literal);
    }

    /// Get the span of all side annotations.
    #[inline]
    pub fn compute_side_span(&self) -> MultiSpan {
        Self::compute_side_span_from_tree(&self.tree)
    }

    /// Get the span of all side annotations from a tree.
    #[inline]
    pub fn compute_side_span_from_tree(tree: &NodeTree) -> MultiSpan {
        MultiSpan::new(tree.get_side_annotation_spans())
    }

    /// Reset the parser.
    pub(crate) fn reset(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");
        self.scanner.reset();
        self.token_stream.clear_split_token();
        self.expression_stack_depth = 0;
        self.statement_stack_depth = 0;
        let mut options = ParserOptions::default();
        options.set_disallow_ambiguous_tree_literal(
            self.language.supports_jsx() && self.language.is_typescript(),
        );
        self.options = options;
        self.errors.clear();
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            *speculation_stats = ParserSpeculationStats::default();
        }
    }

    /// Start a parser timing scope.
    pub(crate) fn timing_scope(
        &self,
        tag: crate::parse::timing::ParserTimingTag,
    ) -> ParserTimingScope {
        #[cfg(not(feature = "parser_timings"))]
        {
            let _ = tag;
            ParserTimingScope::disabled()
        }

        #[cfg(feature = "parser_timings")]
        {
            let Some(timings) = self.timings.as_ref() else {
                return ParserTimingScope::disabled();
            };
            let timings_ptr = NonNull::from(timings.as_ref());
            ParserTimingScope::new(Some(timings_ptr), tag)
        }
    }

    /// Snapshot timing entries recorded by the parser.
    pub fn timing_snapshot(&self) -> Option<Vec<crate::parse::timing::ParserTimingEntry>> {
        #[cfg(not(feature = "parser_timings"))]
        {
            None
        }

        #[cfg(feature = "parser_timings")]
        {
            self.timings.as_ref().map(|timings| timings.snapshot())
        }
    }

    /// Snapshot speculative parser dispatch counters.
    pub fn speculation_snapshot(&self) -> Option<ParserSpeculationStats> {
        self.speculation_stats
    }

    /// Return the current semantic tokens.
    #[inline]
    pub(crate) fn tokens(&self) -> &[TokenSpan] {
        self.token_stream.tokens()
    }

    /// Ensure a token exists at the given index.
    #[inline]
    pub(crate) fn ensure_token(&mut self, index: usize) {
        self.token_stream.ensure_token(index);
    }

    /// Return true when tree literal lexing is enabled.
    #[inline]
    pub(crate) fn allow_tree_literals(&self) -> bool {
        self.token_stream.allow_tree_literals()
    }

    /// Set whether tree literal lexing is enabled.
    #[inline]
    pub(crate) fn set_allow_tree_literals(&mut self, allow: bool) {
        self.token_stream.set_allow_tree_literals(allow);
        self.invalidate_scanner_lookahead_cache();
    }

    /// Return true when lexing is currently inside a tree literal.
    #[inline]
    pub(crate) fn in_tree_literal(&self) -> bool {
        self.token_stream.in_tree_literal()
    }

    /// Return true when lexing is inside any tree expression container.
    #[inline]
    pub(crate) fn in_tree_expression_container(&self) -> bool {
        self.token_stream.in_tree_expression_container()
    }

    /// Enter tree opening tag lex mode.
    #[inline]
    pub(crate) fn enter_tree_opening_tag(&mut self) {
        self.token_stream.enter_tree_opening_tag();
        self.invalidate_scanner_lookahead_cache();
    }

    /// Return the next non newline token index from a start index.
    #[inline]
    pub(crate) fn next_non_newline_index_from_stream(&mut self, start: usize) -> usize {
        self.token_stream.next_non_newline_index_from(start)
    }

    /// Return the first non-newline token index from a start index.
    #[inline]
    pub(crate) fn first_non_newline_index_from(&mut self, start: usize) -> usize {
        // split tokens only exist in parser state, so keep parser indexed lookups here
        if self.has_active_split() {
            if self.token_type_at(start) != TokenType::Newline {
                return start;
            }
            return self.next_non_newline_index_from_stream(start);
        }

        self.next_non_newline_index_from_stream(start)
    }

    /// Return cursor information for the first non-newline token from a start index.
    #[inline]
    pub(crate) fn scanner_cursor_from(&mut self, start: usize) -> NonNewlineTokenCursor {
        // split tokens are parser local and do not exist in token stream cursors
        if self.has_active_split() {
            let index = self.first_non_newline_index_from(start);
            let token_type = self.token_type_at(index);
            let skipped_newline_count = index.saturating_sub(start);
            let has_line_break_before = if skipped_newline_count > 0 {
                true
            } else {
                self.line_terminator_before_index(index)
            };
            return NonNewlineTokenCursor {
                index,
                token_type,
                skipped_newline_count,
                has_line_break_before,
            };
        }

        let TokenStreamCursor {
            index,
            token_type,
            skipped_newline_count,
            has_line_break_before,
        } = self.token_stream.scanner_cursor_from(start);

        NonNewlineTokenCursor {
            index,
            token_type,
            skipped_newline_count,
            has_line_break_before,
        }
    }

    /// Return scanner-style cursor information at the current parser position.
    #[inline]
    pub(crate) fn current_scanner_cursor(&mut self) -> NonNewlineTokenCursor {
        self.peek_scanner_facts().1
    }

    /// Advance to the current scanner cursor and return it.
    #[inline]
    pub(crate) fn advance_to_scanner_cursor(&mut self) -> NonNewlineTokenCursor {
        let cursor = self.current_scanner_cursor();
        if cursor.index != self.scanner.pos() {
            self.advance_to(cursor.index);
        }
        cursor
    }

    /// Return the matching pair index for an opening token index, lexing ahead if needed.
    #[inline]
    pub(crate) fn matching_pair_or_lex(&mut self, index: usize) -> Option<usize> {
        // fully materialized streams can serve pair lookups without incremental lex checks
        if self.token_stream.is_lexed_to_end() {
            return self.token_stream.matching_pair(index);
        }

        // tree literal lexing needs parser driven mode switches before aggressive lookahead
        if self.allow_tree_literals() && !self.options.is_in_type() {
            return self.token_stream.matching_pair(index);
        }

        self.token_stream.matching_pair_or_lex(index)
    }

    /// Return owned token buffers after lexing to EOF.
    pub fn take_tokens(&mut self) -> (Vec<TokenSpan>, Vec<TokenSpan>) {
        self.token_stream.take_tokens()
    }

    /// Return the EOF span without forcing a full lex.
    #[inline]
    pub(crate) fn eof_span(&self) -> Span {
        Span::new(self.file_id, self.file.len, self.file.len)
    }

    /// Ensure a token exists at the given index and return it.
    #[inline]
    pub(crate) fn token_at(&mut self, index: usize) -> Option<TokenSpan> {
        if let Some(token) = self.tokens().get(index).copied() {
            return Some(token);
        }

        self.token_stream.token(index)
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

    /// Truncate token caches to match the current token count.
    fn truncate_token_caches(&mut self) {
        let len = self.tokens().len();
        self.token_identifiers.truncate(len);
        self.token_identifiers_cached.truncate(len);
        self.delimiter_analyses.truncate(len);
        self.delimiter_analyses_cached.truncate(len);
    }

    /// Ensure identifier caches can index at least `index`.
    #[inline]
    pub(crate) fn ensure_identifier_cache_capacity(&mut self, index: usize) {
        if self.token_identifiers.len() > index {
            return;
        }

        let required_len = index + 1;
        let grown_len = self
            .token_identifiers
            .len()
            .saturating_add(self.token_identifiers.len() / 2)
            .saturating_add(64);
        let new_len = required_len.max(grown_len);
        self.token_identifiers.resize(new_len, None);
        self.token_identifiers_cached.resize(new_len, false);
    }

    /// Ensure delimiter analysis caches can index at least `index`.
    #[inline]
    pub(crate) fn ensure_delimiter_analysis_cache_capacity(&mut self, index: usize) {
        if self.delimiter_analyses.len() > index {
            return;
        }

        let required_len = index + 1;
        let grown_len = self
            .delimiter_analyses
            .len()
            .saturating_add(self.delimiter_analyses.len() / 2)
            .saturating_add(64);
        let new_len = required_len.max(grown_len);
        self.delimiter_analyses
            .resize(new_len, DelimiterAnalysis::default());
        self.delimiter_analyses_cached.resize(new_len, false);
    }

    /// Invalidate cached scanner lookahead after parser state mutations.
    #[inline]
    fn invalidate_scanner_lookahead_cache(&mut self) {
        self.scanner.invalidate_cached_facts();
    }

    /// Return current scanner token facts for the parser position.
    #[inline]
    fn peek_scanner_facts(&mut self) -> (TokenType, NonNewlineTokenCursor) {
        let pos = self.scanner.pos();
        if let Some((cached_pos, token_type, cursor)) = self.scanner.current_scanner_cache
            && cached_pos == pos
        {
            return (token_type, cursor);
        }

        let has_active_split = self.has_active_split();
        let current_raw_token_type = if has_active_split {
            self.token_stream
                .active_split_token()
                .expect("active split token should exist")
                .token
                .ty
        } else {
            self.token_type_at(pos)
        };
        let current_cursor = if has_active_split {
            NonNewlineTokenCursor {
                index: pos,
                token_type: current_raw_token_type,
                skipped_newline_count: 0,
                has_line_break_before: self.line_terminator_before_index(pos),
            }
        } else {
            self.scanner_cursor_from(pos)
        };

        self.scanner.current_scanner_cache = Some((pos, current_raw_token_type, current_cursor));
        (current_raw_token_type, current_cursor)
    }

    /// Return true when a split token is active.
    #[inline]
    pub(crate) fn has_active_split(&self) -> bool {
        self.token_stream.has_active_split()
    }

    /// Get the current token index.
    #[inline]
    pub(crate) fn pos_index(&self) -> usize {
        self.scanner.pos()
    }

    /// Look up a keyword at a token index.
    #[inline]
    pub(crate) fn keyword_for_index(&mut self, index: usize) -> Option<Keyword> {
        if self.token_stream.is_lexed_to_end() && index < self.tokens().len() {
            return self.token_stream.keyword_at_cached(index);
        }

        self.token_stream.keyword_at(index)
    }

    /// Return whether an identifier token contains escape syntax.
    #[inline]
    pub(crate) fn identifier_has_escape_for_index(&mut self, index: usize) -> bool {
        if self.token_stream.is_lexed_to_end() && index < self.tokens().len() {
            return self.token_stream.identifier_has_escape_cached(index);
        }

        self.token_stream.identifier_has_escape(index)
    }

    /// Look up a pre interned identifier at a token index.
    #[inline]
    pub(crate) fn identifier_for_index(&mut self, index: usize) -> Option<StringId> {
        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        self.ensure_identifier_cache_capacity(index);

        if self.token_identifiers_cached[index] {
            return self.token_identifiers[index];
        }

        let token = self.tokens().get(index)?;
        let identifier = if token.token.ty == TokenType::Identifier {
            let span_str = self.file.span_str(token.span);
            Some(self.strings.intern(span_str))
        } else {
            None
        };
        self.token_identifiers[index] = identifier;
        self.token_identifiers_cached[index] = true;
        identifier
    }

    /// Return true when the identifier token at index matches the expected string.
    #[inline]
    pub(crate) fn identifier_equals_at(&mut self, index: usize, expected: &str) -> bool {
        if index >= self.tokens().len() {
            self.ensure_token(index);
        }

        self.tokens().get(index).is_some_and(|token| {
            token.token.ty == TokenType::Identifier && self.file.span_str(token.span) == expected
        })
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

    /// Return whether trivia before the token at index contains a line terminator.
    #[inline]
    pub(crate) fn line_terminator_before_index(&mut self, index: usize) -> bool {
        self.token_stream.line_terminator_before(index)
    }

    /// Return a lookahead index adjusted for an active split token.
    #[inline]
    fn lookahead_index(&self, delta: usize) -> usize {
        self.scanner.lookahead_index(delta, self.has_active_split())
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

    /// Return scanner lookahead facts for the current parser position.
    #[inline]
    pub(crate) fn peek_scanner_lookahead(&mut self) -> ScannerLookahead {
        let pos = self.scanner.pos();
        if let Some((cached_pos, scanner_lookahead)) = self.scanner.scanner_lookahead_cache
            && cached_pos == pos
        {
            return scanner_lookahead;
        }

        let next_raw_index = self.index_for_next();
        let next_raw_token_type = self.token_type_at(next_raw_index);
        let next_cursor = self.scanner_cursor_from(next_raw_index);
        let scanner_lookahead = ScannerLookahead {
            next_raw_token_type,
            next_cursor,
        };

        self.scanner.scanner_lookahead_cache = Some((pos, scanner_lookahead));
        scanner_lookahead
    }

    /// Ensure token caches align with the current token stream after a rewind.
    fn reset_token_caches_after_rewind(&mut self) {
        self.truncate_token_caches();
    }

    /// Parse everything as an implicit namespace with optional trivia attachment.
    fn parse_root_expressions(&mut self, attach_trivia: bool) -> Vec<LocalNodeId<Expression>> {
        // pre lex all tokens for non tree literal mode to keep parse hot paths token driven
        self.prelex_all_tokens_maybe();

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

        // attach trivia only in the full parse pipeline
        if attach_trivia {
            self.attach_trivia();
            self.is_finished = true;
        }

        expressions
    }

    /// Parse everything as an implicit namespace.
    #[tracing::instrument(name = "parser.parse", level = "trace", skip_all, fields(file_id = ?self.file_id))]
    pub fn parse(&mut self) -> Vec<LocalNodeId<Expression>> {
        self.parse_root_expressions(true)
    }

    /// Parse everything as an implicit namespace without attaching trivia.
    pub fn parse_without_trivia(&mut self) -> Vec<LocalNodeId<Expression>> {
        self.parse_root_expressions(false)
    }

    /// Materialize the full semantic token stream for non jsx files.
    #[inline]
    fn prelex_all_tokens_maybe(&mut self) {
        if self.allow_tree_literals() || !self.tokens().is_empty() {
            return;
        }

        self.token_stream.lex_to_end();
    }

    /// Ensure one stable owner for comment and blank trivia in trivia only files.
    fn ensure_trivia_anchor_maybe(
        &mut self,
        expressions: &mut Vec<LocalNodeId<Expression>>,
        consumed_to_end: bool,
    ) {
        // most files already have parsed body expressions and never need a trivia anchor
        if !consumed_to_end && !expressions.is_empty() {
            return;
        }

        // materialize the full stream before trivia ownership checks
        self.token_stream.lex_to_end();

        // skip files without trivia tokens
        if !self.token_stream.has_comment_trivia_tokens()
            && !self.token_stream.has_blank_trivia_tokens()
        {
            return;
        }

        let stub_span = Span::new(self.file_id, 0, self.file.len);

        // comment only files need one returned expression owner
        if expressions.is_empty() {
            let stub = self.tree.insert(Expression::Stub, stub_span);
            expressions.push(stub);
            return;
        }

        // only directive only files need an internal trivia owner
        if !consumed_to_end {
            return;
        }

        // directive-only files with attachable semantic tokens already have stable owners
        if self.token_stream.has_attachable_semantic_tokens() {
            return;
        }

        // insert one internal anchor so trivia can attach without parse errors
        let _ = self.tree.insert(Expression::Stub, stub_span);
    }

    /// Get the current position in the tokens.
    #[inline]
    pub fn pos(&self) -> u32 {
        self.scanner.pos() as u32
    }

    /// Attach trivia after parsing when needed.
    pub fn attach_trivia(&mut self) {
        // materialize the stream so trivia presence flags are complete
        self.token_stream.lex_to_end();

        if !self.token_stream.has_comment_trivia_tokens()
            && !self.token_stream.has_blank_trivia_tokens()
        {
            return;
        }

        let _timing = self.timing_scope(crate::parse::timing::tags::PARSE_ANNOTATIONS_MAIN);
        self.attach_trivia_annotations();
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

        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.with_options_calls += 1;
        }

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
        // snapshot token stream when tree state or split state can affect lookahead
        let should_snapshot_token_stream = (self.allow_tree_literals()
            && !self.options.is_in_type())
            || self.token_stream.has_split_state();
        let token_stream_mark = should_snapshot_token_stream.then(|| self.token_stream.mark());
        ParserMark::new(
            self.scanner.pos(),
            self.tree.mark(),
            token_stream_mark,
            self.errors.len(),
            self.diagnostics.len(),
        )
    }

    /// Get a mark for token rewinds without tree allocation snapshots.
    #[inline(always)]
    pub fn mark_rewind(&self) -> ParserMark {
        // snapshot token stream when tree state or split state can affect lookahead
        let should_snapshot_token_stream = (self.allow_tree_literals()
            && !self.options.is_in_type())
            || self.token_stream.has_split_state();
        let token_stream_mark = should_snapshot_token_stream.then(|| self.token_stream.mark());
        ParserMark {
            pos: self.scanner.pos(),
            tree_mark: None,
            token_stream_mark,
            error_count: None,
            diagnostic_count: None,
            span_override: None,
        }
    }

    /// Get a lightweight mark used for span calculations without rewind support.
    #[inline(always)]
    pub fn mark_span(&self) -> ParserMark {
        ParserMark {
            pos: self.scanner.pos(),
            tree_mark: None,
            token_stream_mark: None,
            error_count: None,
            diagnostic_count: None,
            span_override: None,
        }
    }

    /// Run a closure at a temporary token position and restore parser state afterward.
    pub(crate) fn with_pos<T>(&mut self, pos: usize, func: impl FnOnce(&mut Self) -> T) -> T {
        let mark = self.mark_rewind();
        self.scanner.set_pos(pos);
        let result = func(self);
        self.rewind(mark);
        result
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn rewind(&mut self, mark: ParserMark) {
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.rewind_calls += 1;
        }
        self.scanner.set_pos(mark.pos);
        if let Some(token_stream_mark) = mark.token_stream_mark {
            self.token_stream.restore(token_stream_mark);
            self.reset_token_caches_after_rewind();
        }
    }

    /// Rewind the position to the given mark and remove any nodes created since.
    pub fn restore(&mut self, mark: ParserMark, idx: u32) {
        if let Some(speculation_stats) = self.speculation_stats.as_mut() {
            speculation_stats.restore_calls += 1;
        }
        self.scanner.set_pos(mark.pos);
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
        if let Some(token_stream_mark) = mark.token_stream_mark {
            self.token_stream.restore(token_stream_mark);
            self.reset_token_caches_after_rewind();
        }
    }

    /// Wrap an expression in a statement expression.
    #[inline]
    pub(crate) fn wrap_statement_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        span: Span,
    ) -> LocalNodeId<Expression> {
        // wrap expression as statement first so annotation remap has a stable target
        let statement_id = self.tree.insert(Expression::Statement(expression_id), span);

        // semantic annotations on statement expressions must follow the statement owner
        let _moved = self.tree.move_annotations_if(
            expression_id.id,
            statement_id.id,
            |_annotation_id, _annotation, _annotation_span| true,
        );

        statement_id
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

        let pos = self.scanner.pos();
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
        let pos = self.scanner.pos();
        if pos > 0 {
            self.tokens().get(pos - 1)
        } else {
            None
        }
    }

    /// Get the previous Token type.
    #[inline]
    pub fn prev_token_type(&self) -> TokenType {
        self.prev()
            .map(|token| token.token.ty)
            .unwrap_or(TokenType::End)
    }

    /// Peek the next Token or error.
    #[inline]
    pub fn peek(&mut self) -> ParseResult<&TokenSpan> {
        // return split token if present and not yet consumed
        if self.has_active_split() {
            return Ok(self
                .token_stream
                .active_split_token()
                .expect("active split token should exist"));
        }

        if self.token_stream.has_split_state() {
            self.token_stream.clear_split_token();
            self.invalidate_scanner_lookahead_cache();
        }

        let pos = self.scanner.pos();
        if pos >= self.tokens().len() {
            self.ensure_token(pos);
        }

        self.tokens()
            .get(pos)
            .ok_or(ParseError::unexpected(self.eof_span()))
    }

    /// Peek the next token type, defaulting to End at EOF.
    #[inline]
    pub fn peek_token_type(&mut self) -> TokenType {
        self.peek_scanner_facts().0
    }

    /// Peek the next token type, skipping an active split token.
    #[inline]
    pub fn peek_next_token_type(&mut self) -> TokenType {
        self.peek_scanner_lookahead().next_raw_token_type
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
        // if there's an active split token, consume it and return reference
        if self.token_stream.mark_split_token_consumed() {
            self.scanner.invalidate_cached_facts();
            return Ok(self
                .token_stream
                .split_token_ref()
                .expect("active split token should exist"));
        }

        // clear stale split state before consuming regular tokens
        if self.token_stream.has_split_state() {
            self.token_stream.clear_split_token();
        }

        // normal case: consume from token stream
        let pos = self.scanner.pos();
        if pos >= self.tokens().len() {
            self.ensure_token(pos);
        }

        if pos < self.tokens().len() {
            self.scanner.advance_one();
            self.tokens()
                .get(pos)
                .ok_or(ParseError::unexpected(self.eof_span()))
        } else {
            Err(ParseError::unexpected(self.eof_span()))
        }
    }

    /// Bump the Token position.
    #[inline]
    pub fn bump(&mut self) {
        debug_assert!(!self.is_finished, "parser is already finished");

        // if there's an active split token, mark it as consumed instead of advancing
        if self.has_active_split() {
            self.token_stream.mark_split_token_consumed();
            self.scanner.invalidate_cached_facts();
            return;
        }

        // clear stale split state before advancing regular tokens
        if self.token_stream.has_split_state() {
            self.token_stream.clear_split_token();
        }

        let pos = self.scanner.pos();
        if pos >= self.tokens().len() {
            self.ensure_token(pos);
        }

        debug_assert!(pos < self.tokens().len(), "bump past end of tokens");
        self.scanner.advance_one();
    }

    /// Bump the Token position by a given distance.
    #[inline]
    pub fn bump_by(&mut self, distance: u8) {
        debug_assert!(!self.is_finished, "parser is already finished");

        if self.token_stream.has_split_state() {
            self.token_stream.clear_split_token();
        }

        let next_pos = self.scanner.pos() + (distance as usize);
        if next_pos >= self.tokens().len() {
            self.ensure_token(next_pos);
        }

        debug_assert!(next_pos < self.tokens().len(), "bump past end of tokens");
        self.scanner.advance_by(distance as usize);
    }

    /// Advance the token position to a specific index.
    #[inline]
    pub(crate) fn advance_to(&mut self, pos: usize) {
        debug_assert!(!self.is_finished, "parser is already finished");

        if self.token_stream.has_split_state() {
            self.token_stream.clear_split_token();
        }

        if pos >= self.tokens().len() {
            self.ensure_token(pos);
        }

        debug_assert!(pos <= self.tokens().len(), "advance past end of tokens");
        self.scanner.set_pos(pos);
    }

    /// Split a `<<` (ShiftLeft) token into two `<` tokens.
    /// Consumes the ShiftLeft and stores a synthetic `<` as the pending split token.
    /// Used when `<<` needs to become `<` + `<` in generic contexts like `Extends<<T>()...>`.
    pub fn split_shift_left(&mut self) {
        let pos = self.scanner.pos();
        self.ensure_token(pos);
        let current = &self.tokens()[pos];
        debug_assert_eq!(
            current.token.ty,
            TokenType::ShiftLeft,
            "split_shift_left called on non-ShiftLeft token"
        );

        // span for the second `<` (offset by 1 character)
        let second_span = Span {
            file: current.span.file,
            start: current.span.start + 1,
            end: current.span.end,
        };

        // store synthetic `<` token as pending
        self.token_stream.set_split_token(TokenSpan {
            token: Token {
                ty: TokenType::LessThan,
                len: 1,
                literal: None,
            },
            span: second_span,
        });

        // advance past the ShiftLeft token
        self.scanner.advance_one();
    }

    /// Eat a single `>` token in generic close contexts.
    ///
    /// This handles glued operator tails like `>=`, `>>=`, and `>>>=`
    /// by consuming one `>` and leaving the remainder as a pending split token.
    pub fn eat_type_angle_close(&mut self) -> ParseResult<()> {
        if self.peek_is(TokenType::GreaterThan) {
            self.bump();
            return Ok(());
        }

        let token = *self.peek()?;
        let split_type = match token.token.ty {
            TokenType::GreaterThanOrEqual => TokenType::Assign,
            TokenType::ShiftRightAssign => TokenType::GreaterThanOrEqual,
            TokenType::UnsignedShiftRightAssign => TokenType::ShiftRightAssign,
            _ => return Err(ParseError::unexpected(token.span)),
        };

        let split_span = Span {
            file: token.span.file,
            start: token.span.start + 1,
            end: token.span.end,
        };

        self.token_stream.set_split_token(TokenSpan {
            token: Token {
                ty: split_type,
                len: token.token.len.saturating_sub(1),
                literal: None,
            },
            span: split_span,
        });
        self.scanner.advance_one();
        Ok(())
    }

    /// Check if there's an active (unconsumed) split token of the given type.
    #[inline]
    pub fn has_split_token(&self, token_type: TokenType) -> bool {
        self.token_stream.has_split_token(token_type)
    }

    /// Peek a token at a position.
    #[inline]
    pub fn peek_token_ahead(
        &mut self,
        delta: u32,
        token_type: TokenType,
    ) -> ParseResult<&TokenSpan> {
        let index = self.scanner.pos() + (delta as usize);
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

#[cfg(feature = "parser_timings")]
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

#[cfg(feature = "parser_timings")]
fn parser_timings_from_env() -> Option<Rc<ParserTimings>> {
    timings_enabled_from_env().then(|| Rc::new(ParserTimings::default()))
}

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
#[derive(Debug, Clone)]
pub struct ParserMark {
    /// The token position.
    pos: usize,
    /// Tree allocation snapshot at mark time.
    tree_mark: Option<NodeTreeMark>,
    /// The token stream mark for speculative parsing.
    token_stream_mark: Option<TokenStreamMark>,
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
        tree_mark: NodeTreeMark,
        token_stream_mark: Option<TokenStreamMark>,
        error_count: usize,
        diagnostic_count: usize,
    ) -> Self {
        Self {
            pos,
            tree_mark: Some(tree_mark),
            token_stream_mark,
            error_count: Some(error_count),
            diagnostic_count: Some(diagnostic_count),
            span_override: None,
        }
    }

    /// Create a synthetic mark from a span without capturing token stream state.
    #[inline]
    pub(crate) fn from_span(span: Span) -> Self {
        Self {
            pos: 0,
            tree_mark: None,
            token_stream_mark: None,
            error_count: None,
            diagnostic_count: None,
            span_override: Some(span),
        }
    }
}
