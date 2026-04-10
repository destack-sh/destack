use std::collections::{HashMap, HashSet};

use destack_core::{ImmutableStringPool, StringPool};
use destack_source::{
    DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, FileId, NodeSpanType, Span,
};

use crate::validate::Validator;
use crate::{
    Block, Field, Function, Global, LocalNodeId, Node, NodeTree, Type, Value,
    finalize_function_names,
};

use super::error::{ParseError, ParseResult};
use super::key::{FieldKey, TypeKey};
use super::lexer::Lexer;
use super::token::{Token, TokenType};

/// Options for the MIR parser.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Pointer size in bytes (4 for 32-bit, 8 for 64-bit).
    /// Used for computing struct field offsets.
    pub pointer_bytes: u8,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self { pointer_bytes: 8 }
    }
}

/// Parser for MIR text format.
#[derive(Debug)]
pub struct Parser<'a> {
    /// The tokens to parse.
    pub(super) tokens: Vec<Token<'a>>,
    /// Current position in the tokens.
    pub(super) pos: usize,
    /// The node tree being built.
    pub(super) tree: NodeTree,
    /// The string pool.
    pub(super) strings: StringPool,
    /// The source file id for spans.
    pub(super) file_id: FileId,
    /// The diagnostics produced while parsing.
    pub(super) diagnostics: DiagnosticCollector,
    /// Map from function names to their ids (for forward references).
    pub(super) function_map: HashMap<String, LocalNodeId<Function>>,
    /// Map from global names to their ids (for forward references).
    pub(super) global_map: HashMap<String, LocalNodeId<Global>>,
    /// Map from type alias names to their ids (for references).
    pub(super) type_alias_map: HashMap<String, LocalNodeId<Type>>,
    /// Set of type aliases that have been defined.
    pub(super) type_alias_definitions: HashSet<String>,
    /// Map from symbolic block names to their predeclared block ids.
    pub(super) block_name_map: HashMap<String, LocalNodeId<Block>>,
    /// Map from explicit numeric block labels to their predeclared block ids.
    pub(super) block_id_by_label_index: HashMap<u32, LocalNodeId<Block>>,
    /// Blocks predeclared for the current function body in source order.
    pub(super) predeclared_blocks: Vec<LocalNodeId<Block>>,
    /// Map from symbolic value names to their SSA ids.
    pub(super) value_name_map: HashMap<String, Value>,
    /// Type interner for canonical type ids.
    pub(super) type_intern: HashMap<TypeKey, LocalNodeId<Type>>,
    /// Field interner for canonical field ids.
    pub(super) field_intern: HashMap<FieldKey, LocalNodeId<Field>>,
    /// The function currently being parsed.
    pub(super) current_function: Option<LocalNodeId<Function>>,
    /// The next SSA value id for the current function.
    pub(super) next_value_id: u32,
    /// The number of blocks parsed in the current function so far.
    pub(super) parsed_block_count: usize,
}

#[allow(clippy::type_complexity)]
impl<'a> Parser<'a> {
    /// Create a new parser for a specific file.
    pub fn new(file_id: FileId, source: &'a str, options: ParseOptions) -> Self {
        let tokens = Lexer::lex(source);
        let mut tree = NodeTree::new();
        tree.set_pointer_bytes(options.pointer_bytes);
        Self {
            tokens,
            pos: 0,
            tree,
            strings: StringPool::new(),
            file_id,
            diagnostics: DiagnosticCollector::new(),
            function_map: HashMap::new(),
            global_map: HashMap::new(),
            type_alias_map: HashMap::new(),
            type_alias_definitions: HashSet::new(),
            block_name_map: HashMap::new(),
            block_id_by_label_index: HashMap::new(),
            predeclared_blocks: Vec::new(),
            value_name_map: HashMap::new(),
            type_intern: HashMap::new(),
            field_intern: HashMap::new(),
            current_function: None,
            next_value_id: 0,
            parsed_block_count: 0,
        }
    }

    /// Parse MIR text into a NodeTree and string pool.
    pub fn parse(
        file_id: FileId,
        source: &str,
        options: ParseOptions,
    ) -> ParseResult<(NodeTree, ImmutableStringPool)> {
        let (tree, strings, diagnostics) = Self::parse_recovering(file_id, source, options);

        // fail strictly when parse diagnostics were emitted
        if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            let diagnostics = diagnostics.iter();
            let diagnostic = diagnostics
                .first()
                .expect("error diagnostics must contain at least one entry");
            return Err(ParseError::from_diagnostic(diagnostic));
        }

        // validate the finished tree
        let validator = Validator::new(&tree);
        validator.validate().map_err(|error| {
            let position = error
                .anchor()
                .and_then(|anchor| tree.get_span_by_id(anchor.node.id))
                .map(|span| span.start as usize)
                .unwrap_or(0);
            ParseError::new(error.to_string(), position)
        })?;

