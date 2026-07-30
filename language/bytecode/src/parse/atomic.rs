use crate::{
    AtomicAccess, AtomicOperation, AtomicOrder, CompareExchangeAccess, ExecutionScope, FenceAccess,
    InstructionBuilder, Opcode, ParseError, ParseResult, Parser, RegisterId, RegisterSpan, Scalar,
    StorageSet, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one atomic memory instruction.
    pub(super) fn parse_atomic_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // fixed operations
        if name == "atomic.fence" {
            let results = self.parse_definitions(Opcode::ATOMIC_FENCE)?;

            return self.parse_atomic_fence(&results, function);
        }
        let (operation, scalar) = self.parse_atomic_name(name, token)?;
        let opcode = Opcode::atomic(operation, scalar)
            .ok_or_else(|| ParseError::new("invalid atomic operation", token.span))?;
        let results = self.parse_definitions(opcode)?;

        self.parse_atomic_access(operation, scalar, token, &results, function)
    }

    /// Parse one atomic fence.
    fn parse_atomic_fence(
        &mut self,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let order = self.parse_atomic_order()?;
        let scope = self.parse_optional_scope()?;

        // parse the affected storage set
        self.eat_token(TokenType::Comma)?;
        self.eat_name("storage")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let storage = self.parse_storage_set()?;
        if order == AtomicOrder::Relaxed || storage == StorageSet::NONE {
            return Err(ParseError::new(
                "atomic fence requires ordering and storage",
                self.empty_span(),
            ));
        }
        let access = FenceAccess {
            order,
            scope,
            storage,
        };

        // encode the complete fence access
        let mut instruction = InstructionBuilder::new(Opcode::ATOMIC_FENCE);
        instruction.u32(access.bits());

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one typed atomic memory access.
    fn parse_atomic_access(
        &mut self,
        operation: AtomicOperation,
        scalar: Scalar,
        token: Token,
        results: &[RegisterSpan],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        // parse the pointer and optional scalar value
        let pointer = self.parse_register()?;
        let value = self.parse_atomic_value(operation)?;

        // encode regular operands
        let opcode = Opcode::atomic(operation, scalar)
            .ok_or_else(|| ParseError::new("invalid atomic operation", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(pointer);
        if let Some(value) = value {
            instruction.register(value);
        }

        // encode operation specific operands
        if let Some(replacement) = self.parse_atomic_replacement(operation)? {
            instruction.register(replacement);
        }
        // encode operation specific memory access
        self.eat_token(TokenType::Comma)?;
        let access = self.parse_atomic_access_bits(operation, token)?;
        instruction.u16(access);

        function.emit(instruction, results, self.empty_span())
    }

    /// Parse one typed atomic operation name.
    fn parse_atomic_name(
        &self,
        name: &str,
        token: Token,
    ) -> ParseResult<(AtomicOperation, Scalar)> {
        let name = name
            .strip_prefix("atomic.")
            .ok_or_else(|| ParseError::new("expected atomic operation", token.span))?;
        let (operation, scalar) = name
            .rsplit_once('.')
            .ok_or_else(|| ParseError::new("expected atomic scalar type", token.span))?;
        let operation = AtomicOperation::from_name(operation)
            .ok_or_else(|| ParseError::new("expected atomic operation", token.span))?;
        let scalar = Scalar::from_name(scalar)
            .ok_or_else(|| ParseError::new("expected atomic scalar type", token.span))?;

        // reject operation and scalar combinations outside the ISA
        if !operation.supports(scalar) {
            return Err(ParseError::new(
                "atomic operation does not support its scalar type",
                token.span,
            ));
        }

        Ok((operation, scalar))
    }

    /// Parse the scalar input required by one atomic operation.
    fn parse_atomic_value(
        &mut self,
        operation: AtomicOperation,
    ) -> ParseResult<Option<RegisterId>> {
        if operation == AtomicOperation::Load {
            return Ok(None);
        }

        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;

        Ok(Some(value))
    }

    /// Parse the replacement required by compare exchange operations.
    fn parse_atomic_replacement(
        &mut self,
        operation: AtomicOperation,
    ) -> ParseResult<Option<RegisterId>> {
        if !operation.is_compare_exchange() {
            return Ok(None);
        }

        self.eat_token(TokenType::Comma)?;
        let replacement = self.parse_register()?;

        Ok(Some(replacement))
    }

    /// Parse the memory access required by one atomic operation.
    fn parse_atomic_access_bits(
        &mut self,
        operation: AtomicOperation,
        token: Token,
    ) -> ParseResult<u16> {
        let order = self.parse_atomic_order()?;
        if operation.is_compare_exchange() {
            return self.parse_compare_exchange_access(order, token);
        }

        let scope = self.parse_optional_scope()?;
        if !operation.accepts(order) {
            return Err(ParseError::new("invalid atomic memory order", token.span));
        }
        let access = AtomicAccess { order, scope };

        Ok(access.bits())
    }

    /// Parse the success and failure memory access for compare exchange.
    fn parse_compare_exchange_access(
        &mut self,
        success: AtomicOrder,
        token: Token,
    ) -> ParseResult<u16> {
        self.eat_token(TokenType::Comma)?;
        self.eat_name("failure")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let failure = self.parse_atomic_order()?;
        self.eat_token(TokenType::CloseParenthesis)?;
        let scope = self.parse_optional_scope()?;
        if !success.permits_failure(failure) {
            return Err(ParseError::new(
                "invalid compare exchange order",
                token.span,
            ));
        }
        let access = CompareExchangeAccess {
            success,
            failure,
            scope,
        };

        Ok(access.bits())
    }

    /// Parse one atomic memory order.
    fn parse_atomic_order(&mut self) -> ParseResult<AtomicOrder> {
        let token = self.eat_token(TokenType::Identifier)?;

        AtomicOrder::from_name(self.text(token))
            .ok_or_else(|| ParseError::new("expected atomic memory order", token.span))
    }

    /// Parse an optional accelerated execution scope.
    fn parse_optional_scope(&mut self) -> ParseResult<ExecutionScope> {
        let position = self.cursor.position();
        if !self.eat_token_if(TokenType::Comma) {
            return Ok(ExecutionScope::System);
        }
        if !self.eat_name_if("scope") {
            self.cursor.seek(position);

            return Ok(ExecutionScope::System);
        }
        self.eat_token(TokenType::OpenParenthesis)?;
        let token = self.eat_token(TokenType::Identifier)?;
        let scope = ExecutionScope::from_name(self.text(token))
            .ok_or_else(|| ParseError::new("expected execution scope", token.span))?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(scope)
    }

    /// Parse one atomic fence storage set.
    fn parse_storage_set(&mut self) -> ParseResult<StorageSet> {
        let mut storage = StorageSet::NONE;
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            let token = self.eat_token(TokenType::Identifier)?;
            let selected = match self.text(token) {
                "local" => StorageSet::LOCAL,
                "shared" => StorageSet::SHARED,
                "frame" => StorageSet::FRAME,
                "global" => StorageSet::GLOBAL,
                "device" => StorageSet::DEVICE,
                "workgroup" => StorageSet::WORKGROUP,
                _ => return Err(ParseError::new("expected storage region", token.span)),
            };
            storage.0 |= selected.0;
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }

        Ok(storage)
    }
}
