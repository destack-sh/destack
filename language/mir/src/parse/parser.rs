//! MIR parser.

use std::collections::HashMap;

use crate::{
    BinaryOperator, Block, CastKind, Constant, Function, Global, GlobalInitializer, Instruction,
    Intrinsic, Linkage, Local, LocalNodeId, Mutability, NodeTree, Ownership, SwitchCase,
    Terminator, Type, TypedValue, UnaryOperator, Value,
};
use destack_base::{ImmutableStringPool, StringPool};

use super::error::{ParseError, ParseResult};
use super::lexer::Lexer;
use super::token::{Token, TokenType};

/// Parser for MIR text format.
#[derive(Debug)]
pub struct Parser<'a> {
    /// The tokens to parse.
    tokens: Vec<Token<'a>>,
    /// Current position in the tokens.
    pos: usize,
    /// The node tree being built.
    tree: NodeTree,
    /// The string pool.
    strings: StringPool,
    /// Map from block names to their ids (for forward references).
    block_map: HashMap<String, LocalNodeId<Block>>,
    /// Map from function names to their ids (for forward references).
    function_map: HashMap<String, LocalNodeId<Function>>,
    /// Map from global names to their ids (for forward references).
    global_map: HashMap<String, LocalNodeId<Global>>,
}

impl<'a> Parser<'a> {
    /// Create a new parser.
    pub fn new(source: &'a str) -> Self {
        let tokens = Lexer::lex(source);
        Self {
            tokens,
            pos: 0,
            tree: NodeTree::new(),
            strings: StringPool::new(),
            block_map: HashMap::new(),
            function_map: HashMap::new(),
            global_map: HashMap::new(),
        }
    }

    /// Parse MIR text into a NodeTree and string pool.
    pub fn parse(source: &str) -> ParseResult<(NodeTree, ImmutableStringPool)> {
        let mut parser = Parser::new(source);
        parser.parse_module()?;
        Ok((parser.tree, parser.strings.into_immutable()))
    }

    /// Get current position for error reporting.
    fn pos(&self) -> usize {
        self.peek().map(|t| t.start).unwrap_or(0)
    }

    /// Peek the current token (skipping trivia).
    fn peek(&self) -> Option<&Token<'a>> {
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

    /// Advance past the current token.
    fn bump(&mut self) {
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
    fn peek_token(&self, ty: TokenType) -> bool {
        self.peek().is_some_and(|t| t.ty == ty)
    }

    /// Consume a token of the given type, or return an error.
    fn eat_token(&mut self, ty: TokenType) -> ParseResult<&Token<'a>> {
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
    fn eat_token_maybe(&mut self, ty: TokenType) -> bool {
        if self.peek_token(ty) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Get the text of the current token.
    fn span_str(&self) -> &'a str {
        self.peek().map(|t| t.text).unwrap_or("")
    }

    /// Parse a module (list of globals and functions).
    fn parse_module(&mut self) -> ParseResult<()> {
        while !self.peek_token(TokenType::End) {
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

            if self.peek_token(TokenType::Global) {
                self.parse_global(linkage)?;
            } else if self.peek_token(TokenType::Function) {
                self.parse_function(linkage)?;
            } else {
                return Err(ParseError::new(
                    "expected 'function' or 'global'",
                    self.pos(),
                ));
            }
        }
        Ok(())
    }

    /// Parse a global definition or declaration.
    /// Syntax: `[export|extern] global @name: type [= init] ; var|const`
    fn parse_global(&mut self, linkage: Linkage) -> ParseResult<LocalNodeId<Global>> {
        self.eat_token(TokenType::Global)?;
        self.eat_token(TokenType::At)?;

        // global name
        let name_token = self.eat_token(TokenType::Identifier)?;
        let name = name_token.text.to_string();

        // type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;

        // initializer (required for defined globals, absent for imports)
        let initializer = if linkage.is_import() {
            None
        } else {
            self.eat_token(TokenType::Equals)?;
            Some(self.parse_data_init()?)
        };

        // mutability annotation: ; var or ; const
        let mut mutability = Mutability::Immutable;
        if self.eat_token_maybe(TokenType::Semicolon) {
            if self.eat_token_maybe(TokenType::Var) {
                mutability = Mutability::Mutable;
            } else if self.eat_token_maybe(TokenType::Const) {
                mutability = Mutability::Immutable;
            }
        }

        let name_id = self.strings.intern(&name);
        let global = Global {
            name: name_id,
            ty,
            mutability,
            linkage,
            initializer,
        };
        let id = self.tree.insert(global);
        self.global_map.insert(name, id);
        Ok(id)
    }

    /// Parse a data initializer.
    fn parse_data_init(&mut self) -> ParseResult<GlobalInitializer> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("data initializer", self.pos()))?;

