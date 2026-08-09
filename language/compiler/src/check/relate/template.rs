use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckState, Origin, Relation};

/// One numeric template capture attempt under a constraint head.
enum NumericCapture {
    /// The constraint is not numeric.
    NotNumeric,
    /// The text falls outside the numeric domain.
    OutOfDomain,
    /// The captured numeric literal.
    Captured(dir::GlobalTypeId),
}

impl CheckState<'_> {
    /// Reduce one span head, keeping open roots as leaves.
    fn template_span_head(
        &mut self,
        origin: Origin,
        span: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let span = self.shallow_resolve(span)?;
        if self.root_variable(span)?.is_some() {
            return Ok(span);
        }

        self.reduce_type_head(origin, span)
    }

    /// Return one template's literal segments as owned text.
    fn template_segments(
        &self,
        module: ModuleId,
        strings: dir::TypeListId,
    ) -> CompilerResult<Vec<String>> {
        Ok(self
            .template_strings(module, strings)?
            .iter()
            .map(|segment| self.strings().get(*segment).to_string())
            .collect())
    }

    /// Decide whether one string literal inhabits a template literal pattern.
    pub(in crate::check) fn decide_template_string(
        &mut self,
        origin: Origin,
        text: &str,
        template_module: ModuleId,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<bool> {
        let segments = self.template_segments(template_module, template.strings)?;
        let spans = self.type_ids(template_module, template.spans)?.to_vec();

        // reduce span heads once so alternatives compare structurally
        let mut heads = Vec::with_capacity(spans.len());
        for span in spans {
            heads.push(self.template_span_head(origin, span)?);
        }

        self.match_template_segments(origin, text, &segments, &heads)
    }

    /// Decide whether the whole string domain inhabits one template pattern.
    pub(in crate::check) fn decide_string_inhabits_template(
        &mut self,
        origin: Origin,
        template_module: ModuleId,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<bool> {
        // only fully unconstraining patterns absorb every string
        for segment in self.template_strings(template_module, template.strings)? {
            if !self.strings().get(*segment).is_empty() {
                return Ok(false);
            }
        }
        let spans = self.type_ids(template_module, template.spans)?.to_vec();
        if spans.is_empty() {
            return Ok(false);
        }
        for span in spans {
            let span = self.reduce_type_head(origin, span)?;
            if !matches!(
                self.ty(span)?,
                dir::Type::Primitive(dir::PrimitiveType::String)
            ) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Match one text against interleaved literal segments and span patterns.
    fn match_template_segments(
        &mut self,
        origin: Origin,
        text: &str,
        segments: &[String],
        spans: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        // anchor the match on the leading segment
        let Some(first) = segments.first() else {
            return Ok(text.is_empty());
        };
        let Some(rest) = text.strip_prefix(first.as_str()) else {
            return Ok(false);
        };
        if spans.is_empty() {
            return Ok(segments.len() == 1 && rest.is_empty());
        }

        // try every split point for the first span, shortest capture first
        let span = spans[0];
        let tail_segments = &segments[1..];
        let tail_spans = &spans[1..];
        for split in 0..=rest.len() {
            if !rest.is_char_boundary(split) {
                continue;
            }
            let (candidate, remaining) = rest.split_at(split);
            let matched = self.match_template_span(origin, candidate, span)?;
            if !matched {
                continue;
            }
            if self.match_template_segments(origin, remaining, tail_segments, tail_spans)? {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Return the bound type captured for one open template span, or none
    /// when the text falls outside the span's numeric constraint.
    pub(in crate::check) fn template_capture_bound(
        &mut self,
        origin: Origin,
        span: dir::GlobalTypeId,
        text: &str,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        // numeric parameter constraints capture numeric literals only;
        //  aliasing can root the span under a plain hole, so the span's
        //  own variable answers before its component root
        let immediate = match self.ty(span)? {
            dir::Type::Variable(variable) => Some(variable),
            _ => None,
        };
        let root = self.root_variable(self.shallow_resolve(span)?)?;
        let mut hint = None;
        for variable in [immediate, root].into_iter().flatten() {
            hint = self
                .infer
                .variable_role(variable)?
                .parameter()
                .and_then(|parameter| self.generic_parameter(parameter))
                .and_then(|binding| binding.constraint);
            if hint.is_some() {
                break;
            }
        }
        let module = origin.module();
        if let Some(hint) = hint
            && let Ok(head) = self.reduce_type_head(origin, hint)
            && let Ok(kind) = self.ty(head)
        {
            match self.numeric_template_capture(module, &kind, text)? {
                NumericCapture::Captured(literal) => return Ok(Some(literal)),
                NumericCapture::OutOfDomain => return Ok(None),
                NumericCapture::NotNumeric => {}
            }
        }

        let text = self.strings().intern(text);
        let literal = self.intern_type(dir::Type::Literal(dir::ScalarLiteral::String(text)))?;

        Ok(Some(literal))
    }

    /// Return the literal type captured by one template span.
    pub(in crate::check) fn template_captured_type(
        &mut self,
        origin: Origin,
        module: ModuleId,
        span: dir::GlobalTypeId,
        text: &str,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // numeric constraints capture numeric literals
        let constraint = match self.operation_head(span)? {
            Some(dir::TypeOperation::Infer(infer)) => infer.constraint,
            _ => None,
        };
        if let Some(constraint) = constraint
            && let Ok(head) = self.reduce_type_head(origin, constraint)
            && let Ok(kind) = self.ty(head)
            && let NumericCapture::Captured(literal) =
                self.numeric_template_capture(module, &kind, text)?
        {
            return Ok(literal);
        }

        let text = self.strings().intern(text);

        self.intern_type(dir::Type::Literal(dir::ScalarLiteral::String(text)))
    }

    /// Capture one numeric span text under a reduced constraint head.
    fn numeric_template_capture(
        &mut self,
        _module: ModuleId,
        constraint: &dir::Type,
        text: &str,
    ) -> CompilerResult<NumericCapture> {
        let text = TemplateText(text);
        let literal = match constraint {
            dir::Type::Primitive(dir::PrimitiveType::Float(_)) => {
                let Some(value) = text.number() else {
                    return Ok(NumericCapture::OutOfDomain);
                };

                // integral captures stay comptime integers and adapt
                match text.integer() {
                    Some(value) => dir::ScalarLiteral::Integer(value),
                    None => dir::ScalarLiteral::Float(value),
                }
            }
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => {
                let Some(value) = text.integer() else {
                    return Ok(NumericCapture::OutOfDomain);
                };
                if !integer.fits_literal(value) {
                    return Ok(NumericCapture::OutOfDomain);
                }

                dir::ScalarLiteral::Integer(value)
            }
            dir::Type::Primitive(dir::PrimitiveType::Bigint) => {
                let Some(value) = text.integer() else {
                    return Ok(NumericCapture::OutOfDomain);
                };

                dir::ScalarLiteral::Bigint(value)
            }
            dir::Type::Range(range) => {
                let Some(literal) = text.range(range) else {
                    return Ok(NumericCapture::OutOfDomain);
                };

                literal
            }
            _ => return Ok(NumericCapture::NotNumeric),
        };
        let captured = self.intern_type(dir::Type::Literal(literal))?;

        Ok(NumericCapture::Captured(captured))
    }

    /// Match one captured text against a single span pattern.
    pub(in crate::check) fn match_template_span(
        &mut self,
        origin: Origin,
        text: &str,
        span: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let span = self.template_span_head(origin, span)?;
        let template = TemplateText(text);

        let matched = match self.ty(span)? {
            dir::Type::Primitive(dir::PrimitiveType::String) => true,
            dir::Type::Primitive(dir::PrimitiveType::Float(_)) => template.number().is_some(),
            dir::Type::Primitive(dir::PrimitiveType::Integer(integer)) => {
                template.fits_integer(integer)
            }
            dir::Type::Primitive(dir::PrimitiveType::Bigint) => template.integer().is_some(),
            dir::Type::Primitive(dir::PrimitiveType::Boolean) => text == "true" || text == "false",
            dir::Type::Range(range) => template.range(&range).is_some(),
            // numeric literal spans match every spelling of their value
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                template.integer() == Some(value) || template.number() == Some(value as f64)
            }
            dir::Type::Literal(dir::ScalarLiteral::Float(value)) => {
                template.number() == Some(value)
            }
            dir::Type::Literal(dir::ScalarLiteral::Bigint(value)) => {
                template.integer() == Some(value)
            }
            dir::Type::Literal(literal) => {
                literal.template_text(self.strings()).as_deref() == Some(text)
            }
            dir::Type::Null => text == "null",
            dir::Type::Undefined => text == "undefined",
            dir::Type::Union(union) => {
                let elements = self.type_ids(span.module_id, union.elements)?.to_vec();
                let mut matched = false;
                for element in elements {
                    if self.match_template_span(origin, text, element)? {
                        matched = true;

                        break;
                    }
                }

                matched
            }
            dir::Type::Operation(operation)
                if let dir::TypeOperation::TemplateLiteral(nested) =
                    self.type_operation(span.module_id, operation)? =>
            {
                self.decide_template_string(origin, text, span.module_id, &nested)?
            }
            _ => false,
        };

        Ok(matched)
    }
}

/// One symbolic chunk of a template literal source.
#[derive(Debug, Clone)]
enum TemplatePiece {
    /// Known literal text.
    Text(String),
    /// One interpolated span standing for its whole domain.
    Span(dir::GlobalTypeId),
}

impl CheckState<'_> {
    /// Decide whether one template pattern inhabits another template pattern.
    pub(in crate::check) fn decide_template_template(
        &mut self,
        origin: Origin,
        source_module: ModuleId,
        source: &dir::TemplateLiteralType,
        target_module: ModuleId,
        target: &dir::TemplateLiteralType,
    ) -> CompilerResult<bool> {
        // flatten the source into literal text and span pieces
        let segments = self.template_segments(source_module, source.strings)?;
        let spans = self.type_ids(source_module, source.spans)?.to_vec();
        let mut pieces = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            if !segment.is_empty() {
                pieces.push(TemplatePiece::Text(segment.clone()));
            }
            if let Some(span) = spans.get(index) {
                let span = self.template_span_head(origin, *span)?;
                match self.template_piece_text(span)? {
                    Some(text) => pieces.push(TemplatePiece::Text(text)),
                    None => pieces.push(TemplatePiece::Span(span)),
                }
            }
        }

        // flatten the target into its own segments and reduced spans
        let target_segments = self.template_segments(target_module, target.strings)?;
        let mut target_spans = Vec::new();
        for span in self.type_ids(target_module, target.spans)?.to_vec() {
            target_spans.push(self.template_span_head(origin, span)?);
        }

        self.match_template_pieces(origin, &pieces, 0, "", &target_segments, &target_spans)
    }

    /// Split one text into per-span captures with earliest delimiter rules.
    pub(in crate::check) fn split_template_captures(
        &mut self,
        origin: Origin,
        text: &str,
        template_module: ModuleId,
        template: &dir::TemplateLiteralType,
    ) -> CompilerResult<Option<Vec<(dir::GlobalTypeId, String)>>> {
        let segments = self.template_segments(template_module, template.strings)?;
        let mut spans = Vec::new();
        for span in self.type_ids(template_module, template.spans)?.to_vec() {
            spans.push(self.template_span_head(origin, span)?);
        }

        // anchor the split on the leading segment
        let Some(first) = segments.first() else {
            return Ok(text.is_empty().then(Vec::new));
        };
        let Some(mut rest) = text.strip_prefix(first.as_str()) else {
            return Ok(None);
        };

        let mut parts = Vec::with_capacity(spans.len());
        for (index, span) in spans.iter().copied().enumerate() {
            let is_last = index + 1 == spans.len();
            let next_segment = segments.get(index + 1).cloned().unwrap_or_default();

            // fixed-text spans consume exactly their own text
            if let Some(fixed) = self.template_piece_text(span)? {
                let Some(stripped) = rest.strip_prefix(fixed.as_str()) else {
                    return Ok(None);
                };
                rest = stripped;
                parts.push((span, fixed));

                continue;
            }

            let captured = if is_last {
                // run the last span up to the trailing segment
                let Some(stripped) = rest.strip_suffix(next_segment.as_str()) else {
                    return Ok(None);
                };
                let captured = stripped.to_string();
                rest = "";

                captured
            } else if !next_segment.is_empty() {
                // stop an interior span at the earliest delimiter occurrence
                let Some(found) = rest.find(next_segment.as_str()) else {
                    return Ok(None);
                };
                let captured = rest[..found].to_string();
                rest = &rest[found + next_segment.len()..];

                captured
            } else {
                // adjacent spans keep the earlier span nonempty while text remains
                let mut chars = rest.chars();
                match chars.next() {
                    Some(first) => {
                        rest = chars.as_str();

                        first.to_string()
                    }
                    None => String::new(),
                }
            };
            parts.push((span, captured));
        }
        if !rest.is_empty() {
            return Ok(None);
        }

        Ok(Some(parts))
    }

    /// Return the fixed text of one closed printable span piece.
    pub(in crate::check) fn template_piece_text(
        &self,
        span: dir::GlobalTypeId,
    ) -> CompilerResult<Option<String>> {
        let text = match self.ty(span)? {
            // numeric literals admit many spellings, never one fixed text
            dir::Type::Literal(
                dir::ScalarLiteral::Integer(_)
                | dir::ScalarLiteral::Float(_)
                | dir::ScalarLiteral::Bigint(_),
            ) => None,
            dir::Type::Literal(literal) => literal.template_text(self.strings()),
            dir::Type::Key(dir::StaticKey::Name(name)) => {
                Some(self.strings().get(name).to_string())
            }
            dir::Type::Key(dir::StaticKey::Index(index)) => Some(index.to_string()),
            dir::Type::Null => Some("null".to_string()),
            dir::Type::Undefined => Some("undefined".to_string()),
            _ => None,
        };

        Ok(text)
    }

    /// Match source pieces against the target segments and spans.
    fn match_template_pieces(
        &mut self,
        origin: Origin,
        pieces: &[TemplatePiece],
        piece: usize,
        pending: &str,
        segments: &[String],
        spans: &[dir::GlobalTypeId],
    ) -> CompilerResult<bool> {
        // consume the leading target segment from known text only
        let Some(first) = segments.first() else {
            return Ok(piece >= pieces.len() && pending.is_empty());
        };
        let mut piece = piece;
        let mut pending = pending.to_string();
        let mut needed = first.as_str();
        while !needed.is_empty() {
            if pending.is_empty() {
                match pieces.get(piece) {
                    Some(TemplatePiece::Text(text)) => {
                        pending = text.clone();
                        piece += 1;
                    }
                    // spans cannot guarantee literal text
                    _ => return Ok(false),
                }
            }
            let take = needed.len().min(pending.len());
            if !needed.is_char_boundary(take) || !pending.is_char_boundary(take) {
                return Ok(false);
            }
            if needed[..take] != pending[..take] {
                return Ok(false);
            }
            needed = &needed[take..];
            pending = pending[take..].to_string();
        }
        let Some(span) = spans.first() else {
            let exhausted = piece >= pieces.len() && pending.is_empty() && segments.len() == 1;

            return Ok(exhausted);
        };
        let tail_segments = &segments[1..];
        let tail_spans = &spans[1..];

        // absorb any run of pieces up to each delimiter for a string span
        if matches!(
            self.ty(*span)?,
            dir::Type::Primitive(dir::PrimitiveType::String)
        ) {
            // absorb nothing or any prefix of the pending text
            for split in 0..=pending.len() {
                if !pending.is_char_boundary(split) {
                    continue;
                }
                if self.match_template_pieces(
                    origin,
                    pieces,
                    piece,
                    &pending[split..],
                    tail_segments,
                    tail_spans,
                )? {
                    return Ok(true);
                }
            }

            // absorb whole pieces beyond the pending text
            if pending.is_empty() {
                for stop in piece..=pieces.len() {
                    if stop > piece
                        && self.match_template_pieces(
                            origin,
                            pieces,
                            stop,
                            "",
                            tail_segments,
                            tail_spans,
                        )?
                    {
                        return Ok(true);
                    }
                    // splitting inside a text piece hands its suffix onward
                    if let Some(TemplatePiece::Text(text)) = pieces.get(stop) {
                        for split in 0..=text.len() {
                            if !text.is_char_boundary(split) {
                                continue;
                            }
                            if self.match_template_pieces(
                                origin,
                                pieces,
                                stop + 1,
                                &text[split..],
                                tail_segments,
                                tail_spans,
                            )? {
                                return Ok(true);
                            }
                        }
                    }
                }
            }

            return Ok(false);
        }

        // constrained spans absorb exactly one matching piece
        if pending.is_empty() {
            match pieces.get(piece) {
                Some(TemplatePiece::Text(text)) => {
                    let text = text.clone();
                    if self.match_template_span(origin, &text, *span)?
                        && self.match_template_pieces(
                            origin,
                            pieces,
                            piece + 1,
                            "",
                            tail_segments,
                            tail_spans,
                        )?
                    {
                        return Ok(true);
                    }
                }
                Some(TemplatePiece::Span(source_span)) => {
                    let source_span = *source_span;
                    if self.decide_template_span_domain(origin, source_span, *span)?
                        && self.match_template_pieces(
                            origin,
                            pieces,
                            piece + 1,
                            "",
                            tail_segments,
                            tail_spans,
                        )?
                    {
                        return Ok(true);
                    }
                }
                None => {}
            }
        } else {
            // pending text feeds the constrained span whole
            let text = pending.clone();
            if self.match_template_span(origin, &text, *span)?
                && self.match_template_pieces(
                    origin,
                    pieces,
                    piece,
                    "",
                    tail_segments,
                    tail_spans,
                )?
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Decide whether one span's whole domain fits another span pattern.
    fn decide_template_span_domain(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        match (self.ty(source)?, self.ty(target)?) {
            (_, dir::Type::Primitive(dir::PrimitiveType::String)) => Ok(true),
            (
                dir::Type::Primitive(dir::PrimitiveType::Float(_) | dir::PrimitiveType::Integer(_)),
                dir::Type::Primitive(dir::PrimitiveType::Float(_) | dir::PrimitiveType::Integer(_)),
            ) => Ok(true),
            (
                dir::Type::Primitive(dir::PrimitiveType::Bigint),
                dir::Type::Primitive(dir::PrimitiveType::Bigint),
            ) => Ok(true),
            _ => self.decide_relation(origin, Relation::Assignable, source, target),
        }
    }
}

/// One captured template span text.
#[derive(Debug, Clone, Copy)]
struct TemplateText<'a>(&'a str);

impl TemplateText<'_> {
    /// Parse this text as a number literal.
    fn number(self) -> Option<f64> {
        let text = self.0;
        if text.is_empty() || text.trim() != text {
            return None;
        }
        let (sign, magnitude) = match text.strip_prefix('-') {
            Some(rest) => (-1.0, rest),
            None => (1.0, text),
        };

        // parse radix-prefixed integer forms
        for (prefix, radix) in [
            ("0x", 16),
            ("0X", 16),
            ("0o", 8),
            ("0O", 8),
            ("0b", 2),
            ("0B", 2),
        ] {
            if let Some(digits) = magnitude.strip_prefix(prefix) {
                let value = i128::from_str_radix(digits, radix).ok()?;

                return Some(sign * value as f64);
            }
        }

        // exclude named non-finite values from decimal forms
        let value: f64 = magnitude.parse().ok()?;
        if !value.is_finite() || magnitude.chars().next()?.is_ascii_alphabetic() {
            return None;
        }

        Some(sign * value)
    }

    /// Parse this text as an exact integer literal.
    fn integer(self) -> Option<i64> {
        let text = self.0;
        if text.is_empty() || text.trim() != text {
            return None;
        }
        let (sign, magnitude) = if let Some(magnitude) = text.strip_prefix('-') {
            (-1i128, magnitude)
        } else {
            (1i128, text)
        };
        let (digits, radix) = if let Some(digits) = magnitude
            .strip_prefix("0x")
            .or_else(|| magnitude.strip_prefix("0X"))
        {
            (digits, 16)
        } else if let Some(digits) = magnitude
            .strip_prefix("0o")
            .or_else(|| magnitude.strip_prefix("0O"))
        {
            (digits, 8)
        } else if let Some(digits) = magnitude
            .strip_prefix("0b")
            .or_else(|| magnitude.strip_prefix("0B"))
        {
            (digits, 2)
        } else {
            (magnitude, 10)
        };
        if digits.is_empty() {
            return None;
        }
        let magnitude = i128::from_str_radix(digits, radix).ok()?;
        let value = magnitude.checked_mul(sign)?;

        i64::try_from(value).ok()
    }

    /// Parse this text in one interval's scalar domain.
    fn range(self, range: &dir::RangeType) -> Option<dir::ScalarLiteral> {
        let literal = match range.scalar_domain()? {
            dir::ScalarDomain::Integer => dir::ScalarLiteral::Integer(self.integer()?),
            dir::ScalarDomain::Bigint => dir::ScalarLiteral::Bigint(self.integer()?),
            dir::ScalarDomain::Character => {
                let mut characters = self.0.chars();
                let character = characters.next()?;
                if characters.next().is_some() {
                    return None;
                }

                dir::ScalarLiteral::Character(character)
            }
            dir::ScalarDomain::Float
            | dir::ScalarDomain::String
            | dir::ScalarDomain::Symbol
            | dir::ScalarDomain::Boolean
            | dir::ScalarDomain::Null
            | dir::ScalarDomain::Undefined => return None,
        };

        range.contains_literal(literal).then_some(literal)
    }

    /// Return whether this text fits one integer type.
    fn fits_integer(self, integer: dir::IntegerType) -> bool {
        let Some(value) = self.integer() else {
            return false;
        };

        integer.fits_literal(value)
    }
}
