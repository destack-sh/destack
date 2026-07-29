use std::collections::{HashMap, HashSet};

use destack_core::StringPool;
use destack_source::{
    Content, ContentId, DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, File,
    FileId, NodeSpanList, NodeSpanType, Span,
};

use crate::source::{Lexer, TokenType};
use crate::{
    Block, DispatchTable, DropTable, EffectTable, Function, Global, LayoutTable, LifetimeParameter,
    LifetimeSlot, Local, LocalNodeId, MemoryTable, Node, ProfileTable, StaticId, TargetLayout,
    Tree, Type, TypeTable, Value,
};

use super::error::{ParseError, ParseResult};

/// The result of parsing one MIR source file.
#[derive(Debug)]
pub struct ParsedMir {
    /// The parsed MIR tree.
    pub tree: Tree,
    /// Target ABI layout.
    pub target_layout: TargetLayout,
    /// Canonical MIR type table.
    pub types: TypeTable,
    /// Canonical MIR layout table.
    pub layouts: LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: DispatchTable,
    /// Canonical MIR drop table.
    pub drops: DropTable,
    /// Explicit MIR memory access table.
    pub memory: MemoryTable,
    /// Function and call effect table.
    pub effects: EffectTable,
    /// Static profile counter table.
    pub profile: ProfileTable,
    /// The parsed string pool.
    pub strings: StringPool,
    /// The collected parse diagnostics.
    pub diagnostics: DiagnosticCollection,
}

impl ParsedMir {
    /// Return the parsed tree, strings, and diagnostics.
    pub fn into_parts(
        self,
    ) -> (
        Tree,
        TargetLayout,
        TypeTable,
        LayoutTable,
        DispatchTable,
        DropTable,
        MemoryTable,
        EffectTable,
        ProfileTable,
        StringPool,
        DiagnosticCollection,
    ) {
        (
            self.tree,
            self.target_layout,
            self.types,
            self.layouts,
            self.dispatch,
            self.drops,
            self.memory,
            self.effects,
            self.profile,
            self.strings,
            self.diagnostics,
        )
    }

    /// Return the parsed MIR when no parse errors were emitted.
    pub fn finish(self) -> Result<(Tree, StringPool), DiagnosticCollection> {
        let Self {
            tree,
            target_layout: _,
            types: _,
            layouts: _,
            dispatch: _,
            drops: _,
            memory: _,
            effects: _,
            profile: _,
            strings,
            diagnostics,
        } = self;

        // fail strictly when parse diagnostics were emitted
        if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            return Err(diagnostics);
        }

        Ok((tree, strings))
    }
}

/// Options for the MIR parser.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Pointer size in bytes (4 for 32-bit, 8 for 64-bit).
    /// Used for pointer-sized MIR types and constants.
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
    /// Target ABI layout.
    pub(super) target_layout: TargetLayout,
    /// Canonical MIR type table.
    pub(super) types: TypeTable,
    /// Canonical MIR layout table.
    pub(super) layouts: LayoutTable,
    /// Canonical MIR dispatch table.
    pub(super) dispatch: DispatchTable,
    /// Canonical MIR drop table.
    pub(super) drops: DropTable,
    /// Explicit MIR memory access table.
    pub(super) memory: MemoryTable,
    /// Function and call effect table.
    pub(super) effects: EffectTable,
    /// Static profile counter table.
    pub(super) profile: ProfileTable,
    /// The string pool.
    pub(super) strings: StringPool,
    /// The source file id for spans.
    pub(super) file_id: FileId,
    /// The exact source content id for diagnostics.
    pub(super) content_id: ContentId,
    /// The diagnostics produced while parsing.
    pub(super) diagnostics: DiagnosticCollector,
    /// Map from concrete function names to their ids for forward references.
    pub(super) function_map: HashMap<(String, Vec<StaticId>), LocalNodeId<Function>>,
    /// Map from global names to their ids (for forward references).
    pub(super) global_map: HashMap<String, LocalNodeId<Global>>,
    /// Map from type declaration names to their ids (for references).
    pub(super) type_declaration_map: HashMap<(String, Vec<StaticId>), LocalNodeId<Type>>,
    /// Set of type declarations that have been defined.
    pub(super) type_declaration_definitions: HashSet<(String, Vec<StaticId>)>,
    /// Map from symbolic block names to their predeclared block ids.
    pub(super) block_name_map: HashMap<String, LocalNodeId<Block>>,
    /// Blocks predeclared for the current function body in source order.
    pub(super) predeclared_blocks: Vec<LocalNodeId<Block>>,
    /// SSA values defined in the current function.
    pub(super) defined_values: HashSet<Value>,
    /// Map from symbolic local names to their local ids.
    pub(super) local_name_map: HashMap<String, LocalNodeId<Local>>,
    /// The function currently being parsed.
    pub(super) current_function: Option<LocalNodeId<Function>>,
    /// SSA value types for the current function.
    pub(super) value_types: Vec<Option<LocalNodeId<Type>>>,
    /// The next SSA value id for the current function.
    pub(super) next_value_id: u32,
    /// The number of blocks parsed in the current function so far.
    pub(super) parsed_block_count: usize,
    /// Lifetime names visible in the current signature/type body.
    pub(super) lifetime_scopes: Vec<Vec<(String, LifetimeSlot)>>,
}

