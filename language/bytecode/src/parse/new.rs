use crate::{
    Initialization, InstructionBuilder, New, NewKind, Opcode, ParseError, ParseResult, Parser,
    RelocationTag, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one `new` instruction.
    pub(super) fn parse_new(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let (kind, initialization, is_fallible) = self.parse_new_name(name, token)?;
        let results = self.parse_results(1, true)?;

        // encode the direct allocation site identity
        let allocation = self.parse_allocation_id()?;

        // encode the variable slice length
        let length = if kind == NewKind::Slice {
            self.eat_token(TokenType::Comma)?;
            Some(self.parse_register()?)
        } else {
            None
        };

        // encode the operation selected by its canonical name
        let operation = New {
            kind,
            initialization,
            is_fallible,
        };
        let opcode = Opcode::new(operation);
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.relocation(RelocationTag::ALLOCATION, allocation);
        if let Some(length) = length {
            instruction.register(length);
        }

        // encode explicit success and failure edges
        if operation.is_fallible {
            self.eat_token(TokenType::FatArrow)?;
            instruction.branch(self.parse_label()?);
            self.eat_token(TokenType::Comma)?;
            instruction.branch(self.parse_label()?);
        }

        function.emit(instruction, &results, token.span)
    }

    /// Parse one allocation operation name.
    fn parse_new_name(
        &self,
        name: &str,
        token: Token,
    ) -> ParseResult<(NewKind, Initialization, bool)> {
        let mut components = name.split('.');
        if components.next() != Some("new") {
            return Err(ParseError::new("invalid new operation", token.span));
        }

        // parse value or slice initialization
        let component = components
            .next()
            .ok_or_else(|| ParseError::new("new operation has no initialization", token.span))?;
        let (kind, initialization) = if component == "slice" {
            let initialization = components
                .next()
                .and_then(Initialization::from_name)
                .ok_or_else(|| ParseError::new("invalid slice initialization", token.span))?;

            (NewKind::Slice, initialization)
        } else {
            let initialization = Initialization::from_name(component)
                .ok_or_else(|| ParseError::new("invalid value initialization", token.span))?;

            (NewKind::Value, initialization)
        };

        // parse optional fallibility and reject trailing components
        let is_fallible = match components.next() {
            Some("try") => true,
            None => false,
            Some(_) => return Err(ParseError::new("invalid new operation", token.span)),
        };
        if components.next().is_some() {
            return Err(ParseError::new("invalid new operation", token.span));
        }

        Ok((kind, initialization, is_fallible))
    }
}