        match token.ty {
            // zero initializer
            TokenType::Identifier if token.text == "zeroinit" => {
                self.bump();
                Ok(GlobalInitializer::Zero)
            }
            // string literal -> bytes (UTF-8)
            TokenType::StringLiteral => {
                let token_text = token.text.to_string();
                let token_start = token.start;
                self.bump();
                let value = parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(GlobalInitializer::Bytes(value.into_bytes()))
            }
            // scalar constant
            TokenType::BoolLiteral
            | TokenType::IntLiteral
            | TokenType::FloatLiteral
            | TokenType::CharLiteral => {
                let constant = self.parse_constant()?;
                Ok(GlobalInitializer::Scalar(constant))
            }
            // aggregate: { init, init, ... }
            TokenType::OpenBrace => {
                self.bump();
                let mut elements = Vec::new();
                while !self.peek_token(TokenType::CloseBrace) {
                    elements.push(self.parse_data_init()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBrace)?;
                Ok(GlobalInitializer::Aggregate(elements))
            }
            _ => Err(ParseError::unexpected(
                "data initializer",
                token.ty,
                token.start,
            )),
        }
    }

    /// Parse a function definition or declaration.
    /// Syntax: `[export|extern] function @name(params) -> type { body }`
    fn parse_function(&mut self, linkage: Linkage) -> ParseResult<LocalNodeId<Function>> {
        self.eat_token(TokenType::Function)?;
        self.eat_token(TokenType::At)?;

        // function name
        let name_token = self.eat_token(TokenType::Identifier)?;
        let name = name_token.text.to_string();

        // parameters (with types for imports, typed values for definitions)
        self.eat_token(TokenType::OpenParen)?;
        let parameters = if linkage.is_import() {
            // extern functions only have types, no value names
            self.parse_extern_parameter_list()?
        } else {
            self.parse_typed_value_list()?
        };
        self.eat_token(TokenType::CloseParen)?;

        // return type
        self.eat_token(TokenType::Arrow)?;
        let return_type = self.parse_type()?;

        // imports have no body
        if linkage.is_import() {
            let name_id = self.strings.intern(&name);
            let function = Function {
                name: name_id,
                parameters,
                return_type,
                linkage,
                allocation_mode: crate::AllocationMode::Any,
                coroutine: None,
                locals: Vec::new(),
                blocks: Vec::new(),
                entry: None,
                next_value_id: 0,
            };
            let id = self.tree.insert(function);
            self.function_map.insert(name, id);
            return Ok(id);
        }

        // pre-register the function so it can reference itself (recursion)
        let name_id = self.strings.intern(&name);
        let placeholder = Function {
            name: name_id,
            parameters: parameters.clone(),
            return_type,
            linkage,
            allocation_mode: crate::AllocationMode::Any,
            coroutine: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            entry: None,
            next_value_id: 0,
        };
        let id = self.tree.insert(placeholder);
        self.function_map.insert(name, id);

        // body
        self.eat_token(TokenType::OpenBrace)?;

        // locals
        let mut locals = Vec::new();
        while self.peek_token(TokenType::LocalReference) {
            let local = self.parse_local()?;
            locals.push(local);
        }

        // blocks (parse them and build a mapping from source index to actual ID)
        let mut blocks = Vec::new();
        let mut source_index_to_block: Vec<LocalNodeId<Block>> = Vec::new();
        while self.peek_token(TokenType::BlockRefence) {
            let (block, source_idx) = self.parse_block()?;
            // ensure we have room in the mapping
            while source_index_to_block.len() <= source_idx as usize {
                source_index_to_block.push(LocalNodeId::new(u32::MAX)); // placeholder
            }
            source_index_to_block[source_idx as usize] = block;
            blocks.push(block);
        }

        // impute block references
        for block_id in &blocks {
            self.impute_block_terminators(*block_id, &source_index_to_block);
        }
        let source_index_to_local: Vec<_> = locals.clone();
        for block_id in &blocks {
            self.impute_local_references(*block_id, &source_index_to_local);
        }

        self.eat_token(TokenType::CloseBrace)?;

        // update the function with the parsed body
        let entry = blocks
            .first()
            .copied()
            .ok_or_else(|| ParseError::new("function must have at least one block", self.pos()))?;

        let next_value_id = parameters.iter().map(|p| p.value.0 + 1).max().unwrap_or(0);

        let function = self.tree.get_mut(id);
        function.locals = locals;
        function.blocks = blocks;
        function.entry = Some(entry);
        function.next_value_id = next_value_id;

        Ok(id)
    }

