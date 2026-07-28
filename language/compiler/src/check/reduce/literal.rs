use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Origin, answer};

/// Maximum alternatives one template literal expands into.
const TEMPLATE_EXPANSION_LIMIT: usize = 4096;

impl CheckState<'_> {
    /// Reduce one string mapping operation.
    pub(super) fn reduce_string_mapping_operation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        mapping: dir::StringMapping,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_head(origin, target)?);

        match self.ty(target)? {
            // map one closed string literal
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                let mapped = self.reduce_string_mapping(id.module_id, mapping, value)?;

                Ok(Answer::Ready(Some(mapped)))
            }

            // map one exact name key
            dir::Type::Key(dir::StaticKey::Name(value)) => {
                let mapped = self.reduce_string_mapping(id.module_id, mapping, value)?;

                Ok(Answer::Ready(Some(mapped)))
            }

            // distribute string mappings across union elements
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
                let module = id.module_id;
                let Some(mapped) = answer!(self.reduce_distributed_operation(
                    origin,
                    elements,
                    |state, element| {
                        state.reduce_string_mapping_arm(origin, module, mapping, element)
                    }
                )?) else {
                    return Ok(Answer::Ready(None));
                };

                Ok(Answer::Ready(Some(mapped)))
            }

            // open values stay symbolic
            _ => Ok(Answer::Ready(None)),
        }
    }

    /// Concatenate one template literal type over closed spans.
    pub(super) fn reduce_template_literal(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let strings = self
            .template_strings(id.module_id, template.strings)?
            .to_vec();
        let spans = self.type_ids(id.module_id, template.spans)?.to_vec();
        let module = origin.module();

        // close every interpolated span to its printable choices
        let mut printed: Vec<Vec<String>> = Vec::with_capacity(spans.len());
        for span in spans {
            let span = match self.reduce_type_head(origin, span)? {
                Answer::Ready(span) => span,
                Answer::Pending(_) => return Ok(Answer::Ready(None)),
            };

            // a never span empties the whole template
            if matches!(self.ty(span)?, dir::Type::Never) {
                return Ok(Answer::Ready(Some(
                    self.intern_type(module, dir::Type::Never)?,
                )));
            }

            // union spans distribute their printable alternatives
            let choices = match self.ty(span)? {
                dir::Type::Union(union) => {
                    let elements = self.type_ids(span.module_id, union.elements)?.to_vec();
                    let mut choices = Vec::with_capacity(elements.len());
                    for element in elements {
                        let element = match self.reduce_type_head(origin, element)? {
                            Answer::Ready(element) => element,
                            Answer::Pending(_) => return Ok(Answer::Ready(None)),
                        };
                        match self.template_piece_text(element)? {
                            Some(text) => choices.push(text),
                            None => return Ok(Answer::Ready(None)),
                        }
                    }

                    choices
                }
                _ => match self.template_piece_text(span)? {
                    Some(text) => vec![text],
                    None => return Ok(Answer::Ready(None)),
                },
            };
            printed.push(choices);
        }

        // wide distributions stay symbolic
        let combinations: usize = printed.iter().map(Vec::len).product();
        if combinations > TEMPLATE_EXPANSION_LIMIT {
            return Ok(Answer::Ready(None));
        }

        // interleave literal segments with every printed alternative
        let mut joined = vec![String::new()];
        for (index, segment) in strings.iter().enumerate() {
            let segment = self.strings().get(*segment).to_string();
            for text in &mut joined {
                text.push_str(&segment);
            }
            if let Some(choices) = printed.get(index) {
                let mut expanded = Vec::with_capacity(joined.len() * choices.len());
                for text in &joined {
                    for choice in choices {
                        expanded.push(format!("{text}{choice}"));
                    }
                }
                joined = expanded;
            }
        }
        let mut literals = Vec::with_capacity(joined.len());
        for text in joined {
            let text = self.strings().intern(&text);
            let literal = dir::Type::Literal(dir::ScalarLiteral::String(text));
            literals.push(self.intern_type(module, literal)?);
        }
        let reduced = match literals.as_slice() {
            [single] => *single,
            _ => self.normalized_union_type(module, literals)?,
        };

        Ok(Answer::Ready(Some(reduced)))
    }

    /// Evaluate one static binary operation over literal operands.
    pub(super) fn reduce_static_binary_operation(
        &mut self,
        origin: Origin,
        binary: dir::StaticBinaryType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let left = answer!(self.reduce_type_head(origin, binary.left)?);
        let left_literal = match self.ty(left)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };

        // short-circuit logical joins on a decided left operand
        if let Some(dir::ScalarLiteral::Boolean(value)) = left_literal {
            match (binary.operator, value) {
                (dir::StaticBinaryOperator::And, false) | (dir::StaticBinaryOperator::Or, true) => {
                    return Ok(Answer::Ready(Some(left)));
                }
                (dir::StaticBinaryOperator::And, true) | (dir::StaticBinaryOperator::Or, false) => {
                    let right = answer!(self.reduce_type_head(origin, binary.right)?);

                    return match self.ty(right)? {
                        dir::Type::Literal(dir::ScalarLiteral::Boolean(_)) => {
                            Ok(Answer::Ready(Some(right)))
                        }
                        _ => Ok(Answer::Ready(None)),
                    };
                }
                _ => {}
            }
        }

        // close the right operand after short-circuiting
        let right = answer!(self.reduce_type_head(origin, binary.right)?);
        let right_literal = match self.ty(right)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };
        let (Some(left_literal), Some(right_literal)) = (left_literal, right_literal) else {
            return Ok(Answer::Ready(None));
        };

        // concatenate string literals through module storage
        if let (
            dir::StaticBinaryOperator::Add,
            dir::ScalarLiteral::String(left_value),
            dir::ScalarLiteral::String(right_value),
        ) = (binary.operator, left_literal, right_literal)
        {
            let module = origin.module();
            let joined = {
                let strings = self.strings();

                format!("{}{}", strings.get(left_value), strings.get(right_value))
            };
            let joined = self.strings().intern(&joined);
            let literal = dir::Type::Literal(dir::ScalarLiteral::String(joined));

            return Ok(Answer::Ready(Some(self.intern_type(module, literal)?)));
        }

        // evaluate scalar operators directly
        match binary.operator.apply(left_literal, right_literal) {
            Ok(literal) => {
                let id = self.intern_type(origin.module(), dir::Type::Literal(literal))?;

                Ok(Answer::Ready(Some(id)))
            }
            Err(message) => {
                self.report_static_operation(origin, message)?;

                Ok(Answer::Ready(None))
            }
        }
    }

    /// Evaluate one static unary operation over a literal operand.
    pub(super) fn reduce_static_unary_operation(
        &mut self,
        origin: Origin,
        unary: dir::StaticUnaryType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_head(origin, unary.target)?);
        let literal = match self.ty(target)? {
            dir::Type::Literal(literal) => literal,
            _ => return Ok(Answer::Ready(None)),
        };

        // evaluate operators that are defined for the closed literal
        let evaluated = match (unary.operator, literal) {
            (dir::StaticUnaryOperator::Not, dir::ScalarLiteral::Boolean(value)) => {
                Some(dir::ScalarLiteral::Boolean(!value))
            }
            (dir::StaticUnaryOperator::Negate, dir::ScalarLiteral::Integer(value)) => {
                match value.checked_neg() {
                    Some(negated) => Some(dir::ScalarLiteral::Integer(negated)),
                    None => {
                        self.report_static_operation(origin, "integer negation overflows")?;

                        None
                    }
                }
            }
            (dir::StaticUnaryOperator::Negate, dir::ScalarLiteral::Float(value)) => {
                Some(dir::ScalarLiteral::Float(-value))
            }
            (dir::StaticUnaryOperator::BitwiseNot, dir::ScalarLiteral::Integer(value)) => {
                Some(dir::ScalarLiteral::Integer(!value))
            }
            _ => None,
        };

        match evaluated {
            Some(literal) => {
                let id = self.intern_type(origin.module(), dir::Type::Literal(literal))?;

                Ok(Answer::Ready(Some(id)))
            }
            None => Ok(Answer::Ready(None)),
        }
    }

    /// Apply one compiler string mapping to a string literal.
    fn reduce_string_mapping(
        &mut self,
        module: ModuleId,
        mapping: dir::StringMapping,
        value: dir::StringId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let text = self.strings().get(value).to_string();
        let mapped = mapping.apply(&text);
        let mapped = self.strings().intern(&mapped);
        let literal = dir::Type::Literal(dir::ScalarLiteral::String(mapped));

        self.intern_type(module, literal)
    }

    /// Reduce one compiler string mapping arm.
    fn reduce_string_mapping_arm(
        &mut self,
        origin: Origin,
        module: ModuleId,
        mapping: dir::StringMapping,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let target = answer!(self.reduce_type_head(origin, target)?);
        let reduced = match self.ty(target)? {
            dir::Type::Literal(dir::ScalarLiteral::String(value)) => {
                self.reduce_string_mapping(module, mapping, value)?
            }
            dir::Type::Key(dir::StaticKey::Name(value)) => {
                self.reduce_string_mapping(module, mapping, value)?
            }
            _ => self.intern_operation(
                module,
                dir::TypeOperation::StringMapping { mapping, target },
            )?,
        };

        Ok(Answer::Ready(Some(reduced)))
    }
}
