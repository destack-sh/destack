use destack_source::{NodeSpanRegion, NodeSpanType, Span};

use crate::source::{Token, TokenType};
use crate::{
    AllocationMode, Attribute, AttributeArgs, AttributeIdentifier, AttributeValue, BinaryOperator,
    Binding, Block, BlockParameter, BlockTarget, Call, Callee, CheckConstraint, Function,
    FunctionBody, FunctionHeaderSpans, FunctionKind, FunctionParameter, GenericArgument,
    GenericParameter, Instruction, LifetimeParameter, Linkage, Local, LocalNodeId, Mutability,
    SwitchCase, Terminator, TypeId, TypedValueSpan, Value,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

/// Parsed function header.
#[derive(Debug)]
pub(super) struct ParsedFunctionHeader {
    /// The resolved function id.
    pub(super) function_id: LocalNodeId<Function>,
    /// The role the keyword declares.
    pub(super) kind: FunctionKind,
    /// The function keyword span.
    pub(super) keyword_span: Span,
    /// The parsed function name.
    pub(super) name: String,
    /// The parsed generic arguments.
    pub(super) arguments: Vec<GenericArgument>,
    /// The parsed generic parameters.
    pub(super) generics: Vec<GenericParameter>,
    /// The parsed function name span.
    pub(super) name_span: Span,
    /// The parsed lifetime parameters.
    pub(super) lifetimes: Vec<LifetimeParameter>,
    /// The parsed function parameters.
    pub(super) parameters: Vec<FunctionParameter>,
    /// The parsed parameter spans.
    pub(super) parameter_spans: Vec<TypedValueSpan>,
    /// The parsed return type.
    pub(super) return_type: TypeId,
    /// The full signature span.
    pub(super) signature_span: Span,
    /// The opening parenthesis span.
    pub(super) open_paren_span: Span,
    /// The closing parenthesis span.
    pub(super) close_paren_span: Span,
    /// The return colon span.
    pub(super) return_colon_span: Span,
}

/// Function attributes after extracting first-class function fields.
#[derive(Debug)]
pub(super) struct FunctionAttributes {
    /// The hidden environment type when present.
    pub(super) environment_type: Option<TypeId>,
    /// The runtime binding declaration when present.
    pub(super) binding: Option<Binding>,
    /// Generic attributes that remain attached to the function node.
    pub(super) attributes: Vec<Attribute>,
    /// Spans for generic attributes that remain attached to the function node.
    pub(super) attribute_spans: Vec<Span>,
}

/// Function header parse mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FunctionHeaderMode {
    /// Parse only enough to seed forward references.
    Signature,
    /// Parse the complete header and populate the body value namespace.
    Definition,
}

impl Parser {
    /// Return whether the next token opens a function or constructor.
    pub(super) fn peek_is_function(&self) -> bool {
        self.peek_is(TokenType::Function) || self.peek_is(TokenType::Constructor)
    }