    /// Parse extern function parameter list (types only, no value names).
    /// Returns TypedValue with synthetic value IDs.
    fn parse_extern_parameter_list(&mut self) -> ParseResult<Vec<TypedValue>> {
        let mut parameter_types = Vec::new();
        while !self.peek_token(TokenType::CloseParen) {
            let ty = self.parse_type()?;
            parameter_types.push(ty);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        // create typed parameters (with synthetic values)
        Ok(parameter_types
            .iter()
            .enumerate()
            .map(|(i, &ty)| TypedValue {
                value: Value::new(i as u32),
                ty,
            })
            .collect())
    }

    /// Parse a local variable declaration.
    fn parse_local(&mut self) -> ParseResult<LocalNodeId<Local>> {
        let local_token = self.eat_token(TokenType::LocalReference)?;
        let _local_idx: u32 = local_token
            .text
            .strip_prefix("local")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("local reference", local_token.start))?;

        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;

        // parse annotations: ; owned, var
        let mut ownership = Ownership::Owned;
        let mut mutability = Mutability::Immutable;

        if self.eat_token_maybe(TokenType::Semicolon) {
            // ownership
            if self.peek_token(TokenType::Ownership) {
                let text = self.span_str();
                ownership = match text {
                    "owned" => Ownership::Owned,
                    "borrowed" => Ownership::Borrowed,
                    "copy" => Ownership::Copy,
                    _ => Ownership::Owned,
                };
                self.bump();
            }
            // mutability
            if self.eat_token_maybe(TokenType::Comma) && self.eat_token_maybe(TokenType::Var) {
                mutability = Mutability::Mutable;
            }
        }

        let local = Local::new(ty, mutability, ownership);
        Ok(self.tree.insert(local))
    }

    /// Parse a basic block. Returns (block_id, source_index).
    fn parse_block(&mut self) -> ParseResult<(LocalNodeId<Block>, u32)> {
        let block_token = self.eat_token(TokenType::BlockRefence)?;
        let block_name = block_token.text.to_string();
        let source_idx: u32 = block_name
            .strip_prefix("block")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("block reference", block_token.start))?;

        // parameters
        let parameters = if self.eat_token_maybe(TokenType::OpenParen) {
            let params = self.parse_typed_value_list()?;
            self.eat_token(TokenType::CloseParen)?;
            params
        } else {
            Vec::new()
        };

        self.eat_token(TokenType::Colon)?;

        // instructions and terminator
        let mut instructions = Vec::new();
        let mut terminator = None;

        while !self.peek_token(TokenType::BlockRefence)
            && !self.peek_token(TokenType::CloseBrace)
            && !self.peek_token(TokenType::End)
        {
            // check for terminator keywords
            if self.peek_token(TokenType::Return)
                || self.peek_token(TokenType::Jump)
                || self.peek_token(TokenType::Branch)
                || self.peek_token(TokenType::Switch)
                || self.peek_token(TokenType::Unreachable)
            {
                terminator = Some(self.parse_terminator()?);
                break;
            }

            // otherwise, parse instruction
            let inst = self.eat_instruction()?;
            instructions.push(inst);
        }

        // block
        let block = Block {
            parameters,
            instructions,
            terminator: terminator.unwrap_or(Terminator::Unreachable),
        };

