use std::collections::{HashMap, HashSet};

use destack_core::{ImmutableStringPool, StringPool};
use destack_source::{FileId, Span};

use crate::validate::{Validator, ValidatorOptions};
use crate::{
    AllocationMode, Block, Field, Function, Global, Lifetime, Linkage, LocalNodeId, NodeTree,
    PointerAttributes, Type,
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
    /// Parser options.
    pub(super) options: ParseOptions,
    /// Map from block names to their ids (for forward references).
    pub(super) block_map: HashMap<String, LocalNodeId<Block>>,
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
        Self {
            tokens,
            pos: 0,
            tree: NodeTree::new(),
            strings: StringPool::new(),
            file_id,
            options,
            block_map: HashMap::new(),
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

        Ok((parser.tree, parser.strings.into_immutable()))
    }

    /// Get current position for error reporting.
    pub(super) fn pos(&self) -> usize {
        self.peek().map(|t| t.start).unwrap_or(0)
    }

    /// Build a span for a source slice.
    pub(super) fn span_at(file_id: FileId, start: usize, length: usize) -> Span {
        let start = u32::try_from(start).unwrap_or(u32::MAX);
        let length = u32::try_from(length).unwrap_or(0);
        Span::at(file_id, start, length)
    }

    /// Build a span for a token.
    pub(super) fn span_for_token(file_id: FileId, token: &Token<'_>) -> Span {
        Self::span_at(file_id, token.start, token.text.len())
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
    /// Opcodes can be identifiers or the `struct` keyword (which conflicts
    /// with the type keyword but is also a valid instruction name).
    pub(super) fn eat_opcode(&mut self) -> ParseResult<(&'a str, usize)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;

        match token.ty {
            TokenType::Identifier | TokenType::Struct => {
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

    /// Parse a module (list of type aliases, globals, and functions).
    fn parse_module(&mut self) -> ParseResult<()> {
        // first pass: register all type aliases for forward references
        self.register_all_type_aliases();

        // second pass: register all function names for forward references
        self.register_all_functions();

        // third pass: parse everything
        while !self.peek_token(TokenType::End) {
            // parse optional attributes
            let attributes = self.parse_attributes()?;

            // parse optional linkage prefix: extern or export
            let linkage = if self.peek_token(TokenType::Extern) {
                self.bump();
                Linkage::Import
            } else if self.peek_token(TokenType::Export) {
                self.bump();
                Linkage::Export
            } else {
                Linkage::Local // default
            };

            if self.peek_token(TokenType::Type) {
                if linkage != Linkage::Local {
                    return Err(ParseError::new(
                        "type aliases cannot be extern or export",
                        self.pos(),
                    ));
                }
                self.parse_type_alias(attributes)?;
            } else if self.peek_token(TokenType::Global) {
                self.parse_global(linkage, attributes)?;
            } else if self.peek_token(TokenType::Function) {
                self.parse_function(linkage, attributes)?;
            } else {
                return Err(ParseError::new(
                    "expected 'type', 'function', or 'global'",
                    self.pos(),
                ));
            }
        }

        // set up the validator
        let validator = Validator::new_with_options(&self.tree, ValidatorOptions::strict());

        // map validation errors to source positions
        validator.validate_module().map_err(|error| {
            let position = error
                .anchor()
                .and_then(|anchor| self.tree.get_span_by_id(anchor.node.id))
                .map(|span| span.start as usize)
                .unwrap_or_else(|| self.pos());
            ParseError::new(error.to_string(), position)
        })?;

        Ok(())
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

    /// Pre register all function names to allow forward references.
    ///
    /// This scans for `function @name` patterns without parsing anything else.
    fn register_all_functions(&mut self) {
        let saved_pos = self.pos;

        // scan token by token looking for function declarations
        while !self.peek_token(TokenType::End) {
            // skip attributes
            self.skip_attribute_tokens();

            // skip optional linkage prefix
            if self.peek_token(TokenType::Extern) || self.peek_token(TokenType::Export) {
                self.bump();
            }

            // look for: function @name
            if self.peek_token(TokenType::Function) {
                self.bump();
                if self.peek_token(TokenType::At) {
                    self.bump();
                    if let Some(name) = self.scan_symbol_name() {
                        // register placeholder if not already known
                        if !self.function_map.contains_key(&name) {
                            let name_id = self.strings.intern(&name);
                            let void_ty = self.intern_type(Type::Void);
                            let parameter_attributes = Vec::new();
                            let placeholder = Function {
                                name: name_id,
                                parameters: Vec::new(),
                                parameter_names: Vec::new(),
                                value_types: Vec::new(),
                                return_type: void_ty,
                                return_lifetime: Lifetime::Inferred,
                                memory_effects: None,
                                call_behavior: None,
                                alloc_size: None,
                                parameter_attributes,
                                return_attributes: PointerAttributes::default(),
                                linkage: Linkage::Local,
                                allocation: AllocationMode::Any,
                                coroutine: None,
                                execution_model: None,
                                execution_stage: None,
                                workgroup_size: None,
                                closure_env_type: None,
                                locals: Vec::new(),
                                blocks: Vec::new(),
                                entry: None,
                                next_value_id: 0,
                            };
                            let id = self.tree.insert(placeholder);
                            self.function_map.insert(name, id);
                        }
                        continue;
                    }
                }
            }

            self.bump();
        }

        self.pos = saved_pos;
    }

    /// Pre register all type aliases to allow forward references.
    ///
    /// This scans for `type @name` patterns without parsing anything else.
    fn register_all_type_aliases(&mut self) {
        let saved_pos = self.pos;

        // scan token by token looking for type alias declarations
        while !self.peek_token(TokenType::End) {
            // skip attributes
            self.skip_attribute_tokens();

            // skip optional linkage prefix
            if self.peek_token(TokenType::Extern) || self.peek_token(TokenType::Export) {
                self.bump();
            }

            // look for: type @name
            if self.peek_token(TokenType::Type) {
                self.bump();
                if self.peek_token(TokenType::At) {
                    self.bump();
                    if let Some(name) = self.scan_symbol_name() {
                        // register placeholder if not already known
                        if !self.type_alias_map.contains_key(&name) {
                            let placeholder = self.tree.insert_type(Type::Void);
                            self.type_alias_map.insert(name, placeholder);
                        }
                        continue;
                    }
                }
            }

            self.bump();
        }

        self.pos = saved_pos;
    }

    /// Skip attribute tokens during the pre scan.
    fn skip_attribute_tokens(&mut self) {
        // scan for attribute prefixes
        while self.peek_token(TokenType::Hash) {
            self.bump();

            // require an opening bracket to start the attribute
            if !self.peek_token(TokenType::OpenBracket) {
                continue;
            }

            // consume the attribute contents
            self.bump();
            let mut depth = 1usize;
            while depth > 0 && !self.peek_token(TokenType::End) {
                if self.peek_token(TokenType::OpenBracket) {
                    depth += 1;
                } else if self.peek_token(TokenType::CloseBracket) {
                    depth = depth.saturating_sub(1);
                }
                self.bump();
            }
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