    /// Extract first-class function fields from parsed attributes.
    pub(super) fn extract_function_attributes(
        &mut self,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<FunctionAttributes> {
        // first-class fields
        let mut environment_type = None;
        let mut binding = None;
        let mut retained_attributes = Vec::new();
        let mut retained_attribute_spans = Vec::new();

        // inspect attributes
        for (attribute, attribute_span) in attributes.into_iter().zip(attribute_spans) {
            let name = match attribute.name {
                AttributeIdentifier::Identifier(name) => self.strings.get(name).to_string(),
                AttributeIdentifier::Missing | AttributeIdentifier::Error => {
                    retained_attributes.push(attribute);
                    retained_attribute_spans.push(attribute_span);
                    continue;
                }
            };
            if name.as_str() == "environment" {
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
                continue;
            }

            if name.as_str() == "binding" {
                if binding.is_some() {
                    return Err(ParseError::new("duplicate binding attribute", self.pos()));
                }

                binding = Some(self.parse_binding(&attribute.args)?);
                continue;
            }

            retained_attributes.push(attribute);
            retained_attribute_spans.push(attribute_span);
        }

        Ok(FunctionAttributes {
            environment_type,
            binding,
            attributes: retained_attributes,
            attribute_spans: retained_attribute_spans,
        })
    }

    /// Parse one function header.
    pub(super) fn parse_function_header(
        &mut self,
        linkage: Linkage,
        mode: FunctionHeaderMode,
    ) -> ParseResult<ParsedFunctionHeader> {
        // keyword and name
        let kind = match self.peek_is(TokenType::Constructor) {
            true => FunctionKind::Constructor,
            false => FunctionKind::Function,
        };
        let keyword_token = match kind {
            FunctionKind::Constructor => self.eat_token(TokenType::Constructor)?,
            FunctionKind::Function => self.eat_token(TokenType::Function)?,
        };
        let keyword_start = keyword_token.start();
        let keyword_length = self.tree.source_text(keyword_token.span).len();
        let keyword_span = self.span_at(keyword_start, keyword_length);
        let (name, name_start) = self.parse_symbol_name()?;
        let (arguments, generics, mut lifetimes) = self.parse_declaration_parameters(false)?;
        let name_span = if arguments.is_empty() && generics.is_empty() && lifetimes.is_empty() {
            self.span_at(name_start, name.len())
        } else {
            self.span_between(name_start, self.pos())
        };
        let key = (name.clone(), arguments.clone());
        let function_id = self.function_map.get(&key).copied().ok_or_else(|| {
            ParseError::new(format!("function '{name}' is not declared"), name_start)
        })?;

        // body value namespace
        if mode == FunctionHeaderMode::Definition {
            self.current_function = Some(function_id);
            self.reset_function_parse_state();
        }

        // parameters and result
        let signature_start = self.pos();
        let (parameters, parameter_spans, open_paren_span, close_paren_span) =
            self.parse_function_parameters(linkage, mode)?;
        let return_colon_token = self.eat_token(TokenType::Colon)?;
        let return_colon_start = return_colon_token.start();
        let return_colon_length = self.tree.source_text(return_colon_token.span).len();
        let return_colon_span = self.span_at(return_colon_start, return_colon_length);
        let (return_type, return_type_span) =
            self.parse_function_return_type(return_colon_token, mode)?;
        self.parse_lifetime_where(&mut lifetimes)?;
        let signature_span = self.span_between(signature_start, return_type_span.end as usize);

        Ok(ParsedFunctionHeader {
            function_id,
            kind,
            keyword_span,
            name,
            arguments,
            generics,
            name_span,
            lifetimes,
            parameters,
            parameter_spans,
            return_type,
            signature_span,
            open_paren_span,
            close_paren_span,
            return_colon_span,
        })
    }

    /// Parse a function definition or declaration.
    pub(super) fn parse_function(
        &mut self,
        item_start: usize,
        linkage: Linkage,
        attributes: Vec<Attribute>,
        attribute_spans: Vec<Span>,
    ) -> ParseResult<LocalNodeId<Function>> {
        // function signature
        let header = self.parse_function_header(linkage, FunctionHeaderMode::Definition)?;
        let function_id = header.function_id;

        // function attributes
        let function_attributes = self.extract_function_attributes(attributes, attribute_spans)?;

        // declare an import or a shared specialization without a body
        let is_opaque = linkage == Linkage::Shared && self.peek_is(TokenType::Semicolon);
        if linkage.is_import() || is_opaque {
            let name_id = self.strings.intern(&header.name);
            let symbol = self.tree.get(function_id).symbol;
            let mut function = match linkage.is_import() {
                true => Function::import(
                    self.module,
                    name_id,
                    header.lifetimes.clone(),
                    header.parameters.clone(),
                    header.return_type,
                ),
                false => Function::declare(
                    self.module,
                    name_id,
                    header.lifetimes.clone(),
                    header.parameters.clone(),
                    header.return_type,
                )
                .with_linkage(Linkage::Shared),
            }
            .with_arguments(header.arguments.clone())
            .with_symbol(symbol)
            .with_kind(header.kind);
            function.generics = header.generics.clone();
            function.environment = function_attributes.environment_type;
            function.binding = function_attributes.binding.map(Box::new);
            function.allocation = AllocationMode::Any; // #Incomplete: set proper MIR allocation mode?

            // replace the reserved function with its parsed declaration
            self.tree
                .set_text_span(function_id, self.span_from_parse_start(item_start));
            self.tree.set_keyword_span(function_id, header.keyword_span);
            self.tree.set_main_span(function_id, header.name_span);
            self.tree.set_side_span(
                function_id,
                NodeSpanType::Region(NodeSpanRegion::Type),
                header.signature_span,
            );
            self.tree
                .set_attribute_spans(function_id, function_attributes.attribute_spans);
            self.tree
                .set_function_parameter_spans(function_id, header.parameter_spans);
            self.tree.set_function_header_spans(
                function_id,
                FunctionHeaderSpans::new(
                    header.open_paren_span,
                    header.close_paren_span,
                    header.return_colon_span,
                    None,
                ),
            );
            *self.tree.get_mut(function_id) = function;
            self.current_function = None;
            self.pop_lifetime_scope();

            // record attributes
            if !function_attributes.attributes.is_empty() {
                self.tree
                    .set_attributes(function_id, function_attributes.attributes);
            }

            // optional declaration terminator
            self.eat_token_if(TokenType::Semicolon);

            return Ok(function_id);
        }

        // seed signature state before filling the reserved function
        let name_id = self.strings.intern(&header.name);
        let id = function_id;
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));
        self.tree.set_keyword_span(id, header.keyword_span);
        self.tree.set_main_span(id, header.name_span);
        self.tree.set_side_span(
            id,
            NodeSpanType::Region(NodeSpanRegion::Type),
            header.signature_span,
        );
        self.tree
            .set_attribute_spans(id, function_attributes.attribute_spans);
        self.tree
            .set_function_parameter_spans(id, header.parameter_spans);
        let parameters = header.parameters;
        let (next_value_id, value_types) = Function::parameter_state(&parameters);

