use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, Dependency, Origin, answer};

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
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<Answer<Option<dir::GlobalTypeId>>> {
        let module = origin.module();
        let strings = self.template_strings(module, template.strings)?.to_vec();
        let spans = self.type_ids(module, template.spans)?.to_vec();

        // close every interpolated span to printable text
        let mut printed = Vec::with_capacity(spans.len());
        let mut blockers = SmallVec::<[Dependency; 2]>::new();
        for span in spans {
            let span = match self.reduce_type_head(origin, span)? {
                Answer::Ready(span) => span,
                Answer::Pending(dependencies) => {
                    blockers.extend(dependencies);

                    continue;
                }
            };

            let text = match self.ty(span)? {
                dir::Type::Literal(literal) => literal.template_text(&self.module(module).strings),
                dir::Type::Null => Some("null".to_string()),
                dir::Type::Undefined => Some("undefined".to_string()),
                _ => None,
            };
            match text {
                Some(text) => printed.push(text),
                None => return Ok(Answer::Ready(None)),
            }
        }
        if !blockers.is_empty() {
            return Ok(Answer::pending(blockers));
        }

        // interleave literal segments with printed spans
        let mut joined = String::new();
        for (index, segment) in strings.iter().enumerate() {
            joined.push_str(self.module(module).strings.get(*segment));
            if let Some(text) = printed.get(index) {
                joined.push_str(text);
            }
        }
        let joined = self.module_mut(module).strings.intern(&joined);
        let literal = dir::Type::Literal(dir::ScalarLiteral::String(joined));

        Ok(Answer::Ready(Some(self.intern_type(module, literal)?)))
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
                let strings = &self.module(module).strings;

                format!("{}{}", strings.get(left_value), strings.get(right_value))
            };
            let joined = self.module_mut(module).strings.intern(&joined);
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
        let text = self.module(module).strings.get(value).to_string();
        let mapped = mapping.apply(&text);
        let mapped = self.module_mut(module).strings.intern(&mapped);
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
            _ => self.intern_type(
                module,
                dir::Type::Operation(dir::TypeOperation::StringMapping { mapping, target }),
            )?,
        };

        Ok(Answer::Ready(Some(reduced)))
    }
}
