use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceKind, RegisterId,
    RegisterRange, Space, Symbol, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one dynamic value operation.
    pub(super) fn parse_dynamic_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        match name {
            "dynamic.bind" => self.parse_dynamic_bind(token, results, result_types, function),
            "dynamic.payload" => self.parse_dynamic_payload(token, results, result_types, function),
            "dynamic.type" => self.parse_dynamic_type(token, results, function),
            _ => Err(ParseError::new("unknown dynamic operation", token.span)),
        }
    }

    /// Parse one dynamic value construction.
    fn parse_dynamic_bind(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // match the declared dynamic result
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| ty.is_dynamic())
            .ok_or_else(|| ParseError::new("dynamic.bind requires a dynamic result", token.span))?;

        // match the local managed payload
        let payload = self.parse_register()?;
        let payload_type = function.value_type(payload).ok_or_else(|| {
            ParseError::new("dynamic.bind reads an uninitialized payload", token.span)
        })?;
        let is_local_managed = payload_type.reference_type().is_some_and(|reference| {
            reference.kind() == ReferenceKind::MANAGED && reference.space() == Space::LOCAL
        });
        if !is_local_managed {
            return Err(ParseError::new(
                "dynamic.bind requires a local managed reference",
                token.span,
            ));
        }

        // resolve concrete and constraint runtime types
        self.eat_token(TokenType::Colon)?;
        let concrete = self.parse_type_name()?;
        let constraint = result_type
            .constraint()
            .ok_or_else(|| ParseError::new("dynamic result has no constraint", token.span))?;

        // encode the erased payload and both linked runtime types
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_BIND);
        instruction.register(payload);
        instruction.symbol(Symbol::ty(concrete.0));
        instruction.symbol(Symbol::ty(constraint.0));

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one erased dynamic payload access.
    fn parse_dynamic_payload(
        &mut self,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // match the single word result
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| ty.word_count() == 1)
            .ok_or_else(|| {
                ParseError::new("dynamic.payload requires a one-word result", token.span)
            })?;

        // match the complete dynamic input
        let dynamic = self.parse_register()?;
        if !function
            .value_type(dynamic)
            .is_some_and(ValueType::is_dynamic)
        {
            return Err(ParseError::new(
                "dynamic.payload requires a dynamic value",
                token.span,
            ));
        }

        // encode the erased payload projection
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_PAYLOAD);
        instruction.range(RegisterRange::new(dynamic, 2));

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one dynamic runtime type access.
    fn parse_dynamic_type(
        &mut self,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // match the complete dynamic input
        let dynamic = self.parse_register()?;
        if !function
            .value_type(dynamic)
            .is_some_and(ValueType::is_dynamic)
        {
            return Err(ParseError::new(
                "dynamic.type requires a dynamic value",
                token.span,
            ));
        }

        // encode the runtime type projection
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_TYPE);
        instruction.range(RegisterRange::new(dynamic, 2));

        function.emit(
            instruction,
            results,
            &[ValueType::type_id()],
            self.empty_span(),
        )
    }
}