        // start body value types
        self.value_types = value_types;
        self.next_value_id = next_value_id;

        // populate signature fields
        let function = self.tree.get_mut(id);
        function.name = name_id;
        function.kind = header.kind;
        function.arguments = header.arguments;
        function.generics = header.generics.clone();
        function.parameters = parameters;
        function.lifetimes = header.lifetimes;
        function.return_type = header.return_type;
        function.linkage = linkage;
        function.environment = function_attributes.environment_type;
        function.binding = function_attributes.binding.map(Box::new);

        // body
        let open_brace_token = self.eat_token(TokenType::OpenBrace)?;
        let open_brace_start = open_brace_token.start();
        let open_brace_length = self.tree.source_text(open_brace_token.span).len();
        let open_brace_span = self.span_at(open_brace_start, open_brace_length);
        self.tree.set_function_header_spans(
            id,
            FunctionHeaderSpans::new(
                header.open_paren_span,
                header.close_paren_span,
                header.return_colon_span,
                Some(open_brace_span),
            ),
        );

        // locals
        let mut locals = Vec::new();
        while self.peek_is(TokenType::Local) {
            let recovery_pos = self.pos();
            match self.parse_local() {
                Ok(local) => locals.push(local),
                Err(error) => {
                    self.diagnostics
                        .insert(error.to_diagnostic(self.blob, self.file_id));
                    self.try_recover_to_block(recovery_pos);
                    break;
                }
            }
        }

        // predeclare symbolic block labels so references can resolve forward
        self.predeclare_blocks()?;

        // blocks
        let mut blocks = Vec::new();
        while !self.peek_is(TokenType::CloseBrace) && !self.peek_is(TokenType::End) {
            // block headers
            if self.is_block_label_start() {
                blocks.push(self.parse_block_recovering());
                continue;
            }

            // stray body tokens
            let error = ParseError::new("expected block label", self.pos());
            self.diagnostics
                .insert(error.to_diagnostic(self.blob, self.file_id));
            self.try_recover_to_block(self.pos());
        }

        self.eat_token(TokenType::CloseBrace)?;

        // finalize body
        let entry = blocks
            .first()
            .copied()
            .ok_or_else(|| ParseError::new("function must have at least one block", self.pos()))?;

        let body = FunctionBody::new(
            entry,
            blocks,
            locals,
            std::mem::take(&mut self.value_types),
            self.next_value_id,
            &self.tree,
        );
        self.tree.get_mut(id).set_body(body);