        let id = self.tree.insert(block);
        self.block_map.insert(block_name, id);
        Ok((id, source_idx))
    }

    /// Fixup block references in terminators after all blocks are parsed.
    fn impute_block_terminators(
        &mut self,
        block_id: LocalNodeId<Block>,
        source_to_actual: &[LocalNodeId<Block>],
    ) {
        let block = self.tree.get_mut(block_id);

        fn resolve(idx: LocalNodeId<Block>, mapping: &[LocalNodeId<Block>]) -> LocalNodeId<Block> {
            // idx.id is the source index, look up the actual ID
            mapping.get(idx.id as usize).copied().unwrap_or(idx)
        }

        match &mut block.terminator {
            Terminator::Return { .. } | Terminator::Unreachable => {}
            Terminator::Jump { target, .. } => {
                *target = resolve(*target, source_to_actual);
            }
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                *then_target = resolve(*then_target, source_to_actual);
                *else_target = resolve(*else_target, source_to_actual);
            }
            Terminator::Switch { default, cases, .. } => {
                *default = resolve(*default, source_to_actual);
                for case in cases.iter_mut() {
                    case.target = resolve(case.target, source_to_actual);
                }
            }
            Terminator::Yield { resume, .. } => {
                *resume = resolve(*resume, source_to_actual);
            }
        }
    }

    /// Fixup local references in instructions after all locals are parsed.
    fn impute_local_references(
        &mut self,
        block_id: LocalNodeId<Block>,
        source_to_actual: &[LocalNodeId<Local>],
    ) {
        fn resolve(idx: LocalNodeId<Local>, mapping: &[LocalNodeId<Local>]) -> LocalNodeId<Local> {
            mapping.get(idx.id as usize).copied().unwrap_or(idx)
        }

        let block = self.tree.get(block_id);
        let instructions = block.instructions.clone();

        for inst_id in instructions {
            let inst = self.tree.get_mut(inst_id);
            match inst {
                Instruction::LocalGet { local, .. } => {
                    *local = resolve(*local, source_to_actual);
                }
                Instruction::LocalSet { local, .. } => {
                    *local = resolve(*local, source_to_actual);
                }
                _ => {}
            }
        }
    }

    /// Parse an instruction.
    ///
    /// Instructions come in two forms:
    /// - With destination: `vN = opcode ...`
    /// - Without destination: `opcode ...`
    fn eat_instruction(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        // check if this is an instruction with destination (vN = ...)
        let has_destination = self.peek_token(TokenType::Value);
        if has_destination {
            self.eat_instruction_with_destination()
        } else {
            self.eat_instruction_without_destination()
        }
    }

    /// Parse an instruction that produces a value: `vN = opcode ...`
    fn eat_instruction_with_destination(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        let destination = self.parse_value()?;
        self.eat_token(TokenType::Equals)?;

        let opcode = self.eat_token(TokenType::Identifier)?;
        let opcode_text = opcode.text;

        let instruction = match opcode_text {
            // constant
            "iconst" => {
                let value = self.parse_constant()?;
                Instruction::Const { destination, value }
            }

            // binary ops
            _ if opcode_text.parse::<BinaryOperator>().is_ok() => {
                let operator = opcode_text.parse().unwrap();
                let left = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let right = self.parse_value()?;
                Instruction::Binary {
                    destination,
                    operator,
                    left,
                    right,
                }
            }

            // unary ops
            _ if opcode_text.parse::<UnaryOperator>().is_ok() => {
                let operator = opcode_text.parse().unwrap();
                let argument = self.parse_value()?;
                Instruction::Unary {
                    destination,
                    operator,
                    argument,
                }
            }

            // cast ops
            _ if opcode_text.parse::<CastKind>().is_ok() => {
                let kind = opcode_text.parse().unwrap();
                let argument = self.parse_value()?;
                self.eat_token(TokenType::Arrow)?;
                let to_type = self.parse_type()?;
                Instruction::Cast {
                    destination,
                    kind,
                    argument,
                    to_type,
                }
            }

            // local operations
            "local.get" => {
                let local = self.parse_local_ref()?;
                Instruction::LocalGet { destination, local }
            }

            // global operations
            "global.addr" => {
                let global = self.parse_global_reference()?;
                Instruction::GlobalAddr {
                    destination,
                    global,
                }
            }
            "global.const" => {
                let global = self.parse_global_reference()?;
                Instruction::GlobalConst {
                    destination,
                    global,
                }
            }

            // memory operations
            "load" => {
                let pointer = self.parse_value()?;
                Instruction::Load {
                    destination,
                    pointer,
                }
            }

            // aggregate operations
            "field.get" => {
                let aggregate = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let index = self.parse_int_literal()? as u32;
                Instruction::FieldGet {
                    destination,
                    aggregate,
                    index,
                }
            }
            "field.set" => {
                let aggregate = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let index = self.parse_int_literal()? as u32;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::FieldSet {
                    destination,
                    aggregate,
                    index,
                    value,
                }
            }
            "element.get" => {
                let array = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let index = self.parse_value()?;
                Instruction::ElementGet {
                    destination,
                    array,
                    index,
                }
            }
            "element.set" => {
                let array = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let index = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::ElementSet {
                    destination,
                    array,
                    index,
                    value,
                }
            }

            // function calls
            "call" => {
                let function = self.parse_function_reference()?;
                let arguments = self.parse_call_arguments()?;
                Instruction::Call {
                    destination: Some(destination),
                    function,
                    arguments,
                }
            }
            "call.indirect" => {
                let callee = self.parse_value()?;
                let arguments = self.parse_call_arguments()?;
                Instruction::CallIndirect {
                    destination: Some(destination),
                    callee,
                    arguments,
                }
            }

            // allocation operations
            "managed.alloc" => {
                let layout = self.parse_type()?;
                Instruction::ManagedAlloc {
                    destination,
                    layout,
                }
            }
            "managed.alloc_array" => {
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                Instruction::ManagedAllocArray {
                    destination,
                    element,
                    length,
                }
            }
            "raw.alloc" => {
                let layout = self.parse_type()?;
                Instruction::RawAlloc {
                    destination,
                    layout,
                }
            }
            "stack.alloc" => {
                let layout = self.parse_type()?;
                Instruction::StackAlloc {
                    destination,
                    layout,
                }
            }

            // intrinsics: intrinsic.{name}(args)
            _ if opcode_text.starts_with("intrinsic.") => {
                let opcode_start = opcode.start;
                let intrinsic = parse_intrinsic_name(opcode_text, opcode_start)?;
                let arguments = self.parse_call_arguments()?;
                Instruction::Intrinsic {
                    destination: Some(destination),
                    intrinsic,
                    arguments,
                }
            }

            _ => {
                return Err(ParseError::invalid(
                    &format!("instruction '{opcode_text}'"),
                    opcode.start,
                ));
            }
        };

        Ok(self.tree.insert(instruction))
    }

    /// Parse an instruction without a destination: `opcode ...`
    fn eat_instruction_without_destination(&mut self) -> ParseResult<LocalNodeId<Instruction>> {
        let opcode = self.eat_token(TokenType::Identifier)?;
        let opcode_text = opcode.text;

        let instruction = match opcode_text {
            // local operations
            "local.set" => {
                let local = self.parse_local_ref()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::LocalSet { local, value }
            }

            // memory operations
            "store" => {
                let pointer = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let value = self.parse_value()?;
                Instruction::Store { pointer, value }
            }

            // void calls
            "call" => {
                let function = self.parse_function_reference()?;
                let arguments = self.parse_call_arguments()?;
                Instruction::Call {
                    destination: None,
                    function,
                    arguments,
                }
            }
            "call.indirect" => {
                let callee = self.parse_value()?;
                let arguments = self.parse_call_arguments()?;
                Instruction::CallIndirect {
                    destination: None,
                    callee,
                    arguments,
                }
            }

            // allocation operations (no destination)
            "raw.free" => {
                let pointer = self.parse_value()?;
                Instruction::RawFree { pointer }
            }

            // void intrinsics: intrinsic.{name}(args)
            _ if opcode_text.starts_with("intrinsic.") => {
                let opcode_start = opcode.start;
                let intrinsic = parse_intrinsic_name(opcode_text, opcode_start)?;
                let arguments = self.parse_call_arguments()?;
                Instruction::Intrinsic {
                    destination: None,
                    intrinsic,
                    arguments,
                }
            }

            _ => {
                return Err(ParseError::invalid(
                    &format!("instruction '{opcode_text}'"),
                    opcode.start,
                ));
            }
        };

        Ok(self.tree.insert(instruction))
    }

    /// Parse a terminator.
    fn parse_terminator(&mut self) -> ParseResult<Terminator> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("terminator", self.pos()))?;

        match token.ty {
            TokenType::Return => {
                self.bump();
                let value = if self.peek_token(TokenType::Value) {
                    Some(self.parse_value()?)
                } else {
                    None
                };
                Ok(Terminator::Return { value })
            }

            TokenType::Jump => {
                self.bump();
                let target = self.parse_block_ref()?;
                let arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                Ok(Terminator::Jump { target, arguments })
            }

            TokenType::Branch => {
                self.bump();
                let condition = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let then_target = self.parse_block_ref()?;
                let then_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                self.eat_token(TokenType::Comma)?;
                let else_target = self.parse_block_ref()?;
                let else_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };
                Ok(Terminator::Branch {
                    condition,
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                })
            }

            TokenType::Switch => {
                self.bump();
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let default = self.parse_block_ref()?;
                let default_arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                    let args = self.parse_value_list()?;
                    self.eat_token(TokenType::CloseParen)?;
                    args
                } else {
                    Vec::new()
                };

                // parse cases: value => blockN(args), ...
                let mut cases = Vec::new();
                while self.eat_token_maybe(TokenType::Comma) {
                    let case_value = self.parse_int_literal()?;
                    self.eat_token(TokenType::FatArrow)?;
                    let target = self.parse_block_ref()?;
                    let arguments = if self.eat_token_maybe(TokenType::OpenParen) {
                        let args = self.parse_value_list()?;
                        self.eat_token(TokenType::CloseParen)?;
                        args
                    } else {
                        Vec::new()
                    };
                    cases.push(SwitchCase {
                        value: case_value,
                        target,
                        arguments,
                    });
                }

                Ok(Terminator::Switch {
                    value,
                    default,
                    default_arguments,
                    cases,
                })
            }

            TokenType::Unreachable => {
                self.bump();
                Ok(Terminator::Unreachable)
            }

            _ => Err(ParseError::unexpected("terminator", token.ty, token.start)),
        }
    }

    /// Parse a type.
    fn parse_type(&mut self) -> ParseResult<LocalNodeId<Type>> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("type", self.pos()))?;
        let token_ty = token.ty;
        let token_text = token.text.to_string();
        let token_start = token.start;

        let ty = match token_ty {
            TokenType::Void => {
                self.bump();
                Type::Void
            }
            TokenType::Bool => {
                self.bump();
                Type::Boolean
            }
            TokenType::TypeName => {
                self.bump();
                parse_primitive_type(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("type '{token_text}'"), token_start)
                })?
            }
            TokenType::RawPtr => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let pointee = self.parse_type()?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::RawPointer { pointee }
            }
            TokenType::Ref => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let pointee = self.parse_type()?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::ManagedReference {
                    pointee,
                    is_nullable: false,
                }
            }
            TokenType::RefNullable => {
                self.bump();
                self.eat_token(TokenType::LessThan)?;
                let pointee = self.parse_type()?;
                self.eat_token(TokenType::GreaterThan)?;
                Type::ManagedReference {
                    pointee,
                    is_nullable: true,
                }
            }
            TokenType::OpenBracket => {
                self.bump();
                let element = self.parse_type()?;
                self.eat_token(TokenType::Semicolon)?;
                let length = self.parse_int_literal()? as u64;
                self.eat_token(TokenType::CloseBracket)?;
                Type::Array { element, length }
            }
            TokenType::OpenParen => {
                self.bump();
                let mut elements = Vec::new();
                while !self.peek_token(TokenType::CloseParen) {
                    elements.push(self.parse_type()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseParen)?;
                Type::Tuple { elements }
            }
            TokenType::Fn => {
                self.bump();
                self.eat_token(TokenType::OpenParen)?;
                let mut parameters = Vec::new();
                while !self.peek_token(TokenType::CloseParen) {
                    parameters.push(self.parse_type()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseParen)?;
                self.eat_token(TokenType::Arrow)?;
                let result = self.parse_type()?;
                Type::FunctionPointer { parameters, result }
            }
            _ => {
                return Err(ParseError::unexpected("type", token_ty, token_start));
            }
        };

        Ok(self.tree.insert(ty))
    }

    /// Parse a value reference (vN).
    fn parse_value(&mut self) -> ParseResult<Value> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value", self.pos()))?;
        if token.ty != TokenType::Value {
            return Err(ParseError::unexpected("value", token.ty, token.start));
        }
        let text = token.text.to_string();
        let start = token.start;
        self.bump();

        let idx: u32 = text
            .strip_prefix('v')
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("value reference", start))?;
        Ok(Value::new(idx))
    }

    /// Parse a block reference.
    fn parse_block_ref(&mut self) -> ParseResult<LocalNodeId<Block>> {
        let token = self.eat_token(TokenType::BlockRefence)?;
        let idx: u32 = token
            .text
            .strip_prefix("block")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("block reference", token.start))?;
        Ok(LocalNodeId::new(idx))
    }

    /// Parse a local reference.
    fn parse_local_ref(&mut self) -> ParseResult<LocalNodeId<Local>> {
        let token = self.eat_token(TokenType::LocalReference)?;
        let idx: u32 = token
            .text
            .strip_prefix("local")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("local reference", token.start))?;
        Ok(LocalNodeId::new(idx))
    }

    /// Parse a function reference (@name).
    ///
    /// The function must be declared (either defined or extern) in this module.
    fn parse_function_reference(&mut self) -> ParseResult<LocalNodeId<Function>> {
        self.eat_token(TokenType::At)?;
        let name_token = self.eat_token(TokenType::Identifier)?;
        let name = name_token.text.to_string();
        let start = name_token.start;

        self.function_map
            .get(&name)
            .copied()
            .ok_or_else(|| ParseError::invalid(&format!("function reference '@{name}'"), start))
    }

    /// Parse a global reference (@name).
    ///
    /// The global must be declared (either defined or extern) in this module.
    fn parse_global_reference(&mut self) -> ParseResult<LocalNodeId<Global>> {
        self.eat_token(TokenType::At)?;
        let name_token = self.eat_token(TokenType::Identifier)?;
        let name = name_token.text.to_string();
        let start = name_token.start;

        self.global_map
            .get(&name)
            .copied()
            .ok_or_else(|| ParseError::invalid(&format!("global reference '@{name}'"), start))
    }

    /// Parse call arguments: (v0, v1, ...).
    fn parse_call_arguments(&mut self) -> ParseResult<Vec<Value>> {
        self.eat_token(TokenType::OpenParen)?;
        let args = self.parse_value_list()?;
        self.eat_token(TokenType::CloseParen)?;
        Ok(args)
    }

    /// Parse a comma-separated list of values.
    fn parse_value_list(&mut self) -> ParseResult<Vec<Value>> {
        let mut values = Vec::new();
        while self.peek_token(TokenType::Value) {
            values.push(self.parse_value()?);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }

    /// Parse a comma-separated list of typed values.
    fn parse_typed_value_list(&mut self) -> ParseResult<Vec<TypedValue>> {
        let mut values = Vec::new();
        while self.peek_token(TokenType::Value) {
            let value = self.parse_value()?;
            self.eat_token(TokenType::Colon)?;
            let ty = self.parse_type()?;
            values.push(TypedValue::new(value, ty));
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }
        Ok(values)
    }

    /// Parse a constant.
    fn parse_constant(&mut self) -> ParseResult<Constant> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("constant", self.pos()))?;
        let token_ty = token.ty;
        let token_text = token.text.to_string();
        let token_start = token.start;

        match token_ty {
            TokenType::BoolLiteral => {
                let value = token_text == "true";
                self.bump();
                Ok(Constant::Boolean { value })
            }
            TokenType::IntLiteral => {
                self.bump();
                parse_int_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("integer constant '{token_text}'"), token_start)
                })
            }
            TokenType::FloatLiteral => {
                self.bump();
                parse_float_constant(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("float constant '{token_text}'"), token_start)
                })
            }
            TokenType::StringLiteral => {
                self.bump();
                let value = parse_string_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::String { value })
            }
            TokenType::CharLiteral => {
                self.bump();
                let value = parse_char_literal(&token_text).ok_or_else(|| {
                    ParseError::invalid(&format!("char literal '{token_text}'"), token_start)
                })?;
                Ok(Constant::Char { value })
            }
            _ => Err(ParseError::unexpected("constant", token_ty, token_start)),
        }
    }

    /// Parse an integer literal (just the number, no type suffix).
    fn parse_int_literal(&mut self) -> ParseResult<i64> {
        let token = self.eat_token(TokenType::IntLiteral)?;
        // strip type suffix and parse
        let text = token.text;
        let digits: String = text
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '-')
            .collect();
        digits
            .parse()
            .map_err(|_| ParseError::invalid("integer", token.start))
    }
}

