use destack_core::{EntryRange, Optional, fnv1a_64};
use destack_source::Span;

use crate::{
    Body, FrameSlot, Function, FunctionBuilder, InstructionBuilder, Label, Linkage, Opcode,
    ParseError, ParseResult, Parser, RegisterId, RegisterRange, Symbol, TensorOperand, Token,
    TokenType, ValueType,
};

/// Parser state for one bytecode function definition.
#[derive(Debug)]
pub(super) struct FunctionParser {
    /// Physical bytecode function builder.
    pub(super) builder: FunctionBuilder,
    /// Physical frame slots in declaration order.
    frame_slots: Vec<FrameSlot>,
    /// Logical value types partitioning the physical register file.
    pub(super) registers: Vec<Option<ValueType>>,
    /// Physical function result types in return order.
    pub(super) results: Vec<ValueType>,
    /// Physical parameters delivered when this function resumes.
    pub(super) resume_parameters: Vec<ValueType>,
    /// The hidden callable environment type when present.
    pub(super) environment: Option<ValueType>,
}

impl FunctionParser {
    /// Create parser state for one empty function.
    pub(super) fn new() -> Self {
        Self {
            builder: FunctionBuilder::new(),
            frame_slots: Vec::new(),
            registers: Vec::new(),
            results: Vec::new(),
            resume_parameters: Vec::new(),
            environment: None,
        }
    }

    /// Return whether one dense frame slot has been declared.
    pub(super) fn contains_frame_slot(&self, slot: u32) -> bool {
        (slot as usize) < self.frame_slots.len()
    }

    /// Return one tensor operand from the current register type.
    pub(super) fn tensor(&self, register: RegisterId) -> Option<TensorOperand> {
        let ty = self.value_type(register)?;
        let tensor_type = ty.tensor_type()?;
        let registers = RegisterRange::new(register, ty.word_count());

        Some(TensorOperand::new(registers, tensor_type))
    }

    /// Return one owning tensor operand from the current register type.
    pub(super) fn owning_tensor(&self, register: RegisterId) -> Option<TensorOperand> {
        if !self.value_type(register)?.is_tensor() {
            return None;
        }

        self.tensor(register)
    }

    /// Return one tensor view operand from the current register type.
    pub(super) fn tensor_view(&self, register: RegisterId) -> Option<TensorOperand> {
        if !self.value_type(register)?.is_tensor_view() {
            return None;
        }

        self.tensor(register)
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
        self.assign_values(results, result_types, span)?;

        self.builder.begin_operation();
        self.builder
            .emit(instruction, &definitions)
            .map_err(|error| ParseError::new(error.to_string(), span))?;

        Ok(())
    }

    /// Assign one logical value type to contiguous registers.
    pub(super) fn assign(&mut self, id: RegisterId, ty: ValueType, span: Span) -> ParseResult<()> {
        let end = u32::from(id.0) + u32::from(ty.word_count());
        if end > u32::from(u16::MAX) {
            return Err(ParseError::new(
                "register value exceeds the register file",
                span,
            ));
        }
        let start = id.index();
        let end = end as usize;
        if self.registers.len() < end {
            self.registers.resize(end, None);
        }

        // preserve the first type assigned to each physical register
        if let Some(current) = self.registers[start] {
            if current == ty {
                return Ok(());
            }

            return Err(ParseError::new(
                "register type differs from its first assignment",
                span,
            ));
        }

        // reject starts inside an existing multiword value
        let overlaps_previous = self.registers[..start]
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, ty)| ty.map(|ty| index + ty.word_count() as usize))
            .is_some_and(|previous_end| previous_end > start);
        if overlaps_previous {
            return Err(ParseError::new(
                "register overlaps another register value",
                span,
            ));
        }

        // reject values spanning another logical register start
        if self.registers[start + 1..end].iter().any(Option::is_some) {
            return Err(ParseError::new(
                "register overlaps another register value",
                span,
            ));
        }

        // establish the logical value and reserve its physical words
        self.registers[start] = Some(ty);
        let range = RegisterRange::new(id, ty.word_count());
        self.builder
            .reserve(range)
            .map_err(|error| ParseError::new(error.to_string(), span))?;

        Ok(())
    }

    /// Assign logical types to their starting registers.
    pub(super) fn assign_values(
        &mut self,
        registers: &[RegisterId],
        types: &[ValueType],
        span: Span,
    ) -> ParseResult<()> {
        if registers.len() != types.len() {
            return Err(ParseError::new(
                "instruction result count does not match its types",
                span,
            ));
        }

        for (register, ty) in registers.iter().zip(types) {
            self.assign(*register, *ty, span)?;
        }

        Ok(())
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
        self.registers.get(register.index()).copied().flatten()
    }

    /// Return whether one register begins a value of the requested type.
    pub(super) fn has_type(&self, register: RegisterId, ty: ValueType) -> bool {
        self.value_type(register) == Some(ty)
    }

    /// Return whether one physical register belongs to an initialized value.
    pub(super) fn contains_word(&self, register: RegisterId) -> bool {
        self.registers
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

    /// Return the logical value types partitioning the physical register file.
    pub(super) fn register_types(&self, span: Span) -> ParseResult<Vec<ValueType>> {
        let mut register = 0;
        let mut types = Vec::new();

        // advance over each complete logical value
        while register < self.registers.len() {
            let Some(ty) = self.registers[register] else {
                return Err(ParseError::new(
                    "register file contains an untyped word",
                    span,
                ));
            };
            types.push(ty);
            register += ty.word_count() as usize;
        }

        Ok(types)
    }

    /// Define one branch label at the current code offset.
    pub(super) fn define_label(&mut self, label: Label, span: Span) -> ParseResult<()> {
        self.builder
            .define(label)
            .map_err(|error| ParseError::new(error.to_string(), span))
    }
}

