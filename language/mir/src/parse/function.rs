use crate::{
    AllocationMode, Attribute, Block, CallBehavior, CallSite, Function, Instruction, Lifetime,
    Linkage, Local, LocalNodeId, MemoryEffect, Mutability, Ownership, PointerAttributes,
    Terminator, Type, TypedValue, Value,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Parse a function definition or declaration.
    pub(super) fn parse_function(
        &mut self,
        linkage: Linkage,
        attributes: Vec<Attribute>,
    ) -> ParseResult<LocalNodeId<Function>> {
        // function keyword and name
        self.eat_token(TokenType::Function)?;
        self.eat_token(TokenType::At)?;

        // function name
        let file_id = self.file_id;
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = Self::span_at(file_id, name_start, name.len());
        let function_id = *self
            .function_map
            .get(&name)
            .unwrap_or_else(|| panic!("function @{name} should be pre-registered"));
        self.current_function = Some(function_id);

        // function attributes
        let (execution_model, execution_stage, workgroup_size, closure_env_type) =
            self.resolve_function_attributes(&attributes)?;

        // parameters
        self.eat_token(TokenType::OpenParen)?;
        let parameters = if linkage.is_import() {
            // extern parameters
            self.parse_extern_parameter_list()?
        } else {
            // definition parameters
            self.parse_typed_value_list()?
        };
        self.eat_token(TokenType::CloseParen)?;

        // return type
        self.eat_token(TokenType::Arrow)?;
        let return_type = self.parse_type()?;

        // extern function body
        if linkage.is_import() {
            let name_id = self.strings.intern(&name);
            let parameter_attributes = vec![PointerAttributes::default(); parameters.len()];
            let parameter_count = parameters.len();
            let value_types = seed_value_types(&parameters);
            let next_value_id = value_types.len() as u32;
            let function = Function {
                name: name_id,
                parameters,
                parameter_names: vec![None; parameter_count],
                value_types,
                return_type,
                return_lifetime: Lifetime::Inferred,
                memory_effects: MemoryEffect::unknown(),
                call_behavior: CallBehavior::unknown(),
                alloc_size: None,
                parameter_attributes,
                return_attributes: PointerAttributes::default(),
                linkage,
                allocation: AllocationMode::Any, // #Incomplete: set proper MIR allocation mode?
                coroutine: None,
                execution_model,
                execution_stage,
                workgroup_size,
                closure_env_type,
                locals: Vec::new(),
                blocks: Vec::new(),
                entry: None,
                next_value_id,
            };

            // update the placeholder with the parsed signature
            self.tree.set_span(function_id, name_span);
            *self.tree.get_mut(function_id) = function;
            self.current_function = None;

            // record attributes
            if !attributes.is_empty() {
                self.tree.set_attributes(function_id, attributes);
            }

            return Ok(function_id);
        }

        // update the placeholder signature
        let name_id = self.strings.intern(&name);
        let id = function_id;
        self.tree.set_span(id, name_span);

        // populate signature fields
        let function = self.tree.get_mut(id);
        function.name = name_id;
        function.parameters = parameters.clone();
        function.parameter_names = vec![None; parameters.len()];
        function.value_types = seed_value_types(&parameters);
        function.return_type = return_type;
        function.linkage = linkage;
        function.memory_effects = MemoryEffect::unknown();
        function.call_behavior = CallBehavior::unknown();
        function.alloc_size = None;
        function.parameter_attributes = vec![PointerAttributes::default(); parameters.len()];
        function.return_attributes = PointerAttributes::default();
        function.execution_model = execution_model;
        function.execution_stage = execution_stage;
        function.workgroup_size = workgroup_size;
        function.closure_env_type = closure_env_type;

        // body
        self.eat_token(TokenType::OpenBrace)?;

        // locals
        let mut locals = Vec::new();
        while self.peek_token(TokenType::LocalReference) {
            let local = self.parse_local()?;
            locals.push(local);
        }

        // blocks and source index mapping
        let mut blocks = Vec::new();
        let mut source_index_to_block: Vec<LocalNodeId<Block>> = Vec::new();
        while self.peek_token(TokenType::BlockRefence) {
            let (block, source_idx) = self.parse_block()?;
            // mapping expansion
            while source_index_to_block.len() <= source_idx as usize {
                source_index_to_block.push(LocalNodeId::new(u32::MAX));
            }
            source_index_to_block[source_idx as usize] = block;
            blocks.push(block);
        }

        // impute references
        for block_id in &blocks {
            self.impute_block_terminators(*block_id, &source_index_to_block);
        }
        let source_index_to_local: Vec<_> = locals.clone();
        for block_id in &blocks {
            self.impute_local_references(*block_id, &source_index_to_local);
        }
        self.eat_token(TokenType::CloseBrace)?;

        // finalize body
        let entry = blocks
            .first()
            .copied()
            .ok_or_else(|| ParseError::new("function must have at least one block", self.pos()))?;

        let function = self.tree.get_mut(id);
        function.locals = locals;
        function.blocks = blocks;
        function.entry = Some(entry);
        function.next_value_id = function.value_types.len() as u32;

        self.current_function = None;

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse extern function parameter list with synthetic values.
    /// Return typed values for the external signature.
    fn parse_extern_parameter_list(&mut self) -> ParseResult<Vec<TypedValue>> {
        // collect parameter types
        let mut parameter_types = Vec::new();
        while !self.peek_token(TokenType::CloseParen) {
            let ty = self.parse_type()?;
            parameter_types.push(ty);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        // assign synthetic values
        let parameters: Vec<_> = parameter_types
            .iter()
            .enumerate()
            .map(|(i, &ty)| TypedValue {
                value: Value::new(i as u32),
                ty,
            })
            .collect();

        Ok(parameters)
    }

    /// Parse a local variable declaration.
    fn parse_local(&mut self) -> ParseResult<LocalNodeId<Local>> {
        // local reference
        let local_token = self.eat_token(TokenType::LocalReference)?;
        let _local_idx: u32 = local_token
            .text
            .strip_prefix("local")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("local reference", local_token.start))?;

        // local type
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_type()?;

        // local annotations
        let mut ownership = Ownership::Owned;
        let mut mutability = Mutability::Mutable;

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
            if self.eat_token_maybe(TokenType::Comma)
                && (self.eat_token_maybe(TokenType::Readonly)
                    || self.eat_token_maybe(TokenType::Const))
            {
                mutability = Mutability::Immutable;
            }
        }

        // record the local
        let local = Local::new(ty, mutability, ownership);
        Ok(self.tree.insert(local))
    }

    /// Parse a basic block and return (block_id, source_index).
    fn parse_block(&mut self) -> ParseResult<(LocalNodeId<Block>, u32)> {
        // block header
        let (block_name, block_start, block_span) = {
            let file_id = self.file_id;
            let block_token = self.eat_token(TokenType::BlockRefence)?;
            (
                block_token.text.to_string(),
                block_token.start,
                Self::span_for_token(file_id, block_token),
            )
        };
        let source_idx: u32 = block_name
            .strip_prefix("block")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("block reference", block_start))?;

        // block parameters
        let parameters = if self.eat_token_maybe(TokenType::OpenParen) {
            let params = self.parse_typed_value_list()?;
            self.eat_token(TokenType::CloseParen)?;
            params
        } else {
            Vec::new()
        };

        // record block parameter value types
        for param in &parameters {
            self.record_value_type(param.value, param.ty);
        }

        self.eat_token(TokenType::Colon)?;

        // block contents
        let mut instructions = Vec::new();
        let mut terminator = None;
        let mut terminator_dispatch_facts = None;

        while !self.peek_token(TokenType::BlockRefence)
            && !self.peek_token(TokenType::CloseBrace)
            && !self.peek_token(TokenType::End)
        {
            // direct terminators
            if self.peek_token(TokenType::Return)
                || self.peek_token(TokenType::Jump)
                || self.peek_token(TokenType::Branch)
                || self.peek_token(TokenType::Check)
                || self.peek_token(TokenType::Switch)
                || self.peek_token(TokenType::Yield)
                || self.peek_token(TokenType::Throw)
                || self.peek_token(TokenType::Trap)
                || self.peek_token(TokenType::Unreachable)
                || self.peek_token(TokenType::TailCall)
                || self.peek_token(TokenType::TailCallIndirect)
                || self.peek_token(TokenType::TailCallVirtual)
                || self.peek_token(TokenType::TailCallInterface)
            {
                let (parsed_terminator, dispatch_facts) = self.parse_terminator()?;
                terminator = Some(parsed_terminator);
                terminator_dispatch_facts = dispatch_facts;
                break;
            }

            // exceptional call terminators
            if self.peek_token(TokenType::Call)
                || self.peek_token(TokenType::CallIndirect)
                || self.peek_token(TokenType::CallVirtual)
                || self.peek_token(TokenType::CallInterface)
            {
                let checkpoint = self.pos;

                match self.parse_terminator() {
                    Ok((parsed_terminator, dispatch_facts)) => {
                        terminator = Some(parsed_terminator);
                        terminator_dispatch_facts = dispatch_facts;
                        break;
                    }
                    Err(error) if self.is_call_instruction_error(&error) => {
                        self.pos = checkpoint;
                    }
                    Err(error) => {
                        return Err(error);
                    }
                }
            }

            // instruction
            let inst = self.eat_instruction()?;
            instructions.push(inst);
        }

        // finalize block
        let block = Block {
            parameters,
            instructions,
            terminator: terminator.unwrap_or(Terminator::Unreachable),
        };

        let id = self.tree.insert(block);
        self.tree.set_span(id, block_span);
        if let Some(facts) = terminator_dispatch_facts {
            self.tree
                .dispatch_table
                .insert_callsite_metadata(CallSite::Terminator(id), facts);
        }
        self.block_map.insert(block_name, id);

        Ok((id, source_idx))
    }

    /// Return whether a call-terminator parse error should fall back to a call instruction.
    fn is_call_instruction_error(&self, error: &super::error::ParseError) -> bool {
        error.message.starts_with("expected Identifier, got Arrow")
            || error.message.starts_with("expected Identifier, got Fn")
            || error.message.starts_with("expected Identifier, got Return")
            || error.message == "invalid normal"
            || error.message == "invalid unwind"
    }

    /// Fix block references in terminators after parsing blocks.
    fn impute_block_terminators(
        &mut self,
        block_id: LocalNodeId<Block>,
        source_to_actual: &[LocalNodeId<Block>],
    ) {
        // resolve block id mapping
        let block = self.tree.get_mut(block_id);

        fn resolve(idx: LocalNodeId<Block>, mapping: &[LocalNodeId<Block>]) -> LocalNodeId<Block> {
            mapping.get(idx.id as usize).copied().unwrap_or(idx)
        }

        // rewrite terminator targets
        match &mut block.terminator {
            Terminator::Return { .. }
            | Terminator::Throw { .. }
            | Terminator::Trap { .. }
            | Terminator::Unreachable
            | Terminator::TailCall { .. }
            | Terminator::TailCallIndirect { .. }
            | Terminator::TailCallVirtual { .. }
            | Terminator::TailCallInterface { .. } => {}
            Terminator::Call {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::CallIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::CallVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::CallInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                *normal_target = resolve(*normal_target, source_to_actual);
                *unwind_target = resolve(*unwind_target, source_to_actual);
            }
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
            Terminator::Check {
                success, failure, ..
            } => {
                success.target = resolve(success.target, source_to_actual);
                failure.target = resolve(failure.target, source_to_actual);
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

    /// Fix local references in instructions after parsing locals.
    fn impute_local_references(
        &mut self,
        block_id: LocalNodeId<Block>,
        source_to_actual: &[LocalNodeId<Local>],
    ) {
        fn resolve(idx: LocalNodeId<Local>, mapping: &[LocalNodeId<Local>]) -> LocalNodeId<Local> {
            mapping.get(idx.id as usize).copied().unwrap_or(idx)
        }

        // clone to avoid borrow issues while mutating
        let block = self.tree.get(block_id);
        let instructions = block.instructions.clone();

        // update local references in instructions
        for inst_id in instructions {
            let inst = self.tree.get_mut(inst_id);
            match inst {
                Instruction::LocalGet { local, .. }
                | Instruction::LocalAddr { local, .. }
                | Instruction::LocalSet { local, .. } => {
                    *local = resolve(*local, source_to_actual);
                }
                _ => {}
            }
        }
    }
}

/// Seed a value type table from typed parameters.
fn seed_value_types(parameters: &[TypedValue]) -> Vec<LocalNodeId<Type>> {
    // ensure parameters cover a dense range
    let mut ordered: Vec<_> = parameters.iter().collect();
    ordered.sort_by_key(|param| param.value.0);

    let mut value_types = Vec::with_capacity(ordered.len());
    for (expected, param) in ordered.into_iter().enumerate() {
        if param.value.0 as usize != expected {
            panic!("missing value type for v{expected}");
        }

        value_types.push(param.ty);
    }

    value_types
}
