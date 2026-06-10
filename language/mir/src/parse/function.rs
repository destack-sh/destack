use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::source::{Token, TokenType};
use crate::{
    AllocationMode, Attribute, AttributeArgs, AttributeValue, Block, BlockTarget, Call,
    CheckConstraint, Function, FunctionHeaderSpans, Instruction, IntegerReference, Linkage, Local,
    LocalNodeId, LocalReference, Mutability, Ownership, Parameter, SwitchCase, Terminator,
    TrapKind, TypeReference, TypedValueSpan, Value, ValueReference,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

impl Parser {
    /// Resolve attributes into function metadata.
    pub(super) fn resolve_function_attributes(
        &mut self,
        attributes: &[Attribute],
    ) -> ParseResult<Option<TypeReference>> {
        // metadata output
        let mut environment_type = None;

        // inspect attributes
        for attribute in attributes {
            let name = match attribute.name {
                crate::AttributeIdentifier::Identifier(name) => self.strings.get(name).to_string(),
                crate::AttributeIdentifier::Missing | crate::AttributeIdentifier::Error => continue,
            };
            if name.as_str() == "environment" {
                if environment_type.is_some() {
                    return Err(ParseError::new(
                        "duplicate environment attribute",
                        self.pos(),
                    ));
                }

                let env_type = match &attribute.args {
                    AttributeArgs::Value(AttributeValue::Type(value)) => value.clone(),
                    _ => {
                        return Err(ParseError::new(
                            "environment expects a type value",
                            self.pos(),
                        ));
                    }
                };
                environment_type = Some(env_type);
            }
        }

        Ok(environment_type)
    }

    /// Parse a function definition or declaration.
    pub(super) fn parse_function(
        &mut self,
        item_start: usize,
        linkage: Linkage,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<LocalNodeId<Function>> {
        // function keyword and name
        let keyword_token = self.eat_token(TokenType::Function)?;
        let keyword_start = keyword_token.start;
        let keyword_length = self.tree.source_text(keyword_token.span).len();
        let keyword_span = self.span_at(keyword_start, keyword_length);

        // function name
        let (name, name_start) = self.parse_symbol_name()?;
        let name_span = self.span_at(name_start, name.len());
        let lifetimes = self.parse_lifetimes()?;
        let function_id = self.function_map.get(&name).copied().ok_or_else(|| {
            ParseError::new(format!("function '{name}' is not declared"), name_start)
        })?;
        self.current_function = Some(function_id);
        self.reset_function_parse_state();

        // function metadata
        let environment_type = self.resolve_function_attributes(&attributes)?;

        // parameters
        let signature_start = self.pos();
        let (parameters, parameter_spans, open_paren_span, close_paren_span) =
            self.parse_function_parameters(linkage)?;
        let parameter_names = parameters
            .iter()
            .map(|parameter| match parameter.value {
                ValueReference::Value(value) => self.tree.get(function_id).value_name(value),
                ValueReference::Missing | ValueReference::Error => None,
            })
            .collect::<Vec<_>>();

        // return type
        let return_colon_token = self.eat_token(TokenType::Colon)?;
        let return_colon_start = return_colon_token.start;
        let return_colon_length = self.tree.source_text(return_colon_token.span).len();
        let return_colon_span = self.span_at(return_colon_start, return_colon_length);
        let (return_type, return_type_span) =
            self.parse_return_type_reference_after(return_colon_token);
        let signature_span = self.span_between(signature_start, return_type_span.end as usize);

        // external function body
        if linkage.is_import() {
            let name_id = self.strings.intern(&name);
            let mut function = Function::import(name_id, parameters, return_type);
            function.lifetimes = lifetimes;
            function.parameter_names = parameter_names;
            function.environment = environment_type;
            function.allocation = AllocationMode::Any; // #Incomplete: set proper MIR allocation mode?

            // update the placeholder with the parsed signature
            self.tree
                .set_text_span(function_id, self.span_from_parse_start(item_start));
            self.tree.set_keyword_span(function_id, keyword_span);
            self.tree.set_main_span(function_id, name_span);
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                signature_span,
            );
            self.tree.set_attribute_spans(function_id, attribute_spans);
            self.tree
                .set_function_parameter_spans(function_id, parameter_spans);
            self.tree.set_function_header_spans(
                function_id,
                FunctionHeaderSpans::new(
                    open_paren_span,
                    close_paren_span,
                    return_colon_span,
                    None,
                ),
            );
            *self.tree.get_mut(function_id) = function;
            self.current_function = None;
            self.pop_lifetimes();

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
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));
        self.tree.set_keyword_span(id, keyword_span);
        self.tree.set_main_span(id, name_span);
        self.tree.set_side_span(
            id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            signature_span,
        );
        self.tree.set_attribute_spans(id, attribute_spans);
        self.tree.set_function_parameter_spans(id, parameter_spans);
        let (_, value_types) = Function::parameter_state(&parameters);

        // populate signature fields
        let function = self.tree.get_mut(id);
        function.name = name_id;
        function.parameters = parameters.clone();
        function.lifetimes = lifetimes;
        function.parameter_names = parameter_names;
        function.value_types = value_types;
        function.return_type = return_type;
        function.linkage = linkage;
        function.environment = environment_type;
        // body
        let open_brace_token = self.eat_token(TokenType::OpenBrace)?;
        let open_brace_start = open_brace_token.start;
        let open_brace_length = self.tree.source_text(open_brace_token.span).len();
        let open_brace_span = self.span_at(open_brace_start, open_brace_length);
        self.tree.set_function_header_spans(
            id,
            FunctionHeaderSpans::new(
                open_paren_span,
                close_paren_span,
                return_colon_span,
                Some(open_brace_span),
            ),
        );

        // locals
        let mut locals = Vec::new();
        while self.peek_token(TokenType::Local) {
            let recovery_pos = self.pos();
            match self.parse_local() {
                Ok(local) => locals.push(local),
                Err(error) => {
                    self.diagnostics
                        .insert(error.to_diagnostic(self.content_id, self.file_id));
                    self.try_recover_to_block(recovery_pos);
                    break;
                }
            }
        }

        // predeclare symbolic block labels so references can resolve forward
        self.predeclare_blocks()?;

        // blocks
        let mut blocks = Vec::new();
        while !self.peek_token(TokenType::CloseBrace) && !self.peek_token(TokenType::End) {
            // block headers
            if self.is_block_label_start() {
                blocks.push(self.parse_block_recovering());
                continue;
            }

            // stray body tokens
            let error = ParseError::new("expected block label", self.pos());
            self.diagnostics
                .insert(error.to_diagnostic(self.content_id, self.file_id));
            self.try_recover_to_block(self.pos());
        }

        // resolve body references
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

        self.tree.rebuild_function_places(id);

        self.current_function = None;
        self.pop_lifetimes();
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));

        // record attributes
        if !attributes.is_empty() {
            self.tree.set_attributes(id, attributes);
        }

        Ok(id)
    }

    /// Parse one function parameter list.
    fn parse_function_parameters(
        &mut self,
        linkage: Linkage,
    ) -> ParseResult<(Vec<Parameter>, Vec<TypedValueSpan>, Span, Span)> {
        let open_paren_token = self.eat_token(TokenType::OpenParenthesis)?;
        let open_paren_start = open_paren_token.start;
        let open_paren_length = self.tree.source_text(open_paren_token.span).len();
        let open_paren_span = self.span_at(open_paren_start, open_paren_length);

        let (parameters, parameter_spans) = if linkage.is_import() {
            let mut parameter_types = Vec::new();
            let mut parameter_spans = Vec::new();
            while !self.peek_token(TokenType::CloseParenthesis) {
                let parameter_start = self.pos();
                let (ty, type_span) = self.parse_type_reference_part()?;
                let parameter_span = self.span_from_parse_start(parameter_start);
                parameter_types.push(ty);
                parameter_spans.push(TypedValueSpan::new(parameter_span, None, type_span));
                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            let parameters = parameter_types
                .into_iter()
                .enumerate()
                .map(|(index, ty)| Parameter {
                    value: ValueReference::Value(Value::new(index as u32)),
                    ty,
                })
                .collect();

            (parameters, parameter_spans)
        } else {
            self.parse_typed_values()?
        };

        let close_paren_token = self.eat_token(TokenType::CloseParenthesis)?;
        let close_paren_start = close_paren_token.start;
        let close_paren_length = self.tree.source_text(close_paren_token.span).len();
        let close_paren_span = self.span_at(close_paren_start, close_paren_length);

        Ok((
            parameters,
            parameter_spans,
            open_paren_span,
            close_paren_span,
        ))
    }

    /// Parse a local variable declaration.
    fn parse_local(&mut self) -> ParseResult<LocalNodeId<Local>> {
        // whole declaration
        let local_start = self.pos();

        self.eat_token(TokenType::Local)?;

        // local reference
        let local_token = self.eat_token(TokenType::LocalReference)?;
        let local_name_text = self.tree.source_text(local_token.span).to_string();
        let local_name_start = local_token.start;
        let local_name_length = local_name_text.len();
        let local_span = self.span_at(local_name_start, local_name_length);
        let _local_idx: u32 = local_name_text
            .strip_prefix("local")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| ParseError::invalid("local reference", local_name_start))?;

        // local type
        let colon_token = self.eat_token(TokenType::Colon)?;
        let (ty, type_span) = self.parse_type_reference_after(colon_token, "local type");

        // local annotations
        let mut ownership = Ownership::Owned;
        let mut mutability = Mutability::Mutable;

        // ownership
        if self.eat_token_maybe(TokenType::Comma) && self.peek_token(TokenType::Ownership) {
            let text = self
                .peek()
                .map(|token| self.tree.source_text(token.span))
                .unwrap_or("");
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
        let local_id = self.tree.insert(local);
        self.tree
            .set_text_span(local_id, self.span_from_parse_start(local_start));
        self.tree.set_main_span(local_id, local_span);
        self.tree.set_side_span(
            local_id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            type_span,
        );

        Ok(local_id)
    }

    /// Parse a basic block into its predeclared block id.
    fn parse_block(&mut self) -> ParseResult<LocalNodeId<Block>> {
        let block_start = self.pos();
        let block_id = self.current_predeclared_block_id()?;
        let terminator_id = self.tree.get(block_id).terminator;

        // block header
        let (block_span, block_name) = {
            let block_token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("block label", self.pos()))?;
            let block_span = block_token.span;

            let block_name = match self.token_type(block_token) {
                TokenType::BlockReference => {
                    self.bump();
                    None
                }
                TokenType::Identifier => {
                    let name = self.tree.source_text(block_token.span).to_string();
                    self.bump();
                    let name_id = self.strings.intern(&name);
                    Some(name_id)
                }
                _ => {
                    return Err(ParseError::unexpected(
                        "block label",
                        self.token_type(block_token),
                        block_token.start,
                    ));
                }
            };

            (block_span, block_name)
        };

        // block parameters
        let parameters = if self.eat_token_maybe(TokenType::OpenParenthesis) {
            let params = if self.is_entry_block_parameter_list() {
                self.parse_entry_block_parameters()?
            } else {
                let (params, _) = self.parse_typed_values()?;
                params
            };
            self.eat_token(TokenType::CloseParenthesis)?;
            params
        } else {
            Vec::new()
        };

        // block parameter value types
        for param in &parameters {
            if let ValueReference::Value(value) = param.value
                && let Some(ty) = param.ty.ty()
            {
                self.record_value_type(value, ty)?;
            }
        }

        self.eat_token(TokenType::Colon)?;

        // block contents
        let mut instructions = Vec::new();
        let mut terminator = None;
        let mut terminator_span = None;
        let mut terminator_main_span = None;
        let mut is_broken = false;

        while !self.is_block_label_start()
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
                || self.peek_token(TokenType::Panic)
                || self.peek_token(TokenType::ResumeUnwind)
                || self.peek_token(TokenType::Trap)
                || self.peek_token(TokenType::Unreachable)
                || self.peek_token(TokenType::TailCall)
                || self.peek_token(TokenType::TailCallIndirect)
                || self.peek_token(TokenType::TailCallVirtual)
                || self.peek_token(TokenType::TailCallDynamic)
                || self.is_allocation_try_terminator_line()
                || (self.is_call_terminator_line()
                    && (self.peek_token(TokenType::Call)
                        || self.peek_token(TokenType::CallIndirect)
                        || self.peek_token(TokenType::CallVirtual)
                        || self.peek_token(TokenType::CallDynamic)))
            {
                let recovery_pos = self.pos();
                let main_token = self.peek().cloned();
                match self.parse_terminator() {
                    Ok(parsed_terminator) => {
                        terminator_span = Some(self.span_from_parse_start(recovery_pos));
                        terminator_main_span = main_token.as_ref().map(|token| token.span);
                        terminator = Some(parsed_terminator);
                    }
                    Err(error) => {
                        self.diagnostics
                            .insert(error.to_diagnostic(self.content_id, self.file_id));
                        self.try_recover_to_block(recovery_pos);
                        terminator_span = Some(
                            self.span_at(error.position, self.pos().saturating_sub(error.position)),
                        );
                        terminator_main_span = main_token.as_ref().map(|token| token.span);
                        terminator = Some(Terminator::Error);
                        is_broken = true;
                    }
                }
                break;
            }

            // instruction
            let instruction_start = self.pos();
            let recovery_index = self.pos;
            match self.eat_instruction() {
                Ok(inst) => instructions.push(inst),
                Err(error) => {
                    self.diagnostics
                        .insert(error.to_diagnostic(self.content_id, self.file_id));
                    let continue_block = self.try_recover_in_block(recovery_index);
                    let error_end = self.pos();
                    let error_span = self.span_at(
                        instruction_start,
                        error_end.saturating_sub(instruction_start),
                    );
                    let error_instruction = self.tree.insert(Instruction::Error);
                    self.tree.set_text_span(error_instruction, error_span);
                    instructions.push(error_instruction);
                    is_broken = true;

                    if !continue_block {
                        break;
                    }
                }
            }
        }

        // finalize block
        let parsed_terminator = terminator.unwrap_or(if is_broken {
            Terminator::Error
        } else {
            Terminator::Unreachable
        });

        let block = Block {
            name: block_name,
            parameters,
            instructions,
            terminator: terminator_id,
        };

        *self.tree.get_mut(terminator_id) = parsed_terminator;
        *self.tree.get_mut(block_id) = block;
        self.tree
            .set_text_span(block_id, self.span_from_parse_start(block_start));
        self.tree.set_main_span(block_id, block_span);

        if let Some(terminator_span) = terminator_span {
            self.tree.set_text_span(terminator_id, terminator_span);
        }

        if let Some(terminator_main_span) = terminator_main_span {
            self.tree.set_main_span(terminator_id, terminator_main_span);
        }

        Ok(block_id)
    }

    /// Parse one block and recover to the next block boundary on failure.
    fn parse_block_recovering(&mut self) -> LocalNodeId<Block> {
        let block_id = match self.current_predeclared_block_id() {
            Ok(block_id) => block_id,
            Err(error) => {
                self.diagnostics
                    .insert(error.to_diagnostic(self.content_id, self.file_id));
                self.create_error_block(self.pos())
            }
        };
        let terminator_id = self.tree.get(block_id).terminator;
        let recovery_pos = self.pos();

        let parsed_block = match self.parse_block() {
            Ok(block_id) => block_id,
            Err(error) => {
                self.diagnostics
                    .insert(error.to_diagnostic(self.content_id, self.file_id));
                self.try_recover_to_block(recovery_pos);

                let error_start = error.position;
                let error_end = self.pos();
                let error_span = self.span_at(error_start, error_end.saturating_sub(error_start));
                let block = Block {
                    name: None,
                    parameters: Vec::new(),
                    instructions: Vec::new(),
                    terminator: terminator_id,
                };

                *self.tree.get_mut(terminator_id) = Terminator::Error;
                *self.tree.get_mut(block_id) = block;
                self.tree.set_text_span(block_id, error_span);
                self.tree.set_text_span(terminator_id, error_span);

                block_id
            }
        };

        self.parsed_block_count += 1;
        parsed_block
    }

    /// Return the predeclared block id for the current source position.
    fn current_predeclared_block_id(&self) -> ParseResult<LocalNodeId<Block>> {
        self.predeclared_blocks
            .get(self.parsed_block_count)
            .copied()
            .ok_or_else(|| {
                ParseError::new(
                    format!(
                        "missing predeclared block for parsed block {}",
                        self.parsed_block_count
                    ),
                    self.pos(),
                )
            })
    }

    /// Create an error block when recovery state is missing.
    fn create_error_block(&mut self, position: usize) -> LocalNodeId<Block> {
        let error_span = self.span_at(position, 0);
        let terminator_id = self.tree.insert(Terminator::Error);
        let block_id = self.tree.insert(Block::new(terminator_id));
        self.tree.set_text_span(block_id, error_span);
        self.tree.set_text_span(terminator_id, error_span);

        block_id
    }

    /// Recover to the next block boundary in the current function body.
    fn try_recover_to_block(&mut self, recovery_pos: usize) {
        // make forward progress before scanning for the next block
        if self.pos() == recovery_pos {
            self.bump();
        }

        while !self.peek_token(TokenType::CloseBrace) && !self.peek_token(TokenType::End) {
            if self.is_block_label_start() {
                return;
            }

            self.bump();
        }
    }

    /// Recover within one block and return whether the block can continue.
    fn try_recover_in_block(&mut self, recovery_index: usize) -> bool {
        self.pos = recovery_index;

        while self.pos < self.tree.tokens().len() {
            self.skip_raw_trivia_except_newline();

            if self.peek_token(TokenType::CloseBrace)
                || self.peek_token(TokenType::End)
                || self.is_block_label_start()
            {
                return false;
            }

            let Some(token) = self.tree.tokens().get(self.pos) else {
                return false;
            };

            if self.token_type(token) == TokenType::Newline {
                self.pos += 1;
                self.skip_raw_trivia_except_newline();

                return !(self.peek_token(TokenType::CloseBrace)
                    || self.peek_token(TokenType::End)
                    || self.is_block_label_start());
            }

            self.pos += 1;
        }

        false
    }

    /// Return whether the current block header is the entry block parameter mirror.
    fn is_entry_block_parameter_list(&self) -> bool {
        let Some(function_id) = self.current_function else {
            return false;
        };

        let function = self.tree.get(function_id);
        self.parsed_block_count == 0 && !function.parameters.is_empty()
    }

    /// Parse the entry block parameter mirror and reuse the function parameters.
    fn parse_entry_block_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        let function_id = self.current_function.ok_or_else(|| {
            ParseError::new(
                "entry block parameters require a current function",
                self.pos(),
            )
        })?;
        let parameters = self.tree.get(function_id).parameters.clone();

        for (parameter_index, parameter) in parameters.iter().enumerate() {
            let (value, _) = self.parse_entry_block_parameter()?;
            if value != parameter.value {
                return Err(ParseError::new(
                    format!(
                        "entry block parameter {parameter_index} expected value {:?} got {:?}",
                        parameter.value, value
                    ),
                    self.pos(),
                ));
            }

            self.eat_token(TokenType::Colon)?;
            let (ty, _) = self.parse_type_reference_part()?;
            if ty != parameter.ty {
                return Err(ParseError::new(
                    format!(
                        "entry block parameter {parameter_index} expected type {:?} got {:?}",
                        parameter.ty, ty
                    ),
                    self.pos(),
                ));
            }

            if parameter_index + 1 < parameters.len() {
                self.eat_token(TokenType::Comma)?;
            }
        }

        Ok(parameters)
    }

    /// Parse one entry block parameter and resolve it to the mirrored function parameter.
    fn parse_entry_block_parameter(&mut self) -> ParseResult<(ValueReference, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("entry block parameter", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Value => {
                let text = self.tree.source_text(token.span).to_string();
                self.bump();

                let index: u32 = text
                    .strip_prefix('v')
                    .and_then(|text| text.parse().ok())
                    .ok_or_else(|| {
                        ParseError::invalid("entry block parameter", span.start as usize)
                    })?;

                Ok((ValueReference::Value(Value::new(index)), span))
            }
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start;
                self.bump();

                let value = self
                    .value_name_map
                    .get(&name)
                    .copied()
                    .map(ValueReference::Value)
                    .ok_or_else(|| ParseError::new(format!("undefined value '{name}'"), start))?;

                Ok((value, span))
            }
            _ => Err(ParseError::unexpected(
                "entry block parameter",
                self.token_type(token),
                token.start,
            )),
        }
    }

    /// Return whether the current line contains a call terminator continuation.
    fn is_call_terminator_line(&self) -> bool {
        let mut token_index = self.pos;
        let tokens = self.tree.tokens();

        while let Some(token) = tokens.get(token_index) {
            if self.token_type(token) == TokenType::Newline
                || self.token_type(token) == TokenType::End
            {
                break;
            }

            if self.token_type(token) == TokenType::Arrow {
                let next_token = tokens
                    .iter()
                    .skip(token_index + 1)
                    .take_while(|next_token| {
                        self.token_type(next_token) != TokenType::Newline
                            && self.token_type(next_token) != TokenType::End
                    })
                    .find(|next_token| !next_token.is_trivia());

                if next_token.is_some_and(|next_token| self.is_block_reference_token(next_token)) {
                    return true;
                }
            }

            token_index += 1;
        }

        false
    }

    /// Return whether a token can start a block reference in this function.
    fn is_block_reference_token(&self, token: &Token) -> bool {
        match self.token_type(token) {
            TokenType::BlockReference => true,
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span);
                self.block_name_map.contains_key(name)
            }
            _ => false,
        }
    }

    /// Parse a block terminator.
    pub(super) fn parse_terminator(&mut self) -> ParseResult<Terminator> {
        let token = self
            .peek()
            .cloned()
            .ok_or_else(|| ParseError::unexpected_end("terminator", self.pos()))?;

        match self.token_type(&token) {
            TokenType::Return => {
                self.bump();
                let value = if self.is_value_reference_start() {
                    Some(self.parse_value()?)
                } else {
                    None
                };
                Ok(Terminator::Return { value })
            }
            TokenType::Call => {
                self.bump();
                let (function, arguments, signature) = self.parse_direct_call_target()?;
                let (target, unwind) = self.parse_continuation()?;
                Ok(Terminator::Call {
                    function,
                    call: Call::new(arguments, signature),
                    target,
                    unwind,
                })
            }
            TokenType::Jump => {
                self.bump();
                let target = BlockTarget {
                    block: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };

                Ok(Terminator::Jump { target })
            }
            TokenType::Branch => {
                self.bump();
                let condition = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let then_target = BlockTarget {
                    block: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };
                self.eat_token(TokenType::Comma)?;
                let else_target = BlockTarget {
                    block: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };

                Ok(Terminator::Branch {
                    condition,
                    then_target,
                    else_target,
                })
            }
            TokenType::Check => {
                self.bump();
                let constraint = self.parse_check_kind()?;
                self.eat_token(TokenType::Arrow)?;
                let success = BlockTarget {
                    block: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };
                self.eat_token(TokenType::Comma)?;
                let failure = BlockTarget {
                    block: self.parse_block_ref()?,
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
                let default = BlockTarget {
                    block: self.parse_block_ref()?,
                    arguments: self.parse_optional_block_arguments()?,
                };

                let mut cases = Vec::new();
                while self.eat_token_maybe(TokenType::Comma) {
                    let case_value = self.parse_int_literal()?;
                    self.eat_token(TokenType::FatArrow)?;
                    let target = BlockTarget {
                        block: self.parse_block_ref()?,
                        arguments: self.parse_optional_block_arguments()?,
                    };
                    cases.push(SwitchCase {
                        value: IntegerReference::Integer(case_value),
                        target,
                    });
                }

                Ok(Terminator::Switch {
                    value,
                    default,
                    cases,
                })
            }
            TokenType::Yield => {
                self.bump();
                let value = self.parse_value()?;
                let (resume, unwind) = self.parse_continuation()?;

                Ok(Terminator::Yield {
                    value,
                    resume,
                    unwind,
                })
            }
            TokenType::Trap => {
                self.bump();

                // abort has no payload
                if self.tree.source_text(token.span) == "trap.abort" {
                    return Ok(Terminator::Trap {
                        kind: TrapKind::Abort,
                        payload: None,
                    });
                }

                Err(ParseError::invalid("expected `trap.abort`", token.start))
            }
            TokenType::Panic => {
                self.bump();
                let payload = if self.is_value_reference_start() {
                    Some(self.parse_value()?)
                } else {
                    None
                };
                Ok(Terminator::Panic { payload })
            }
            TokenType::ResumeUnwind => {
                self.bump();
                Ok(Terminator::ResumeUnwind)
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
            TokenType::CallIndirect => {
                self.bump();
                let (callee, arguments, signature) = self.parse_indirect_call_target()?;
                let (target, unwind) = self.parse_continuation()?;
                Ok(Terminator::CallIndirect {
                    callee,
                    call: Call::new(arguments, signature),
                    target,
                    unwind,
                })
            }
            TokenType::TailCallVirtual => {
                self.bump();
                let (receiver, class, slot, arguments, signature) =
                    self.parse_class_call_target()?;
                Ok(Terminator::TailCallVirtual {
                    receiver,
                    class,
                    slot,
                    call: Call::new(arguments, signature),
                })
            }
            TokenType::CallVirtual => {
                self.bump();
                let (receiver, class, slot, arguments, signature) =
                    self.parse_class_call_target()?;
                let (target, unwind) = self.parse_continuation()?;
                Ok(Terminator::CallVirtual {
                    receiver,
                    class,
                    slot,
                    call: Call::new(arguments, signature),
                    target,
                    unwind,
                })
            }
            TokenType::TailCallDynamic => {
                self.bump();
                let (receiver, constraint, slot, arguments, signature) =
                    self.parse_dynamic_call_target()?;
                Ok(Terminator::TailCallDynamic {
                    receiver,
                    constraint,
                    slot,
                    call: Call::new(arguments, signature),
                })
            }
            TokenType::CallDynamic => {
                self.bump();
                let (receiver, constraint, slot, arguments, signature) =
                    self.parse_dynamic_call_target()?;
                let (target, unwind) = self.parse_continuation()?;
                Ok(Terminator::CallDynamic {
                    receiver,
                    constraint,
                    slot,
                    call: Call::new(arguments, signature),
                    target,
                    unwind,
                })
            }
            TokenType::Identifier => self.parse_allocation_try_terminator(&token),
            _ => Err(ParseError::unexpected(
                "terminator",
                self.token_type(&token),
                token.start,
            )),
        }
    }

    /// Return whether the current line starts a fallible allocation terminator.
    fn is_allocation_try_terminator_line(&self) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };
        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        matches!(
            self.tree.source_text(token.span),
            "new.zeroed.try" | "new.uninit.try" | "new.slice.zeroed.try" | "new.slice.uninit.try"
        )
    }

    /// Parse a fallible allocation terminator.
    fn parse_allocation_try_terminator(&mut self, token: &Token) -> ParseResult<Terminator> {
        let opcode = self.tree.source_text(token.span).to_string();
        self.bump();

        match opcode.as_str() {
            "new.zeroed.try" => {
                let layout = self.parse_type()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewZeroedTry {
                    layout: layout.into(),
                    success,
                    failure,
                })
            }
            "new.uninit.try" => {
                let layout = self.parse_type()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewUninitTry {
                    layout: layout.into(),
                    success,
                    failure,
                })
            }
            "new.slice.zeroed.try" => {
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewSliceZeroedTry {
                    element: element.into(),
                    length,
                    success,
                    failure,
                })
            }
            "new.slice.uninit.try" => {
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewSliceUninitTry {
                    element: element.into(),
                    length,
                    success,
                    failure,
                })
            }
            _ => Err(ParseError::unexpected(
                "terminator",
                self.token_type(token),
                token.start,
            )),
        }
    }

    /// Parse success and failure targets for a fallible allocation terminator.
    fn parse_allocation_targets(&mut self) -> ParseResult<(BlockTarget, BlockTarget)> {
        self.eat_token(TokenType::Arrow)?;

        let success = BlockTarget {
            block: self.parse_block_ref()?,
            arguments: self.parse_optional_block_arguments()?,
        };
        self.eat_token(TokenType::Comma)?;
        let failure = BlockTarget {
            block: self.parse_block_ref()?,
            arguments: self.parse_optional_block_arguments()?,
        };

        Ok((success, failure))
    }

    /// Parse a check kind and its operands.
    fn parse_check_kind(&mut self) -> ParseResult<CheckConstraint> {
        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("check kind", self.pos()))?;
        let kind_text = self.tree.source_text(kind_token.span).to_string();
        let kind_start = kind_token.start;

        match self.token_type(kind_token) {
            TokenType::Identifier | TokenType::TypeName | TokenType::Type => self.bump(),
            _ => {
                return Err(ParseError::unexpected(
                    "check kind",
                    self.token_type(kind_token),
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
                let (expected, _) = self.parse_type_reference_part()?;

                Ok(CheckConstraint::Type { value, expected })
            }
            "variantTag" => {
                if kind_parts.len() != 1 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let expected = self.parse_constant()?;

                Ok(CheckConstraint::Variant { value, expected })
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
                let (expected, _) = self.parse_type_reference_part()?;

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
                let (expected, _) = self.parse_type_reference_part()?;

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
    fn parse_optional_block_arguments(&mut self) -> ParseResult<Vec<ValueReference>> {
        if self.eat_token_maybe(TokenType::OpenParenthesis) {
            let arguments = self.parse_value_list()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            Ok(arguments)
        } else {
            Ok(Vec::new())
        }
    }

    /// Parse the continuation for a call terminator.
    /// Parse one continuation: `-> target`, with an optional `| target` unwind alternative.
    fn parse_continuation(&mut self) -> ParseResult<(BlockTarget, Option<BlockTarget>)> {
        self.eat_token(TokenType::Arrow)?;
        let target = BlockTarget {
            block: self.parse_block_ref()?,
            arguments: self.parse_optional_block_arguments()?,
        };

        // the unwind alternative
        if !self.eat_token_maybe(TokenType::Pipe) {
            return Ok((target, None));
        }

        let unwind = BlockTarget {
            block: self.parse_block_ref()?,
            arguments: self.parse_optional_block_arguments()?,
        };

        Ok((target, Some(unwind)))
    }

    /// Collect and predeclare blocks before parsing the function body.
    fn predeclare_blocks(&mut self) -> ParseResult<()> {
        let mut token_index = self.pos;

        while token_index < self.tree.tokens().len() {
            let Some(token) = self.tree.tokens().get(token_index).cloned() else {
                break;
            };

            if self.token_type(&token) == TokenType::CloseBrace
                || self.token_type(&token) == TokenType::End
            {
                break;
            }

            if !self.is_block_label_token(token_index) {
                token_index += 1;
                continue;
            }

            let terminator_id = self.tree.insert(Terminator::Unreachable);
            let block_id = self.tree.insert(Block::new(terminator_id));
            self.predeclared_blocks.push(block_id);

            match self.token_type(&token) {
                TokenType::BlockReference => {
                    let source_index = self
                        .tree
                        .source_text(token.span)
                        .strip_prefix('b')
                        .and_then(|text| text.parse::<u32>().ok())
                        .ok_or_else(|| ParseError::invalid("block label", token.start))?;

                    if self
                        .block_id_by_label_index
                        .insert(source_index, block_id)
                        .is_some()
                    {
                        return Err(ParseError::new(
                            format!(
                                "duplicate block label '{}'",
                                self.tree.source_text(token.span)
                            ),
                            token.start,
                        ));
                    }
                }
                TokenType::Identifier => {
                    if self
                        .block_name_map
                        .insert(self.tree.source_text(token.span).to_string(), block_id)
                        .is_some()
                    {
                        return Err(ParseError::new(
                            format!(
                                "duplicate block label '{}'",
                                self.tree.source_text(token.span)
                            ),
                            token.start,
                        ));
                    }
                }
                _ => unreachable!("block label predeclaration should only see block labels"),
            }

            token_index += 1;
        }

        Ok(())
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
                    if let LocalReference::Local(local_id) = *local {
                        *local = source_to_actual
                            .get(local_id.id as usize)
                            .copied()
                            .map(LocalReference::Local)
                            .unwrap_or(*local);
                    }
                }
                _ => {}
            }
        }
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
