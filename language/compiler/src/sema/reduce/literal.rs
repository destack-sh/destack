use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin};

/// The member count past which a template literal stops expanding.
const TEMPLATE_EXPANSION_LIMIT: usize = 100_000;

/// The printable alternatives one template span closes to.
enum SpanChoices {
    /// Every alternative the span prints.
    Closed(Vec<String>),
    /// The span stays symbolic.
    Open,
    /// The span alone outruns the expansion bound.
    TooComplex,
}

impl CheckState<'_> {
    /// Reduce one string mapping operation.
    pub(super) fn reduce_string_mapping_operation(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        mapping: dir::StringMapping,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.normalize(origin, target)?;

        match self.ty(target)? {
            // map one closed string literal
            dir::Type::Literal(dir::Literal::String(value)) => {
                let mapped = self.reduce_string_mapping(id.module_id, mapping, value)?;

                Ok(Some(mapped))
            }

            // map one exact name key
            dir::Type::Key(dir::StaticKey::Name(value)) => {
                let mapped = self.reduce_string_mapping(id.module_id, mapping, value)?;

                Ok(Some(mapped))
            }

            // distribute string mappings across union elements
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 4]> =
                    SmallVec::from_slice(self.type_ids(target.module_id, union.elements)?);
                let module = id.module_id;
                let Some(mapped) =
                    self.reduce_distributed_operation(origin, elements, |state, element| {
                        state.reduce_string_mapping_arm(origin, module, mapping, element)
                    })?
                else {
                    return Ok(None);
                };

                Ok(Some(mapped))
            }

            // map through a symbolic template, segment by segment
            dir::Type::Operation(operation)
                if let dir::TypeOperation::TemplateLiteral(template) =
                    self.type_operation(target.module_id, operation)? =>
            {
                let mapped = self.map_template_literal(target, mapping, &template)?;

                Ok(Some(mapped))
            }

            // open values stay symbolic
            _ => Ok(None),
        }
    }

    /// Apply one string mapping through a template literal's segments and spans.
    fn map_template_literal(
        &mut self,
        id: dir::GlobalTypeId,
        mapping: dir::StringMapping,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let mut strings: SmallVec<[_; 4]> = self
            .template_strings(id.module_id, template.strings)?
            .into();
        let mut spans: SmallVec<[_; 4]> = self.type_ids(id.module_id, template.spans)?.into();

        match mapping {
            // reach every segment and span for a whole-string mapping
            dir::StringMapping::Uppercase | dir::StringMapping::Lowercase => {
                for string in &mut strings {
                    *string = self.map_string(mapping, *string);
                }
                for span in &mut spans {
                    *span = self.intern_operation(dir::TypeOperation::StringMapping {
                        mapping,
                        target: *span,
                    })?;
                }
            }
            // reach only the leading segment or span for a first-character mapping
            dir::StringMapping::Capitalize | dir::StringMapping::Uncapitalize => {
                let leads_with_text = strings
                    .first()
                    .is_some_and(|string| !self.strings().get(*string).is_empty());
                if leads_with_text {
                    strings[0] = self.map_string(mapping, strings[0]);
                } else if let Some(span) = spans.first_mut() {
                    *span = self.intern_operation(dir::TypeOperation::StringMapping {
                        mapping,
                        target: *span,
                    })?;
                }
            }
        }

        let strings = self.intern_strings(&strings)?;
        let spans = self.intern_type_ids(&spans)?;

        self.intern_operation(dir::TypeOperation::TemplateLiteral(
            dir::TemplateLiteralType { strings, spans },
        ))
    }

    /// Apply one string mapping to interned text.
    fn map_string(&mut self, mapping: dir::StringMapping, value: dir::StringId) -> dir::StringId {
        let text = self.strings().get(value).to_string();
        let mapped = mapping.apply(&text);

        self.strings().intern(&mapped)
    }

    /// Concatenate one template literal type over closed spans.
    pub(super) fn reduce_template_literal(
        &mut self,
        origin: Origin,
        id: dir::GlobalTypeId,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let strings: SmallVec<[_; 4]> = self
            .template_strings(id.module_id, template.strings)?
            .into();
        let spans: SmallVec<[_; 8]> = self.type_ids(id.module_id, template.spans)?.into();

        // splice nested templates into one flat segment and span list
        let mut segments = Vec::with_capacity(strings.len());
        let mut flat_spans = Vec::with_capacity(spans.len());
        let mut spliced = false;
        let mut segment = self.strings().get(strings[0]).to_string();
        for (index, span) in spans.iter().enumerate() {
            let span = self.normalize(origin, *span)?;
            match self.ty(span)? {
                dir::Type::Operation(operation)
                    if let dir::TypeOperation::TemplateLiteral(inner) =
                        self.type_operation(span.module_id, operation)? =>
                {
                    let inner_strings: SmallVec<[_; 4]> =
                        self.template_strings(span.module_id, inner.strings)?.into();
                    let inner_spans: SmallVec<[_; 4]> =
                        self.type_ids(span.module_id, inner.spans)?.into();
                    segment.push_str(self.strings().get(inner_strings[0]));
                    for (inner_index, inner_span) in inner_spans.iter().enumerate() {
                        flat_spans.push(self.normalize(origin, *inner_span)?);
                        segments.push(std::mem::take(&mut segment));
                        segment = self
                            .strings()
                            .get(inner_strings[inner_index + 1])
                            .to_string();
                    }
                    spliced = true;
                }
                _ => {
                    flat_spans.push(span);
                    segments.push(std::mem::take(&mut segment));
                }
            }
            segment.push_str(self.strings().get(strings[index + 1]));
        }
        segments.push(segment);

        // close every interpolated span to its printable choices
        let mut printed: Vec<Vec<String>> = Vec::with_capacity(flat_spans.len());
        for &span in &flat_spans {
            // empty the whole template on a never span
            if matches!(self.ty(span)?, dir::Type::Never) {
                return Ok(Some(self.intern_type(dir::Type::Never)?));
            }

            match self.template_span_choices(origin, span)? {
                SpanChoices::Closed(choices) => printed.push(choices),
                // reject a span past the expansion bound
                SpanChoices::TooComplex => {
                    self.report_template_literal_too_complex(origin)?;

                    return Ok(Some(self.intern_type(dir::Type::Error)?));
                }
                // keep the flattened template symbolic while a span stays open
                SpanChoices::Open => {
                    if !spliced {
                        return Ok(None);
                    }
                    let strings = segments
                        .iter()
                        .map(|segment| self.strings().intern(segment))
                        .collect::<SmallVec<[_; 4]>>();
                    let strings = self.intern_strings(&strings)?;
                    let spans = self.intern_type_ids(&flat_spans)?;
                    let flattened = self.intern_operation(dir::TypeOperation::TemplateLiteral(
                        dir::TemplateLiteralType { strings, spans },
                    ))?;

                    return Ok(Some(flattened));
                }
            }
        }

        // reject distributions past the expansion bound
        let combinations = printed
            .iter()
            .map(Vec::len)
            .try_fold(1usize, |product, len| product.checked_mul(len));
        if combinations.is_none_or(|combinations| combinations > TEMPLATE_EXPANSION_LIMIT) {
            self.report_template_literal_too_complex(origin)?;

            return Ok(Some(self.intern_type(dir::Type::Error)?));
        }

        // interleave literal segments with every printed alternative
        let mut joined = vec![String::new()];
        for (index, segment) in segments.iter().enumerate() {
            for text in &mut joined {
                text.push_str(segment);
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
            let literal = dir::Type::Literal(dir::Literal::String(text));
            literals.push(self.intern_type(literal)?);
        }
        let reduced = match literals.as_slice() {
            [single] => *single,
            _ => self.normalized_union_type(literals)?,
        };

        Ok(Some(reduced))
    }

    /// Return the printable alternatives of one closed template span.
    fn template_span_choices(
        &mut self,
        origin: Origin,
        span: dir::GlobalTypeId,
    ) -> CompilerResult<SpanChoices> {
        let choices = match self.ty(span)? {
            // distribute a union span's printable alternatives
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(span.module_id, union.elements)?.into();
                let mut choices = Vec::with_capacity(elements.len());
                for element in elements {
                    let element = self.normalize(origin, element)?;
                    match self.template_span_choices(origin, element)? {
                        SpanChoices::Closed(texts) => choices.extend(texts),
                        open => return Ok(open),
                    }
                }

                choices
            }
            // print both values for a boolean span
            dir::Type::Primitive(dir::PrimitiveType::Boolean) => {
                vec!["false".to_string(), "true".to_string()]
            }
            // print every integer of a bounded range span
            dir::Type::Range(dir::RangeType {
                start: Some(dir::Literal::Integer(start)),
                end: Some(dir::Literal::Integer(end)),
                is_inclusive,
            }) => {
                let end = if is_inclusive { end } else { end - 1 };
                if end < start {
                    return Ok(SpanChoices::Closed(Vec::new()));
                }
                let fits = usize::try_from(end - start + 1)
                    .is_ok_and(|count| count <= TEMPLATE_EXPANSION_LIMIT);
                if !fits {
                    return Ok(SpanChoices::TooComplex);
                }

                (start..=end).map(|value| value.to_string()).collect()
            }
            _ => match self.template_piece_text(span)? {
                Some(text) => vec![text],
                None => return Ok(SpanChoices::Open),
            },
        };

        Ok(SpanChoices::Closed(choices))
    }

    /// Evaluate one static binary operation over literal operands.
    pub(super) fn reduce_static_binary_operation(
        &mut self,
        origin: Origin,
        binary: dir::StaticBinaryType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let left = self.normalize(origin, binary.left)?;
        let left_literal = match self.ty(left)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };

        // short-circuit logical joins on a decided left operand
        if let Some(dir::Literal::Boolean(value)) = left_literal {
            match (binary.operator, value) {
                (dir::StaticBinaryOperator::And, false) | (dir::StaticBinaryOperator::Or, true) => {
                    return Ok(Some(left));
                }
                (dir::StaticBinaryOperator::And, true) | (dir::StaticBinaryOperator::Or, false) => {
                    let right = self.normalize(origin, binary.right)?;

                    return match self.ty(right)? {
                        dir::Type::Literal(dir::Literal::Boolean(_)) => Ok(Some(right)),
                        _ => Ok(None),
                    };
                }
                _ => {}
            }
        }

        // close the right operand after short-circuiting
        let right = self.normalize(origin, binary.right)?;
        let right_literal = match self.ty(right)? {
            dir::Type::Literal(literal) => Some(literal),
            _ => None,
        };
        let (Some(left_literal), Some(right_literal)) = (left_literal, right_literal) else {
            return Ok(None);
        };

        // concatenate string literals through module storage
        if let (
            dir::StaticBinaryOperator::Add,
            dir::Literal::String(left_value),
            dir::Literal::String(right_value),
        ) = (binary.operator, left_literal, right_literal)
        {
            let _module = origin.module();
            let joined = {
                let strings = self.strings();

                format!("{}{}", strings.get(left_value), strings.get(right_value))
            };
            let joined = self.strings().intern(&joined);
            let literal = dir::Type::Literal(dir::Literal::String(joined));

            return Ok(Some(self.intern_type(literal)?));
        }

        // evaluate scalar operators directly
        match binary.operator.apply(left_literal, right_literal) {
            Ok(literal) => {
                let id = self.intern_type(dir::Type::Literal(literal))?;

                Ok(Some(id))
            }
            Err(message) => {
                self.report_static_operation(origin, message)?;

                Ok(None)
            }
        }
    }

    /// Evaluate one static unary operation over a literal operand.
    pub(super) fn reduce_static_unary_operation(
        &mut self,
        origin: Origin,
        unary: dir::StaticUnaryType,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.normalize(origin, unary.target)?;
        let literal = match self.ty(target)? {
            dir::Type::Literal(literal) => literal,
            _ => return Ok(None),
        };

        // evaluate the scalar operator directly
        match unary.operator.apply(literal) {
            Ok(literal) => {
                let id = self.intern_type(dir::Type::Literal(literal))?;

                Ok(Some(id))
            }
            Err(message) => {
                self.report_static_operation(origin, message)?;

                Ok(None)
            }
        }
    }

    /// Apply one compiler string mapping to a string literal.
    fn reduce_string_mapping(
        &mut self,
        _module: ModuleId,
        mapping: dir::StringMapping,
        value: dir::StringId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let text = self.strings().get(value).to_string();
        let mapped = mapping.apply(&text);
        let mapped = self.strings().intern(&mapped);
        let literal = dir::Type::Literal(dir::Literal::String(mapped));

        self.intern_type(literal)
    }

    /// Reduce one compiler string mapping arm.
    fn reduce_string_mapping_arm(
        &mut self,
        origin: Origin,
        module: ModuleId,
        mapping: dir::StringMapping,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let target = self.normalize(origin, target)?;
        let reduced = match self.ty(target)? {
            dir::Type::Literal(dir::Literal::String(value)) => {
                self.reduce_string_mapping(module, mapping, value)?
            }
            dir::Type::Key(dir::StaticKey::Name(value)) => {
                self.reduce_string_mapping(module, mapping, value)?
            }
            _ => self.intern_operation(dir::TypeOperation::StringMapping { mapping, target })?,
        };

        Ok(Some(reduced))
    }
}