impl Parser {
    /// Create a new parser for a specific file.
    pub fn new(file: &File, options: ParseOptions) -> ParseResult<Self> {
        let Content::Text { content } = file.content.payload() else {
            return Err(ParseError::new("MIR parser requires text content", 0));
        };

        let tree = Tree::with_parsed_source(content.clone(), Lexer::lex(file.id, content));
        let target_layout = TargetLayout::for_pointer_bytes(options.pointer_bytes);

        Ok(Self {
            pos: 0,
            tree,
            target_layout,
            types: TypeTable::default(),
            layouts: LayoutTable::default(),
            dispatch: DispatchTable::default(),
            drops: DropTable::default(),
            memory: MemoryTable::default(),
            effects: EffectTable::default(),
            profile: ProfileTable::default(),
            strings: StringPool::new(),
            file_id: file.id,
            content_id: file.content_id(),
            diagnostics: DiagnosticCollector::new(),
            function_map: HashMap::new(),
            global_map: HashMap::new(),
            type_declaration_map: HashMap::new(),
            type_declaration_definitions: HashSet::new(),
            block_name_map: HashMap::new(),
            predeclared_blocks: Vec::new(),
            defined_values: HashSet::new(),
            local_name_map: HashMap::new(),
            current_function: None,
            value_types: Vec::new(),
            next_value_id: 0,
            parsed_block_count: 0,
            lifetime_scopes: Vec::new(),
        })
    }

    /// Parse MIR text and return the parsed source bundle.
    pub fn parse(file: &File, options: ParseOptions) -> ParseResult<ParsedMir> {
        let mut parser = Parser::new(file, options)?;

        // parse the semantic MIR
        parser.parse_module();

        // attach source comments after the node graph exists
        parser.attach_comment_ownership();

        // rebuild derived primitive type cache
        parser.types.rebuild_primitive_types(&parser.tree);

        Ok(ParsedMir {
            tree: parser.tree,
            target_layout: parser.target_layout,
            types: parser.types,
            layouts: parser.layouts,
            dispatch: parser.dispatch,
            drops: parser.drops,
            memory: parser.memory,
            effects: parser.effects,
            profile: parser.profile,
            strings: parser.strings,
            diagnostics: parser.diagnostics.take_collection(),
        })
    }

    /// Apply one ordered segment span list to one MIR node.
    pub(super) fn set_segment_spans<T>(
        &mut self,
        id: LocalNodeId<T>,
        segment_spans: &[Span],
    ) -> ParseResult<()>
    where
        T: Node,
    {
        // ordered source parts
        for (index, span) in segment_spans.iter().copied().enumerate() {
            let segment_index = u16::try_from(index).map_err(|_| {
                ParseError::new(
                    format!("too many segment spans for node {}", id.id),
                    self.pos(),
                )
            })?;
            self.tree.set_side_span(
                id,
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
                span,
            );
        }

        Ok(())
    }

    /// Parse one declaration name.
    pub(super) fn parse_symbol_name(&mut self) -> ParseResult<(String, usize)> {
        let name_token = self.eat_token(TokenType::Identifier)?;
        let start = name_token.start();
        let name = self.tree.source_text(name_token.span).to_string();

        Ok((name, start))
    }

    /// Parse concrete function generic arguments.
    pub(super) fn parse_function_arguments(&mut self) -> ParseResult<Vec<StaticId>> {
        if !self.eat_token_if(TokenType::LessThan) {
            return Ok(Vec::new());
        }

        let mut arguments = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            arguments.push(self.parse_static()?);

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::GreaterThan)?;

