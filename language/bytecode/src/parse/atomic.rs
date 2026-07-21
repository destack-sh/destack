use crate::{
    AtomicAccess, AtomicOperation, AtomicOrder, CompareExchangeAccess, ExecutionScope, FenceAccess,
    Opcode, ParseError, ParseResult, Parser, RegisterId, Scalar, StorageSet, Token, TokenType,
    ValueType,
};

use super::builder::InstructionBuilder;
use super::function::FunctionBuilder;

impl Parser<'_> {
    /// Parse one atomic memory instruction.
    pub(super) fn parse_atomic_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        // fixed operations
        if name == "atomic.fence" {
            return self.parse_atomic_fence(results, function);
        }
        if name == "atomic.wake" || name == "atomic.wakeAll" {
            return self.parse_atomic_wake(name, token, results, function);
        }

        self.parse_atomic_access(name, token, results, result_types, function)
    }

    /// Parse one atomic fence.
    fn parse_atomic_fence(
        &mut self,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
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

        function.emit(instruction, results, &[], self.empty_span())
    }

    /// Parse one bounded or unbounded atomic wake.
    fn parse_atomic_wake(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        let opcode = if name == "atomic.wake" {
            Opcode::ATOMIC_WAKE
        } else {
            Opcode::ATOMIC_WAKE_ALL
        };
        let address = self.parse_register()?;
        if !function.has_type(address, ValueType::address()) {
            return Err(ParseError::new(
                "atomic wake requires a native address",
                token.span,
            ));
        }

        // encode the result and target address
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(address);

        // parse the bounded wake count
        if opcode == Opcode::ATOMIC_WAKE {
            self.eat_token(TokenType::Comma)?;
            let count = self.parse_register()?;
            if !function.has_type(count, ValueType::scalar(Scalar::Uint64)) {
                return Err(ParseError::new(
                    "atomic wake count is not uint64",
                    token.span,
                ));
            }
            instruction.register(count);
        }

        // encode the optional execution scope
        let scope = self.parse_optional_scope()?;
        instruction.u16(scope as u16);

        function.emit(
            instruction,
            results,
            &[ValueType::scalar(Scalar::Uint64)],
            self.empty_span(),
        )
    }

    /// Parse one typed atomic memory access.
    fn parse_atomic_access(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionBuilder,
    ) -> ParseResult<()> {
        let (operation, scalar) = self.parse_atomic_name(name, token)?;

        // parse the address and optional scalar value
        let address = self.parse_register()?;
        if !function.has_type(address, ValueType::address()) {
            return Err(ParseError::new(
                "atomic operation requires a native address",
                token.span,
            ));
        }
        let scalar_type = ValueType::scalar(scalar);
        let value = self.parse_atomic_value(operation, scalar_type, token, function)?;
        let expected_results = if operation == AtomicOperation::Store {
            Vec::new()
        } else if operation.is_compare_exchange() {
            vec![scalar_type, ValueType::scalar(Scalar::Boolean)]
        } else {
            vec![operation.result_type(scalar)]
        };
        if result_types != expected_results {
            return Err(ParseError::new(
                "atomic results do not match its operation",
                token.span,
            ));
        }

        // encode regular operands
        let opcode = Opcode::atomic(operation, scalar)
            .ok_or_else(|| ParseError::new("invalid atomic operation", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        instruction.register(address);
        if let Some(value) = value {
            instruction.register(value);
        }

        // encode operation specific operands
        if let Some(replacement) =
            self.parse_atomic_replacement(operation, scalar_type, token, function)?
        {
            instruction.register(replacement);
        }
        if let Some(timeout) = self.parse_atomic_timeout(operation, token, function)? {
            instruction.register(timeout);
        }

        // encode operation specific memory access
        self.eat_token(TokenType::Comma)?;
        let access = self.parse_atomic_access_bits(operation, token)?;
        instruction.u16(access);

        function.emit(instruction, results, &expected_results, self.empty_span())
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
        scalar_type: ValueType,
        token: Token,
        function: &FunctionBuilder,
    ) -> ParseResult<Option<RegisterId>> {
        if operation == AtomicOperation::Load {
            return Ok(None);
        }

        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;
        if !function.has_type(value, scalar_type) {
            return Err(ParseError::new(
                "atomic value does not match its scalar type",
                token.span,
            ));
        }

        Ok(Some(value))
    }

    /// Parse the replacement required by compare exchange operations.
    fn parse_atomic_replacement(
        &mut self,
        operation: AtomicOperation,
        scalar_type: ValueType,
        token: Token,
        function: &FunctionBuilder,
    ) -> ParseResult<Option<RegisterId>> {
        if !operation.is_compare_exchange() {
            return Ok(None);
        }

        self.eat_token(TokenType::Comma)?;
        let replacement = self.parse_register()?;
        if !function.has_type(replacement, scalar_type) {
            return Err(ParseError::new(
                "atomic replacement does not match its scalar type",
                token.span,
            ));
        }

        Ok(Some(replacement))
    }

    /// Parse the timeout required by one timed wait.
    fn parse_atomic_timeout(
        &mut self,
        operation: AtomicOperation,
        token: Token,
        function: &FunctionBuilder,
    ) -> ParseResult<Option<RegisterId>> {
        if operation != AtomicOperation::WaitTimed {
            return Ok(None);
        }

        self.eat_token(TokenType::Comma)?;
        let timeout = self.parse_register()?;
        if !function.has_type(timeout, ValueType::scalar(Scalar::Uint64)) {
            return Err(ParseError::new(
                "atomic wait timeout is not uint64",
                token.span,
            ));
        }

        Ok(Some(timeout))
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
        if !self.eat_token_if(TokenType::Comma) {
            return Ok(ExecutionScope::System);
        }
        self.eat_name("scope")?;
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
                "static" => StorageSet::STATIC,
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
