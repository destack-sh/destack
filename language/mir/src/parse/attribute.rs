use crate::{
    Attribute, AttributeArgs, AttributeKeyValue, AttributeValue, ExecutionModel, ExecutionStage,
    FloatValue,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;
use super::token::TokenType;

#[allow(clippy::type_complexity)]
impl<'a> Parser<'a> {
    /// Parse an attribute list prefix.
    pub(super) fn parse_attributes(&mut self) -> ParseResult<Vec<Attribute>> {
        // attribute blocks
        let mut attributes = Vec::new();
        while self.peek_token(TokenType::Hash) {
            let attribute = self.parse_attribute()?;
            attributes.push(attribute);
        }

        Ok(attributes)
    }

    /// Parse a single attribute.
    fn parse_attribute(&mut self) -> ParseResult<Attribute> {
        // attribute header
        self.eat_token(TokenType::Hash)?;
        self.eat_token(TokenType::OpenBracket)?;

        // name and arguments
        let name_token = self.eat_token(TokenType::Identifier)?;
        let name_text = name_token.text.to_string();
        let name = self.strings.intern(&name_text);
        let args = if self.eat_token_maybe(TokenType::OpenParen) {
            let args = self.parse_attribute_args()?;
            self.eat_token(TokenType::CloseParen)?;
            args
        } else {
            AttributeArgs::None
        };

        // close and return
        self.eat_token(TokenType::CloseBracket)?;

        Ok(Attribute { name, args })
    }

    /// Parse a single attribute argument list.
    fn parse_attribute_args(&mut self) -> ParseResult<AttributeArgs> {
        // empty list
        if self.peek_token(TokenType::CloseParen) {
            return Ok(AttributeArgs::None);
        }

        // key value list
        if self.peek_token(TokenType::Identifier)
            && self
                .peek_nth_token(1)
                .is_some_and(|token| token.ty == TokenType::Equals)
        {
            let mut pairs = Vec::new();
            loop {
                let key_token = self.eat_token(TokenType::Identifier)?;
                let key_text = key_token.text.to_string();
                let key = self.strings.intern(&key_text);
                self.eat_token(TokenType::Equals)?;
                let value = self.parse_attribute_value()?;
                pairs.push(AttributeKeyValue { key, value });

                if !self.eat_token_maybe(TokenType::Comma) {
                    break;
                }
            }

            return Ok(AttributeArgs::KeyValues(pairs));
        }

        // single value or list
        let first = self.parse_attribute_value()?;
        if !self.eat_token_maybe(TokenType::Comma) {
            return Ok(AttributeArgs::Value(first));
        }

        // value list
        let mut values = vec![first];
        loop {
            let value = self.parse_attribute_value()?;
            values.push(value);
            if !self.eat_token_maybe(TokenType::Comma) {
                break;
            }
        }

        Ok(AttributeArgs::Values(values))
    }

    /// Parse a single attribute value.
    fn parse_attribute_value(&mut self) -> ParseResult<AttributeValue> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("attribute value", self.pos()))?;
        let token_ty = token.ty;
        let token_text = token.text.to_string();
        let token_start = token.start;

        // parse value
        match token_ty {
            TokenType::Identifier => {
                self.bump();
                Ok(AttributeValue::Identifier(self.strings.intern(&token_text)))
            }
            TokenType::BoolLiteral => {
                self.bump();
                let value = token_text == "true";
                Ok(AttributeValue::Boolean(value))
            }
            TokenType::IntLiteral => {
                let value = self.parse_int_literal()?;
                Ok(AttributeValue::Integer(value))
            }
            TokenType::FloatLiteral => {
                self.bump();
                let value = self.parse_attribute_float(&token_text, token_start)?;
                Ok(AttributeValue::Float(value))
            }
            TokenType::StringLiteral => {
                self.bump();
                let value =
                    super::constant::parse_string_literal(&token_text).ok_or_else(|| {
                        ParseError::invalid(&format!("string literal '{token_text}'"), token_start)
                    })?;
                Ok(AttributeValue::String(self.strings.intern(&value)))
            }
            TokenType::OpenBracket => {
                // list literal
                self.eat_token(TokenType::OpenBracket)?;
                let mut values = Vec::new();
                while !self.peek_token(TokenType::CloseBracket) {
                    values.push(self.parse_attribute_value()?);
                    if !self.eat_token_maybe(TokenType::Comma) {
                        break;
                    }
                }
                self.eat_token(TokenType::CloseBracket)?;

                Ok(AttributeValue::List(values))
            }
            _ => Err(ParseError::unexpected(
                "attribute value",
                token_ty,
                token_start,
            )),
        }
    }

    /// Parse a float literal for attributes.
    fn parse_attribute_float(&mut self, text: &str, start: usize) -> ParseResult<FloatValue> {
        // split off a type suffix when present
        let (digits, width) = if let Some(pos) = text.rfind('f') {
            let (digits, suffix) = text.split_at(pos);
            let suffix_digits = &suffix[1..];
            if !suffix_digits.is_empty() && suffix_digits.chars().all(|c| c.is_ascii_digit()) {
                let width = suffix_digits
                    .parse::<u16>()
                    .map_err(|_| ParseError::invalid("float width", start))?;
                (digits, Some(width))
            } else {
                (text, None)
            }
        } else {
            (text, None)
        };

        // normalize numeric separators
        let digits = digits.replace('_', "");

        // parse the numeric value
        let value = match width {
            Some(32) => {
                let value: f32 = digits
                    .parse()
                    .map_err(|_| ParseError::invalid("float literal", start))?;
                value as f64
            }
            Some(64) => digits
                .parse::<f64>()
                .map_err(|_| ParseError::invalid("float literal", start))?,
            Some(_) => {
                return Err(ParseError::invalid("float width", start));
            }
            None => digits
                .parse::<f64>()
                .map_err(|_| ParseError::invalid("float literal", start))?,
        };

        Ok(FloatValue::from_f64(value))
    }

    /// Resolve attributes into function metadata.
    pub(super) fn resolve_function_attributes(
        &mut self,
        attributes: &[Attribute],
    ) -> ParseResult<(Option<ExecutionModel>, Option<ExecutionStage>, Option<[u32; 3]>)> {
        // metadata outputs
        let mut execution_model = None;
        let mut execution_stage = None;
        let mut workgroup_size = None;

        // inspect attributes
        for attribute in attributes {
            let name = {
                let name_ref = self.strings.get(attribute.name);
                name_ref.to_string()
            };
            match name.as_str() {
                "execution_model" => {
                    if execution_model.is_some() {
                        return Err(ParseError::new(
                            "duplicate execution_model attribute",
                            self.pos(),
                        ));
                    }
                    let value = match &attribute.args {
                        AttributeArgs::Value(AttributeValue::Identifier(value)) => {
                            self.strings.get(*value)
                        }
                        AttributeArgs::Value(AttributeValue::String(value)) => {
                            self.strings.get(*value)
                        }
                        _ => {
                            return Err(ParseError::new(
                                "execution_model expects an identifier",
                                self.pos(),
                            ));
                        }
                    };
                    let value = value.to_string();
                    let model = ExecutionModel::try_from(value.as_str()).map_err(|_| {
                        ParseError::invalid(&format!("execution model '{value}'"), self.pos())
                    })?;
                    execution_model = Some(model);
                }
                "execution_stage" => {
                    if execution_stage.is_some() {
                        return Err(ParseError::new(
                            "duplicate execution_stage attribute",
                            self.pos(),
                        ));
                    }
                    let value = match &attribute.args {
                        AttributeArgs::Value(AttributeValue::Identifier(value)) => {
                            self.strings.get(*value)
                        }
                        AttributeArgs::Value(AttributeValue::String(value)) => {
                            self.strings.get(*value)
                        }
                        _ => {
                            return Err(ParseError::new(
                                "execution_stage expects an identifier",
                                self.pos(),
                            ));
                        }
                    };
                    let value = value.to_string();
                    let stage = ExecutionStage::try_from(value.as_str()).map_err(|_| {
                        ParseError::invalid(&format!("execution stage '{value}'"), self.pos())
                    })?;
                    execution_stage = Some(stage);
                }
                "workgroup_size" => {
                    if workgroup_size.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroup_size attribute",
                            self.pos(),
                        ));
                    }
                    let size = self.parse_workgroup_size(&attribute.args)?;
                    workgroup_size = Some(size);
                }
                _ => {}
            }
        }

        Ok((execution_model, execution_stage, workgroup_size))
    }

    /// Parse a workgroup_size attribute.
    fn parse_workgroup_size(&mut self, args: &AttributeArgs) -> ParseResult<[u32; 3]> {
        // list or key values
        let dims = match args {
            AttributeArgs::Value(AttributeValue::Integer(value)) => [*value, 1, 1],
            AttributeArgs::Values(values) => self.parse_workgroup_dims_from_values(values)?,
            AttributeArgs::Value(AttributeValue::List(values)) => {
                self.parse_workgroup_dims_from_values(values)?
            }
            AttributeArgs::KeyValues(pairs) => self.parse_workgroup_dims_from_pairs(pairs)?,
            _ => {
                return Err(ParseError::new(
                    "workgroup_size expects one to three integer values",
                    self.pos(),
                ));
            }
        };

        self.check_workgroup_size(dims)
    }

    /// Parse positional workgroup sizes into a full 3D array.
    fn parse_workgroup_dims_from_values(
        &mut self,
        values: &[AttributeValue],
    ) -> ParseResult<[i64; 3]> {
        // require between one and three values
        if values.is_empty() || values.len() > 3 {
            return Err(ParseError::new(
                "workgroup_size expects one to three integer values",
                self.pos(),
            ));
        }

        // default missing dimensions to 1
        let mut dims = [1i64; 3];
        for (index, value) in values.iter().enumerate() {
            match value {
                AttributeValue::Integer(value) => dims[index] = *value,
                _ => {
                    return Err(ParseError::new(
                        "workgroup_size values must be integers",
                        self.pos(),
                    ));
                }
            }
        }

        Ok(dims)
    }

    /// Parse keyed workgroup sizes into a full 3D array.
    fn parse_workgroup_dims_from_pairs(
        &mut self,
        pairs: &[AttributeKeyValue],
    ) -> ParseResult<[i64; 3]> {
        // collect keyed values
        let mut x = None;
        let mut y = None;
        let mut z = None;
        for pair in pairs {
            let key = self.strings.get(pair.key);
            let key = key.as_ref();
            let value = match &pair.value {
                AttributeValue::Integer(value) => *value,
                _ => {
                    return Err(ParseError::new(
                        "workgroup_size values must be integers",
                        self.pos(),
                    ));
                }
            };
            match key {
                "x" => {
                    if x.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroup_size x value",
                            self.pos(),
                        ));
                    }
                    x = Some(value);
                }
                "y" => {
                    if y.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroup_size y value",
                            self.pos(),
                        ));
                    }
                    y = Some(value);
                }
                "z" => {
                    if z.is_some() {
                        return Err(ParseError::new(
                            "duplicate workgroup_size z value",
                            self.pos(),
                        ));
                    }
                    z = Some(value);
                }
                _ => {
                    return Err(ParseError::invalid(
                        &format!("workgroup_size key '{key}'"),
                        self.pos(),
                    ));
                }
            }
        }

        // require x and default missing dimensions to 1
        let x = x.ok_or_else(|| ParseError::new("workgroup_size requires x", self.pos()))?;
        let y = y.unwrap_or(1);
        let z = z.unwrap_or(1);

        Ok([x, y, z])
    }

    /// Validate and coerce workgroup size values.
    fn check_workgroup_size(&mut self, dims: [i64; 3]) -> ParseResult<[u32; 3]> {
        // validate dimensions
        let mut size = [0u32; 3];
        for (index, dim) in dims.into_iter().enumerate() {
            if dim < 0 {
                return Err(ParseError::new(
                    "workgroup_size values must be non-negative",
                    self.pos(),
                ));
            }

            size[index] = dim as u32;
        }

        Ok(size)
    }
}