        Ok(arguments)
    }

    /// Parse concrete generic arguments followed by lifetime binders.
    pub(super) fn parse_declaration_parameters(
        &mut self,
    ) -> ParseResult<(Vec<StaticId>, Vec<LifetimeParameter>)> {
        if !self.eat_token_if(TokenType::LessThan) {
            self.lifetime_scopes.push(Vec::new());
            return Ok((Vec::new(), Vec::new()));
        }

        // parse concrete arguments before lifetime binders
        let mut arguments = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) && !self.peek_is(TokenType::Lifetime) {
            arguments.push(self.parse_static()?);

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        // declare the remaining lifetime binders
        let mut lifetimes = Vec::new();
        let mut scope = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            let name_token = self.eat_token(TokenType::Lifetime)?;
            let name = self.tree.source_text(name_token.span).to_string();
            if scope.iter().any(|(candidate, _)| candidate == &name) {
                return Err(ParseError::invalid(
                    "duplicate lifetime parameter",
                    name_token.start(),
                ));
            }

            let slot = LifetimeSlot(scope.len() as u32);
            scope.push((name.clone(), slot));
            lifetimes.push(LifetimeParameter::new(Some(self.strings.intern(&name))));

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::GreaterThan)?;
        self.lifetime_scopes.push(scope);

        Ok((arguments, lifetimes))
    }

    /// Parse optional lifetime parameters after a declaration name.
    pub(super) fn parse_lifetime_parameters(&mut self) -> ParseResult<Vec<LifetimeParameter>> {
        if !self.eat_token_if(TokenType::LessThan) {
            self.lifetime_scopes.push(Vec::new());
            return Ok(Vec::new());
        }

        let mut lifetimes = Vec::new();
        let mut scope = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            let name_token = self.eat_token(TokenType::Lifetime)?;
            let name = self.tree.source_text(name_token.span).to_string();
            if scope.iter().any(|(candidate, _)| candidate == &name) {
                return Err(ParseError::invalid(
                    "duplicate lifetime parameter",
                    name_token.start(),
                ));
            }
            let slot = LifetimeSlot(scope.len() as u32);
            scope.push((name.clone(), slot));
            lifetimes.push(LifetimeParameter::new(Some(self.strings.intern(&name))));

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::GreaterThan)?;
        self.lifetime_scopes.push(scope);

        Ok(lifetimes)
    }

    /// Parse one optional trailing lifetime where clause.
    pub(super) fn parse_lifetime_where(
        &mut self,
        lifetimes: &mut [LifetimeParameter],
    ) -> ParseResult<()> {
        if !self.eat_name_if("where") {
            return Ok(());
        }

        loop {
            let left = self.parse_scope_lifetime()?;
            self.eat_token(TokenType::Colon)?;
            let right = self.parse_scope_lifetime()?;
            lifetimes[left.0 as usize].outlives.push(right);
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        Ok(())
    }

    /// Parse one lifetime name against the active scope.
    fn parse_scope_lifetime(&mut self) -> ParseResult<LifetimeSlot> {
        let token = self.eat_token(TokenType::Lifetime)?;
        let name = self.tree.source_text(token.span);
        let slot = self.lifetime_scopes.last().and_then(|scope| {
            scope
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, slot)| *slot)
        });
        let Some(slot) = slot else {
            return Err(ParseError::invalid("lifetime parameter", token.start()));
        };

        Ok(slot)
    }

    /// Parse a lifetime parameter scope around one parser operation.
    pub(super) fn parse_lifetime_scope<T>(
        &mut self,
        parse: impl FnOnce(&mut Self, Vec<LifetimeParameter>) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let lifetime_scope_count = self.lifetime_scopes.len();
        let lifetimes = self.parse_lifetime_parameters();
        let result = lifetimes.and_then(|lifetimes| parse(self, lifetimes));
        self.restore_lifetime_scopes(lifetime_scope_count);

        result
    }

    /// Leave the current lifetime parameter scope.
    pub(super) fn pop_lifetime_scope(&mut self) {
        self.lifetime_scopes.pop();
    }

    /// Restore the lifetime scope stack to a previous depth.
    pub(super) fn restore_lifetime_scopes(&mut self, count: usize) {
        self.lifetime_scopes.truncate(count);
    }

    /// Resolve one named lifetime in the visible lifetime scopes.
    pub(super) fn lifetime_slot(&self, name: &str) -> Option<LifetimeSlot> {
        for scope in self.lifetime_scopes.iter().rev() {
            for (candidate, slot) in scope {
                if candidate == name {
                    return Some(*slot);
                }
            }
        }

        None
    }

    /// Record the type for a value in the current function.
    pub(super) fn record_value_type(
        &mut self,
        value: Value,
        ty: LocalNodeId<Type>,
    ) -> ParseResult<()> {
        if self.current_function.is_some() {
            let existing = self.value_types.get(value.0 as usize).copied().flatten();
            if let Some(existing) = existing {
                if existing != ty {
                    return Err(ParseError::new(
                        format!("value {value:?} has mismatched types {existing:?} and {ty:?}"),
                        self.pos(),
                    ));
                }
                return Ok(());
            }

            self.resize_value_slots(value);
            self.value_types[value.0 as usize] = Some(ty);
        }

        Ok(())
    }

    /// Resize current function SSA side tables for one value.
    pub(super) fn resize_value_slots(&mut self, value: Value) {
        let index = value.0 as usize;
        let value_count = index + 1;

        if self.value_types.len() < value_count {
            self.value_types.resize(value_count, None);
        }
    }

    /// Reset per-function parse state.
    pub(super) fn reset_function_parse_state(&mut self) {
        self.block_name_map.clear();
        self.predeclared_blocks.clear();
        self.defined_values.clear();
        self.local_name_map.clear();
        self.value_types.clear();
        self.next_value_id = 0;
        self.parsed_block_count = 0;
    }

    /// Return whether the current token starts a value definition.
    pub(super) fn is_value_definition_start(&self) -> bool {
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
        self.peek_is(TokenType::Identifier)
    }

    /// Return whether the current token starts a block label.
    pub(super) fn is_block_label_start(&self) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        if !self.is_token_at_line_start(token) {
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

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        if !self.is_token_at_line_start(token) {
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
