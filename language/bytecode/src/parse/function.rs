use destack_core::{EntryRange, Optional, fnv1a_64};
use destack_source::Span;

use super::symbol::FunctionTypeDefinition;
use crate::{
    FrameSlot, Function, FunctionBuilder, FunctionTypeId, InstructionBuilder, Label, Linkage,
    Opcode, ParseError, ParseResult, Parser, RegisterId, RegisterRange, Symbol, Token, TokenType,
    ValueType,
};

/// Parser state for one bytecode function definition.
#[derive(Debug)]
pub(super) struct FunctionParser {
    /// Physical bytecode function builder.
    pub(super) builder: FunctionBuilder,
    /// The frame slots declared before the function body.
    pub(super) frame_slot_count: u32,
    /// Register types established by parameters and instructions.
    pub(super) register_types: Vec<Option<ValueType>>,
    /// Logical function result types in return order.
    pub(super) results: Vec<ValueType>,
    /// Logical parameters delivered when this function resumes.
    pub(super) resume_parameters: Vec<ValueType>,
    /// The hidden callable environment type when present.
    pub(super) environment: Option<ValueType>,
}

impl FunctionParser {
    /// Create parser state for one empty function.
    pub(super) fn new() -> Self {
        Self {
            builder: FunctionBuilder::new(),
            frame_slot_count: 0,
            register_types: Vec::new(),
            results: Vec::new(),
            resume_parameters: Vec::new(),
            environment: None,
        }
    }

    /// Encode one complete instruction into this function.
    pub(super) fn emit(
        &mut self,
        instruction: InstructionBuilder,
        results: &[RegisterId],
        result_types: &[ValueType],
        span: Span,
    ) -> ParseResult<()> {
        if results.len() != result_types.len() {
            return Err(ParseError::new(
                "instruction result count does not match its operation",
                span,
            ));
        }

        // encode destination operands from the opcode layout
        let definitions = results
            .iter()
            .zip(result_types)
            .map(|(register, ty)| RegisterRange::new(*register, ty.word_count()))
            .collect::<Vec<_>>();

        // require initialized register operands
        for register in &instruction.registers {
            if !self.contains_word(*register) {
                return Err(ParseError::new(
                    "instruction reads an uninitialized register",
                    span,
                ));
            }
        }

        // require initialized register ranges
        for range in &instruction.ranges {
            if !self.contains_range(*range) {
                return Err(ParseError::new(
                    "instruction reads an invalid register range",
                    span,
                ));
            }
        }

        // establish each result before physical emission
        if !self.assign_values(results, result_types) {
            return Err(ParseError::new(
                "instruction result overlaps another register value",
                span,
            ));
        }

        self.builder.begin_operation();
        self.builder
            .emit(instruction, &definitions)
            .map_err(|error| ParseError::new(error.to_string(), span))?;

        Ok(())
    }

    /// Assign one logical value type to contiguous registers.
    pub(super) fn assign(&mut self, register: RegisterId, ty: ValueType) -> bool {
        let end = u32::from(register.0) + u32::from(ty.word_count());
        if end > u32::from(u16::MAX) {
            return false;
        }
        let end = end as u16;
        if !Self::assign_type(&mut self.register_types, register, ty) {
            return false;
        }
        let range = RegisterRange::new(register, end - register.0);
        self.builder.reserve(range);

        true
    }

    /// Assign logical types to their starting registers.
    pub(super) fn assign_values(&mut self, registers: &[RegisterId], types: &[ValueType]) -> bool {
        if registers.len() != types.len() {
            return false;
        }

        for (register, ty) in registers.iter().zip(types) {
            if !self.assign(*register, *ty) {
                return false;
            }
        }

        true
    }

    /// Return whether registers contain the requested logical value types.
    pub(super) fn values_match(&self, registers: &[RegisterId], types: &[ValueType]) -> bool {
        if registers.len() != types.len() {
            return false;
        }

        registers
            .iter()
            .zip(types)
            .all(|(register, ty)| self.value_type(*register) == Some(*ty))
    }