/// Parse a primitive type from string.
fn parse_primitive_type(s: &str) -> Option<Type> {
    Some(match s {
        "i8" => Type::Int {
            width: 8,
            signed: true,
        },
        "i16" => Type::Int {
            width: 16,
            signed: true,
        },
        "i32" => Type::Int {
            width: 32,
            signed: true,
        },
        "i64" => Type::Int {
            width: 64,
            signed: true,
        },
        "i128" => Type::Int {
            width: 128,
            signed: true,
        },
        "i256" => Type::Int {
            width: 256,
            signed: true,
        },
        "u8" => Type::Int {
            width: 8,
            signed: false,
        },
        "u16" => Type::Int {
            width: 16,
            signed: false,
        },
        "u32" => Type::Int {
            width: 32,
            signed: false,
        },
        "u64" => Type::Int {
            width: 64,
            signed: false,
        },
        "u128" => Type::Int {
            width: 128,
            signed: false,
        },
        "u256" => Type::Int {
            width: 256,
            signed: false,
        },
        "f32" => Type::Float { width: 32 },
        "f64" => Type::Float { width: 64 },
        _ => return None,
    })
}

/// Parse an integer constant with type suffix.
fn parse_int_constant(s: &str) -> Option<Constant> {
    // find where the type suffix starts
    let suffix_start = s.find(|c: char| c.is_ascii_alphabetic())?;
    let (digits, suffix) = s.split_at(suffix_start);

    let is_signed = suffix.starts_with('i');
    let width: u8 = suffix[1..].parse().ok()?;

    if is_signed {
        let value: i64 = digits.parse().ok()?;
        Some(Constant::Int {
            value,
            width,
            is_signed: true,
        })
    } else {
        let value: u64 = digits.parse().ok()?;
        Some(Constant::UInt { value, width })
    }
}

