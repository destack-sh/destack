use crate::{
    Coroutine, Function, FunctionBuilder, FunctionId, InstructionBuilder, Label, Opcode, Parameter,
    ParseError, ParseResult, Parser, RegisterId, RegisterSpan, RelocationTag, Token, TokenType,
    TypeId,
};
use destack_core::{EntryRange, Optional};
use destack_source::Span;

/// Parser state for one physical bytecode function.
#[derive(Debug)]
pub(super) struct FunctionParser {
    /// Encoded function body under construction.
    pub(super) builder: FunctionBuilder,
}

impl FunctionParser {
    /// Create parser state for one function.
    pub(super) fn new(parameters: &[Parameter], span: Span) -> ParseResult<Self> {
        let mut builder = FunctionBuilder::new();

        // reserve every entry register even when the body never reads it
        for parameter in parameters {
            builder
                .reserve(parameter.registers)
                .map_err(|error| ParseError::new(error.to_string(), span))?;
        }

        Ok(Self { builder })
    }

    /// Encode one complete instruction into this function.
    pub(super) fn emit(
        &mut self,
        instruction: InstructionBuilder,
        results: &[RegisterSpan],
        span: Span,
    ) -> ParseResult<()> {
        self.builder.begin_operation();
        self.builder
            .emit(instruction, results)
            .map_err(|error| ParseError::new(error.to_string(), span))?;

        Ok(())
    }

    /// Define one branch label at the current code offset.
    pub(super) fn define(&mut self, label: Label, span: Span) -> ParseResult<()> {
        self.builder
            .define(label)
            .map_err(|error| ParseError::new(error.to_string(), span))
    }
}