    /// Return the logical value type beginning at one register.
    pub(super) fn value_type(&self, register: RegisterId) -> Option<ValueType> {
        self.register_types.get(register.index()).copied().flatten()
    }

    /// Return whether one register begins a value of the requested type.
    pub(super) fn has_type(&self, register: RegisterId, ty: ValueType) -> bool {
        self.value_type(register) == Some(ty)
    }

    /// Return whether one physical register belongs to an initialized value.
    pub(super) fn contains_word(&self, register: RegisterId) -> bool {
        self.register_types
            .iter()
            .enumerate()
            .take(register.index() + 1)
            .rev()
            .find_map(|(start, ty)| ty.map(|ty| start + ty.word_count() as usize))
            .is_some_and(|end| register.index() < end)
    }

    /// Return whether every physical register in one range is initialized.
    pub(super) fn contains_range(&self, range: RegisterRange) -> bool {
        let start = u32::from(range.start.0);
        let end = start + u32::from(range.word_count);
        if end > u32::from(u16::MAX) {
            return false;
        }

        (start..end).all(|register| self.contains_word(RegisterId(register as u16)))
    }

    /// Define one branch label at the current code offset.
    pub(super) fn define_label(&mut self, label: Label, span: Span) -> ParseResult<()> {
        self.builder
            .define(label)
            .map_err(|error| ParseError::new(error.to_string(), span))
    }

    /// Assign one logical type without overlapping another logical value.
    pub(super) fn assign_type(
        types: &mut Vec<Option<ValueType>>,
        register: RegisterId,
        ty: ValueType,
    ) -> bool {
        let start = register.index();
        let old_width = types
            .get(start)
            .copied()
            .flatten()
            .map_or(0, |ty| ty.word_count() as usize);
        let new_width = ty.word_count() as usize;
        let end = start + old_width.max(new_width);
        if types.len() < end {
            types.resize(end, None);
        }

        // reject starts inside an existing wide logical value
        let overlaps_previous = types[..start]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, ty)| ty.map(|ty| index + ty.word_count() as usize))
            .is_some_and(|end| end > start);
        if overlaps_previous {
            return false;
        }

        // reject new width spanning another logical value start
        if types[start + 1..start + new_width]
            .iter()
            .any(Option::is_some)
        {
            return false;
        }

        // replace the complete prior logical value
        types[start..end].fill(None);
        types[start] = Some(ty);

        true
    }
}

