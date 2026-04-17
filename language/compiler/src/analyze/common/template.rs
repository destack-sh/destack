use std::collections::HashSet;

use destack_dir::{
    Freshness, IntType, LocalNodeIdAny, LocalTypeId, PrimitiveType, ScalarLiteral, StaticKey,
    StringId, Type, TypeLiteral, TypeTable,
};

use super::{NormalizationMode, RelationMode, TypeContext};
use crate::{Assignability, Compiler};

const TEMPLATE_LITERAL_KEY_EXPANSION_LIMIT: usize = 256;

/// Represent the source segment that maps to a single template span.
#[derive(Debug, Clone)]
struct TemplateSpanSegment {
    /// Literal fragments captured for this span.
    literals: Vec<String>,
    /// Source template spans captured for this span.
    spans: Vec<LocalTypeId>,
}

impl TemplateSpanSegment {
    /// Create an empty segment.
    fn new() -> Self {
        // build empty literal and span buffers
        Self {
            literals: Vec::new(),
            spans: Vec::new(),
        }
    }

    /// Return the concatenated literal string when no spans exist.
    fn literal_string(&self) -> Option<String> {
        // reject segments with captured spans
        if !self.spans.is_empty() {
            return None;
        }

        // concatenate literal fragments
        Some(self.literals.join(""))
    }
}

/// The resolved span type used for template matching.
#[derive(Debug, Clone, Copy)]
enum TemplateSpanMatch {
    /// The span can match any string content.
    Any,
    /// The span must match a concrete type.
    Type(LocalTypeId),
}

/// The resolved key shape for a template literal.
#[derive(Debug, Clone)]
pub(super) enum TemplateLiteralKeyShape {
    /// Literal property keys produced by the template.
    Literal(Vec<StaticKey>),
    /// A string index signature key shape for broad templates.
    StringIndex,
}

