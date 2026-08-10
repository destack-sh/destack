use crate::{
    ConvertMode, ElementOperation, IndexReduceOperation, InstructionBuilder, Opcode, Operand,
    ParseError, ParseResult, Parser, ReduceOperation, RelocationTag, ScatterOperation,
    TensorOperand, TensorOperation, Token, TokenType,
};

use super::function::FunctionParser;

impl Parser<'_> {
    /// Parse one tensor instruction from its exact operand layout.
    pub(super) fn parse_tensor_operation(
        &mut self,
        name: &str,
        token: Token,
        function: &mut FunctionParser,
    ) -> ParseResult<()> {
        let name = name
            .strip_prefix("tensor.")
            .ok_or_else(|| ParseError::new("expected tensor operation", token.span))?;
        let operation = TensorOperation::from_name(name)
            .ok_or_else(|| ParseError::new("unknown tensor operation", token.span))?;
        let opcode = Opcode::tensor(operation);
        let results = self.parse_definitions(opcode)?;
        let layout = opcode
            .layout()
            .ok_or_else(|| ParseError::new("tensor operation has no operand layout", token.span))?;
        let mut instruction = InstructionBuilder::new(opcode);
        let operands = layout
            .operands()
            .iter()
            .skip_while(|operand| matches!(operand, Operand::Result | Operand::ResultRange));

        // parse every encoded operand in layout order
        for (index, operand) in operands.enumerate() {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            self.parse_tensor_operand(operation, *operand, token, &mut instruction)?;
        }

        function.emit(instruction, &results, self.empty_span())
    }

    /// Parse one encoded tensor instruction operand.
    fn parse_tensor_operand(
        &mut self,
        operation: TensorOperation,
        operand: Operand,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        match operand {
            Operand::Register => instruction.register(self.parse_register()?),
            Operand::RegisterList => self.parse_register_list(token, instruction)?,
            Operand::RegisterSpan => instruction.span(self.parse_register_span()?),
            Operand::Tensor => {
                let tensor = self.parse_tensor_value()?;
                instruction.tensor(tensor);
            }
            Operand::TensorList => self.parse_tensor_list(token, instruction)?,
            Operand::Layout => {
                let layout = self.parse_layout_id()?;
                instruction.relocation(RelocationTag::LAYOUT, layout.0);
            }
            Operand::Allocation => {
                let allocation = self.parse_allocation_id()?;
                instruction.relocation(RelocationTag::ALLOCATION, allocation);
            }
            Operand::Operator => {
                let operator = self.parse_tensor_operator(operation, token)?;
                instruction.u16(operator);
            }
            Operand::Unsigned16 => instruction.u16(self.parse_u16()?),
            Operand::Unsigned16List => {
                let values = self.parse_u16_values()?;
                instruction
                    .u16s(&values)
                    .map_err(|error| ParseError::new(error.to_string(), token.span))?;
            }
            Operand::Bits64List => {
                let values = self.parse_u64_values()?;
                instruction
                    .u64s(&values)
                    .map_err(|error| ParseError::new(error.to_string(), token.span))?;
            }
            Operand::ContractionAxes => self.parse_contraction_axes(token, instruction)?,
            Operand::ConvolutionAxes => self.parse_convolution_axes(token, instruction)?,
            Operand::Window => self.parse_window(token, instruction)?,
            Operand::ConvolutionGroups => {
                self.parse_convolution_groups(instruction)?;
            }
            Operand::GatherAxes | Operand::ScatterAxes => {
                self.parse_index_axes(token, instruction)?;
            }
            _ => return Err(ParseError::new("invalid tensor operand layout", token.span)),
        }

        Ok(())
    }

    /// Parse one tensor register span and runtime layout.
    fn parse_tensor_value(&mut self) -> ParseResult<TensorOperand> {
        let registers = self.parse_register_span()?;
        self.eat_token(TokenType::At)?;
        let layout = self.parse_layout_id()?;

        Ok(TensorOperand::new(registers, layout))
    }

    /// Parse one bracketed tensor operand list.
    fn parse_tensor_list(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut tensors = Vec::new();
        while !self.eat_token_if(TokenType::CloseBracket) {
            tensors.push(self.parse_tensor_value()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseBracket)?;

                break;
            }
        }
        instruction
            .tensors(&tensors)
            .map_err(|error| ParseError::new(error.to_string(), token.span))
    }

    /// Parse one bracketed register list.
    fn parse_register_list(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut registers = Vec::new();
        while !self.eat_token_if(TokenType::CloseBracket) {
            registers.push(self.parse_register()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseBracket)?;

                break;
            }
        }
        instruction
            .registers(&registers)
            .map_err(|error| ParseError::new(error.to_string(), token.span))
    }

    /// Parse one bracketed unsigned 16-bit list.
    fn parse_u16_values(&mut self) -> ParseResult<Vec<u16>> {
        self.eat_token(TokenType::OpenBracket)?;

        self.parse_u16_list(TokenType::CloseBracket)
    }

    /// Parse one bracketed unsigned 64-bit list.
    fn parse_u64_values(&mut self) -> ParseResult<Vec<u64>> {
        self.eat_token(TokenType::OpenBracket)?;
        let mut values = Vec::new();
        while !self.eat_token_if(TokenType::CloseBracket) {
            values.push(self.parse_u64()?);
            if !self.eat_token_if(TokenType::Comma) {
                self.eat_token(TokenType::CloseBracket)?;

                break;
            }
        }

        Ok(values)
    }

    /// Parse one tensor operation code with its canonical name.
    fn parse_tensor_operator(
        &mut self,
        operation: TensorOperation,
        token: Token,
    ) -> ParseResult<u16> {
        let value = self.eat_token(TokenType::Identifier)?;
        let name = self.text(value);
        let code = match operation {
            TensorOperation::Element | TensorOperation::Compare => {
                ElementOperation::from_name(name)
                    .filter(|element| operation.accepts(*element))
                    .map(ElementOperation::code)
            }
            TensorOperation::Convert => {
                ConvertMode::from_name(name).map(|operation| operation as u16)
            }
            TensorOperation::Reduce => {
                ReduceOperation::from_name(name).map(|operation| operation as u16)
            }
            TensorOperation::IndexReduce => {
                IndexReduceOperation::from_name(name).map(|operation| operation as u16)
            }
            TensorOperation::Scatter => {
                ScatterOperation::from_name(name).map(|operation| operation as u16)
            }
            _ => None,
        };

        code.ok_or_else(|| ParseError::new("invalid tensor operation code", token.span))
    }

    /// Parse four tensor contraction axis lists.
    fn parse_contraction_axes(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for index in 0..4 {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            let values = self.parse_u16_values()?;
            instruction
                .u16s(&values)
                .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse three tensor convolution dimension mappings.
    fn parse_convolution_axes(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for index in 0..3 {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            self.eat_token(TokenType::OpenParenthesis)?;
            instruction.u16(self.parse_u16()?);
            self.eat_token(TokenType::Comma)?;
            instruction.u16(self.parse_u16()?);
            self.eat_token(TokenType::Comma)?;
            let spatial = self.parse_u16_values()?;
            instruction
                .u16s(&spatial)
                .map_err(|error| ParseError::new(error.to_string(), token.span))?;
            self.eat_token(TokenType::CloseParenthesis)?;
        }
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse one tensor convolution window.
    fn parse_window(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name("window")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for index in 0..5 {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            let values = self.parse_u64_values()?;
            instruction
                .u64s(&values)
                .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        }
        self.eat_token(TokenType::Comma)?;
        let reversals = self.parse_u16_values()?;
        instruction
            .u16s(&reversals)
            .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse tensor convolution feature and batch group counts.
    fn parse_convolution_groups(
        &mut self,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name("groups")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        instruction.u32(self.parse_u32()?);
        self.eat_token(TokenType::Comma)?;
        instruction.u32(self.parse_u32()?);
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }

    /// Parse gather or scatter dimension mappings.
    fn parse_index_axes(
        &mut self,
        token: Token,
        instruction: &mut InstructionBuilder,
    ) -> ParseResult<()> {
        self.eat_name("axes")?;
        self.eat_token(TokenType::OpenParenthesis)?;
        for index in 0..3 {
            if index > 0 {
                self.eat_token(TokenType::Comma)?;
            }
            let values = self.parse_u16_values()?;
            instruction
                .u16s(&values)
                .map_err(|error| ParseError::new(error.to_string(), token.span))?;
        }
        self.eat_token(TokenType::Comma)?;
        instruction.u16(self.parse_u16()?);
        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(())
    }
}
