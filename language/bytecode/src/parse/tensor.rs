use crate::{
    ConvertMode, FloatOperation, IndexReduceOperation, InstructionBuilder, IntegerOperation,
    Opcode, ParseError, ParseResult, Parser, ReduceOperation, RegisterId, RegisterRange, Scalar,
    ScatterOperation, Symbol, TensorOperation, TieBreak, Token, TokenType, ValueType,
};

use super::function::FunctionParser;

/// Logical tensor operands collected while one instruction is parsed.
#[derive(Debug, Default)]
struct TensorOperands {
    /// Registers that must contain tensor values.
    tensors: Vec<RegisterId>,
    /// Registers that must contain tensor views.
    views: Vec<RegisterId>,
    /// Registers with one exact required value type.
    typed: Vec<(RegisterId, ValueType)>,
    /// Tensor registers that must share one runtime tensor type.
    same_type: Vec<RegisterId>,
}

impl TensorOperands {
    /// Return whether every collected operand matches its tensor role.
    fn matches(&self, function: &FunctionParser) -> bool {
        let are_tensors_valid = self.tensors.iter().all(|register| {
            function
                .value_type(*register)
                .is_some_and(ValueType::is_tensor)
        });
        let are_views_valid = self.views.iter().all(|register| {
            function
                .value_type(*register)
                .is_some_and(ValueType::is_tensor_view)
        });
        let are_typed_values_valid = self
            .typed
            .iter()
            .all(|(register, ty)| function.has_type(*register, *ty));

        // compare runtime tensor identity across matching operands
        let expected_type = self
            .same_type
            .first()
            .and_then(|register| function.value_type(*register));
        let are_types_equal = self
            .same_type
            .iter()
            .all(|register| function.value_type(*register) == expected_type);

        are_tensors_valid && are_views_valid && are_typed_values_valid && are_types_equal
    }

    /// Return the common tensor value type when one was collected.
    fn common_type(&self, function: &FunctionParser) -> Option<ValueType> {
        self.same_type
            .first()
            .and_then(|register| function.value_type(*register))
    }
}

