use crate::{
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, ReferenceKind, RegisterId,
    RegisterRange, Token, TokenType, ValueType,
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

        // match a managed payload in the dynamic value's space
        let payload = self.parse_register()?;
        let payload_type = function.value_type(payload).ok_or_else(|| {
            ParseError::new("dynamic.bind reads an uninitialized payload", token.span)
        })?;
        let dynamic_reference = result_type.dynamic_reference().ok_or_else(|| {
            ParseError::new("dynamic result has no payload reference", token.span)
        })?;
        let payload_reference = payload_type.reference_type();
        if dynamic_reference.kind() != ReferenceKind::MANAGED
            || payload_reference != Some(dynamic_reference)
        {
            return Err(ParseError::new(
                "dynamic.bind requires a managed reference in the dynamic value's space",
                token.span,
            ));
        }

        // resolve the concrete implementation selected by this binding
        self.eat_token(TokenType::Colon)?;
        let concrete = self.parse_type_name()?;
        let constraint = result_type
            .constraint()
            .ok_or_else(|| ParseError::new("dynamic result has no constraint", token.span))?;
        // encode the erased payload and linked dispatch table
        let mut instruction = InstructionBuilder::new(Opcode::DYNAMIC_BIND);
        instruction.register(payload);
        instruction.dynamic_table(concrete, constraint);

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
        // match the complete dynamic input
        let dynamic = self.parse_register()?;
        let dynamic_type = function.value_type(dynamic).filter(|ty| ty.is_dynamic());
        let dynamic_type = dynamic_type.ok_or_else(|| {
            ParseError::new("dynamic.payload requires a dynamic value", token.span)
        })?;

        // match the erased managed payload result
        let reference = dynamic_type
            .dynamic_reference()
            .ok_or_else(|| ParseError::new("dynamic value has no payload reference", token.span))?;
        let result_type = result_types
            .first()
            .copied()
            .filter(|ty| ty.reference_type() == Some(reference))
            .ok_or_else(|| {
                ParseError::new(
                    "dynamic.payload result must match its managed payload reference",
                    token.span,
                )
            })?;

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