impl Parser<'_> {
    /// Collect one function header before any function body is parsed.
    pub(super) fn collect_function_header(&mut self, linkage: Linkage) -> ParseResult<()> {
        let name = self.eat_token(TokenType::Identifier)?;
        let text = self.text(name).to_string();
        let id = self.function_symbol(&text, name.span)?;
        let mut function = FunctionParser::new();
        let (parameters, results, resume_parameters) =
            self.parse_function_header(linkage, name.span, &mut function)?;
        let declaration = &mut self.symbols.function_declarations[id.index()];
        declaration.parameters = parameters;
        declaration.results = results;
        declaration.environment = function.environment;
        declaration.resume_parameters = resume_parameters;

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
        let (parameters, results, resume_parameters) =
            self.parse_function_header(linkage, name.span, &mut function)?;
        let declaration = &self.symbols.function_declarations[id.index()];
        if parameters != declaration.parameters || results != declaration.results {
            return Err(ParseError::new(
                "function type changed after indexing",
                name.span,
            ));
        }
        if resume_parameters != declaration.resume_parameters {
            return Err(ParseError::new(
                "resume parameters changed after indexing",
                name.span,
            ));
        }
        if function.environment != declaration.environment {
            return Err(ParseError::new(
                "function environment changed after indexing",
                name.span,
            ));
        }
        function.results.clone_from(&results);
        function.resume_parameters.clone_from(&resume_parameters);

        // persist the physical callable ABI
        let parameters = self.object.push_value_types(parameters);
        let results = self.object.push_value_types(results);
        let resume_parameters = self.object.push_value_types(resume_parameters);
        if linkage == Linkage::EXTERNAL {
            let function = Function::new(
                self.object.intern_string(&text),
                Body::new(
                    parameters,
                    results,
                    resume_parameters,
                    Optional::none(),
                    function.builder.register_count(),
                    EntryRange::empty(),
                    EntryRange::empty(),
                    EntryRange::empty(),
                    Optional::none(),
                    0,
                    0,
                ),
                0,
                linkage,
            );
            self.object.push_function(function);

            return Ok(());
        }

        self.eat_token(TokenType::OpenBrace)?;

        // parse the complete logical frame first
        while self.peek_name("slot") {
            self.parse_frame_slot(&mut function)?;
        }

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

        // require every register-backed slot to name a complete value range
        for slot in &function.frame_slots {
            if slot
                .registers()
                .is_some_and(|registers| !function.contains_range(registers))
            {
                return Err(ParseError::new(
                    "frame slot references an invalid register range",
                    self.empty_span(),
                ));
            }
        }

        // finish the fixed register partition and physical function body
        let environment = function.environment;
        let register_types = function.register_types(self.empty_span())?;
        let body = function
            .builder
            .build()
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        // append the body to the object code section
        let code_hash = fnv1a_64(&body.code);
        let code = self.object.push_code(
            &body.code,
            body.relocations.iter().copied(),
            body.dynamic_relocations.iter().copied(),
        );

        // retain function-relative logical operation offsets
        let operation_offsets = self
            .object
            .push_operation_offsets(body.operation_offsets.iter().copied());
        let register_types = self.object.push_value_types(register_types);

        // append the resolved frame rows in function order
        let slot_start = self.object.frame_slot_count() as u32;
        for slot in function.frame_slots.iter().copied() {
            self.object.push_frame_slot(slot);
        }
        let slot_len = function.frame_slots.len() as u32;

        // publish the complete function row
        let function = Function::new(
            self.object.intern_string(&text),
            Body::new(
                parameters,
                results,
                resume_parameters,
                Optional::from(environment),
                body.register_count,
                register_types,
                EntryRange::new(slot_start, slot_len),
                operation_offsets,
                Optional::some(code),
                body.counter_count,
                body.sampler_count,
            ),
            code_hash,
            linkage,
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
    ) -> ParseResult<(Vec<ValueType>, Vec<ValueType>, Vec<ValueType>)> {
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
        Ok((parameters, results, resume))
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
                    || !ty.is_initialized_reference())
            {
                return Err(ParseError::new(
                    "environment must be one leading reference word",
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
            function.assign(register, ty, self.previous().span)?;
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
    fn parse_frame_slot(&mut self, function: &mut FunctionParser) -> ParseResult<()> {
        self.eat_name("slot")?;
        let slot = self.eat_token(TokenType::Identifier)?;
        let slot_index = function.frame_slots.len();
        let expected = format!("s{slot_index}");
        if self.text(slot) != expected {
            return Err(ParseError::new("frame slots must be dense", slot.span));
        }
        self.eat_token(TokenType::Colon)?;
        let ty = self.parse_storage_type()?;
        let slot = if self.eat_token_if(TokenType::Equal) {
            let register = self.parse_register()?;
            self.eat_token(TokenType::OpenBracket)?;
            let word_count = self.parse_u16()?;
            self.eat_token(TokenType::CloseBracket)?;
            if word_count == 0 {
                return Err(ParseError::new(
                    "frame register range must not be empty",
                    slot.span,
                ));
            }

            FrameSlot::from_registers(ty, RegisterRange::new(register, word_count))
        } else {
            FrameSlot::new(ty)
        };
        function.frame_slots.push(slot);

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
        match name {
            "function.address" => self.parse_function_address(results, function),
            "function.bind" => self.parse_function_bind(token, results, result_types, function),
            "function.environment" => {
                self.parse_function_environment(token, results, result_types, function)
            }
            "function.environment.current" => {
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
        if self.symbols.function_declarations[target.index()]
            .environment
            .is_some()
        {
            return Err(ParseError::new(
                "function.address requires an environment-free function",
                name.span,
            ));
        }
        let mut instruction = InstructionBuilder::new(Opcode::FUNCTION_ADDRESS);
        instruction.symbol(Symbol::function(target.0));

        function.emit(
            instruction,
            results,
            &[ValueType::function_pointer()],
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
        let name = self.eat_token(TokenType::Identifier)?;
        let target = self.function_symbol(self.text(name), name.span)?;
        self.eat_token(TokenType::Comma)?;
        let environment = self.parse_register()?;
        let environment_type = function.value_type(environment).ok_or_else(|| {
            ParseError::new(
                "function.bind reads an uninitialized environment",
                token.span,
            )
        })?;
        if !environment_type.is_initialized_reference() {
            return Err(ParseError::new(
                "function environment must be one initialized reference",
                token.span,
            ));
        }
        let target_environment = self.symbols.function_declarations[target.index()].environment;
        if target_environment != Some(environment_type) {
            return Err(ParseError::new(
                "function environment does not match its target",
                token.span,
            ));
        }

        // encode the linked code pointer and captured environment
        let mut instruction = InstructionBuilder::new(Opcode::FUNCTION_BIND);
        instruction.symbol(Symbol::function(target.0));
        instruction.register(environment);

        function.emit(instruction, results, &[result_type], self.empty_span())
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
            .filter(|ty| ty.is_initialized_reference())
            .ok_or_else(|| {
                ParseError::new(
                    "function.environment requires one reference result",
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