impl Parser<'_> {
    /// Collect one function type before any function body is parsed.
    pub(super) fn collect_function_type(&mut self, linkage: Linkage) -> ParseResult<()> {
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();
        let id = self.function_symbol(&text, name.span)?;
        let mut function = FunctionParser::new();
        let (definition, resume_parameters) =
            self.parse_function_header(linkage, name.span, &mut function)?;
        self.symbols.function_declarations[id.index()].resume_parameters = resume_parameters;

        // reuse one canonical id for every identical function type
        if let Some(index) = self
            .symbols
            .function_type_definitions
            .iter()
            .position(|existing| *existing == definition)
        {
            self.symbols.function_declarations[id.index()].function_type =
                FunctionTypeId(index as u32);

            return Ok(());
        }

        // append one new anonymous function type
        let function_type = FunctionTypeId(self.symbols.function_type_definitions.len() as u32);
        self.symbols.function_declarations[id.index()].function_type = function_type;
        self.insert_function_type(function_type, definition.clone(), name.span)?;
        let appended = self.object.push_function_type(
            Optional::none(),
            definition.parameters,
            definition.results,
        );
        if appended != function_type {
            return Err(ParseError::new("function types are not dense", name.span));
        }

        Ok(())
    }

    /// Parse one bytecode function declaration or definition.
    pub(super) fn parse_function(&mut self, linkage: Linkage) -> ParseResult<()> {
        self.eat_name("function")?;
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();
        let id = self.function_symbol(&text, name.span)?;
        if id.index() != self.object.function_count() {
            return Err(ParseError::new("duplicate function", name.span));
        }

        // parse the indexed function header
        let mut function = FunctionParser::new();
        let (parsed, resume_parameters) =
            self.parse_function_header(linkage, name.span, &mut function)?;
        if resume_parameters != self.symbols.function_declarations[id.index()].resume_parameters {
            return Err(ParseError::new(
                "resume parameters changed after indexing",
                name.span,
            ));
        }
        let function_type = self.symbols.function_declarations[id.index()].function_type;
        if parsed != self.symbols.function_type_definitions[function_type.index()] {
            return Err(ParseError::new(
                "function type changed after indexing",
                name.span,
            ));
        }
        function.results.clone_from(&parsed.results);
        function.resume_parameters.clone_from(&resume_parameters);
        if linkage == Linkage::EXTERNAL {
            let function = Function::new(
                self.object.intern_string(&text),
                function_type,
                EntryRange::empty(),
                linkage,
                Optional::none(),
                function.builder.register_count(),
                EntryRange::empty(),
                EntryRange::empty(),
                Optional::none(),
                0,
                0,
                0,
            );
            self.object.push_function(function);

            return Ok(());
        }

        self.eat_token(TokenType::OpenBrace)?;
        let slot_start = self.object.frame_slot_count() as u32;
        let resume_parameters = self.object.push_value_types(resume_parameters);

        // parse the complete logical frame first
        while self.peek_name("slot") {
            let slot_index = self.object.frame_slot_count() as u32 - slot_start;
            self.parse_frame_slot(slot_index)?;
        }
        function.frame_slot_count = self.object.frame_slot_count() as u32 - slot_start;

        // parse labels and instructions after frame declarations
        while !self.eat_token_if(TokenType::CloseBrace) {
            if self.peek_name("slot") {
                return Err(ParseError::new(
                    "slot declarations must precede instructions",
                    self.peek().span,
                ));
            } else if self.is_label() {
                let label = self.bump();
                let label_id = self.parse_label_token(label)?;
                self.eat_token(TokenType::Colon)?;
                function.define_label(label_id, label.span)?;
            } else {
                self.parse_instruction(&mut function)?;
            }
        }

        let slot_len = self.object.frame_slot_count() as u32 - slot_start;

        // finish the physical function body
        let environment = function.environment;
        let body = function
            .builder
            .build()
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        // append the body to the object code section
        let code_hash = fnv1a_64(&body.code);
        let code = self
            .object
            .push_code(&body.code, body.relocations.iter().copied());

        // retain function-relative logical operation offsets
        let operation_offsets = self
            .object
            .push_operation_offsets(body.operation_offsets.iter().copied());

        // publish the complete function row
        let function = Function::new(
            self.object.intern_string(&text),
            function_type,
            resume_parameters,
            linkage,
            Optional::from(environment),
            body.register_count,
            EntryRange::new(slot_start, slot_len),
            operation_offsets,
            Optional::some(code),
            body.counter_count,
            body.sampler_count,
            code_hash,
        );
        self.object.push_function(function);

        Ok(())
    }

    /// Parse one function parameter, resume, and result declaration.
    fn parse_function_header(
        &mut self,
        linkage: Linkage,
        span: Span,
        function: &mut FunctionParser,
    ) -> ParseResult<(FunctionTypeDefinition, Vec<ValueType>)> {
        let parameters = if linkage == Linkage::EXTERNAL {
            self.parse_value_types()?
        } else {
            self.parse_parameters(function)?
        };
        let resume = self.parse_resume_parameters()?;
        if linkage == Linkage::EXTERNAL && !resume.is_empty() {
            return Err(ParseError::new("external functions cannot resume", span));
        }

        // parse the result declaration
        self.eat_token(TokenType::Colon)?;
        let results = self.parse_results()?;
        let definition = FunctionTypeDefinition {
            parameters,
            results,
        };

        Ok((definition, resume))
    }

    /// Parse the logical values delivered when this function resumes.
    fn parse_resume_parameters(&mut self) -> ParseResult<Vec<ValueType>> {
        if !self.eat_name_if("resume") {
            return Ok(Vec::new());
        }

        self.parse_value_types()
    }

    /// Parse logical function parameters and assign their physical registers.
    fn parse_parameters(&mut self, function: &mut FunctionParser) -> ParseResult<Vec<ValueType>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut parameters = Vec::new();
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            let is_environment = self.eat_name_if("environment");
            let register = self.parse_register()?;
            self.eat_token(TokenType::Colon)?;
            let ty = self.parse_value_type()?;

            // place the hidden environment in the first physical register
            if is_environment
                && (function.environment.is_some()
                    || !parameters.is_empty()
                    || register != RegisterId(0)
                    || ty.word_count() != 1)
            {
                return Err(ParseError::new(
                    "environment must be one leading register word",
                    self.previous().span,
                ));
            }

            // pack parameters into the function's leading register window
            if register.0 != function.builder.register_count() {
                return Err(ParseError::new(
                    "function parameters must occupy one contiguous register window",
                    self.previous().span,
                ));
            }
            if !function.assign(register, ty) {
                return Err(ParseError::new(
                    "parameter registers do not match its type",
                    self.previous().span,
                ));
            }
            if is_environment {
                function.environment = Some(ty);
            } else {
                parameters.push(ty);
            }

            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }

        Ok(parameters)
    }

    /// Parse logical function results.
    fn parse_results(&mut self) -> ParseResult<Vec<ValueType>> {
        if self.eat_name_if("void") {
            return Ok(Vec::new());
        }
        if self.peek_is(TokenType::OpenParenthesis) {
            return self.parse_value_types();
        }

        let result = self.parse_value_type()?;

        Ok(vec![result])
    }

    /// Parse one frame slot.
    fn parse_frame_slot(&mut self, slot_index: u32) -> ParseResult<()> {
        self.eat_name("slot")?;
        let slot = self.eat_token(TokenType::Identifier)?;
        let expected = format!("s{slot_index}");
        if self.text(slot) != expected {
            return Err(ParseError::new("frame slots must be dense", slot.span));
        }
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_storage_type()?;
        self.object.push_frame_slot(FrameSlot { ty });

        Ok(())
    }

    /// Return whether the next tokens form an instruction label.
    pub(super) fn is_label(&mut self) -> bool {
        let position = self.cursor.position();
        let token = self.bump();
        let is_label_name = self.text(token).strip_prefix('l').is_some_and(|index| {
            !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
        });
        let is_label =
            token.ty == TokenType::Identifier && is_label_name && self.peek_is(TokenType::Colon);
        self.cursor.seek(position);

        is_label
    }

    /// Parse one comma-separated register group.
    pub(super) fn parse_registers(&mut self) -> ParseResult<Vec<RegisterId>> {
        let mut registers = vec![self.parse_register()?];
        while self.eat_token_if(TokenType::Comma) {
            registers.push(self.parse_register()?);
        }

        Ok(registers)
    }

    /// Parse one register name.
    pub(super) fn parse_register(&mut self) -> ParseResult<RegisterId> {
        let token = self.eat_token(TokenType::Identifier)?;
        let Some(index) = self.text(token).strip_prefix('r') else {
            return Err(ParseError::new("expected register", token.span));
        };
        let index = index
            .parse::<u16>()
            .map_err(|_| ParseError::new("expected register", token.span))?;
        if index == u16::MAX {
            return Err(ParseError::new(
                "register exceeds the bytecode register file",
                token.span,
            ));
        }

        Ok(RegisterId(index))
    }

    /// Parse one function value operation.
    pub(super) fn parse_function_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match Opcode::from_name(name) {
            Some(Opcode::FUNCTION_ADDRESS) => self.parse_function_address(results, function),
            Some(Opcode::FUNCTION_BIND) => {
                self.parse_function_bind(token, results, result_types, function)
            }
            Some(Opcode::FUNCTION_POINTER) => self.parse_function_pointer(token, results, function),
            Some(Opcode::FUNCTION_ENVIRONMENT) => {
                self.parse_function_environment(token, results, result_types, function)
            }
            Some(Opcode::FUNCTION_ENVIRONMENT_CURRENT) => {
                self.parse_current_environment(token, results, result_types, function)
            }
            _ => Err(ParseError::new("unknown function operation", token.span)),
        }
    }

    /// Parse one linked function address.
    fn parse_function_address(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let name = self.eat_token(TokenType::Identifier)?;
        let target = self.function_symbol(self.text(name), name.span)?;
        let function_type = self.symbols.function_declarations[target.index()].function_type;
        let mut instruction = InstructionBuilder::new(Opcode::FUNCTION_ADDRESS);
        instruction.symbol(Symbol::function(target.0));

        function.emit(
            instruction,
            results,
            &[ValueType::function_pointer(function_type)],
            self.empty_span(),
        )
    }

    /// Parse one function and captured environment binding.
    fn parse_function_bind(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| ty.is_function())
            .ok_or_else(|| {
                ParseError::new("function.bind requires a function result", token.span)
            })?;
        let function_type = result_type
            .function_type()
            .ok_or_else(|| ParseError::new("function result has no callable type", token.span))?;
        let name = self.eat_token(TokenType::Identifier)?;
        let target = self.function_symbol(self.text(name), name.span)?;
        let target_type = self.symbols.function_declarations[target.index()].function_type;
        if !self
            .symbols
            .function_types_match(target_type, function_type)
        {
            return Err(ParseError::new(
                "bound function does not match its result type",
                name.span,
            ));
        }
        self.eat_token(TokenType::Comma)?;
        let environment = self.parse_register()?;
        let environment_type = function.value_type(environment).ok_or_else(|| {
            ParseError::new(
                "function.bind reads an uninitialized environment",
                token.span,
            )
        })?;
        if environment_type.word_count() != 1 {
            return Err(ParseError::new(
                "function environment must occupy one register word",
                token.span,
            ));
        }

        // encode the linked code pointer and captured environment
        let mut instruction = InstructionBuilder::new(Opcode::FUNCTION_BIND);
        instruction.symbol(Symbol::function(target.0));
        instruction.register(environment);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one bare pointer projection from a function value.
    fn parse_function_pointer(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let value = self.parse_register()?;
        let value_type = function
            .value_type(value)
            .filter(|ty| ty.is_function())
            .ok_or_else(|| {
                ParseError::new("function.pointer requires a function value", token.span)
            })?;
        let function_type = value_type
            .function_type()
            .ok_or_else(|| ParseError::new("function value has no callable type", token.span))?;
        let mut instruction = InstructionBuilder::new(Opcode::FUNCTION_POINTER);
        instruction.range(RegisterRange::new(value, value_type.word_count()));

        function.emit(
            instruction,
            results,
            &[ValueType::function_pointer(function_type)],
            self.empty_span(),
        )
    }

    /// Parse one captured environment projection from a function value.
    fn parse_function_environment(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| ty.word_count() == 1)
            .ok_or_else(|| {
                ParseError::new(
                    "function.environment requires a one-word result",
                    token.span,
                )
            })?;
        let value = self.parse_register()?;
        let value_type = function
            .value_type(value)
            .filter(|ty| ty.is_function())
            .ok_or_else(|| {
                ParseError::new("function.environment requires a function value", token.span)
            })?;
        let mut instruction = InstructionBuilder::new(Opcode::FUNCTION_ENVIRONMENT);
        instruction.value_type(result_type);
        instruction.range(RegisterRange::new(value, value_type.word_count()));

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one current captured environment access.
    fn parse_current_environment(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| Some(*ty) == function.environment)
            .ok_or_else(|| {
                ParseError::new(
                    "function.environment.current must match the function environment",
                    token.span,
                )
            })?;
        let instruction = InstructionBuilder::new(Opcode::FUNCTION_ENVIRONMENT_CURRENT);

        function.emit(instruction, results, &[result_type], self.empty_span())
    }
}