/// Parse a float constant with type suffix.
fn parse_float_constant(s: &str) -> Option<Constant> {
    let suffix_start = s.rfind('f')?;
    let (digits, suffix) = s.split_at(suffix_start);
    let width: u8 = suffix[1..].parse().ok()?;

    if width == 32 {
        let value: f32 = digits.parse().ok()?;
        Some(Constant::Float {
            bits: value.to_bits() as u64,
            width,
        })
    } else {
        let value: f64 = digits.parse().ok()?;
        Some(Constant::Float {
            bits: value.to_bits(),
            width,
        })
    }
}

/// Parse a string literal, handling escape sequences.
fn parse_string_literal(s: &str) -> Option<String> {
    let s = s.strip_prefix('"')?.strip_suffix('"')?;
    parse_escape_sequences(s)
}

/// Parse a char literal, handling escape sequences.
fn parse_char_literal(s: &str) -> Option<char> {
    let s = s.strip_prefix('\'')?.strip_suffix('\'')?;
    let unescaped = parse_escape_sequences(s)?;
    let mut chars = unescaped.chars();
    let c = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(c)
}

/// Parse escape sequences in a string.
fn parse_escape_sequences(s: &str) -> Option<String> {
    let mut result = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            let escaped = chars.next()?;
            let replacement = match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '\\' => '\\',
                '"' => '"',
                '\'' => '\'',
                '0' => '\0',
                'x' => {
                    // \xHH - two hex digits
                    let h1 = chars.next()?.to_digit(16)?;
                    let h2 = chars.next()?.to_digit(16)?;
                    char::from_u32(h1 * 16 + h2)?
                }
                'u' => {
                    // \u{HHHH} - Unicode escape
                    if chars.next()? != '{' {
                        return None;
                    }
                    let mut value = 0u32;
                    loop {
                        match chars.next()? {
                            '}' => break,
                            c => {
                                let digit = c.to_digit(16)?;
                                value = value * 16 + digit;
                            }
                        }
                    }
                    char::from_u32(value)?
                }
                _ => return None,
            };
            result.push(replacement);
        } else {
            result.push(c);
        }
    }

    Some(result)
}

/// Parse an intrinsic name from an opcode like `intrinsic.sqrt`.
fn parse_intrinsic_name(opcode_text: &str, pos: usize) -> ParseResult<Intrinsic> {
    let name = opcode_text
        .strip_prefix("intrinsic.")
        .ok_or_else(|| ParseError::invalid("intrinsic opcode", pos))?;
    name.parse::<Intrinsic>()
        .map_err(|_| ParseError::invalid(&format!("intrinsic '{name}'"), pos))
}
