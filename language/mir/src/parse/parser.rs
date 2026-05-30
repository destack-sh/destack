use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_source::{
    DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, FileContentId, FileId,
    NodeSpanList, NodeSpanType, Span,
};

use crate::source::{Lexer, Token, TokenType};
use crate::{
    Block, Field, Function, Global, LocalNodeId, Node, Tree, Type, Value, finalize_function_names,
};

use super::error::{ParseError, ParseResult};
use super::key::{FieldKey, TypeKey};

/// The result of parsing one MIR source file.
#[derive(Debug)]
pub struct ParsedMir {
    /// The parsed MIR tree.
    pub tree: Tree,
    /// The parsed string pool.
    pub strings: StringPool,
    /// The collected parse diagnostics.
    pub diagnostics: DiagnosticCollection,
}

impl ParsedMir {
    /// Return the parsed tree, strings, and diagnostics.
    pub fn into_parts(self) -> (Tree, StringPool, DiagnosticCollection) {
        (self.tree, self.strings, self.diagnostics)
    }

    /// Return the parsed MIR when no parse errors were emitted.
    pub fn finish(self) -> ParseResult<(Tree, StringPool)> {
        let Self {
            tree,
            strings,
            diagnostics,
        } = self;

        // fail strictly when parse diagnostics were emitted
        if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            let Some(diagnostic) = diagnostics.iter().next() else {
                return Err(ParseError::new("parser emitted an empty error set", 0));
            };
            return Err(ParseError::from_diagnostic(diagnostic));
        }