/// Parsed numeric string parts.
#[derive(Debug, Clone, Copy)]
struct NumericStringParts<'a> {
    /// Trimmed input string.
    trimmed: &'a str,
    /// Whether the original string had surrounding whitespace.
    has_whitespace: bool,
    /// Whether the literal includes a leading minus sign.
    is_negative: bool,
    /// Whether the literal has any explicit sign.
    has_sign: bool,
    /// Radix for non decimal literals.
    radix: Option<u32>,
    /// The digit payload without prefix or sign.
    digits: &'a str,
}

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Collect string values for string ids.
    fn collect_string_parts(&self, strings: &[StringId]) -> Vec<String> {
        // allocate output buffer
        let mut parts = Vec::with_capacity(strings.len());

        // copy string values
        for string_id in strings {
            let value = self.repository.strings.get(*string_id);
            parts.push(value.to_string());
        }

        parts
    }

    /// Match a template literal's string parts against a concrete string.
    pub(crate) fn match_template_literal_to_string(
        &self,
        strings: &[StringId],
        value: &str,
    ) -> Option<Vec<String>> {
        // collect literal parts
        let parts = self.collect_string_parts(strings);

        // require at least one literal part
        if parts.is_empty() {
            return None;
        }

        // handle single literal templates
        if parts.len() == 1 {
            if parts[0] == value {
                return Some(Vec::new());
            }

            return None;
        }

        // require prefix match
        if !value.starts_with(parts[0].as_str()) {
            return None;
        }

        // allocate span storage
        let mut spans = Vec::with_capacity(parts.len().saturating_sub(1));

        // track current offset
        let mut index = parts[0].len();

        // walk each literal boundary
        for part_index in 0..(parts.len() - 1) {
            let next_literal = parts[part_index + 1].as_str();
            let remaining = &value[index..];

            // handle trailing empty literal
            if next_literal.is_empty() && part_index + 1 == parts.len() - 1 {
                spans.push(remaining.to_string());
                index = value.len();
                break;
            }

            // handle empty literal boundary
            if next_literal.is_empty() {
                if remaining.is_empty() {
                    spans.push(String::new());
                } else {
                    let mut chars = remaining.chars();
                    let first = chars.next()?;
                    let first_len = first.len_utf8();
                    spans.push(remaining[..first_len].to_string());
                    index += first_len;
                }
                continue;
            }

            // locate next literal
            let pos = remaining.find(next_literal)?;

            // record span to the next literal
            spans.push(remaining[..pos].to_string());
            index += pos + next_literal.len();
        }

        // require full coverage
        if index != value.len() {
            return None;
        }

        Some(spans)
    }

    /// Check whether a string literal matches a template literal type.
    pub(crate) fn template_literal_matches_string(
        &self,
        ctx: &mut TypeContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        value: &str,
    ) -> bool {
        // treat `${string}` templates as string
        if self.template_literal_is_string_supertype(&mut ctx.reborrow(), strings, spans) {
            return true;
        }

        // match literal parts against the string
        let Some(span_values) = self.match_template_literal_to_string(strings, value) else {
            return false;
        };

        // require span counts to match
        if span_values.len() != spans.len() {
            return false;
        }

        // validate each span
        let mut visited = HashSet::new();
        for (span_ty_id, span_value) in spans.iter().zip(span_values.iter()) {
            if !self.template_span_matches_string(
                &mut ctx.reborrow(),
                *span_ty_id,
                span_value,
                &mut visited,
            ) {
                return false;
            }
        }

        // accept when all spans match
        true
    }

    /// Check whether one template literal type is assignable to another.
    pub(crate) fn template_literal_matches_template(
        &self,
        ctx: &mut TypeContext<'_>,
        target_strings: &[StringId],
        target_spans: &[LocalTypeId],
        source_strings: &[StringId],
        source_spans: &[LocalTypeId],
    ) -> bool {
        // handle literal only templates
        if target_strings.len() == 1 && target_spans.is_empty() {
            if source_strings.len() == 1 && source_spans.is_empty() {
                return target_strings[0] == source_strings[0];
            }

            return false;
        }

        // allow string supertype targets
        if self.template_literal_is_string_supertype(
            &mut ctx.reborrow(),
            target_strings,
            target_spans,
        ) {
            return true;
        }

        // require aligned literal and span counts
        if target_strings.len() != target_spans.len() + 1 {
            return false;
        }

        // require aligned literal and span counts
        if source_strings.len() != source_spans.len() + 1 {
            return false;
        }

        // split source spans into target segments
        let Some(segments) =
            self.match_template_literal_segments(target_strings, source_strings, source_spans)
        else {
            return false;
        };

        // validate segment counts
        if segments.len() != target_spans.len() {
            return false;
        }

        // check each target span against its segment
        for (segment, span_ty_id) in segments.iter().zip(target_spans.iter()) {
            if !self.template_segment_matches_span_type(&mut ctx.reborrow(), *span_ty_id, segment) {
                return false;
            }
        }

        // accept when every segment fits
        true
    }

    /// Match source literal parts against target literal boundaries.
    fn match_template_literal_segments(
        &self,
        target_strings: &[StringId],
        source_strings: &[StringId],
        source_spans: &[LocalTypeId],
    ) -> Option<Vec<TemplateSpanSegment>> {
        // collect target literal parts
        let target_parts = self.collect_string_parts(target_strings);

        // collect source literal parts
        let source_parts = self.collect_string_parts(source_strings);

        // require non empty literal parts
        if target_parts.is_empty() || source_parts.is_empty() {
            return None;
        }

        // require aligned prefix
        let first_target = target_parts[0].as_str();
        if !source_parts[0].starts_with(first_target) {
            return None;
        }

        // prepare segment tracking
        let mut segments = Vec::with_capacity(target_parts.len().saturating_sub(1));
        let mut source_index = 0usize;
        let mut source_offset = first_target.len();

        // scan target literal boundaries
        for target_index in 0..(target_parts.len() - 1) {
            let next_literal = target_parts[target_index + 1].as_str();
            let mut segment = TemplateSpanSegment::new();

            // capture the remaining source when the next literal is empty
            if next_literal.is_empty() {
                let source_literal = source_parts.get(source_index)?;
                let remainder = &source_literal[source_offset..];
                if !remainder.is_empty() {
                    segment.literals.push(remainder.to_string());
                }

                if source_index < source_spans.len() {
                    segment
                        .spans
                        .extend_from_slice(&source_spans[source_index..]);
                }

                for literal in source_parts.iter().skip(source_index + 1) {
                    if !literal.is_empty() {
                        segment.literals.push(literal.to_string());
                    }
                }

                source_index = source_parts.len() - 1;
                source_offset = source_parts.last().map(|value| value.len()).unwrap_or(0);
                segments.push(segment);
                continue;
            }

            // advance through source literals until the next boundary
            loop {
                let source_literal = source_parts.get(source_index)?;

                // find the next literal in the remaining source slice
                let remainder = &source_literal[source_offset..];
                if let Some(pos) = remainder.find(next_literal) {
                    let prefix = &remainder[..pos];
                    if !prefix.is_empty() {
                        segment.literals.push(prefix.to_string());
                    }

                    source_offset += pos + next_literal.len();
                    break;
                }

                // capture the rest of the literal and advance spans
                if !remainder.is_empty() {
                    segment.literals.push(remainder.to_string());
                }

                // require another source span to advance
                if source_index >= source_spans.len() {
                    return None;
                }

                // record the source span and advance
                segment.spans.push(source_spans[source_index]);
                source_index += 1;
                source_offset = 0;
            }

            segments.push(segment);
        }

        // require the trailing literal to be fully consumed
        if source_index + 1 != source_parts.len() {
            return None;
        }

        // ensure no trailing characters remain
        let trailing = &source_parts[source_index][source_offset..];
        if !trailing.is_empty() {
            return None;
        }

        Some(segments)
    }

    /// Check whether a template literal segment fits within a span type.
    fn template_segment_matches_span_type(
        &self,
        ctx: &mut TypeContext<'_>,
        span_ty_id: LocalTypeId,
        segment: &TemplateSpanSegment,
    ) -> bool {
        // allow infer variables
        if let Type::InferVar { .. } = ctx.types.get_type(span_ty_id) {
            return true;
        }

        // match literal only segments against the span type
        if let Some(literal) = segment.literal_string() {
            let mut visited = HashSet::new();
            return self.template_span_matches_string(
                &mut ctx.reborrow(),
                span_ty_id,
                &literal,
                &mut visited,
            );
        }

        // allow a single span segment when assignable
        if segment.literals.is_empty() && segment.spans.len() == 1 {
            // allow string spans to accept any stringifiable template span
            if self.span_is_string_compatible_for_template(&mut ctx.reborrow(), span_ty_id) {
                return true;
            }

            return self.is_type_assignable(&mut ctx.reborrow(), span_ty_id, segment.spans[0])
                == Assignability::Assignable;
        }

        // require string like span types for mixed segments
        matches!(
            ctx.types.get_type(span_ty_id),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            } | Type::TypeLiteral {
                value: TypeLiteral::Any,
            }
        )
    }

    /// Check whether a span type can accept any string and a stringifiable template span.
    fn span_is_string_compatible_for_template(
        &self,
        ctx: &mut TypeContext<'_>,
        span_ty_id: LocalTypeId,
    ) -> bool {
        let mut visited = HashSet::new();
        self.span_is_string_supertype(&mut ctx.reborrow(), span_ty_id, &mut visited)
            && self.span_is_template_stringifiable(&mut ctx.reborrow(), span_ty_id, &mut visited)
    }

    /// Resolve the span type used for template matching.
    fn resolve_template_span_match_type(
        &self,
        ctx: &mut TypeContext<'_>,
        span_ty_id: LocalTypeId,
        source_id: LocalNodeIdAny,
    ) -> TemplateSpanMatch {
        // resolve static parameter constraints when available
        if let Type::Reference { symbol, .. } = ctx.types.get_type(span_ty_id)
            && self.symbol_is_static_parameter(ctx.symbol_type_view(), *symbol)
        {
            let symbol = *symbol;
            let mut ctx = ctx.reborrow();
            let constraint_id = self.static_parameter_constraint_type(&mut ctx, symbol, source_id);
            let Some(constraint_id) = constraint_id else {
                return TemplateSpanMatch::Any;
            };
            if matches!(
                ctx.types.get_type(constraint_id),
                Type::TypeLiteral {
                    value: TypeLiteral::Unknown
                }
            ) {
                return TemplateSpanMatch::Any;
            }

            return TemplateSpanMatch::Type(constraint_id);
        }

        TemplateSpanMatch::Type(span_ty_id)
    }

    /// Check whether a span type can appear inside a template literal.
    fn span_is_template_stringifiable(
        &self,
        ctx: &mut TypeContext<'_>,
        span_ty_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve the span type for matching
        let source_id = ctx.types.get_type_source(span_ty_id);
        let span_match =
            self.resolve_template_span_match_type(&mut ctx.reborrow(), span_ty_id, source_id);

        let span_ty_id = match span_match {
            TemplateSpanMatch::Any => return true,
            TemplateSpanMatch::Type(span_ty_id) => span_ty_id,
        };

        // break recursion cycles
        if !visited.insert(span_ty_id) {
            return false;
        }

        // normalize the span type
        let normalized_id =
            self.normalize_type(&mut ctx.reborrow(), span_ty_id, NormalizationMode::Flow);
        let span_ty = ctx.types.get_type(normalized_id).clone();

        // check template string compatibility
        let stringifiable = match span_ty {
            Type::TypeLiteral { value } => match value {
                TypeLiteral::Any => true,
                TypeLiteral::Primitive(PrimitiveType::String) => true,
                TypeLiteral::Primitive(PrimitiveType::Number) => true,
                TypeLiteral::Primitive(PrimitiveType::Bigint) => true,
                TypeLiteral::Primitive(PrimitiveType::Boolean) => true,
                TypeLiteral::Primitive(PrimitiveType::Int(_)) => true,
                TypeLiteral::Primitive(PrimitiveType::Float(_)) => true,
                TypeLiteral::Null => true,
                TypeLiteral::Undefined => true,
                TypeLiteral::ScalarLiteral(ScalarLiteral::RegexString { .. }) => false,
                TypeLiteral::ScalarLiteral(_) => true,
                _ => false,
            },
            Type::TemplateLiteral { .. } => true,
            Type::Union { elements } => elements.iter().all(|element_id| {
                self.span_is_template_stringifiable(&mut ctx.reborrow(), *element_id, visited)
            }),
            Type::Intersection { elements } => elements.iter().all(|element_id| {
                self.span_is_template_stringifiable(&mut ctx.reborrow(), *element_id, visited)
            }),
            _ if span_ty.is_infer() => true,
            _ => false,
        };

        // release the recursion guard
        visited.remove(&span_ty_id);
        stringifiable
    }

    /// Check whether a span type can match a string literal.
    pub(crate) fn template_span_matches_string(
        &self,
        ctx: &mut TypeContext<'_>,
        span_ty_id: LocalTypeId,
        value: &str,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve the span type for matching
        let source_id = ctx.types.get_type_source(span_ty_id);
        let span_match =
            self.resolve_template_span_match_type(&mut ctx.reborrow(), span_ty_id, source_id);

        let span_ty_id = match span_match {
            TemplateSpanMatch::Any => return true,
            TemplateSpanMatch::Type(span_ty_id) => span_ty_id,
        };

        // break recursion cycles
        if !visited.insert(span_ty_id) {
            return false;
        }

        // normalize the span type
        let normalized_id =
            self.normalize_type(&mut ctx.reborrow(), span_ty_id, NormalizationMode::Flow);
        let span_ty = ctx.types.get_type(normalized_id).clone();

        // evaluate matching rules
        let matches = match span_ty {
            Type::TypeLiteral { value: literal } => {
                self.literal_type_matches_string(&literal, value)
            }
            Type::TemplateLiteral { strings, spans } => {
                self.template_literal_matches_string(&mut ctx.reborrow(), &strings, &spans, value)
            }
            Type::Union { elements } => elements.iter().any(|element_id| {
                self.template_span_matches_string(&mut ctx.reborrow(), *element_id, value, visited)
            }),
            Type::Intersection { elements } => elements.iter().all(|element_id| {
                self.template_span_matches_string(&mut ctx.reborrow(), *element_id, value, visited)
            }),
            _ if span_ty.is_infer() => true,
            _ => false,
        };

        // release the recursion guard
        visited.remove(&span_ty_id);
        matches
    }

    /// Map a literal string into a scalar type for template inference.
    pub(crate) fn template_infer_literal_type(
        &self,
        constraint_ty_id: Option<LocalTypeId>,
        value: &str,
        source_id: LocalNodeIdAny,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        // resolve the constraint type
        let target = constraint_ty_id.map(|constraint| types.get_type(constraint).clone());

        // select the inference strategy
        match target {
            Some(Type::TypeLiteral {
                value: TypeLiteral::Null,
            }) => {
                if value == "null" {
                    return Some(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Null,
                        },
                        source_id,
                    ));
                }

                return None;
            }
            Some(Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            }) => {
                if value == "undefined" {
                    return Some(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Undefined,
                        },
                        source_id,
                    ));
                }

                return None;
            }
            _ => {}
        }

        let scalar = match target {
            Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Number),
            }) => {
                if let Some(literal) = self.number_literal_from_string(value) {
                    let type_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        source_id,
                    );
                    self.set_type_freshness(types, type_id, Freshness::Regular);
                    return Some(type_id);
                }

                if self.string_is_number_literal(value) {
                    return Some(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Number),
                        },
                        source_id,
                    ));
                }

                return None;
            }
            Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Float(float_type)),
            }) => {
                if let Some(literal) = self.number_literal_from_string(value) {
                    let literal = match literal {
                        ScalarLiteral::Integer(value) => ScalarLiteral::Float(value as f64),
                        other => other,
                    };

                    let type_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        source_id,
                    );
                    self.set_type_freshness(types, type_id, Freshness::Regular);
                    return Some(type_id);
                }

                if self.string_is_number_literal(value) {
                    return Some(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Float(float_type)),
                        },
                        source_id,
                    ));
                }

                return None;
            }
            Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Bigint),
            }) => {
                // prefer literal inference when possible
                if let Some(literal) = self.bigint_literal_from_string(value) {
                    let type_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        source_id,
                    );
                    self.set_type_freshness(types, type_id, Freshness::Regular);
                    return Some(type_id);
                }

                // fall back to primitive bigint for non canonical strings
                if self.string_is_bigint_literal(value) {
                    return Some(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Bigint),
                        },
                        source_id,
                    ));
                }

                return None;
            }
            Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            }) => self.boolean_literal_from_string(value),
            Some(Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
            }) => {
                // prefer literal inference when possible
                if let Some(literal) = self.int_literal_from_string(value, &int_type) {
                    let type_id = types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::ScalarLiteral(literal),
                        },
                        source_id,
                    );
                    self.set_type_freshness(types, type_id, Freshness::Regular);
                    return Some(type_id);
                }

                // fall back to primitive int for non canonical strings
                if self.string_is_int_literal(value, &int_type) {
                    return Some(types.insert_type_from_any(
                        Type::TypeLiteral {
                            value: TypeLiteral::Primitive(PrimitiveType::Int(int_type)),
                        },
                        source_id,
                    ));
                }

                return None;
            }
            _ => Some(ScalarLiteral::String(self.repository.strings.intern(value))),
        }?;

        let type_id = types.insert_type_from_any(
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(scalar),
            },
            source_id,
        );
        self.set_type_freshness(types, type_id, Freshness::Regular);
        Some(type_id)
    }

    /// Check whether a type literal matches a string literal value.
    fn literal_type_matches_string(&self, literal: &TypeLiteral, value: &str) -> bool {
        // match scalar literal types
        if let TypeLiteral::ScalarLiteral(scalar) = literal {
            return self.scalar_literal_matches_string(scalar, value);
        }

        // match primitive and keyword literals
        match literal {
            TypeLiteral::Any => true,
            TypeLiteral::Primitive(PrimitiveType::String) => true,
            TypeLiteral::Primitive(PrimitiveType::Number) => self.string_is_number_literal(value),
            TypeLiteral::Primitive(PrimitiveType::Bigint) => self.string_is_bigint_literal(value),
            TypeLiteral::Primitive(PrimitiveType::Boolean) => {
                matches!(value, "true" | "false")
            }
            TypeLiteral::Primitive(PrimitiveType::Int(int_type)) => {
                self.string_is_int_literal(value, int_type)
            }
            TypeLiteral::Primitive(PrimitiveType::Float(_)) => self.string_is_number_literal(value),
            TypeLiteral::Null => value == "null",
            TypeLiteral::Undefined => value == "undefined",
            _ => false,
        }
    }

    /// Check whether a scalar literal matches a string literal value.
    fn scalar_literal_matches_string(&self, scalar: &ScalarLiteral, value: &str) -> bool {
        // match the scalar literal kind
        match scalar {
            ScalarLiteral::Null => value == "null",
            ScalarLiteral::String(string_id) => {
                self.repository.strings.get(*string_id).as_ref() == value
            }
            ScalarLiteral::Boolean(value_boolean) => {
                matches!((value, *value_boolean), ("true", true) | ("false", false))
            }
            ScalarLiteral::Integer(value_int) => {
                self.string_matches_integer_literal(value, *value_int)
            }
            ScalarLiteral::Bigint(value_bigint) => {
                self.string_matches_bigint_literal(value, *value_bigint)
            }
            ScalarLiteral::Float(value_float) => {
                self.string_matches_float_literal(value, *value_float)
            }
            ScalarLiteral::Character(character) => {
                // match single character strings
                let mut chars = value.chars();
                let Some(first) = chars.next() else {
                    return false;
                };
                first == *character && chars.next().is_none()
            }
            ScalarLiteral::RegexString { .. } => {
                // regex literals are not string literals
                false
            }
        }
    }

    /// Check whether a string literal matches an integer literal value.
    fn string_matches_integer_literal(&self, value: &str, expected: i64) -> bool {
        // parse the numeric literal
        let Some(parsed) = self.number_literal_from_string(value) else {
            return false;
        };

        // require an integer literal match
        matches!(parsed, ScalarLiteral::Integer(value) if value == expected)
    }

    /// Check whether a string literal matches a bigint literal value.
    fn string_matches_bigint_literal(&self, value: &str, expected: i64) -> bool {
        // parse the bigint literal
        let Some(parsed) = self.bigint_literal_from_string(value) else {
            return false;
        };

        // require a bigint literal match
        matches!(parsed, ScalarLiteral::Bigint(value) if value == expected)
    }

    /// Check whether a string literal matches a float literal value.
    fn string_matches_float_literal(&self, value: &str, expected: f64) -> bool {
        // parse the numeric literal
        let Some(parsed) = self.number_literal_from_string(value) else {
            return false;
        };

        // require a float literal match
        matches!(parsed, ScalarLiteral::Float(value) if value == expected)
    }

    /// Parse a numeric string into its parts.
    fn parse_numeric_string_parts<'a>(&self, value: &'a str) -> Option<NumericStringParts<'a>> {
        // trim whitespace
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }

        // track trimmed whitespace
        let has_whitespace = trimmed.len() != value.len();

        // split optional sign
        let (has_sign, is_negative, rest) = if let Some(rest) = trimmed.strip_prefix('+') {
            (true, false, rest)
        } else if let Some(rest) = trimmed.strip_prefix('-') {
            (true, true, rest)
        } else {
            (false, false, trimmed)
        };

        // detect radix prefixes
        let (radix, digits) = if rest.starts_with("0x") || rest.starts_with("0X") {
            (Some(16), &rest[2..])
        } else if rest.starts_with("0b") || rest.starts_with("0B") {
            (Some(2), &rest[2..])
        } else if rest.starts_with("0o") || rest.starts_with("0O") {
            (Some(8), &rest[2..])
        } else {
            (None, rest)
        };

        // require at least one digit
        if digits.is_empty() {
            return None;
        }

        Some(NumericStringParts {
            trimmed,
            has_whitespace,
            is_negative,
            has_sign,
            radix,
            digits,
        })
    }

    /// Check whether a string literal can be interpreted as a number.
    fn string_is_number_literal(&self, value: &str) -> bool {
        // parse numeric value
        self.parse_number_string_value(value).is_some()
    }

    /// Check whether a string literal is a valid bigint literal.
    fn string_is_bigint_literal(&self, value: &str) -> bool {
        // parse literal parts
        let Some(parts) = self.parse_numeric_string_parts(value) else {
            return false;
        };

        // reject whitespace for bigint forms
        if parts.has_whitespace {
            return false;
        }

        // reject explicit plus sign
        if parts.has_sign && !parts.is_negative {
            return false;
        }

        // handle non decimal radix
        if let Some(radix) = parts.radix {
            return self.radix_digits_valid(parts.digits, radix);
        }

        // require decimal digits
        if !parts.digits.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }

        // reject decimal leading zeros
        if parts.digits.len() > 1 && parts.digits.starts_with('0') {
            return false;
        }

        true
    }

    /// Check whether a string literal can be parsed as an integer of the given type.
    fn string_is_int_literal(&self, value: &str, int_type: &IntType) -> bool {
        // parse integer value
        self.parse_int_string_value(value, int_type).is_some()
    }

    /// Parse a string literal into a number literal.
    fn number_literal_from_string(&self, value: &str) -> Option<ScalarLiteral> {
        // parse canonical numeric representation
        let parsed = self.parse_canonical_number_string(value)?;

        // emit integer literals when in range
        if parsed.fract() == 0.0 && parsed >= i64::MIN as f64 && parsed <= i64::MAX as f64 {
            return Some(ScalarLiteral::Integer(parsed as i64));
        }

        // fall back to float literal
        Some(ScalarLiteral::Float(parsed))
    }

    /// Parse a string literal into an int literal for a specific int type.
    fn int_literal_from_string(&self, value: &str, int_type: &IntType) -> Option<ScalarLiteral> {
        // parse canonical numeric representation
        let parsed = self.parse_canonical_number_string(value)?;

        // require integer values
        if parsed.fract() != 0.0 {
            return None;
        }

        // reject values outside i128
        if parsed < i128::MIN as f64 || parsed > i128::MAX as f64 {
            return None;
        }

        // enforce integer width constraints
        let parsed = parsed as i128;
        if !self.is_integer_literal_assignable(parsed, int_type) {
            return None;
        }

        // downcast for storage
        let parsed = i64::try_from(parsed).ok()?;
        Some(ScalarLiteral::Integer(parsed))
    }

    /// Parse a string literal into a bigint literal.
    fn bigint_literal_from_string(&self, value: &str) -> Option<ScalarLiteral> {
        // parse literal parts
        let parts = self.parse_numeric_string_parts(value)?;

        // reject non canonical whitespace
        if parts.has_whitespace {
            return None;
        }

        // reject explicit plus sign
        if parts.has_sign && !parts.is_negative {
            return None;
        }

        // reject non decimal forms for literal inference
        if parts.radix.is_some() {
            return None;
        }

        // require decimal digits
        if !parts.digits.chars().all(|ch| ch.is_ascii_digit()) {
            return None;
        }

        // reject leading zeros
        if parts.digits.len() > 1 && parts.digits.starts_with('0') {
            return None;
        }

        // reject negative zero
        if parts.is_negative && parts.digits == "0" {
            return None;
        }

        // parse and apply sign
        let parsed = parts.digits.parse::<i128>().ok()?;
        let parsed = if parts.is_negative { -parsed } else { parsed };
        let parsed = i64::try_from(parsed).ok()?;
        Some(ScalarLiteral::Bigint(parsed))
    }

    /// Parse a numeric string into a f64 when possible.
    fn parse_number_string_value(&self, value: &str) -> Option<f64> {
        // parse literal parts
        let parts = self.parse_numeric_string_parts(value)?;

        // reject whitespace
        if parts.has_whitespace {
            return None;
        }

        // reject NaN and Infinity text
        if matches!(parts.trimmed, "NaN" | "Infinity" | "-Infinity") {
            return None;
        }

        // handle non decimal radix
        if let Some(radix) = parts.radix {
            // reject explicit sign for radix literals
            if parts.has_sign {
                return None;
            }

            return self.parse_radix_digits_to_f64(parts.digits, radix);
        }

        // parse decimal form
        let parsed = parts.trimmed.parse::<f64>().ok()?;
        if parsed.is_finite() {
            return Some(parsed);
        }

        // allow exponent overflow numeric forms (matching JS literal grammar)
        if parsed.is_infinite() && (parts.trimmed.contains('e') || parts.trimmed.contains('E')) {
            return Some(parsed);
        }

        None
    }

    /// Parse a numeric string into an integer if it satisfies the given int type.
    fn parse_int_string_value(&self, value: &str, int_type: &IntType) -> Option<i128> {
        // parse numeric value
        let parsed = self.parse_number_string_value(value)?;

        // require integer values
        if parsed.fract() != 0.0 {
            return None;
        }

        // ensure value fits in i128
        if parsed < i128::MIN as f64 || parsed > i128::MAX as f64 {
            return None;
        }

        // enforce integer width constraints
        let parsed = parsed as i128;
        if !self.is_integer_literal_assignable(parsed, int_type) {
            return None;
        }

        Some(parsed)
    }

    /// Parse a canonical number literal from a string.
    fn parse_canonical_number_string(&self, value: &str) -> Option<f64> {
        // parse literal parts
        let parts = self.parse_numeric_string_parts(value)?;

        // reject non canonical whitespace
        if parts.has_whitespace {
            return None;
        }

        // parse numeric value
        let parsed = self.parse_number_string_value(parts.trimmed)?;
        let canonical = self.number_to_js_string(parsed);

        // require canonical string form
        if canonical == parts.trimmed {
            return Some(parsed);
        }

        None
    }

    /// Convert a number into its JS string representation.
    fn number_to_js_string(&self, value: f64) -> String {
        // handle NaN
        if value.is_nan() {
            return "NaN".to_string();
        }

        // handle infinity
        if value.is_infinite() {
            return if value.is_sign_positive() {
                "Infinity".to_string()
            } else {
                "-Infinity".to_string()
            };
        }

        // normalize zero
        if value == 0.0 {
            return "0".to_string();
        }

        // use exponential formatting for extreme values
        let abs = value.abs();
        if !(1e-6..1e21).contains(&abs) {
            let mut formatted = format!("{value:e}");
            if let Some(exponent_index) = formatted.find('e') {
                let exponent = &formatted[(exponent_index + 1)..];
                if !exponent.starts_with('-') && !exponent.starts_with('+') {
                    // add explicit plus for positive exponents
                    formatted.insert(exponent_index + 1, '+');
                }
            }
            return formatted;
        }

        // use decimal formatting within the normal range
        value.to_string()
    }

    /// Parse radix digits into a f64 value.
    fn parse_radix_digits_to_f64(&self, digits: &str, radix: u32) -> Option<f64> {
        // validate digit characters
        if !self.radix_digits_valid(digits, radix) {
            return None;
        }

        // accumulate numeric value
        let mut value = 0f64;
        for ch in digits.chars() {
            let digit = ch.to_digit(radix)? as f64;
            value = value * f64::from(radix) + digit;

            // reject overflowing radix values
            if !value.is_finite() {
                return None;
            }
        }

        Some(value)
    }

    /// Validate that radix digits are valid for a given base.
    fn radix_digits_valid(&self, digits: &str, radix: u32) -> bool {
        // require at least one digit
        if digits.is_empty() {
            return false;
        }

        digits.chars().all(|ch| ch.is_digit(radix))
    }

    /// Check whether a template literal type accepts all strings.
    pub(crate) fn template_literal_is_string_supertype(
        &self,
        ctx: &mut TypeContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
    ) -> bool {
        // require aligned literal and span counts
        if strings.len() != spans.len() + 1 {
            return false;
        }

        // ensure all literal parts are empty
        let mut has_span = false;
        for string_id in strings {
            if !self.repository.strings.get(*string_id).is_empty() {
                return false;
            }
        }

        // verify every span accepts all strings
        let mut visited = HashSet::new();
        for span in spans {
            if !self.span_is_string_supertype(&mut ctx.reborrow(), *span, &mut visited) {
                return false;
            }
            has_span = true;
        }

        has_span
    }

    /// Resolve the key shape for a template literal type.
    pub(super) fn template_literal_key_shape(
        &self,
        ctx: &mut TypeContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        relation_mode: RelationMode,
    ) -> Option<TemplateLiteralKeyShape> {
        // ensure the template literal layout is valid
        if strings.len() != spans.len() + 1 {
            return None;
        }

        // template literals that accept all strings become string index keys
        if self.template_literal_is_string_supertype(&mut ctx.reborrow(), strings, spans) {
            return Some(TemplateLiteralKeyShape::StringIndex);
        }

        // expand spans into literal string values when possible
        let mut span_values = Vec::new();
        let mut visited = HashSet::new();
        for span in spans {
            let Some(values) = self.template_literal_span_literal_values(
                &mut ctx.reborrow(),
                *span,
                relation_mode,
                &mut visited,
            ) else {
                return Some(TemplateLiteralKeyShape::StringIndex);
            };
            span_values.push(values);
        }

        // build all literal string combinations
        let mut combinations = vec![self.repository.strings.get(strings[0]).to_string()];
        for (index, values) in span_values.iter().enumerate() {
            let suffix = self.repository.strings.get(strings[index + 1]);
            let mut next = Vec::new();
            for base in combinations.iter() {
                for value in values.iter() {
                    let mut combined = String::new();
                    combined.push_str(base);
                    combined.push_str(value);
                    combined.push_str(suffix.as_ref());
                    next.push(combined);
                    if next.len() > TEMPLATE_LITERAL_KEY_EXPANSION_LIMIT {
                        return Some(TemplateLiteralKeyShape::StringIndex);
                    }
                }
            }
            combinations = next;
        }

        // intern literal keys
        let mut literal_keys = Vec::new();
        for value in combinations {
            let key_id = self.repository.strings.intern(&value);
            literal_keys.push(StaticKey::Name(key_id));
        }

        Some(TemplateLiteralKeyShape::Literal(literal_keys))
    }

    /// Collect literal string values for a template literal span type.
    fn template_literal_span_literal_values(
        &self,
        ctx: &mut TypeContext<'_>,
        span_type_id: LocalTypeId,
        relation_mode: RelationMode,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<Vec<String>> {
        // resolve static parameter constraints when available
        let source_id = ctx.types.get_type_source(span_type_id);
        let span_type_id = match self.resolve_template_span_match_type(
            &mut ctx.reborrow(),
            span_type_id,
            source_id,
        ) {
            TemplateSpanMatch::Any => return None,
            TemplateSpanMatch::Type(span_type_id) => span_type_id,
        };

        // normalize span types before extracting literals
        let mut normalize_visited = Vec::new();
        let normalized_id = self.normalize_type_inner(
            &mut ctx.reborrow(),
            span_type_id,
            NormalizationMode::Assign,
            relation_mode,
            &mut normalize_visited,
        );
        if normalized_id != span_type_id {
            return self.template_literal_span_literal_values(
                &mut ctx.reborrow(),
                normalized_id,
                relation_mode,
                visited,
            );
        }

        // avoid recursion cycles
        if !visited.insert(span_type_id) {
            return None;
        }

        // collect literal values from the normalized span type
        let result = match ctx.types.get_type(span_type_id).clone() {
            Type::TypeLiteral {
                value: TypeLiteral::ScalarLiteral(literal),
            } => {
                let value = match literal {
                    ScalarLiteral::Null => "null".to_string(),
                    ScalarLiteral::String(name) => self.repository.strings.get(name).to_string(),
                    ScalarLiteral::Integer(value) => value.to_string(),
                    ScalarLiteral::Bigint(value) => value.to_string(),
                    ScalarLiteral::Float(value) => value.to_string(),
                    ScalarLiteral::Boolean(value) => value.to_string(),
                    ScalarLiteral::Character(value) => value.to_string(),
                    ScalarLiteral::RegexString { .. } => return None,
                };
                Some(vec![value])
            }
            Type::TypeLiteral {
                value: TypeLiteral::Null,
            } => Some(vec!["null".to_string()]),
            Type::TypeLiteral {
                value: TypeLiteral::Undefined,
            } => Some(vec!["undefined".to_string()]),
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::Boolean),
            } => Some(vec!["true".to_string(), "false".to_string()]),
            Type::Union { elements } => {
                let mut values = Vec::new();
                for element in elements {
                    let element_values = self.template_literal_span_literal_values(
                        &mut ctx.reborrow(),
                        element,
                        relation_mode,
                        visited,
                    )?;
                    values.extend(element_values);
                }
                Some(values)
            }
            Type::TemplateLiteral { strings, spans } => self.template_literal_string_values(
                &mut ctx.reborrow(),
                &strings,
                &spans,
                relation_mode,
                visited,
            ),
            _ => None,
        };

        visited.remove(&span_type_id);

        result
    }

    /// Expand template literal types into literal string values when possible.
    fn template_literal_string_values(
        &self,
        ctx: &mut TypeContext<'_>,
        strings: &[StringId],
        spans: &[LocalTypeId],
        relation_mode: RelationMode,
        visited: &mut HashSet<LocalTypeId>,
    ) -> Option<Vec<String>> {
        // validate template structure
        if strings.len() != spans.len() + 1 {
            return None;
        }

        // collect values for each span
        let mut span_values = Vec::new();
        for span in spans {
            let values = self.template_literal_span_literal_values(
                &mut ctx.reborrow(),
                *span,
                relation_mode,
                visited,
            )?;
            span_values.push(values);
        }

        // build all string combinations
        let mut combinations = vec![self.repository.strings.get(strings[0]).to_string()];
        for (index, values) in span_values.iter().enumerate() {
            let suffix = self.repository.strings.get(strings[index + 1]);
            let mut next = Vec::new();
            for base in combinations.iter() {
                for value in values.iter() {
                    let mut combined = String::new();
                    combined.push_str(base);
                    combined.push_str(value);
                    combined.push_str(suffix.as_ref());
                    next.push(combined);
                    if next.len() > TEMPLATE_LITERAL_KEY_EXPANSION_LIMIT {
                        return None;
                    }
                }
            }
            combinations = next;
        }

        Some(combinations)
    }

    /// Check whether a span type can accept any string.
    fn span_is_string_supertype(
        &self,
        ctx: &mut TypeContext<'_>,
        span_ty_id: LocalTypeId,
        visited: &mut HashSet<LocalTypeId>,
    ) -> bool {
        // resolve the span type for matching
        let source_id = ctx.types.get_type_source(span_ty_id);
        let span_match =
            self.resolve_template_span_match_type(&mut ctx.reborrow(), span_ty_id, source_id);

        let span_ty_id = match span_match {
            TemplateSpanMatch::Any => return true,
            TemplateSpanMatch::Type(span_ty_id) => span_ty_id,
        };

        // break recursion cycles
        if !visited.insert(span_ty_id) {
            return false;
        }

        // normalize the span type
        let normalized_id =
            self.normalize_type(&mut ctx.reborrow(), span_ty_id, NormalizationMode::Flow);
        let span_ty = ctx.types.get_type(normalized_id).clone();

        // check for string like spans
        let accepts_all = match span_ty {
            Type::TypeLiteral {
                value: TypeLiteral::Primitive(PrimitiveType::String),
            }
            | Type::TypeLiteral {
                value: TypeLiteral::Any,
            } => true,
            Type::TemplateLiteral { strings, spans } => {
                self.template_literal_is_string_supertype(&mut ctx.reborrow(), &strings, &spans)
            }
            Type::Union { elements } => elements.iter().any(|element_id| {
                self.span_is_string_supertype(&mut ctx.reborrow(), *element_id, visited)
            }),
            _ if span_ty.is_infer() => true,
            _ => false,
        };

        // release the recursion guard
        visited.remove(&span_ty_id);
        accepts_all
    }

    /// Parse a string literal into a boolean literal.
    fn boolean_literal_from_string(&self, value: &str) -> Option<ScalarLiteral> {
        // match boolean literal text
        match value {
            "true" => Some(ScalarLiteral::Boolean(true)),
            "false" => Some(ScalarLiteral::Boolean(false)),
            _ => None,
        }
    }
}