impl Parser<'_> {
    /// Parse one bytecode function declaration or definition.
    pub(super) fn parse_function(&mut self) -> ParseResult<()> {
        let is_async = self.eat_name_if("async");
        self.eat_name("function")?;
        let is_generator = self.eat_token_if(TokenType::Star);
        let coroutine = Coroutine::new(is_async, is_generator);
        let (function_id, parameters, parameter_range, result) =
            self.parse_function_declaration(coroutine)?;

        // declarations have no physical body
        if !self.eat_token_if(TokenType::OpenBrace) {
            return Ok(());
        }
        if !self.definitions.insert(function_id) {
            return Err(ParseError::new(
                "function is already defined",
                self.previous().span,
            ));
        }

        // parse the physical body and derive its register width
        let mut function = FunctionParser::new(&parameters, self.previous().span)?;
        while !self.eat_token_if(TokenType::CloseBrace) {
            if self.is_label() {
                let token = self.bump();
                let label = self.parse_label_token(token)?;
                self.eat_token(TokenType::Colon)?;
                function.define(label, token.span)?;
            } else {
                self.parse_instruction(&mut function)?;
            }
        }

        let body = function
            .builder
            .build()
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        // append function-owned sections and publish the physical row
        let operations = self.object.push_operations(body.operations);
        let code = self.object.push_code(&body.code, body.relocations);
        let function = Function::new(
            Optional::some(code),
            parameter_range,
            EntryRange::empty(),
            operations,
            result,
            body.register_count,
            coroutine,
            body.counter_count,
            body.sampler_count,
        );
        self.declarations[function_id.index()] = Some(function);

        Ok(())
    }

    /// Parse one physical function declaration.
    fn parse_function_declaration(
        &mut self,
        coroutine: Coroutine,
    ) -> ParseResult<(FunctionId, Vec<Parameter>, EntryRange<Parameter>, TypeId)> {
        let token = self.eat_token(TokenType::Identifier)?;
        let name = self.text(token).to_string();
        let id = self.function_id(name);
        let parameters = self.parse_parameters()?;
        self.eat_token(TokenType::Colon)?;
        let result = self.parse_type_id()?;
        let range = self.object.push_parameters(parameters.iter().copied());
        let declaration = Function::declaration(range, result, coroutine);
        self.declarations[id.index()] = Some(declaration);

        Ok((id, parameters, range, result))
    }

    /// Parse physical entry parameters in declaration order.
    fn parse_parameters(&mut self) -> ParseResult<Vec<Parameter>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut parameters = Vec::new();

        // parse each register range and semantic type identity
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            if !parameters.is_empty() {
                self.eat_token(TokenType::Comma)?;
            }
            parameters.push(self.parse_parameter()?);
        }

        Ok(parameters)
    }

    /// Parse one physical entry parameter.
    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        let start = self.parse_register()?;
        self.eat_token(TokenType::Colon)?;
        let token = self.eat_token(TokenType::Identifier)?;

        // distinguish a range end from a one-word parameter type
        let (registers, ty) = if let Some(end) = Self::numbered(self.text(token), 'r') {
            let end = u16::try_from(end)
                .ok()
                .map(RegisterId)
                .ok_or_else(|| ParseError::new("expected register", token.span))?;
            if end.0 < start.0 {
                return Err(ParseError::new(
                    "register span ends before it starts",
                    token.span,
                ));
            }
            self.eat_token(TokenType::Colon)?;
            let ty = self.parse_type_id()?;
            let registers = RegisterSpan::new(start, end.0 - start.0 + 1);

            (registers, ty)
        } else {
            let ty = TypeId::from_name(self.text(token))
                .ok_or_else(|| ParseError::new("expected type id", token.span))?;

            (RegisterSpan::new(start, 1), ty)
        };

        Ok(Parameter::new(registers, ty))
    }

    /// Return whether the next tokens form a branch label declaration.
    pub(super) fn is_label(&mut self) -> bool {
        let position = self.cursor.position();
        let token = self.bump();
        let is_label = token.ty == TokenType::Identifier
            && Self::numbered(self.text(token), 'b').is_some()
            && self.peek_is(TokenType::Colon);
        self.cursor.seek(position);

        is_label
    }

    /// Parse one physical register id.
    pub(super) fn parse_register(&mut self) -> ParseResult<RegisterId> {
        let token = self.eat_token(TokenType::Identifier)?;
        let index = Self::numbered(self.text(token), 'r')
            .and_then(|index| u16::try_from(index).ok())
            .ok_or_else(|| ParseError::new("expected register", token.span))?;

        Ok(RegisterId(index))
    }

    /// Parse one contiguous register span with an inclusive end.
    pub(super) fn parse_register_span(&mut self) -> ParseResult<RegisterSpan> {
        let start = self.parse_register()?;
        let word_count = if self.eat_token_if(TokenType::Colon) {
            let end = self.parse_register()?;
            if end.0 < start.0 {
                return Err(ParseError::new(
                    "register span ends before it starts",
                    self.previous().span,
                ));
            }

            end.0 - start.0 + 1
        } else {
            1
        };

        Ok(RegisterSpan::new(start, word_count))
    }

    /// Parse one object-local function id.
    pub(super) fn parse_function_id(&mut self) -> ParseResult<FunctionId> {
        let token = self.eat_token(TokenType::Identifier)?;
        let name = self.text(token).to_string();
        let id = self.function_id(name);

        Ok(id)
    }

    /// Intern one object-local function name.
    fn function_id(&mut self, name: String) -> FunctionId {
        if let Some(id) = self.functions.get(&name).copied() {
            return id;
        }

        let id = FunctionId(self.function_names.len() as u32);
        self.function_names.push(name.clone());
        self.declarations.push(None);
        self.functions.insert(name, id);

        id
    }

    /// Parse one function value operation.
    pub(super) fn parse_function_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let opcode = match name {
            "function.address" => Opcode::FUNCTION_ADDRESS,
            "function.bind" => Opcode::FUNCTION_BIND,
            _ => return Err(ParseError::new("unknown function operation", token.span)),
        };
        let results = self.parse_definitions(opcode)?;
        let target = self.parse_function_id()?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.relocation(RelocationTag::FUNCTION, target.0);

        if opcode == Opcode::FUNCTION_BIND {
            self.eat_token(TokenType::Comma)?;
            instruction.register(self.parse_register()?);
        }

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one numeric identity with an exact prefix.
    fn numbered(text: &str, prefix: char) -> Option<u32> {
        let index = text.strip_prefix(prefix)?;

        index.parse().ok()
    }
}
