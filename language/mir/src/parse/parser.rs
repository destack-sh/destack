use std::collections::{HashMap, HashSet};

use destack_core::{ImmutableStringPool, StringPool};
use destack_source::{FileId, Span};

use crate::validate::Validator;
use crate::{Field, Function, Global, LocalNodeId, NodeTree, Type, Value};

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
    /// Map from function names to their ids (for forward references).
    pub(super) function_map: HashMap<String, LocalNodeId<Function>>,
    /// Map from global names to their ids (for forward references).
    pub(super) global_map: HashMap<String, LocalNodeId<Global>>,
    /// Map from type alias names to their ids (for references).
    pub(super) type_alias_map: HashMap<String, LocalNodeId<Type>>,
    /// Set of type aliases that have been defined.
    pub(super) type_alias_definitions: HashSet<String>,
    /// Type interner for canonical type ids.
    pub(super) type_intern: HashMap<TypeKey, LocalNodeId<Type>>,
    /// Field interner for canonical field ids.
    pub(super) field_intern: HashMap<FieldKey, LocalNodeId<Field>>,
    /// The function currently being parsed.
    pub(super) current_function: Option<LocalNodeId<Function>>,
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
            function_map: HashMap::new(),
            global_map: HashMap::new(),
            type_alias_map: HashMap::new(),
            type_alias_definitions: HashSet::new(),
            type_intern: HashMap::new(),
            field_intern: HashMap::new(),
            current_function: None,
        }
    }

    /// Parse MIR text into a NodeTree and string pool.
    pub fn parse(
        file_id: FileId,
        source: &str,
        options: ParseOptions,
    ) -> ParseResult<(NodeTree, ImmutableStringPool)> {
        let mut parser = Parser::new(file_id, source, options);
        parser.parse_module()?;

        // validate the finished tree
        let validator = Validator::new(&parser.tree);
        validator.validate().map_err(|error| {
            let position = error
                .anchor()
                .and_then(|anchor| parser.tree.get_span_by_id(anchor.node.id))
                .map(|span| span.start as usize)
                .unwrap_or_else(|| parser.pos());
            ParseError::new(error.to_string(), position)
        })?;

        Ok((parser.tree, parser.strings.into_immutable()))
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
            return Err(ParseError::unexpected(
                &format!("{ty:?}"),
                token.ty,
                token.start,
            ));
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
            _ => Err(ParseError::unexpected("opcode", token.ty, token.start)),
        }
    }

    /// Get the text of the current token.
    pub(super) fn span_str(&self) -> &'a str {
        self.peek().map(|t| t.text).unwrap_or("")
    }

    /// Parse a symbol name after `@`.
    pub(super) fn parse_symbol_name(&mut self) -> ParseResult<(String, usize)> {
        // read the base identifier segment
        let name_token = self.eat_token(TokenType::Identifier)?;
        let mut name = name_token.text.to_string();
        let start = name_token.start;

        // consume additional colon segments
        while self.peek_token(TokenType::Colon)
            && self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::Identifier)
        {
            self.eat_token(TokenType::Colon)?;
            let segment = self.eat_token(TokenType::Identifier)?;
            name.push(':');
            name.push_str(segment.text);
        }

        // return the complete name and start offset
        Ok((name, start))
    }

    /// Scan a symbol name without emitting errors.
    pub(super) fn scan_symbol_name(&mut self) -> Option<String> {
        // ensure the next token is a symbol segment
        let token = self.peek()?;
        if token.ty != TokenType::Identifier {
            return None;
        }

        // capture the base segment
        let mut name = token.text.to_string();
        self.bump();

        // consume additional colon segments
        while self.peek_token(TokenType::Colon)
            && self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::Identifier)
        {
            self.bump();
            let token = self.peek()?;
            name.push(':');
            name.push_str(token.text);
            self.bump();
        }

        // return the scanned name
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
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
