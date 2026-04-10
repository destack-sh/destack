use crate::{
    AllocationMode, Attribute, AttributeArgs, AttributeKeyValue, AttributeValue, Block, Call,
    CallBehavior, CheckConstraint, CheckTarget, ExecutionModel, ExecutionStage, Function,
    Instruction, Lifetime, Linkage, Local, LocalNodeId, MemoryEffect, Mutability, Ownership,
    PointerAttribute, SwitchCase, Terminator, TrapKind, Type, TypedValue, Value,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

impl<'a> Parser<'a> {
    /// Resolve attributes into function metadata.
    pub(super) fn resolve_function_attributes(
        &mut self,
        attributes: &[Attribute],
    ) -> ParseResult<(
        Option<ExecutionModel>,
        Option<ExecutionStage>,
        Option<[u32; 3]>,
        Option<LocalNodeId<Type>>,
    )> {
        // metadata outputs
        let mut execution_model = None;
        let mut execution_stage = None;
        let mut workgroup_size = None;
        let mut environment_type = None;

        // inspect attributes
        for attribute in attributes {
            let name = {
                let name = self.strings.get(attribute.name);
                name.to_string()
            };
            match name.as_str() {
                "executionModel" => {
                    if execution_model.is_some() {
                        return Err(ParseError::new(
                            "duplicate executionModel attribute",
                            self.pos(),
                        ));
                    }

                    let value = match &attribute.args {
                        AttributeArgs::Value(AttributeValue::Identifier(value)) => {
                            self.strings.get(*value)
                        }
                        AttributeArgs::Value(AttributeValue::String(value)) => {
                            self.strings.get(*value)
                        }
                        _ => {
                            return Err(ParseError::new(
                                "executionModel expects an identifier",
                                self.pos(),
                            ));
                        }
                    };
                    let model = ExecutionModel::try_from(value.as_ref()).map_err(|_| {
                        ParseError::invalid(
                            &format!("execution model '{}'", value.as_ref()),
                            self.pos(),
                        )
                    })?;
                    execution_model = Some(model);
                }
                "executionStage" => {
                    if execution_stage.is_some() {
                        return Err(ParseError::new(
                            "duplicate executionStage attribute",
                            self.pos(),
                        ));
                    }

                    let value = match &attribute.args {
                        AttributeArgs::Value(AttributeValue::Identifier(value)) => {
                            self.strings.get(*value)
                        }
                        AttributeArgs::Value(AttributeValue::String(value)) => {
                            self.strings.get(*value)
                        }
                        _ => {
                            return Err(ParseError::new(
                                "executionStage expects an identifier",
                                self.pos(),
                            ));
                        }
                    };
                    let stage = ExecutionStage::try_from(value.as_ref()).map_err(|_| {
                        ParseError::invalid(
                            &format!("execution stage '{}'", value.as_ref()),
                            self.pos(),
                        )
                    })?;
                    execution_stage = Some(stage);
                }
                "workgroupSize" => {
                    if workgroup_size.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroupSize attribute",
                            self.pos(),
                        ));
                    }

                    workgroup_size = Some(self.parse_workgroup_size(&attribute.args)?);
                }
                "environment" => {
                    if environment_type.is_some() {
                        return Err(ParseError::new(
                            "duplicate environment attribute",
                            self.pos(),
                        ));
                    }

                    let env_type = match &attribute.args {
                        AttributeArgs::Value(AttributeValue::Type(value)) => *value,
                        _ => {
                            return Err(ParseError::new(
                                "environment expects a type value",
                                self.pos(),
                            ));
                        }
                    };
                    environment_type = Some(env_type);
                }
                _ => {}
            }
        }

        Ok((
            execution_model,
            execution_stage,
            workgroup_size,
            environment_type,
        ))
    }

    /// Parse a function definition or declaration.
    pub(super) fn parse_function(
        &mut self,
        linkage: Linkage,
        attributes: Vec<Attribute>,
    ) -> ParseResult<LocalNodeId<Function>> {
        // function keyword and name
        self.eat_token(TokenType::Function)?;

        // function name
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = self.span_at(name_start, name.len());
        let function_id = *self
            .function_map
            .get(&name)
            .unwrap_or_else(|| panic!("function {name} should be pre-registered"));
        self.current_function = Some(function_id);

        // function metadata
        let (execution_model, execution_stage, workgroup_size, environment_type) =
            self.resolve_function_attributes(&attributes)?;

        // parameters
        self.eat_token(TokenType::OpenParen)?;
        let parameters = if linkage.is_import() {
            // extern parameters
            let mut parameter_types = Vec::new();
            while !self.peek_token(TokenType::CloseParen) {
                parameter_types.push(self.parse_type()?);
                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            parameter_types
                .iter()
                .enumerate()
                .map(|(index, &ty)| TypedValue {
                    value: Value::new(index as u32),
                    ty,
                })
                .collect()
        } else {
            // definition parameters
            self.parse_typed_value_list()?
        };
        self.eat_token(TokenType::CloseParen)?;

        // return type
        self.eat_token(TokenType::Colon)?;
        let return_type = self.parse_type()?;

        // extern function body
        if linkage.is_import() {
            let name_id = self.strings.intern(&name);
            let parameter_attributes = vec![PointerAttribute::default(); parameters.len()];
            let parameter_count = parameters.len();
            let value_types = self.seed_value_types(&parameters);
            let next_value_id = value_types.len() as u32;
            let function = Function {
                name: name_id,
                parameters,
                parameter_names: vec![None; parameter_count],
                value_types,
                return_type,
                return_lifetime: Lifetime::Inferred,
                memory_effect: MemoryEffect::unknown(),
                call_behavior: CallBehavior::unknown(),
                allocation_size: None,
                parameter_attributes,
                return_attribute: PointerAttribute::default(),
                linkage,
                allocation: AllocationMode::Any, // #Incomplete: set proper MIR allocation mode?
                suspension: None,
                execution_model,
                execution_stage,
                workgroup_size,
                environment: environment_type,
                locals: Vec::new(),
                blocks: Vec::new(),
                entry: None,
                next_value_id,
            };

            // update the placeholder with the parsed signature
            self.tree.set_text_span(function_id, name_span);
            *self.tree.get_mut(function_id) = function;
            self.current_function = None;

            // record attributes
            if !attributes.is_empty() {
                self.tree.set_attributes(function_id, attributes);
            }

            // optional declaration terminator
            self.eat_token_maybe(TokenType::Semicolon);

            return Ok(function_id);
        }

        // seed signature data before mutating the placeholder
        let name_id = self.strings.intern(&name);
        let id = function_id;
        self.tree.set_text_span(id, name_span);
        let value_types = self.seed_value_types(&parameters);

        // populate signature fields
        let function = self.tree.get_mut(id);
        function.name = name_id;
        function.parameters = parameters.clone();
        function.parameter_names = vec![None; parameters.len()];
        function.value_types = value_types;
        function.return_type = return_type;
        function.linkage = linkage;
        function.memory_effect = MemoryEffect::unknown();
        function.call_behavior = CallBehavior::unknown();
        function.allocation_size = None;
        function.parameter_attributes = vec![PointerAttribute::default(); parameters.len()];
        function.return_attribute = PointerAttribute::default();
        function.execution_model = execution_model;
        function.execution_stage = execution_stage;
        function.workgroup_size = workgroup_size;
        function.environment = environment_type;

        // body
        self.eat_token(TokenType::OpenBrace)?;

        // locals
        let mut locals = Vec::new();
        while self.peek_token(TokenType::Local) {
            let local = self.parse_local()?;
            locals.push(local);
        }

        // blocks and source index mapping
        let mut blocks = Vec::new();
        let mut source_index_to_block = Vec::<Option<LocalNodeId<Block>>>::new();
        while self.peek_token(TokenType::BlockRefence) {
            let (block, source_idx) = self.parse_block()?;

            while source_index_to_block.len() <= source_idx as usize {
                source_index_to_block.push(None);
            }

            source_index_to_block[source_idx as usize] = Some(block);
            blocks.push(block);
        }

        // resolve body references
        for block_id in &blocks {
            self.resolve_block_terminators(*block_id, &source_index_to_block);
        }
        let source_index_to_local: Vec<_> = locals.clone();
        for block_id in &blocks {
            self.resolve_local_references(*block_id, &source_index_to_local);
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

    /// Parse a workgroupSize attribute.
    fn parse_workgroup_size(&mut self, args: &AttributeArgs) -> ParseResult<[u32; 3]> {
        // list or key values
        let dims = match args {
            AttributeArgs::Value(AttributeValue::Integer(value)) => [*value, 1, 1],
            AttributeArgs::Values(values) => self.parse_workgroup_dims_from_values(values)?,
            AttributeArgs::Value(AttributeValue::List(values)) => {
                self.parse_workgroup_dims_from_values(values)?
            }
            AttributeArgs::KeyValues(pairs) => self.parse_workgroup_dims_from_pairs(pairs)?,
            _ => {
                return Err(ParseError::new(
                    "workgroupSize expects one to three integer values",
                    self.pos(),
                ));
            }
        };

        self.check_workgroup_size(dims)
    }

    /// Parse positional workgroup sizes into a full 3D array.
    fn parse_workgroup_dims_from_values(
        &mut self,
        values: &[AttributeValue],
    ) -> ParseResult<[i64; 3]> {
        // require between one and three values
        if values.is_empty() || values.len() > 3 {
            return Err(ParseError::new(
                "workgroupSize expects one to three integer values",
                self.pos(),
            ));
        }

        // default missing dimensions to 1
        let mut dims = [1i64; 3];
        for (index, value) in values.iter().enumerate() {
            match value {
                AttributeValue::Integer(value) => dims[index] = *value,
                _ => {
                    return Err(ParseError::new(
                        "workgroupSize values must be integers",
                        self.pos(),
                    ));
                }
            }
        }

        Ok(dims)
    }

    /// Parse keyed workgroup sizes into a full 3D array.
    fn parse_workgroup_dims_from_pairs(
        &mut self,
        pairs: &[AttributeKeyValue],
    ) -> ParseResult<[i64; 3]> {
        // collect keyed values
        let mut x = None;
        let mut y = None;
        let mut z = None;
        for pair in pairs {
            let key = self.strings.get(pair.key);
            let value = match &pair.value {
                AttributeValue::Integer(value) => *value,
                _ => {
                    return Err(ParseError::new(
                        "workgroupSize values must be integers",
                        self.pos(),
                    ));
                }
            };

            match key.as_ref() {
                "x" => {
                    if x.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroupSize x value",
                            self.pos(),
                        ));
                    }
                    x = Some(value);
                }
                "y" => {
                    if y.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroupSize y value",
                            self.pos(),
                        ));
                    }
                    y = Some(value);
                }
                "z" => {
                    if z.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroupSize z value",
                            self.pos(),
                        ));
                    }
                    z = Some(value);
                }
                _ => {
                    return Err(ParseError::invalid(
                        &format!("workgroupSize key '{}'", key.as_ref()),
                        self.pos(),
                    ));
                }
            }
        }

        // require x and default missing dimensions to 1
        let x = x.ok_or_else(|| ParseError::new("workgroupSize requires x", self.pos()))?;
        let y = y.unwrap_or(1);
        let z = z.unwrap_or(1);

        Ok([x, y, z])
    }

    /// Validate and coerce workgroup size values.
    fn check_workgroup_size(&mut self, dims: [i64; 3]) -> ParseResult<[u32; 3]> {
        // validate dimensions
        let mut size = [0u32; 3];
        for (index, dim) in dims.into_iter().enumerate() {
            if dim < 0 {
                return Err(ParseError::new(
                    "workgroupSize values must be non-negative",
                    self.pos(),
                ));
            }

            size[index] = dim as u32;
        }

        Ok(size)
    }

    /// Parse a local variable declaration.
    fn parse_local(&mut self) -> ParseResult<LocalNodeId<Local>> {
        self.eat_token(TokenType::Local)?;

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

        // ownership
        if self.eat_token_maybe(TokenType::Comma) && self.peek_token(TokenType::Ownership) {
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
        if self.eat_token_maybe(TokenType::Comma) && self.eat_token_maybe(TokenType::Readonly) {
            mutability = Mutability::Immutable;
        }

        // record the local
        let local = Local::new(ty, mutability, ownership);
        Ok(self.tree.insert(local))
    }

    /// Parse a basic block and return (block_id, source_index).
    fn parse_block(&mut self) -> ParseResult<(LocalNodeId<Block>, u32)> {
        // block header
        let (block_span, source_idx) = {
            let block_token = self.eat_token(TokenType::BlockRefence)?;
            let block_start = block_token.start;
            let block_length = block_token.text.len();
            let source_idx = block_token
                .text
                .strip_prefix('b')
                .and_then(|text| text.parse().ok())
                .ok_or_else(|| ParseError::invalid("block reference", block_token.start))?;

            (self.span_at(block_start, block_length), source_idx)
        };

        // block parameters
        let parameters = if self.eat_token_maybe(TokenType::OpenParen) {
            let params = self.parse_typed_value_list()?;
            self.eat_token(TokenType::CloseParen)?;
            params
        } else {
            Vec::new()
        };

        // block parameter value types
        for param in &parameters {
            self.record_value_type(param.value, param.ty);
        }

        self.eat_token(TokenType::Colon)?;

        // block contents
        let mut instructions = Vec::new();
        let mut terminator = None;

        while !self.peek_token(TokenType::BlockRefence)
            && !self.peek_token(TokenType::CloseBrace)
            && !self.peek_token(TokenType::End)
        {
            // terminators
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
                || (self.is_call_terminator_line()
                    && (self.peek_token(TokenType::Invoke)
                        || self.peek_token(TokenType::InvokeIndirect)
                        || self.peek_token(TokenType::InvokeVirtual)
                        || self.peek_token(TokenType::InvokeInterface)))
            {
                let parsed_terminator = self.parse_terminator()?;
                terminator = Some(parsed_terminator);
                break;
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
        self.tree.set_text_span(id, block_span);

        Ok((id, source_idx))
    }

    /// Return whether the current line contains call continuations.
    fn is_call_terminator_line(&self) -> bool {
        let mut token_index = self.pos;
        let mut saw_arrow = false;

        while let Some(token) = self.tokens.get(token_index) {
            if token.ty == TokenType::Newline || token.ty == TokenType::End {
                break;
            }

            if !token.ty.is_trivia() {
                if token.ty == TokenType::Arrow {
                    saw_arrow = true;
                } else if saw_arrow && token.ty == TokenType::Catch {
                    return true;
                }
            }

            token_index += 1;
        }

        false
    }

    /// Parse a block terminator.
    pub(super) fn parse_terminator(&mut self) -> ParseResult<Terminator> {
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
            TokenType::Invoke => {
                self.bump();
                let (function, arguments, signature) = self.parse_direct_call_target()?;
                let (normal_target, normal_arguments, unwind_target, unwind_arguments) =
                    self.parse_call_continuations()?;
                Ok(Terminator::Invoke {
                    function,
                    call: Call::new(arguments, signature),
                    normal_target,
                    normal_arguments,
                    unwind_target,
                    unwind_arguments,
                })
            }
            TokenType::Jump => {
                self.bump();
                let target = self.parse_block_ref()?;
                let arguments = self.parse_optional_block_arguments()?;
                Ok(Terminator::Jump { target, arguments })
            }
            TokenType::Branch => {
                self.bump();
                let condition = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let then_target = self.parse_block_ref()?;
                let then_arguments = self.parse_optional_block_arguments()?;
                self.eat_token(TokenType::Comma)?;
                let else_target = self.parse_block_ref()?;
                let else_arguments = self.parse_optional_block_arguments()?;
                Ok(Terminator::Branch {
                    condition,
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                })
            }
            TokenType::Check => {
                self.bump();
                let constraint = self.parse_check_kind()?;
                self.eat_token(TokenType::Arrow)?;
                let success = CheckTarget {
                    target: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };
                self.eat_token(TokenType::Comma)?;
                let failure = CheckTarget {
                    target: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };
                Ok(Terminator::Check {
                    constraint,
                    success,
                    failure,
                })
            }
            TokenType::Switch => {
                self.bump();
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let default = self.parse_block_ref()?;
                let default_arguments = self.parse_optional_block_arguments()?;

                let mut cases = Vec::new();
                while self.eat_token_maybe(TokenType::Comma) {
                    let case_value = self.parse_int_literal()?;
                    self.eat_token(TokenType::FatArrow)?;
                    let target = self.parse_block_ref()?;
                    let arguments = self.parse_optional_block_arguments()?;
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
            TokenType::Yield => {
                self.bump();
                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let resume = self.parse_block_ref()?;
                let resume_arguments = self.parse_optional_block_arguments()?;
                Ok(Terminator::Yield {
                    value,
                    resume,
                    resume_arguments,
                })
            }
            TokenType::Throw => {
                self.bump();
                let value = self.parse_value()?;
                Ok(Terminator::Throw { value })
            }
            TokenType::Trap => {
                let trap_kind = self
                    .peek()
                    .cloned()
                    .expect("peeked trap token before consuming it");
                self.bump();

                // abort has no payload
                if trap_kind.text == "trap.abort" {
                    return Ok(Terminator::Trap {
                        kind: TrapKind::Abort,
                        payload: None,
                    });
                }

                // panic carries a payload
                if trap_kind.text == "trap.panic" {
                    let payload = self.parse_value()?;
                    return Ok(Terminator::Trap {
                        kind: TrapKind::Panic,
                        payload: Some(payload),
                    });
                }

                Err(ParseError::invalid(
                    "expected `trap.abort` or `trap.panic`",
                    trap_kind.start,
                ))
            }
            TokenType::Unreachable => {
                self.bump();
                Ok(Terminator::Unreachable)
            }
            TokenType::TailCall => {
                self.bump();
                let (function, arguments, signature) = self.parse_direct_call_target()?;
                Ok(Terminator::TailCall {
                    function,
                    call: Call::new(arguments, signature),
                })
            }
            TokenType::TailCallIndirect => {
                self.bump();
                let (callee, arguments, signature) = self.parse_indirect_call_target()?;
                Ok(Terminator::TailCallIndirect {
                    callee,
                    call: Call::new(arguments, signature),
                })
            }
            TokenType::InvokeIndirect => {
                self.bump();
                let (callee, arguments, signature) = self.parse_indirect_call_target()?;
                let (normal_target, normal_arguments, unwind_target, unwind_arguments) =
                    self.parse_call_continuations()?;
                Ok(Terminator::InvokeIndirect {
                    callee,
                    call: Call::new(arguments, signature),
                    normal_target,
                    normal_arguments,
                    unwind_target,
                    unwind_arguments,
                })
            }
            TokenType::TailCallVirtual => {
                self.bump();
                let (receiver, declaring_type, slot_id, arguments, signature) =
                    self.parse_virtual_call_target()?;
                Ok(Terminator::TailCallVirtual {
                    receiver,
                    declaring_type,
                    slot_id,
                    declared_target: None,
                    call: Call::new(arguments, signature),
                })
            }
            TokenType::InvokeVirtual => {
                self.bump();
                let (receiver, declaring_type, slot_id, arguments, signature) =
                    self.parse_virtual_call_target()?;
                let (normal_target, normal_arguments, unwind_target, unwind_arguments) =
                    self.parse_call_continuations()?;
                Ok(Terminator::InvokeVirtual {
                    receiver,
                    declaring_type,
                    slot_id,
                    declared_target: None,
                    call: Call::new(arguments, signature),
                    normal_target,
                    normal_arguments,
                    unwind_target,
                    unwind_arguments,
                })
            }
            TokenType::TailCallInterface => {
                self.bump();
                let (receiver, declaring_type, slot_id, arguments, signature) =
                    self.parse_interface_call_target()?;
                Ok(Terminator::TailCallInterface {
                    receiver,
                    declaring_type,
                    slot_id,
                    declared_target: None,
                    call: Call::new(arguments, signature),
                })
            }
            TokenType::InvokeInterface => {
                self.bump();
                let (receiver, declaring_type, slot_id, arguments, signature) =
                    self.parse_interface_call_target()?;
                let (normal_target, normal_arguments, unwind_target, unwind_arguments) =
                    self.parse_call_continuations()?;
                Ok(Terminator::InvokeInterface {
                    receiver,
                    declaring_type,
                    slot_id,
                    declared_target: None,
                    call: Call::new(arguments, signature),
                    normal_target,
                    normal_arguments,
                    unwind_target,
                    unwind_arguments,
                })
            }
            _ => Err(ParseError::unexpected("terminator", token.ty, token.start)),
        }
    }

    /// Parse a check kind and its operands.
    fn parse_check_kind(&mut self) -> ParseResult<CheckConstraint> {
        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("check kind", self.pos()))?;
        let kind_text = kind_token.text;
        let kind_start = kind_token.start;

        match kind_token.ty {
            TokenType::Identifier | TokenType::TypeName | TokenType::Type => self.bump(),
            _ => {
                return Err(ParseError::unexpected(
                    "check kind",
                    kind_token.ty,
                    kind_start,
                ));
            }
        }

        let kind_parts: Vec<_> = kind_text.split('.').collect();
        let head = kind_parts.first().copied().unwrap_or_default();

        match head {
            "bounds" => {
                let signedness = kind_parts.get(1).copied().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "s" => true,
                    "u" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };

                if kind_parts.len() != 2 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let index = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let collection = self.parse_value()?;

                Ok(CheckConstraint::Bounds {
                    index,
                    length,
                    collection,
                    is_signed,
                })
            }
            "null" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                Ok(CheckConstraint::Null { value })
            }
            "zeroDivisor" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let divisor = self.parse_value()?;
                Ok(CheckConstraint::DivZero { divisor })
            }
            "dynamicType" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_type()?;

                Ok(CheckConstraint::Type { value, expected })
            }
            "unionTag" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_int_literal()?;
                let expected = u64::try_from(expected)
                    .map_err(|_| ParseError::invalid("check union tag", kind_start))?;

                Ok(CheckConstraint::Union { value, expected })
            }
            "receiverType" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_type()?;

                Ok(CheckConstraint::ReceiverType { receiver, expected })
            }
            "interfaceConformance" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let receiver = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_type()?;

                Ok(CheckConstraint::Implements { receiver, expected })
            }
            "shiftRange" => {
                let signedness = kind_parts.get(1).copied().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "s" => true,
                    "u" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };

                if kind_parts.len() != 2 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let bit_width = self.parse_int_literal()?;
                let bit_width = u8::try_from(bit_width)
                    .map_err(|_| ParseError::invalid("check bit width", kind_start))?;

                Ok(CheckConstraint::ShiftRange {
                    value,
                    bit_width,
                    is_signed,
                })
            }
            "narrowRange" => {
                let signedness = kind_parts.get(1).copied().ok_or_else(|| {
                    ParseError::invalid(&format!("check kind '{kind_text}'"), kind_start)
                })?;
                let is_signed = match signedness {
                    "s" => true,
                    "u" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };

                if kind_parts.len() != 2 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let to_width = self.parse_int_literal()?;
                let to_width = u8::try_from(to_width)
                    .map_err(|_| ParseError::invalid("check width", kind_start))?;

                Ok(CheckConstraint::Narrow {
                    value,
                    to_width,
                    is_signed,
                })
            }
            _ if kind_parts.len() >= 4 && kind_parts[kind_parts.len() - 2] == "overflow" => {
                let signedness = kind_parts[kind_parts.len() - 1];
                let is_signed = match signedness {
                    "s" => true,
                    "u" => false,
                    _ => {
                        return Err(ParseError::invalid(
                            &format!("check kind '{kind_text}'"),
                            kind_start,
                        ));
                    }
                };
                let operator_text = kind_parts[..kind_parts.len() - 2].join(".");
                let operator = parse_overflow_check_operator(&operator_text).ok_or_else(|| {
                    ParseError::invalid(&format!("check operator '{operator_text}'"), kind_start)
                })?;

                let left = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let right = self.parse_value()?;

                Ok(CheckConstraint::Overflow {
                    operator,
                    left,
                    right,
                    is_signed,
                })
            }
            _ => Err(ParseError::invalid(
                &format!("check kind '{kind_text}'"),
                kind_start,
            )),
        }
    }

    /// Parse optional block arguments like `(v0, v1)`.
    fn parse_optional_block_arguments(&mut self) -> ParseResult<Vec<Value>> {
        if self.eat_token_maybe(TokenType::OpenParen) {
            let arguments = self.parse_value_list()?;
            self.eat_token(TokenType::CloseParen)?;
            Ok(arguments)
        } else {
            Ok(Vec::new())
        }
    }

    /// Parse success and exception continuations for an invoke terminator.
    fn parse_call_continuations(
        &mut self,
    ) -> ParseResult<(
        LocalNodeId<Block>,
        Vec<Value>,
        LocalNodeId<Block>,
        Vec<Value>,
    )> {
        self.eat_token(TokenType::Arrow)?;
        let normal_target = self.parse_block_ref()?;
        let normal_arguments = self.parse_optional_block_arguments()?;

        self.eat_token(TokenType::Comma)?;
        self.eat_token(TokenType::Catch)?;
        let unwind_target = self.parse_block_ref()?;
        let unwind_arguments = self.parse_optional_block_arguments()?;

        Ok((
            normal_target,
            normal_arguments,
            unwind_target,
            unwind_arguments,
        ))
    }

    /// Resolve block references in terminators after parsing blocks.
    fn resolve_block_terminators(
        &mut self,
        block_id: LocalNodeId<Block>,
        source_to_actual: &[Option<LocalNodeId<Block>>],
    ) {
        let block = self.tree.get_mut(block_id);

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
            Terminator::Invoke {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeIndirect {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeVirtual {
                normal_target,
                unwind_target,
                ..
            }
            | Terminator::InvokeInterface {
                normal_target,
                unwind_target,
                ..
            } => {
                *normal_target = source_to_actual
                    .get(normal_target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*normal_target);
                *unwind_target = source_to_actual
                    .get(unwind_target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*unwind_target);
            }
            Terminator::Jump { target, .. } => {
                *target = source_to_actual
                    .get(target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*target);
            }
            Terminator::Branch {
                then_target,
                else_target,
                ..
            } => {
                *then_target = source_to_actual
                    .get(then_target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*then_target);
                *else_target = source_to_actual
                    .get(else_target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*else_target);
            }
            Terminator::Check {
                success, failure, ..
            } => {
                success.target = source_to_actual
                    .get(success.target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(success.target);
                failure.target = source_to_actual
                    .get(failure.target.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(failure.target);
            }
            Terminator::Switch { default, cases, .. } => {
                *default = source_to_actual
                    .get(default.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*default);
                for case in cases.iter_mut() {
                    case.target = source_to_actual
                        .get(case.target.id as usize)
                        .copied()
                        .flatten()
                        .unwrap_or(case.target);
                }
            }
            Terminator::Yield { resume, .. } => {
                *resume = source_to_actual
                    .get(resume.id as usize)
                    .copied()
                    .flatten()
                    .unwrap_or(*resume);
            }
        }
    }

    /// Resolve local references in instructions after parsing locals.
    fn resolve_local_references(
        &mut self,
        block_id: LocalNodeId<Block>,
        source_to_actual: &[LocalNodeId<Local>],
    ) {
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
                    *local = source_to_actual
                        .get(local.id as usize)
                        .copied()
                        .unwrap_or(*local);
                }
                _ => {}
            }
        }
    }

    /// Seed a value type table from typed parameters.
    fn seed_value_types(&self, parameters: &[TypedValue]) -> Vec<LocalNodeId<Type>> {
        // sort parameters into value order
        let mut ordered: Vec<_> = parameters.iter().collect();
        ordered.sort_by_key(|parameter| parameter.value.0);

        // validate that the parameters form a dense prefix
        let mut value_types = Vec::with_capacity(ordered.len());
        for (expected, parameter) in ordered.into_iter().enumerate() {
            if parameter.value.0 as usize != expected {
                panic!("missing value type for v{expected}");
            }

            value_types.push(parameter.ty);
        }

        value_types
    }
}

/// Parse the canonical operator family used in overflow checks.
fn parse_overflow_check_operator(text: &str) -> Option<crate::BinaryOperator> {
    Some(match text {
        "int.add" => crate::BinaryOperator::Add,
        "int.sub" => crate::BinaryOperator::Subtract,
        "int.mul" => crate::BinaryOperator::Multiply,
        "int.div" => crate::BinaryOperator::SignedDivide,
        "int.rem" => crate::BinaryOperator::SignedRemainder,
        _ => return None,
    })
}
