use crate::{
    Function, FunctionBuilder, FunctionId, InstructionBuilder, Label, Opcode, ParseError,
    ParseResult, Parser, RegisterId, RegisterSpan, RelocationTag, Token, TokenType,
};
use tspp_core::Optional;
use tspp_source::Span;

/// Parser state for one physical bytecode function.
#[derive(Debug)]
pub(super) struct FunctionParser {
    /// Encoded function body under construction.
    pub(super) builder: FunctionBuilder,
}

impl FunctionParser {
    /// Create parser state for one function.
    pub(super) fn new() -> Self {
        Self {
            builder: FunctionBuilder::new(),
        }
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
    /// Parse one physical bytecode function declaration or definition.
    pub(super) fn parse_function(&mut self) -> ParseResult<()> {
        let is_external = self.eat_name_if("external");
        self.eat_name("function")?;
        let token = self.eat_token(TokenType::Identifier)?;
        let name = self.text(token).to_string();
        let function_id = self.function_id(name);

        // external declarations have no physical body
        if is_external {
            return Ok(());
        }

        self.eat_token(TokenType::OpenBrace)?;
        if self.functions[function_id.index()].code().is_some() {
            return Err(ParseError::new(
                "function is already defined",
                self.previous().span,
            ));
        }

        // parse the physical body and derive its register width
        let mut function = FunctionParser::new();
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

        // append function-owned sections and publish the physical entry
        let operations = self.object.push_operations(body.operations);
        let code = self.object.push_code(&body.code, body.relocations);
        let function = Function::new(Optional::some(code), operations, body.register_count);
        self.functions[function_id.index()] = function;

        Ok(())
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
        let position = self.cursor.position();
        let has_end = self.eat_token_if(TokenType::Colon) && self.is_register();
        let word_count = if has_end {
            let end = self.parse_register()?;
            if end.0 < start.0 {
                return Err(ParseError::new(
                    "register span ends before it starts",
                    self.previous().span,
                ));
            }

            end.0 - start.0 + 1
        } else {
            self.cursor.seek(position);

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
        if let Some(id) = self.function_ids.get(&name).copied() {
            return id;
        }

        let id = FunctionId(self.function_names.len() as u32);
        self.function_names.push(name.clone());
        self.functions.push(Function::declaration());
        self.function_ids.insert(name, id);

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