impl Parser<'_> {
    /// Parse one tensor instruction.
    pub(super) fn parse_tensor_operation(
        &mut self,
        name: &str,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let operation = self.resolve_tensor_operation(name, token)?;
        let mut instruction = InstructionBuilder::new(Opcode::tensor(operation));
        let result_scalar = result_types
            .first()
            .and_then(|ty| ty.tensor_scalar().or_else(|| ty.scalar_type()));
        let mut operands = TensorOperands::default();
        match operation {
            TensorOperation::Element | TensorOperation::Compare => self.parse_tensor_element(
                operation,
                name,
                token,
                result_scalar,
                function,
                &mut instruction,
                &mut operands,
            )?,
            TensorOperation::Select => self.parse_tensor_select(
                token,
                result_scalar,
                function,
                &mut instruction,
                &mut operands,
            )?,
            TensorOperation::Transpose | TensorOperation::Broadcast => {
                self.parse_tensor_reorder(operation, &mut instruction, &mut operands)?
            }
            TensorOperation::Reshape => {
                self.parse_tensor_reshape(&mut instruction, &mut operands)?
            }
            TensorOperation::Slice => self.parse_tensor_slice(&mut instruction, &mut operands)?,
            TensorOperation::Pad => {
                self.parse_tensor_pad(token, result_scalar, &mut instruction, &mut operands)?
            }
            TensorOperation::Concat => {
                self.parse_tensor_concat(token, &mut instruction, &mut operands)?
            }
            TensorOperation::Splat => {
                self.parse_tensor_splat(token, result_scalar, &mut instruction, &mut operands)?
            }
            TensorOperation::Convert => self.parse_tensor_convert(
                token,
                result_scalar,
                function,
                &mut instruction,
                &mut operands,
            )?,
            TensorOperation::Bitcast => {
                self.parse_tensor_bitcast(&mut instruction, &mut operands)?
            }
            TensorOperation::Reduce => {
                self.parse_tensor_reduce(token, result_scalar, &mut instruction, &mut operands)?
            }
            TensorOperation::IndexReduce => {
                self.parse_tensor_index_reduce(token, function, &mut instruction, &mut operands)?
            }
            TensorOperation::Contract => {
                self.parse_tensor_contract(token, result_scalar, &mut instruction, &mut operands)?
            }
            TensorOperation::Gather => self.parse_tensor_gather(&mut instruction, &mut operands)?,
            TensorOperation::Scatter => {
                self.parse_tensor_scatter(token, result_scalar, &mut instruction, &mut operands)?
            }
            TensorOperation::Load | TensorOperation::Extract => self.parse_tensor_load(
                operation,
                token,
                result_scalar,
                function,
                &mut instruction,
                &mut operands,
            )?,
            TensorOperation::Store => {
                self.parse_tensor_store(token, function, &mut instruction, &mut operands)?
            }
            TensorOperation::Fill => {
                self.parse_tensor_fill(token, function, &mut instruction, &mut operands)?
            }
            TensorOperation::Copy => {
                self.parse_tensor_copy(token, function, &mut instruction, &mut operands)?
            }
            TensorOperation::View => self.parse_tensor_view(
                token,
                result_types,
                function,
                &mut instruction,
                &mut operands,
            )?,
            TensorOperation::Convolution => {
                let scalar = result_scalar
                    .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
                let inputs = self.parse_tensor_convolution(&mut instruction, scalar)?;
                operands.tensors.extend(inputs);
            }
        }

        self.emit_tensor(
            operation,
            token,
            results,
            result_types,
            function,
            instruction,
            operands,
        )
    }

    /// Emit one completely parsed tensor instruction.
    fn emit_tensor(
        &self,
        operation: TensorOperation,
        token: Token,
        results: &[RegisterId],
        result_types: &[ValueType],
        function: &mut FunctionParser,
        mut instruction: InstructionBuilder,
        operands: TensorOperands,
    ) -> ParseResult<()> {
        // reject mismatched logical operands
        if !operands.matches(function) {
            return Err(ParseError::new(
                "instruction inputs do not match its tensor representation",
                token.span,
            ));
        }
        let matching_type = operands.common_type(function);

        // attach runtime identity to every tensor-producing operation
        let is_tensor_result = !matches!(
            operation,
            TensorOperation::Load
                | TensorOperation::Extract
                | TensorOperation::Store
                | TensorOperation::Fill
                | TensorOperation::Copy
        );
        let result_type = result_types.first().copied();
        let target = if is_tensor_result {
            let result_type = result_type
                .filter(|ty| ty.is_tensor() || ty.is_tensor_view())
                .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
            let target = result_type
                .tensor_type()
                .ok_or_else(|| ParseError::new("tensor result has no runtime type", token.span))?;
            let scalar = result_type.tensor_scalar().ok_or_else(|| {
                ParseError::new("tensor result has no scalar representation", token.span)
            })?;
            instruction.scalar(scalar);
            instruction.symbol(Symbol::ty(target.0));

            Some(target)
        } else {
            None
        };

        // preserve exact runtime identity across identity-preserving operations
        let preserves_identity = matches!(
            operation,
            TensorOperation::Element | TensorOperation::Select
        );
        if preserves_identity && matching_type.and_then(ValueType::tensor_type) != target {
            return Err(ParseError::new(
                "tensor result does not match its input type",
                token.span,
            ));
        }

        // emit side-effecting operations without a declared result
        let is_value_result = is_tensor_result
            || matches!(operation, TensorOperation::Load | TensorOperation::Extract);
        if !is_value_result {
            return function.emit(instruction, results, &[], self.empty_span());
        }
        let result_type = result_type
            .ok_or_else(|| ParseError::new("expected instruction result", token.span))?;

        function.emit(instruction, results, &[result_type], self.empty_span())
    }

    /// Parse one elementwise tensor operation or comparison.
    fn parse_tensor_element(
        &mut self,
        operation: TensorOperation,
        name: &str,
        token: Token,
        result_scalar: Option<Scalar>,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        if operation == TensorOperation::Compare && result_scalar != Some(Scalar::Boolean) {
            return Err(ParseError::new(
                "tensor comparison requires a boolean tensor result",
                token.span,
            ));
        }

        // read an explicit comparison operator or use the operation name
        let operator_name = if operation == TensorOperation::Compare {
            let operator = self.eat_token(TokenType::Identifier)?;
            let operator = self.text(operator).to_string();
            self.eat_token(TokenType::Comma)?;

            operator
        } else {
            name.to_string()
        };

        // derive the exact operand count from the scalar operation
        let input_count = if operation == TensorOperation::Compare {
            2
        } else {
            let scalar = result_scalar
                .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
            let operator = self.resolve_tensor_operator(&operator_name, scalar, token)?;

            if scalar.is_integer() || scalar == Scalar::Boolean {
                IntegerOperation::from_code(operator as u8)
                    .map(IntegerOperation::input_count)
                    .ok_or_else(|| {
                        ParseError::new("expected integer tensor operation", token.span)
                    })?
            } else {
                FloatOperation::from_code(operator as u8)
                    .map(FloatOperation::input_count)
                    .ok_or_else(|| {
                        ParseError::new("expected floating tensor operation", token.span)
                    })?
            }
        };
        let inputs = self.parse_exact_registers(input_count)?;

        // select the operand representation and encode the scalar operation
        let scalar = if operation == TensorOperation::Compare {
            inputs
                .first()
                .and_then(|register| function.value_type(*register))
                .and_then(ValueType::tensor_scalar)
                .ok_or_else(|| ParseError::new("expected tensor input", token.span))?
        } else {
            result_scalar
                .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?
        };
        let operator = self.resolve_tensor_operator(&operator_name, scalar, token)?;
        instruction
            .registers(&inputs)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        operands.same_type.extend(inputs.iter().copied());
        operands.tensors.extend(inputs);
        instruction.scalar(scalar);
        instruction.u16(operator);

        Ok(())
    }

    /// Parse one elementwise tensor selection.
    fn parse_tensor_select(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let condition = self.parse_register()?;
        let condition_scalar = function
            .value_type(condition)
            .and_then(ValueType::tensor_scalar);
        if condition_scalar != Some(Scalar::Boolean) {
            return Err(ParseError::new(
                "tensor selection requires a boolean tensor mask",
                token.span,
            ));
        }

        // read the mask and two selected tensor values
        instruction.register(condition);
        self.eat_token(TokenType::Comma)?;
        let left = self.parse_register()?;
        instruction.register(left);
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        instruction.register(right);
        operands.tensors.extend([condition, left, right]);
        operands.same_type.extend([left, right]);

        // encode the selected element representation
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(scalar);

        Ok(())
    }

    /// Parse one tensor value reduction.
    fn parse_tensor_reduce(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let reduction = self.eat_token(TokenType::Identifier)?;
        let reduction = ReduceOperation::from_name(self.text(reduction))
            .ok_or_else(|| ParseError::new("expected tensor reduction", reduction.span))?;

        // parse the input and initialized accumulator
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register()?;
        instruction.register(input);
        operands.tensors.push(input);
        self.eat_token(TokenType::Comma)?;
        let initial = self.parse_register()?;
        instruction.register(initial);

        // encode the accumulator representation and reduction axes
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(scalar);
        operands.typed.push((initial, ValueType::scalar(scalar)));
        instruction.u16(reduction as u16);
        self.eat_token(TokenType::Comma)?;

        self.parse_named_u16s("axes", instruction)
    }

    /// Parse one tensor index reduction.
    fn parse_tensor_index_reduce(
        &mut self,
        token: Token,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let reduction = self.eat_token(TokenType::Identifier)?;
        let reduction = IndexReduceOperation::from_name(self.text(reduction))
            .ok_or_else(|| ParseError::new("expected index reduction", reduction.span))?;

        // parse the tensor input and representation
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register()?;
        instruction.register(input);
        operands.tensors.push(input);
        let scalar = function
            .value_type(input)
            .and_then(ValueType::tensor_scalar)
            .ok_or_else(|| ParseError::new("expected tensor input", token.span))?;
        instruction.scalar(scalar);
        instruction.u16(reduction as u16);

        // parse the reduction axis
        self.eat_token(TokenType::Comma)?;
        self.eat_name("axis")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        instruction.u16(self.parse_u16()?);
        self.eat_token(TokenType::CloseParenthesis)?;

        // parse equal value selection
        self.eat_token(TokenType::Comma)?;
        self.eat_name("tieBreak")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let tie = self.eat_token(TokenType::Identifier)?;
        let tie = TieBreak::from_name(self.text(tie))
            .ok_or_else(|| ParseError::new("expected tie break", tie.span))?;
        instruction.u16(tie as u16);

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse one tensor transpose or broadcast.
    fn parse_tensor_reorder(
        &mut self,
        operation: TensorOperation,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);
        let name = if operation == TensorOperation::Transpose {
            "permutation"
        } else {
            "axes"
        };

        self.eat_token(TokenType::Comma)?;
        self.parse_named_u16s(name, instruction)
    }

    /// Parse one tensor reshape.
    fn parse_tensor_reshape(
        &mut self,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);

        self.eat_token(TokenType::Comma)?;
        let shape = self.parse_named_registers("shape", instruction)?;
        operands.typed.extend(
            shape
                .into_iter()
                .map(|register| (register, ValueType::scalar(Scalar::Uint64))),
        );

        Ok(())
    }

    /// Parse one strided tensor slice.
    fn parse_tensor_slice(
        &mut self,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);

        // parse each dynamic slice component
        for name in ["offsets", "sizes", "strides"] {
            self.eat_token(TokenType::Comma)?;
            let registers = self.parse_named_registers(name, instruction)?;
            operands.typed.extend(
                registers
                    .into_iter()
                    .map(|register| (register, ValueType::scalar(Scalar::Uint64))),
            );
        }

        Ok(())
    }

    /// Parse one tensor padding operation.
    fn parse_tensor_pad(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);

        // parse the padding value
        self.eat_token(TokenType::Comma)?;
        self.eat_name("value")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let value = self.parse_register()?;
        instruction.register(value);
        self.eat_token(TokenType::CloseParenthesis)?;
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(scalar);
        operands.typed.push((value, ValueType::scalar(scalar)));

        // parse each padding extent
        for name in ["low", "high", "interior"] {
            self.eat_token(TokenType::Comma)?;
            let registers = self.parse_named_registers(name, instruction)?;
            operands.typed.extend(
                registers
                    .into_iter()
                    .map(|register| (register, ValueType::scalar(Scalar::Uint64))),
            );
        }

        Ok(())
    }

    /// Parse one tensor concatenation.
    fn parse_tensor_concat(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let inputs = self.parse_named_registers("tensors", instruction)?;
        if inputs.is_empty() {
            return Err(ParseError::new(
                "tensor concatenation requires at least one input",
                token.span,
            ));
        }
        operands.tensors.extend(inputs);

        // parse the concatenation axis
        self.eat_token(TokenType::Comma)?;
        self.eat_name("axis")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        instruction.u16(self.parse_u16()?);

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse one tensor splat.
    fn parse_tensor_splat(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let value = self.parse_tensor_input(instruction)?;
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(scalar);
        operands.typed.push((value, ValueType::scalar(scalar)));

        Ok(())
    }

    /// Parse one tensor element conversion.
    fn parse_tensor_convert(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let mode = self.eat_token(TokenType::Identifier)?;
        let mode = ConvertMode::from_name(self.text(mode))
            .ok_or_else(|| ParseError::new("expected tensor conversion mode", mode.span))?;

        // parse the source tensor representation
        self.eat_token(TokenType::Comma)?;
        let input = self.parse_register()?;
        instruction.register(input);
        operands.tensors.push(input);
        let source = function
            .value_type(input)
            .and_then(ValueType::tensor_scalar)
            .ok_or_else(|| ParseError::new("expected tensor input", token.span))?;
        instruction.scalar(source);

        // encode the target representation and conversion behavior
        let target = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(target);
        instruction.u16(mode as u16);

        Ok(())
    }

    /// Parse one tensor bitcast.
    fn parse_tensor_bitcast(
        &mut self,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);

        Ok(())
    }

    /// Parse one tensor contraction.
    fn parse_tensor_contract(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let left = self.parse_tensor_input(instruction)?;
        operands.tensors.push(left);
        self.eat_token(TokenType::Comma)?;
        let right = self.parse_register()?;
        instruction.register(right);
        operands.tensors.push(right);
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(scalar);

        // parse paired contraction dimensions
        self.eat_token(TokenType::Comma)?;
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for name in ["leftBatch", "rightBatch", "leftContract", "rightContract"] {
            self.parse_named_u16s(name, instruction)?;
            self.eat_token_if(TokenType::Comma);
        }

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse one tensor gather.
    fn parse_tensor_gather(
        &mut self,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);
        self.eat_token(TokenType::Comma)?;
        let indices = self.parse_register()?;
        instruction.register(indices);
        operands.tensors.push(indices);

        // parse the gather dimension map
        self.eat_token(TokenType::Comma)?;
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for name in ["outputOffset", "collapsedInput", "indexToInput"] {
            self.parse_named_u16s(name, instruction)?;
            self.eat_token(TokenType::Comma)?;
        }
        self.eat_name("indexVector")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        instruction.u16(self.parse_u16()?);
        self.eat_token(TokenType::CloseParenthesis)?;
        self.eat_token_if(TokenType::Comma);
        self.eat_token(TokenType::CloseParenthesis)?;

        // parse each gathered slice extent
        self.eat_token(TokenType::Comma)?;
        self.parse_named_u64s("sliceSizes", instruction)
    }

    /// Parse one tensor scatter.
    fn parse_tensor_scatter(
        &mut self,
        token: Token,
        result_scalar: Option<Scalar>,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let input = self.parse_tensor_input(instruction)?;
        operands.tensors.push(input);
        self.eat_token(TokenType::Comma)?;
        let indices = self.parse_register()?;
        instruction.register(indices);
        operands.tensors.push(indices);
        self.eat_token(TokenType::Comma)?;
        let updates = self.parse_register()?;
        instruction.register(updates);
        operands.tensors.push(updates);
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected tensor result type", token.span))?;
        instruction.scalar(scalar);

        // parse the scatter dimension map
        self.eat_token(TokenType::Comma)?;
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for name in ["updateWindow", "insertedInput", "indexToInput"] {
            self.parse_named_u16s(name, instruction)?;
            self.eat_token(TokenType::Comma)?;
        }
        self.eat_name("indexVector")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        instruction.u16(self.parse_u16()?);
        self.eat_token(TokenType::CloseParenthesis)?;
        self.eat_token_if(TokenType::Comma);
        self.eat_token(TokenType::CloseParenthesis)?;

        // parse the update behavior
        self.eat_token(TokenType::Comma)?;
        self.eat_name("mode")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let update = self.eat_token(TokenType::Identifier)?;
        let update = ScatterOperation::from_name(self.text(update))
            .ok_or_else(|| ParseError::new("expected scatter mode", update.span))?;
        instruction.u16(update as u16);

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse one tensor scalar load or extract.
    fn parse_tensor_load(
        &mut self,
        operation: TensorOperation,
        token: Token,
        result_scalar: Option<Scalar>,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        // parse a view range or tensor value
        if operation == TensorOperation::Load {
            let view = self.parse_tensor_view_input(token, function, instruction)?;
            operands.views.push(view);
        } else {
            let tensor = self.parse_register()?;
            instruction.register(tensor);
            operands.tensors.push(tensor);
        }

        // parse scalar indices and representation
        self.eat_token(TokenType::Comma)?;
        let indices = self.parse_register_indices(instruction)?;
        operands.typed.extend(
            indices
                .into_iter()
                .map(|register| (register, ValueType::scalar(Scalar::Uint64))),
        );
        let scalar = result_scalar
            .ok_or_else(|| ParseError::new("expected scalar result type", token.span))?;
        instruction.scalar(scalar);

        Ok(())
    }

    /// Parse one tensor scalar store.
    fn parse_tensor_store(
        &mut self,
        token: Token,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let view = self.parse_tensor_view_input(token, function, instruction)?;
        operands.views.push(view);

        // parse scalar indices and stored value
        self.eat_token(TokenType::Comma)?;
        let indices = self.parse_register_indices(instruction)?;
        operands.typed.extend(
            indices
                .into_iter()
                .map(|register| (register, ValueType::scalar(Scalar::Uint64))),
        );
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;
        instruction.register(value);
        let scalar = function
            .value_type(view)
            .and_then(ValueType::tensor_scalar)
            .ok_or_else(|| ParseError::new("expected tensor view", token.span))?;
        instruction.scalar(scalar);
        operands.typed.push((value, ValueType::scalar(scalar)));

        Ok(())
    }

    /// Parse one tensor fill.
    fn parse_tensor_fill(
        &mut self,
        token: Token,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let view = self.parse_tensor_view_input(token, function, instruction)?;
        operands.views.push(view);
        self.eat_token(TokenType::Comma)?;
        let value = self.parse_register()?;
        instruction.register(value);
        let scalar = function
            .value_type(view)
            .and_then(ValueType::tensor_scalar)
            .ok_or_else(|| ParseError::new("expected tensor view", token.span))?;
        instruction.scalar(scalar);
        operands.typed.push((value, ValueType::scalar(scalar)));

        Ok(())
    }

    /// Parse one tensor copy.
    fn parse_tensor_copy(
        &mut self,
        token: Token,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        let source = self.parse_tensor_view_input(token, function, instruction)?;
        let source_type = function
            .value_type(source)
            .ok_or_else(|| ParseError::new("expected tensor view", token.span))?;
        operands.views.push(source);

        // match the target view representation
        self.eat_token(TokenType::Comma)?;
        let target = self.parse_tensor_view_input(token, function, instruction)?;
        operands.views.push(target);
        operands.typed.push((target, source_type));

        // encode the copied element representation
        let scalar = source_type
            .tensor_scalar()
            .ok_or_else(|| ParseError::new("expected tensor view", token.span))?;
        instruction.scalar(scalar);

        Ok(())
    }

    /// Parse one inline tensor view.
    fn parse_tensor_view(
        &mut self,
        token: Token,
        result_types: &[ValueType],
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
        operands: &mut TensorOperands,
    ) -> ParseResult<()> {
        result_types
            .first()
            .copied()
            .filter(|ty| ty.is_tensor_view())
            .ok_or_else(|| ParseError::new("expected tensor view result", token.span))?;
        let input = self.parse_tensor_view_input(token, function, instruction)?;
        operands.views.push(input);

        // parse each derived view component
        for name in ["offsets", "sizes", "strides"] {
            self.eat_token(TokenType::Comma)?;
            let registers = self.parse_named_registers(name, instruction)?;
            operands.typed.extend(
                registers
                    .into_iter()
                    .map(|register| (register, ValueType::scalar(Scalar::Uint64))),
            );
        }

        Ok(())
    }

    /// Parse and encode one tensor view register range.
    fn parse_tensor_view_input(
        &mut self,
        token: Token,
        function: &FunctionParser,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<RegisterId> {
        let view = self.parse_register()?;
        let view_type = function
            .value_type(view)
            .filter(|ty| ty.is_tensor_view())
            .ok_or_else(|| ParseError::new("expected tensor view", token.span))?;
        instruction.range(RegisterRange::new(view, view_type.word_count()));

        Ok(view)
    }

    /// Select one tensor operation from its canonical name.
    fn resolve_tensor_operation(&self, text: &str, token: Token) -> ParseResult<TensorOperation> {
        let mut components = text.split('.');
        let prefix = components.next();
        let Some(name) = components.next() else {
            return Err(ParseError::new("expected tensor operation", token.span));
        };
        if components.next().is_some() {
            return Err(ParseError::new("invalid tensor operation", token.span));
        }

        // parse one dedicated tensor operation
        if prefix == Some("tensor") {
            return TensorOperation::from_name(name)
                .ok_or_else(|| ParseError::new("expected tensor operation", token.span));
        }

        // parse one elementwise scalar operation
        let is_element = match prefix {
            Some("int") => IntegerOperation::from_name(name).is_some(),
            Some("float") => FloatOperation::from_name(name).is_some(),
            _ => false,
        };
        if !is_element {
            return Err(ParseError::new("expected tensor operation", token.span));
        }

        Ok(TensorOperation::Element)
    }

    /// Encode the scalar operator selected by one tensor operation.
    fn resolve_tensor_operator(
        &self,
        text: &str,
        scalar: Scalar,
        token: Token,
    ) -> ParseResult<u16> {
        let mut components = text.split('.');
        let prefix = components.next();
        let operation = components.next();
        let operator = if operation == Some("compare") {
            components.next()
        } else {
            operation
        };
        let Some(operator) = operator else {
            return Err(ParseError::new(
                "expected tensor element operation",
                token.span,
            ));
        };

        // match the operation prefix against the element representation
        let expected_prefix = if scalar.is_float() { "float" } else { "int" };
        if prefix != Some(expected_prefix) {
            return Err(ParseError::new(
                "tensor operation does not match its scalar representation",
                token.span,
            ));
        }

        if scalar.is_integer() || scalar == Scalar::Boolean {
            IntegerOperation::from_name(operator)
                .map(|operation| operation as u16)
                .ok_or_else(|| ParseError::new("expected integer tensor operation", token.span))
        } else {
            FloatOperation::from_name(operator)
                .map(|operation| operation as u16)
                .ok_or_else(|| ParseError::new("expected floating tensor operation", token.span))
        }
    }

    /// Parse and append one tensor input register.
    fn parse_tensor_input(
        &mut self,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<RegisterId> {
        let input = self.parse_register()?;
        instruction.register(input);

        Ok(input)
    }

    /// Parse and encode one named register list.
    fn parse_named_registers(
        &mut self,
        name: &str,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<Vec<RegisterId>> {
        self.eat_name(name)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut registers = Vec::new();

        // parse registers until the named list closes
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            registers.push(self.parse_register()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }
        instruction
            .registers(&registers)
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        Ok(registers)
    }

    /// Parse and encode one bracketed register index list.
    fn parse_register_indices(
        &mut self,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<Vec<RegisterId>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut registers = Vec::new();

        // parse registers until the index list closes
        while !self.eat_token_if(TokenType::CloseBracket) {
            registers.push(self.parse_register()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseBracket)?;

                break;
            }
        }
        instruction
            .registers(&registers)
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        Ok(registers)
    }

    /// Parse and encode one named unsigned 16-bit list.
    fn parse_named_u16s(
        &mut self,
        name: &str,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name(name)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let values = self.parse_u16_list(TokenType::CloseParenthesis)?;
        instruction
            .u16s(&values)
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        Ok(())
    }

    /// Parse and encode one named exact 64-bit list.
    fn parse_named_u64s(
        &mut self,
        name: &str,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name(name)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut values = Vec::new();

        // parse exact values until the named list closes
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            let token = self.bump();
            let value = match token.ty {
                TokenType::Integer => self
                    .text(token)
                    .replace('_', "")
                    .parse::<u64>()
                    .map_err(|_| ParseError::new("expected unsigned 64-bit value", token.span))?,
                TokenType::Identifier if self.text(token) == "false" => 0,
                TokenType::Identifier if self.text(token) == "true" => 1,
                _ => return Err(ParseError::new("expected exact 64-bit value", token.span)),
            };
            values.push(value);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }
        instruction
            .u64s(&values)
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        Ok(())
    }

    /// Parse one tensor convolution.
    fn parse_tensor_convolution(
        &mut self,
        instruction: &mut InstructionBuilder,
        scalar: Scalar,
    ) -> ParseResult<[RegisterId; 2]> {
        let input = self.parse_register()?;
        instruction.register(input);
        self.eat_token(TokenType::Comma)?;
        let kernel = self.parse_register()?;
        instruction.register(kernel);
        instruction.scalar(scalar);

        self.eat_token(TokenType::Comma)?;
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for (single, spatial) in [
            (["inputBatch", "inputFeature"], "inputSpatial"),
            (
                ["kernelInputFeature", "kernelOutputFeature"],
                "kernelSpatial",
            ),
            (["outputBatch", "outputFeature"], "outputSpatial"),
        ] {
            for name in single {
                self.parse_named_u16(name, instruction)?;
                self.eat_token(TokenType::Comma)?;
            }
            self.parse_named_u16s(spatial, instruction)?;
            self.eat_token_if(TokenType::Comma);
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        self.eat_token(TokenType::Comma)?;
        self.eat_name("window")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for name in ["strides", "paddingLow", "paddingHigh"] {
            self.parse_named_u64s(name, instruction)?;
            self.eat_token(TokenType::Comma)?;
        }
        for name in ["baseDilation", "windowDilation"] {
            self.parse_named_u64s(name, instruction)?;
            self.eat_token_if(TokenType::Comma);
        }
        self.parse_named_booleans("reversal", instruction)?;
        self.eat_token_if(TokenType::Comma);
        self.eat_token(TokenType::CloseParenthesis)?;

        self.eat_token(TokenType::Comma)?;
        self.eat_name("groups")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for name in ["feature", "batch"] {
            self.eat_name(name)?;
            self.eat_token(TokenType::OpenParenthesis)?;
            instruction.u32(self.parse_u32()?);
            self.eat_token(TokenType::CloseParenthesis)?;
            self.eat_token_if(TokenType::Comma);
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok([input, kernel])
    }

    /// Parse and encode one named unsigned 16-bit value.
    fn parse_named_u16(
        &mut self,
        name: &str,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name(name)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        instruction.u16(self.parse_u16()?);
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse and encode one named boolean list as unsigned 16-bit values.
    fn parse_named_booleans(
        &mut self,
        name: &str,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name(name)?;
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut values = Vec::new();

        // parse booleans until the named list closes
        while !self.eat_token_if(TokenType::CloseParenthesis) {
            let token = self.eat_token(TokenType::Identifier)?;
            let value = match self.text(token) {
                "false" => 0,
                "true" => 1,
                _ => return Err(ParseError::new("expected boolean", token.span)),
            };
            values.push(value);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseParenthesis)?;

                break;
            }
        }
        instruction
            .u16s(&values)
            .map_err(|error| ParseError::new(error.to_string(), self.empty_span()))?;

        Ok(())
    }
}