        Ok((tree, strings))
    }

    /// Parse MIR text into partial MIR plus shared diagnostics.
    pub fn parse_recovering(
        file_id: FileId,
        source: &str,
        options: ParseOptions,
    ) -> (NodeTree, ImmutableStringPool, DiagnosticCollection) {
        let mut parser = Parser::new(file_id, source, options);
        parser.parse_module_recovering();

        parser.finalize_generated_names();

        let diagnostics = parser.diagnostics.take_collection();
        let strings = parser.strings.into_immutable();

        (parser.tree, strings, diagnostics)
    }

    /// Finalize generated names for every parsed function.
    fn finalize_generated_names(&mut self) {
        let function_ids: Vec<_> = self
            .tree
            .iter_nodes::<Function>()
            .map(|(function_id, _)| function_id)
            .collect();

        for function_id in function_ids {
            finalize_function_names(&mut self.tree, &mut self.strings, function_id);
        }
    }

    /// Get current position for error reporting.
    pub(super) fn pos(&self) -> usize {
        self.peek().map(|t| t.start).unwrap_or(0)
    }

    /// Build a span for a source slice.
    pub(super) fn span_at(&self, start: usize, length: usize) -> Span {
        let start = u32::try_from(start).unwrap_or(u32::MAX);
        let length = u32::try_from(length).unwrap_or(0);
        Span::at(self.file_id, start, length)
    }

    /// Build a span for one parsed range.
    pub(super) fn span_between(&self, start: usize, end: usize) -> Span {
        self.span_at(start, end.saturating_sub(start))
    }

    /// Build a span from one parse start to the last consumed token.
    pub(super) fn span_from_parse_start(&self, start: usize) -> Span {
        let end = self.last_consumed_token_end().unwrap_or(start);
        self.span_between(start, end)
    }

    /// Build a span for a token.
    pub(super) fn span_for_token(&self, token: &Token<'_>) -> Span {
        self.span_at(token.start, token.text.len())
    }

    /// Peek the current token (skipping trivia).
    pub(super) fn peek(&self) -> Option<&Token<'a>> {
        let mut pos = self.pos;
        while pos < self.tokens.len() {
            let token = &self.tokens[pos];
            if !token.ty.is_trivia() {
                return Some(token);
            }
            pos += 1;
        }
        None
    }

    /// Peek the Nth non-trivia token, where 0 is the current token.
    pub(super) fn peek_nth_token(&self, n: usize) -> Option<&Token<'a>> {
        let mut pos = self.pos;
        let mut seen = 0usize;
        while pos < self.tokens.len() {
            let token = &self.tokens[pos];
            if !token.ty.is_trivia() {
                if seen == n {
                    return Some(token);
                }
                seen += 1;
            }
            pos += 1;
        }
        None
    }

    /// Advance past the current token.
    pub(super) fn bump(&mut self) {
        while self.pos < self.tokens.len() {
            let is_trivia = self.tokens[self.pos].ty.is_trivia();
            self.pos += 1;
            if !is_trivia {
                break;
            }
        }
        // skip trailing trivia
        while self.pos < self.tokens.len() && self.tokens[self.pos].ty.is_trivia() {
            self.pos += 1;
        }
    }

    /// Check if current token matches the given type.
    pub(super) fn peek_token(&self, ty: TokenType) -> bool {
        self.peek().is_some_and(|t| t.ty == ty)
    }

    /// Consume a token of the given type, or return an error.
    pub(super) fn eat_token(&mut self, ty: TokenType) -> ParseResult<&Token<'a>> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end(&format!("{ty:?}"), self.pos()))?;
        if token.ty != ty {
            return Err(ParseError::unexpected_token(&format!("{ty:?}"), token));
        }
        // return current token, then advance
        let pos = self.pos;
        self.bump();
        // find the token we just consumed
        for i in pos..self.pos {
            if !self.tokens[i].ty.is_trivia() {
                return Ok(&self.tokens[i]);
            }
        }
        Ok(&self.tokens[pos])
    }

    /// Consume a token if it matches, returning true if consumed.
    pub(super) fn eat_token_maybe(&mut self, ty: TokenType) -> bool {
        if self.peek_token(ty) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Parse an instruction opcode.
    ///
    /// Opcodes can be identifiers or reserved opcode keywords that also have
    /// dedicated token kinds.
    pub(super) fn eat_opcode(&mut self) -> ParseResult<(&'a str, usize)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;

        match token.ty {
            TokenType::Identifier
            | TokenType::Const
            | TokenType::Struct
            | TokenType::Call
            | TokenType::CallIndirect
            | TokenType::CallVirtual
            | TokenType::CallInterface => {
                let text = token.text;
                let start = token.start;
                self.bump();
                Ok((text, start))
            }
            _ => Err(ParseError::unexpected_token("opcode", token)),
        }
    }

    /// Get the text of the current token.
    pub(super) fn span_str(&self) -> &'a str {
        self.peek().map(|t| t.text).unwrap_or("")
    }

    /// Return the exclusive end of the last consumed non trivia token.
    pub(super) fn last_consumed_token_end(&self) -> Option<usize> {
        let mut index = self.pos;
        while index > 0 {
            index -= 1;
            let token = &self.tokens[index];
            if !token.ty.is_trivia() {
                return Some(token.start + token.text.len());
            }
        }

        None
    }

    /// Apply one ordered segment span list to one MIR node.
    pub(super) fn set_segment_spans<T>(&mut self, id: LocalNodeId<T>, segment_spans: &[Span])
    where
        T: Node,
    {
        // ordered source parts
        for (index, span) in segment_spans.iter().copied().enumerate() {
            let segment_index = u16::try_from(index)
                .unwrap_or_else(|_| panic!("too many segment spans for node {}", id.id));
            self.tree
                .set_side_span(id, NodeSpanType::Segment(segment_index), span);
        }
    }

    /// Parse a symbol name after `@`.
    pub(super) fn parse_symbol_name(&mut self) -> ParseResult<(String, usize)> {
        let name_token = self.eat_token(TokenType::Identifier)?;
        Ok((name_token.text.to_string(), name_token.start))
    }

    /// Scan a symbol name without emitting errors.
    pub(super) fn scan_symbol_name(&mut self) -> Option<String> {
        let token = self.peek()?;
        if token.ty != TokenType::Identifier {
            return None;
        }

        let name = token.text.to_string();
        self.bump();
        Some(name)
    }

    /// Record the type for a value in the current function.
    pub(super) fn record_value_type(&mut self, value: Value, ty: LocalNodeId<Type>) {
        if let Some(function_id) = self.current_function {
            let function = self.tree.get_mut(function_id);
            let existing = function.value_type(value);
            if let Some(existing) = existing {
                if existing != ty {
                    panic!("value {value:?} has mismatched types {existing:?} and {ty:?}");
                }
                return;
            }

            function.set_value_type(value, ty);
        }
    }

    /// Reset per-function parse state.
    pub(super) fn reset_function_parse_state(&mut self) {
        self.block_name_map.clear();
        self.block_id_by_label_index.clear();
        self.predeclared_blocks.clear();
        self.value_name_map.clear();
        self.next_value_id = 0;
        self.parsed_block_count = 0;
    }

    /// Skip raw trivia tokens except newline.
    pub(super) fn skip_raw_trivia_except_newline(&mut self) {
        while self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            if !token.ty.is_trivia() || token.ty == TokenType::Newline {
                break;
            }

            self.pos += 1;
        }
    }

    /// Return whether the current token starts a value definition.
    pub(super) fn is_value_definition_start(&self) -> bool {
        if self.peek_token(TokenType::Value) {
            return true;
        }

        let Some(token) = self.peek() else {
            return false;
        };

        if token.ty != TokenType::Identifier {
            return false;
        }

        matches!(
            self.peek_nth_token(1).map(|token| token.ty),
            Some(TokenType::Colon)
        )
    }

    /// Return whether the current token starts a value reference.
    pub(super) fn is_value_reference_start(&self) -> bool {
        self.peek()
            .is_some_and(|token| matches!(token.ty, TokenType::Value | TokenType::Identifier))
    }

    /// Return whether the current token starts a block label.
    pub(super) fn is_block_label_start(&self) -> bool {
        if self.peek_token(TokenType::BlockRefence) {
            return true;
        }

        let Some(token) = self.peek() else {
            return false;
        };

        if token.ty != TokenType::Identifier {
            return false;
        }

        let Some(raw_index) = self
            .tokens
            .iter()
            .enumerate()
            .skip(self.pos)
            .find_map(|(index, token)| (!token.ty.is_trivia()).then_some(index))
        else {
            return false;
        };

        let mut saw_colon = false;
        for token in self.tokens.iter().skip(raw_index + 1) {
            match token.ty {
                TokenType::Newline | TokenType::End => break,
                TokenType::Equals => return false,
                TokenType::Colon => saw_colon = true,
                _ => {}
            }
        }

        saw_colon
    }

    /// Return whether the token at one raw index starts a line-local block label.
    pub(super) fn is_block_label_token(&self, token_index: usize) -> bool {
        let Some(token) = self.tokens.get(token_index) else {
            return false;
        };

        if !matches!(token.ty, TokenType::BlockRefence | TokenType::Identifier) {
            return false;
        }

        if self.tokens[..token_index]
            .iter()
            .rev()
            .take_while(|token| token.ty != TokenType::Newline)
            .any(|token| !token.ty.is_trivia())
        {
            return false;
        }

        let mut saw_label_marker = false;
        let mut next_index = token_index + 1;
        while let Some(next_token) = self.tokens.get(next_index) {
            match next_token.ty {
                TokenType::Newline | TokenType::End => break,
                TokenType::Equals => return false,
                TokenType::Colon | TokenType::OpenParen => saw_label_marker = true,
                _ => {}
            }

            next_index += 1;
        }

        saw_label_marker
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