        self.current_function = None;
        self.pop_lifetime_scope();
        self.tree
            .set_text_span(id, self.span_from_parse_start(item_start));

        // record attributes
        if !function_attributes.attributes.is_empty() {
            self.tree.set_attributes(id, function_attributes.attributes);
        }

        Ok(id)
    }

    /// Parse one function parameter list.
    fn parse_function_parameters(
        &mut self,
        linkage: Linkage,
        mode: FunctionHeaderMode,
    ) -> ParseResult<(Vec<FunctionParameter>, Vec<TypedValueSpan>, Span, Span)> {
        let open_paren_token = self.eat_token(TokenType::OpenParenthesis)?;
        let open_paren_start = open_paren_token.start();
        let open_paren_length = self.tree.source_text(open_paren_token.span).len();
        let open_paren_span = self.span_at(open_paren_start, open_paren_length);

        let (parameters, parameter_spans) = if linkage.is_import() {
            let mut parameters = Vec::new();
            let mut parameter_spans = Vec::new();
            while !self.peek_is(TokenType::CloseParenthesis) {
                let parameter_start = self.pos();
                let (ty, type_span) = self.parse_type_use_part()?;
                let parameter_span = self.span_from_parse_start(parameter_start);
                let value = Value::new(parameters.len() as u32);
                parameters.push(FunctionParameter { value, ty });
                parameter_spans.push(TypedValueSpan::new(parameter_span, None, type_span));
                if !self.eat_token_if(TokenType::Comma) {
                    break;
                }
            }

            (parameters, parameter_spans)
        } else {
            let mut parameters = Vec::new();
            let mut parameter_spans = Vec::new();
            let mut next_value_id = 0u32;
            while self.is_value_definition_start() {
                let parameter_start = self.pos();
                let (value, name_span) = match mode {
                    FunctionHeaderMode::Signature => {
                        self.parse_signature_parameter(&mut next_value_id)?
                    }
                    FunctionHeaderMode::Definition => self.parse_value_definition_part()?,
                };
                let colon_token = self.eat_token(TokenType::Colon)?;
                let (ty, type_span) = self.parse_function_parameter_type(colon_token, mode)?;
                let parameter_span = self.span_from_parse_start(parameter_start);
                parameters.push(FunctionParameter { value, ty });
                parameter_spans.push(TypedValueSpan::new(
                    parameter_span,
                    Some(name_span),
                    type_span,
                ));
                if !self.eat_token_if(TokenType::Comma) {
                    break;
                }
            }

            (parameters, parameter_spans)
        };

        let close_paren_token = self.eat_token(TokenType::CloseParenthesis)?;
        let close_paren_start = close_paren_token.start();
        let close_paren_length = self.tree.source_text(close_paren_token.span).len();
        let close_paren_span = self.span_at(close_paren_start, close_paren_length);

        Ok((
            parameters,
            parameter_spans,
            open_paren_span,
            close_paren_span,
        ))
    }

    /// Parse one function parameter type.
    fn parse_function_parameter_type(
        &mut self,
        colon_token: Token,
        mode: FunctionHeaderMode,
    ) -> ParseResult<(TypeId, Span)> {
        // real definitions recover and report local type holes
        if mode == FunctionHeaderMode::Definition {
            return Ok(self.parse_type_use_after(colon_token, "parameter type"));
        }

        self.parse_type_use_part()
    }

    /// Parse one function return type.
    fn parse_function_return_type(
        &mut self,
        colon_token: Token,
        mode: FunctionHeaderMode,
    ) -> ParseResult<(TypeId, Span)> {
        // real definitions recover and report local type holes
        if mode == FunctionHeaderMode::Definition {
            return Ok(self.parse_return_type_use_after(colon_token));
        }

        // signature scanning must not treat a body brace as a return type
        if self.peek_is(TokenType::OpenBrace) && !self.is_return_structural_type_start() {
            let start = colon_token.span.end as usize;
            let span = self.span_at(start, 0);
            let ty = self.error_type();

            return Ok((ty, span));
        }

        self.parse_type_use_part()
    }

    /// Parse one named parameter without recording it in the body namespace.
    fn parse_signature_parameter(&mut self, next_value_id: &mut u32) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("value definition", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start();
                self.bump();

                let value = self.parse_value_id(&name, start)?;
                *next_value_id = (*next_value_id).max(value.id() + 1);

                Ok((value, span))
            }
            _ => Err(ParseError::unexpected(
                "value definition",
                self.token_type(token),
                token.start(),
            )),
        }
    }

    /// Parse a local variable declaration.
    fn parse_local(&mut self) -> ParseResult<LocalNodeId<Local>> {
        // whole declaration
        let local_start = self.pos();

        self.eat_token(TokenType::Local)?;

        // local reference
        let local_token = self.eat_token(TokenType::Identifier)?;
        let local_name_text = self.tree.source_text(local_token.span).to_string();
        let local_name_start = local_token.start();
        let local_name_length = local_name_text.len();
        let local_span = self.span_at(local_name_start, local_name_length);
        if self.local_name_map.contains_key(&local_name_text) {
            return Err(ParseError::new(
                format!("duplicate local name '{local_name_text}'"),
                local_name_start,
            ));
        }

        // local type
        let colon_token = self.eat_token(TokenType::Colon)?;
        let (ty, type_span) = self.parse_type_use_after(colon_token, "local type");

        // local annotations
        let mut mutability = Mutability::Mutable;
        while self.eat_token_if(TokenType::Comma) {
            // mutability
            if self.eat_token_if(TokenType::Readonly) {
                mutability = Mutability::Immutable;
            }
            // reject unknown local qualifiers
            else {
                return Err(ParseError::invalid("local qualifier", self.pos()));
            }
        }

        // record the local
        let local = Local::new(ty, mutability);
        let local_id = self.tree.insert(local);
        self.local_name_map
            .insert(local_name_text.clone(), local_id);
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
        let block_span = {
            let block_token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("block label", self.pos()))?;
            let block_span = block_token.span;

            match self.token_type(block_token) {
                TokenType::Identifier => {
                    self.bump();
                }
                _ => {
                    return Err(ParseError::unexpected(
                        "block label",
                        self.token_type(block_token),
                        block_token.start(),
                    ));
                }
            }

            block_span
        };

        // block parameters
        let parameters = if self.eat_token_if(TokenType::OpenParenthesis) {
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
            self.record_value_type(param.value, param.ty)?;
        }

        self.eat_token(TokenType::Colon)?;

        // block contents
        let mut instructions = Vec::new();
        let mut terminator = None;
        let mut terminator_span = None;
        let mut terminator_main_span = None;
        let mut is_broken = false;

        while !self.is_block_label_start()
            && !self.peek_is(TokenType::CloseBrace)
            && !self.peek_is(TokenType::End)
        {
            // terminators
            if self
                .peek()
                .is_some_and(|token| self.token_type(token).is_terminator())
                || self.is_allocation_try_terminator_line()
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
                            .insert(error.to_diagnostic(self.blob, self.file_id));
                        self.try_recover_to_block(recovery_pos);
                        terminator_span = Some(self.span_between(error.position(), self.pos()));
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
            match self.parse_instruction() {
                Ok(inst) => instructions.push(inst),
                Err(error) => {
                    self.diagnostics
                        .insert(error.to_diagnostic(self.blob, self.file_id));
                    let continue_block = self.try_recover_in_block(recovery_index);
                    let error_end = self.pos();
                    let error_span = self.span_between(instruction_start, error_end);
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
                    .insert(error.to_diagnostic(self.blob, self.file_id));
                self.create_error_block(self.pos())
            }
        };
        let terminator_id = self.tree.get(block_id).terminator;
        let recovery_pos = self.pos();

        let parsed_block = match self.parse_block() {
            Ok(block_id) => block_id,
            Err(error) => {
                self.diagnostics
                    .insert(error.to_diagnostic(self.blob, self.file_id));
                self.try_recover_to_block(recovery_pos);

                let error_start = error.position();
                let error_end = self.pos();
                let error_span = self.span_between(error_start, error_end);
                let block = Block {
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

        while !self.peek_is(TokenType::CloseBrace) && !self.peek_is(TokenType::End) {
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

            if self.peek_is(TokenType::CloseBrace)
                || self.peek_is(TokenType::End)
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

                return !(self.peek_is(TokenType::CloseBrace)
                    || self.peek_is(TokenType::End)
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
    fn parse_entry_block_parameters(&mut self) -> ParseResult<Vec<BlockParameter>> {
        let function_id = self.current_function.ok_or_else(|| {
            ParseError::new(
                "entry block parameters require a current function",
                self.pos(),
            )
        })?;
        let parameters = self
            .tree
            .get(function_id)
            .parameters
            .iter()
            .map(FunctionParameter::block_parameter)
            .collect::<Vec<_>>();

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
            let (ty, _) = self.parse_type_use_part()?;
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
    fn parse_entry_block_parameter(&mut self) -> ParseResult<(Value, Span)> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("entry block parameter", self.pos()))?;
        let span = token.span;

        match self.token_type(token) {
            TokenType::Identifier => {
                let name = self.tree.source_text(token.span).to_string();
                let start = token.start();
                self.bump();

                let value = self.parse_value_id(&name, start)?;
                if !self.defined_values.contains(&value) {
                    return Err(ParseError::new(format!("undefined value '{name}'"), start));
                }

                Ok((value, span))
            }
            _ => Err(ParseError::unexpected(
                "entry block parameter",
                self.token_type(token),
                token.start(),
            )),
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
                let value = if !self.has_line_break_after(&token) && self.is_value_reference_start()
                {
                    Some(self.parse_value()?)
                } else {
                    None
                };
                Ok(Terminator::Return { value })
            }
            token_type if token_type.is_invoke() => {
                let call = self.parse_terminator_call(&token)?;
                let (target, unwind) = self.parse_invoke_continuation()?;

                Ok(Terminator::Invoke {
                    call,
                    target,
                    unwind,
                })
            }
            TokenType::Jump => {
                self.bump();
                let target = self.parse_block_target()?;

                Ok(Terminator::Jump { target })
            }
            TokenType::Branch => {
                self.bump();
                let condition = self.parse_value()?;
                self.eat_token(TokenType::FatArrow)?;
                let then_target = self.parse_block_target()?;
                self.eat_token(TokenType::Pipe)?;
                let else_target = self.parse_block_target()?;

                Ok(Terminator::Branch {
                    condition,
                    then_target,
                    else_target,
                })
            }
            TokenType::Check => {
                self.bump();
                if self.has_line_break_after(&token) {
                    return Err(ParseError::unexpected_end("check kind", self.pos()));
                }

                let constraint = self.parse_check_kind()?;
                self.eat_token(TokenType::FatArrow)?;
                let success = self.parse_block_target()?;
                self.eat_token(TokenType::Pipe)?;
                let failure = self.parse_block_target()?;
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
                let default = self.parse_block_target()?;

                let mut cases = Vec::new();
                while self.eat_token_if(TokenType::Comma) {
                    let case_value = self.parse_int_literal()?;
                    self.eat_token(TokenType::FatArrow)?;
                    let target = self.parse_block_target()?;
                    cases.push(SwitchCase {
                        value: case_value,
                        target,
                    });
                }
                let cases = self.tree.add_switch_cases(&cases);

                Ok(Terminator::Switch {
                    value,
                    default,
                    cases,
                })
            }
            TokenType::Abort => {
                self.bump();
                let payload =
                    if !self.has_line_break_after(&token) && self.is_value_reference_start() {
                        Some(self.parse_value()?)
                    } else {
                        None
                    };
                Ok(Terminator::Abort { payload })
            }
            TokenType::Panic => {
                self.bump();
                let payload =
                    if !self.has_line_break_after(&token) && self.is_value_reference_start() {
                        Some(self.parse_value()?)
                    } else {
                        None
                    };
                Ok(Terminator::Panic { payload })
            }
            TokenType::UnwindResume => {
                self.bump();
                Ok(Terminator::UnwindResume)
            }
            TokenType::Unreachable => {
                self.bump();
                Ok(Terminator::Unreachable)
            }
            token_type if token_type.is_tail_call() => {
                let call = self.parse_terminator_call(&token)?;

                Ok(Terminator::TailCall { call })
            }
            TokenType::Identifier if self.tree.source_text(token.span) == "variant.switch" => {
                self.parse_variant_switch_terminator()
            }
            TokenType::Identifier => self.parse_allocation_try_terminator(&token),
            _ => Err(ParseError::unexpected(
                "terminator",
                self.token_type(&token),
                token.start(),
            )),
        }
    }

    /// Parse one call shared by invoke and tail-call terminators.
    fn parse_terminator_call(&mut self, token: &Token) -> ParseResult<Call> {
        let token_type = self.token_type(token);
        self.bump();

        // parse the dispatch-specific callable target
        let (callee, arguments, signature) = match token_type {
            TokenType::Invoke | TokenType::TailCall => self.parse_direct_call_target()?,
            TokenType::InvokeWitness | TokenType::TailCallWitness => {
                self.parse_witness_call_target()?
            }
            TokenType::InvokeIndirect | TokenType::TailCallIndirect => {
                let (value, arguments, signature) = self.parse_indirect_call_target()?;

                (Callee::Indirect { value }, arguments, signature)
            }
            TokenType::InvokeVirtual | TokenType::TailCallVirtual => {
                let (receiver, class, slot, arguments, signature) =
                    self.parse_virtual_call_target()?;
                let callee = Callee::Virtual {
                    receiver,
                    class,
                    slot,
                };

                (callee, arguments, signature)
            }
            TokenType::InvokeDynamic | TokenType::TailCallDynamic => {
                let (receiver, constraint, slot, arguments, signature) =
                    self.parse_dynamic_call_target()?;
                let callee = Callee::Dynamic {
                    receiver,
                    constraint,
                    slot,
                };

                (callee, arguments, signature)
            }
            _ => {
                return Err(ParseError::unexpected(
                    "invoke or tail call",
                    token_type,
                    token.start(),
                ));
            }
        };
        let arguments = self.tree.add_values(&arguments);

        Ok(Call::new(callee, arguments, signature))
    }

    /// Parse a variant switch terminator.
    fn parse_variant_switch_terminator(&mut self) -> ParseResult<Terminator> {
        self.bump();
        let value = self.parse_value()?;

        // parse indexed cases and one optional trailing else target
        let mut cases = Vec::new();
        let mut default = None;
        while self.eat_token_if(TokenType::Comma) {
            // one trailing else target names the non-exhaustive default
            let is_else = self
                .peek()
                .is_some_and(|token| self.tree.source_text(token.span) == "else");
            if is_else {
                self.bump();
                default = Some(self.parse_block_target()?);
                break;
            }
            let case_value = self.parse_int_literal()?;
            self.eat_token(TokenType::FatArrow)?;
            let target = self.parse_block_target()?;
            cases.push(SwitchCase {
                value: case_value,
                target,
            });
        }
        let cases = self.tree.add_switch_cases(&cases);

        Ok(Terminator::VariantSwitch {
            value,
            default,
            cases,
        })
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
            "new.zeroed.try"
                | "new.uninit.try"
                | "new.slice.zeroed.try"
                | "new.slice.uninit.try"
                | "variant.switch"
        )
    }

    /// Parse a fallible allocation terminator.
    fn parse_allocation_try_terminator(&mut self, token: &Token) -> ParseResult<Terminator> {
        let opcode = self.tree.source_text(token.span).to_string();
        self.bump();

        match opcode.as_str() {
            "new.zeroed.try" => {
                let storage_type = self.parse_type()?;
                let space = self.parse_allocation_space()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewZeroedTry {
                    storage_type,
                    space,
                    success,
                    failure,
                })
            }
            "new.uninit.try" => {
                let storage_type = self.parse_type()?;
                let space = self.parse_allocation_space()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewUninitTry {
                    storage_type,
                    space,
                    success,
                    failure,
                })
            }
            "new.slice.zeroed.try" => {
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                let space = self.parse_allocation_space()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewSliceZeroedTry {
                    element,
                    length,
                    space,
                    success,
                    failure,
                })
            }
            "new.slice.uninit.try" => {
                let element = self.parse_type()?;
                self.eat_token(TokenType::Comma)?;
                let length = self.parse_value()?;
                let space = self.parse_allocation_space()?;
                let (success, failure) = self.parse_allocation_targets()?;

                Ok(Terminator::NewSliceUninitTry {
                    element,
                    length,
                    space,
                    success,
                    failure,
                })
            }
            _ => Err(ParseError::unexpected(
                "terminator",
                self.token_type(token),
                token.start(),
            )),
        }
    }

    /// Parse success and failure targets for a fallible allocation terminator.
    fn parse_allocation_targets(&mut self) -> ParseResult<(BlockTarget, BlockTarget)> {
        self.eat_token(TokenType::FatArrow)?;

        let success = self.parse_block_target()?;
        self.eat_token(TokenType::Pipe)?;
        let failure = self.parse_block_target()?;

        Ok((success, failure))
    }

    /// Parse a check kind and its operands.
    fn parse_check_kind(&mut self) -> ParseResult<CheckConstraint> {
        let kind_token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("check kind", self.pos()))?;
        let kind_text = self.tree.source_text(kind_token.span).to_string();
        let kind_start = kind_token.start();

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
            "div" if kind_parts.get(1) == Some(&"zero") => {
                if kind_parts.len() != 2 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let divisor = self.parse_value()?;
                Ok(CheckConstraint::DivZero { divisor })
            }
            "is" if kind_parts.get(1) == Some(&"type") => {
                if kind_parts.len() != 2 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let (expected, _) = self.parse_type_use_part()?;

                Ok(CheckConstraint::IsType { value, expected })
            }
            "is" if kind_parts.get(1) == Some(&"subtype") => {
                if kind_parts.len() != 2 {
                    return Err(ParseError::invalid(
                        &format!("check kind '{kind_text}'"),
                        kind_start,
                    ));
                }

                let value = self.parse_value()?;
                self.eat_token(TokenType::Comma)?;
                let (expected, _) = self.parse_type_use_part()?;

                Ok(CheckConstraint::IsSubtype { value, expected })
            }
            "shift" if kind_parts.get(1) == Some(&"range") => {
                let signedness = kind_parts.get(2).copied().ok_or_else(|| {
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

                if kind_parts.len() != 3 {
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
            "narrow" if kind_parts.get(1) == Some(&"range") => {
                let signedness = kind_parts.get(2).copied().ok_or_else(|| {
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

                if kind_parts.len() != 3 {
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
            _ if kind_parts.len() >= 3 && kind_parts[kind_parts.len() - 2] == "overflow" => {
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
        if self.eat_token_if(TokenType::OpenParenthesis) {
            let arguments = self.parse_value_list()?;
            self.eat_token(TokenType::CloseParenthesis)?;
            Ok(arguments)
        } else {
            Ok(Vec::new())
        }
    }

    /// Parse one control-flow edge target: a block and its optional arguments.
    fn parse_block_target(&mut self) -> ParseResult<BlockTarget> {
        let block = self.parse_block_ref()?;
        let arguments = self.parse_optional_block_arguments()?;
        let arguments = self.tree.add_values(&arguments);

        Ok(BlockTarget::new(block, arguments))
    }

    /// Parse mandatory normal and unwind continuations for one invoke.
    fn parse_invoke_continuation(&mut self) -> ParseResult<(BlockTarget, BlockTarget)> {
        self.eat_token(TokenType::FatArrow)?;
        let target = self.parse_block_target()?;
        self.eat_token(TokenType::Pipe)?;
        let unwind = self.parse_block_target()?;

        Ok((target, unwind))
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
                TokenType::Identifier => {
                    let token_text = self.tree.source_text(token.span).to_string();
                    if self
                        .block_name_map
                        .insert(token_text.clone(), block_id)
                        .is_some()
                    {
                        return Err(ParseError::new(
                            format!("duplicate block label '{token_text}'"),
                            token.start(),
                        ));
                    }
                }
                _ => unreachable!("block label predeclaration should only see block labels"),
            }

            token_index += 1;
        }

        Ok(())
    }
}

/// Parse the canonical operator family used in overflow checks.
fn parse_overflow_check_operator(text: &str) -> Option<BinaryOperator> {
    Some(match text {
        "add" => BinaryOperator::Add,
        "sub" => BinaryOperator::Subtract,
        "mul" => BinaryOperator::Multiply,
        "div" => BinaryOperator::Divide,
        "rem" => BinaryOperator::Remainder,
        _ => return None,
    })
}