        Ok((tree, strings))
    }
}

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
pub struct Parser {
    /// Current position in the tokens.
    pub(super) pos: usize,
    /// The tree being built.
    pub(super) tree: Tree,
    /// The string pool.
    pub(super) strings: StringPool,
    /// The source file id for spans.
    pub(super) file_id: FileId,
    /// The exact source content id for diagnostics.
    pub(super) content_id: FileContentId,
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
impl Parser {
    /// Create a new parser for a specific file.
    pub fn new(file_id: FileId, source: &str, options: ParseOptions) -> Self {
        let content_id = FileContentId::for_text(source);
        let mut tree = Tree::with_parsed_source(source.to_string(), Lexer::lex(file_id, source));
        tree.set_pointer_bytes(options.pointer_bytes);

        Self {
            pos: 0,
            tree,
            strings: StringPool::new(),
            file_id,
            content_id,
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

    /// Parse MIR text and return the parsed source bundle.
    pub fn parse(file_id: FileId, source: &str, options: ParseOptions) -> ParsedMir {
        let mut parser = Parser::new(file_id, source, options);

        // parse the semantic MIR
        parser.parse_module();

        // attach source comments after the node graph exists
        parser.attach_comment_ownership();

        // synthesize any generated function names
        parser.finalize_generated_names();

        ParsedMir {
            tree: parser.tree,
            strings: parser.strings,
            diagnostics: parser.diagnostics.take_collection(),
        }
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

    /// Return the MIR token type for one source token.
    pub(super) fn token_type(&self, token: &Token) -> TokenType {
        let text = self.tree.source_text(token.span);

        match token.ty {
            TokenType::Identifier => TokenType::from_identifier(text),
            TokenType::Question => TokenType::Unknown,
            ty => ty,
        }
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

    /// Return whether source between one token and the next token crosses a line.
    pub(super) fn has_line_break_after(&self, token: &Token) -> bool {
        let Some(next) = self.peek() else {
            return false;
        };

        let start = token.span.end;
        let end = next.span.start;
        if end <= start {
            return false;
        }

        let span = Span::at(self.file_id, start, end - start);
        self.tree
            .source_text(span)
            .bytes()
            .any(|byte| matches!(byte, b'\n' | b'\r'))
    }

    /// Peek the current token (skipping trivia).
    pub(super) fn peek(&self) -> Option<&Token> {
        let mut pos = self.pos;
        let tokens = self.tree.tokens();

        while pos < tokens.len() {
            let token = &tokens[pos];
            if !token.is_trivia() {
                return Some(token);
            }
            pos += 1;
        }
        None
    }

    /// Peek the Nth non-trivia token, where 0 is the current token.
    pub(super) fn peek_nth_token(&self, n: usize) -> Option<&Token> {
        let mut pos = self.pos;
        let mut seen = 0usize;
        let tokens = self.tree.tokens();

        while pos < tokens.len() {
            let token = &tokens[pos];
            if !token.is_trivia() {
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
        let tokens = self.tree.tokens();

        while self.pos < tokens.len() {
            let is_trivia = tokens[self.pos].is_trivia();
            self.pos += 1;
            if !is_trivia {
                break;
            }
        }

        // skip trailing trivia
        while self.pos < tokens.len() && tokens[self.pos].is_trivia() {
            self.pos += 1;
        }
    }

    /// Return whether the current source token matches one token type.
    pub(super) fn peek_token(&self, ty: TokenType) -> bool {
        self.peek()
            .is_some_and(|token| self.token_type(token) == ty)
    }

    /// Consume one source token with the expected token type.
    pub(super) fn eat_token(&mut self, ty: TokenType) -> ParseResult<Token> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end(&format!("{ty:?}"), self.pos()))?;
        if self.token_type(token) != ty {
            return Err(ParseError::unexpected_token(&format!("{ty:?}"), token));
        }

        // return current token, then advance
        let pos = self.pos;
        self.bump();
        let tokens = self.tree.tokens();

        // find the token we just consumed
        for token in tokens.iter().take(self.pos).skip(pos) {
            if !token.is_trivia() {
                return Ok(*token);
            }
        }
        Ok(tokens[pos])
    }

    /// Consume one source token when it matches one token type.
    pub(super) fn eat_token_maybe(&mut self, ty: TokenType) -> bool {
        if self.peek_token(ty) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the current identifier when it matches the expected text.
    pub(super) fn eat_identifier_text(&mut self, expected: &str) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        let text = self.tree.source_text(token.span);
        if text != expected {
            return false;
        }

        self.bump();
        true
    }

    /// Parse an instruction opcode.
    ///
    /// Opcodes can be identifiers or reserved opcode keywords that also have
    /// dedicated token kinds.
    pub(super) fn eat_opcode(&mut self) -> ParseResult<(String, usize)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("opcode", self.pos()))?;

        match self.token_type(token) {
            TokenType::Identifier
            | TokenType::Const
            | TokenType::Struct
            | TokenType::Call
            | TokenType::CallIndirect
            | TokenType::CallVirtual
            | TokenType::CallDynamic => {
                let text = self.tree.source_text(token.span).to_string();
                let start = token.start;
                self.bump();
                Ok((text, start))
            }
            _ => Err(ParseError::unexpected_token("opcode", token)),
        }
    }

    /// Return the exclusive end of the last consumed non trivia token.
    pub(super) fn last_consumed_token_end(&self) -> Option<usize> {
        let mut index = self.pos;
        let tokens = self.tree.tokens();

        while index > 0 {
            index -= 1;
            let token = &tokens[index];
            if !token.is_trivia() {
                return Some(token.span.end as usize);
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
            self.tree.set_side_span(
                id,
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
                span,
            );
        }
    }

    /// Parse a symbol name after `@`.
    pub(super) fn parse_symbol_name(&mut self) -> ParseResult<(String, usize)> {
        let name_token = self.eat_token(TokenType::Identifier)?;
        let name_span = name_token.span;
        let name_start = name_token.span.start as usize;
        let name = self.tree.source_text(name_span).to_string();

        Ok((name, name_start))
    }

    /// Scan a symbol name without emitting errors.
    pub(super) fn scan_symbol_name(&mut self) -> Option<String> {
        let token = self.peek()?;
        if self.token_type(token) != TokenType::Identifier {
            return None;
        }

        let name = self.tree.source_text(token.span).to_string();
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
        let tokens = self.tree.tokens();

        while self.pos < tokens.len() {
            let token = &tokens[self.pos];
            let ty = self.token_type(token);
            if !token.is_trivia() || ty == TokenType::Newline {
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

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        matches!(
            self.peek_nth_token(1).map(|token| self.token_type(token)),
            Some(TokenType::Colon)
        )
    }

    /// Return whether the current token starts a value reference.
    pub(super) fn is_value_reference_start(&self) -> bool {
        self.peek().is_some_and(|token| {
            matches!(
                self.token_type(token),
                TokenType::Value | TokenType::Identifier
            )
        })
    }

    /// Return whether the current token starts a block label.
    pub(super) fn is_block_label_start(&self) -> bool {
        if self.peek_token(TokenType::BlockReference) {
            return true;
        }

        let Some(token) = self.peek() else {
            return false;
        };

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        let Some(raw_index) = self
            .tree
            .tokens()
            .iter()
            .enumerate()
            .skip(self.pos)
            .find_map(|(index, token)| (!token.is_trivia()).then_some(index))
        else {
            return false;
        };

        let mut saw_colon = false;
        for token in self.tree.tokens().iter().skip(raw_index + 1) {
            match self.token_type(token) {
                TokenType::Newline | TokenType::End => break,
                TokenType::Equal => return false,
                TokenType::Colon => saw_colon = true,
                _ => {}
            }
        }

        saw_colon
    }

    /// Return whether the token at one raw index starts a line-local block label.
    pub(super) fn is_block_label_token(&self, token_index: usize) -> bool {
        let tokens = self.tree.tokens();

        let Some(token) = tokens.get(token_index) else {
            return false;
        };

        if !matches!(
            self.token_type(token),
            TokenType::BlockReference | TokenType::Identifier
        ) {
            return false;
        }

        if tokens[..token_index]
            .iter()
            .rev()
            .take_while(|token| self.token_type(token) != TokenType::Newline)
            .any(|token| !token.is_trivia())
        {
            return false;
        }

        let mut saw_colon = false;
        let mut next_index = token_index + 1;
        while let Some(next_token) = tokens.get(next_index) {
            match self.token_type(next_token) {
                TokenType::Newline | TokenType::End => break,
                TokenType::Equal => return false,
                TokenType::Colon => saw_colon = true,
                _ => {}
            }

            next_index += 1;
        }

        saw_colon
    }
}
