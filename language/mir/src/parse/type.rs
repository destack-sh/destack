use crate::source::{Token, TokenType};
use destack_source::Span;

use crate::{
    Access, Copy, Extent, Field, FieldSpan, GenericArgument, GenericParameter,
    GenericParameterDomain, LanguageItem, Lifetime, LifetimeParameter, Multiplicity, Reference,
    SignatureParameter, Space, Static, StaticId, Storage, Type, TypeDeclarationSpans, TypeId,
    VariantCase,
};

use super::error::{ParseError, ParseResult};
use super::parser::Parser;

/// Parsed reference-like type qualifiers.
#[derive(Debug, Clone)]
struct ReferenceQualifiers {
    /// The reference ownership kind.
    kind: Option<Reference>,
    /// The explicit reference lifetime.
    lifetime: Lifetime,
    /// Whether a lifetime was written, the erased wildcard among them.
    has_lifetime: bool,
    /// The referenced storage, written or named by the lifetime's region parameter.
    storage: Option<Storage>,
    /// The exposed access mode.
    access: Option<Access>,
}

impl ReferenceQualifiers {
    /// Create empty reference qualifiers.
    fn new() -> Self {
        Self {
            kind: None,
            lifetime: Lifetime::empty(),
            has_lifetime: false,
            storage: None,
            access: None,
        }
    }

    /// Finish qualifiers that require an ownership kind.
    fn finish(
        self,
        expected: &'static str,
        pos: usize,
    ) -> ParseResult<ResolvedReferenceQualifiers> {
        let kind = self
            .kind
            .ok_or_else(|| ParseError::invalid(expected, pos))?;
        if kind == Reference::Unique && !self.lifetime.is_empty() {
            return Err(ParseError::invalid("reference lifetime", pos));
        }
        if matches!(kind, Reference::Borrowed) && !self.has_lifetime {
            return Err(ParseError::invalid("borrowed reference lifetime", pos));
        }

        // use explicit storage or the complete storage of the lifetime's region parameter
        let storage = match (self.storage, self.lifetime.extents.as_slice()) {
            (Some(storage), _) => storage,
            (None, [Extent::Parameter(index)]) => Storage::Parameter(*index),
            (None, _) => return Err(ParseError::invalid("reference storage", pos)),
        };

        Ok(ResolvedReferenceQualifiers {
            kind,
            lifetime: self.lifetime,
            storage,
            access: self
                .access
                .ok_or_else(|| ParseError::invalid("reference access", pos))?,
        })
    }
}

/// Resolved reference-like type qualifiers.
#[derive(Debug, Clone)]
struct ResolvedReferenceQualifiers {
    /// The reference ownership kind.
    kind: Reference,
    /// The explicit reference lifetime.
    lifetime: Lifetime,
    /// The referenced storage.
    storage: Storage,
    /// The exposed access mode.
    access: Access,
}

impl Parser {
    /// Parse a type expression and return its enclosing span.
    pub(super) fn parse_type_part(&mut self) -> ParseResult<(TypeId, Span)> {
        let type_start = self.pos();
        let ty = self.parse_type()?;
        let span = self.span_from_parse_start(type_start);

        Ok((ty, span))
    }

    /// Parse a type use and return its enclosing span.
    pub(super) fn parse_type_use_part(&mut self) -> ParseResult<(TypeId, Span)> {
        let type_start = self.pos();
        let ty = self.parse_type()?;
        let span = self.span_from_parse_start(type_start);

        Ok((ty, span))
    }

    /// Parse a type use after one required delimiter.
    pub(super) fn parse_type_use_after(
        &mut self,
        delimiter: Token,
        expected: &'static str,
    ) -> (TypeId, Span) {
        self.parse_type_use_recovering(delimiter, expected, true)
    }

    /// Parse a return type use after its required delimiter.
    pub(super) fn parse_return_type_use_after(&mut self, delimiter: Token) -> (TypeId, Span) {
        let is_structural_type_allowed = self.is_return_structural_type_start();

        self.parse_type_use_recovering(delimiter, "return type", is_structural_type_allowed)
    }

    /// Parse a type use after one required delimiter and recover holes locally.
    fn parse_type_use_recovering(
        &mut self,
        delimiter: Token,
        expected: &'static str,
        allow_structural_type: bool,
    ) -> (TypeId, Span) {
        let hole_position = delimiter.span.end as usize;

        if self.is_missing_type_position(&delimiter, allow_structural_type) {
            let error = ParseError::new(format!("expected {expected}"), hole_position);

            return self.recovered_type(error, false);
        }

        let type_start = self.pos();
        match self.parse_type() {
            Ok(ty) => {
                let span = self.span_from_parse_start(type_start);

                (ty, span)
            }
            Err(error) => self.recovered_type(error, true),
        }
    }

    /// Emit one type recovery diagnostic and return the canonical error type.
    fn recovered_type(&mut self, error: ParseError, should_advance: bool) -> (TypeId, Span) {
        self.diagnostics
            .insert(error.to_diagnostic(self.blob, self.file_id));

        if should_advance && self.peek().is_some() {
            self.bump();
        }

        let error_end = self.pos();
        let span = self.span_between(error.position(), error_end);
        let ty = self.error_type();

        (ty, span)
    }

    /// Apply generic arguments to an identified base type.
    fn apply_type_arguments(
        &mut self,
        base: TypeId,
        arguments: Vec<GenericArgument>,
    ) -> ParseResult<TypeId> {
        if arguments.is_empty() {
            return Ok(base);
        }

        let copy = self.tree.copy(base);

        self.intern_type(Type::Application { base, arguments }, copy)
    }

    /// Parse a type expression and append its span as one source segment.
    pub(super) fn parse_type_segment(
        &mut self,
        segment_spans: &mut Vec<Span>,
    ) -> ParseResult<TypeId> {
        let (ty, span) = self.parse_type_use_part()?;
        segment_spans.push(span);

        Ok(ty)
    }

    /// Return whether a token can start a type in a value position.
    pub(super) fn peek_type(&self, kind: TokenType) -> bool {
        matches!(
            kind,
            TokenType::Void
                | TokenType::Boolean
                | TokenType::Identifier
                | TokenType::TypeName
                | TokenType::Ref
                | TokenType::Vector
                | TokenType::Newtype
                | TokenType::LessThan
                | TokenType::OpenParenthesis
                | TokenType::OpenBracket
                | TokenType::OpenBrace
        )
    }

    /// Return whether a type position is structurally empty.
    fn is_missing_type_position(
        &self,
        delimiter: &Token,
        is_structural_type_allowed: bool,
    ) -> bool {
        let Some(token) = self.peek() else {
            return true;
        };
        let ty = self.token_type(token);
        let is_type_start = self.peek_type(ty);
        let is_structural_type = ty == TokenType::OpenBrace;

        // recover when the next declaration starts after a type hole
        if self.has_line_break_after(delimiter) && self.peek_identifier_label() {
            return true;
        }

        if is_type_start && (is_structural_type_allowed || !is_structural_type) {
            return false;
        }

        if self.has_line_break_after(delimiter) {
            return true;
        }

        matches!(
            ty,
            TokenType::CloseParenthesis
                | TokenType::OpenBrace
                | TokenType::CloseBrace
                | TokenType::CloseBracket
                | TokenType::Comma
                | TokenType::Semicolon
                | TokenType::Equal
                | TokenType::End
        )
    }

    /// Return whether the next tokens look like a label or named declaration.
    fn peek_identifier_label(&self) -> bool {
        let Some(name) = self.peek_nth_token(0) else {
            return false;
        };

        let Some(delimiter) = self.peek_nth_token(1) else {
            return false;
        };

        self.token_type(name) == TokenType::Identifier
            && self.token_type(delimiter) == TokenType::Colon
    }

    /// Return whether a return type starts with a complete structural type.
    pub(super) fn is_return_structural_type_start(&self) -> bool {
        let tokens = self.tree.tokens();
        let Some((open_index, open_token)) = tokens
            .iter()
            .enumerate()
            .skip(self.pos)
            .find(|(_, token)| !token.is_trivia())
        else {
            return false;
        };

        if self.token_type(open_token) != TokenType::OpenBrace {
            return true;
        }

        let mut depth = 0usize;
        let mut close_index = open_index;
        while let Some(token) = tokens.get(close_index) {
            match self.token_type(token) {
                TokenType::OpenBrace => depth += 1,
                TokenType::CloseBrace => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                TokenType::End => return false,
                _ => {}
            }

            close_index += 1;
        }

        if depth != 0 {
            return false;
        }

        tokens
            .iter()
            .skip(close_index + 1)
            .find(|token| !token.is_trivia())
            .is_some_and(|token| {
                matches!(
                    self.token_type(token),
                    TokenType::OpenBrace | TokenType::Semicolon
                )
            })
    }

    /// Parse a type expression.
    pub(super) fn parse_type(&mut self) -> ParseResult<TypeId> {
        let (kind, token_start, token_text) = {
            let token = self
                .peek()
                .ok_or_else(|| ParseError::unexpected_end("type", self.pos()))?;
            (
                self.token_type(token),
                token.start(),
                self.tree.source_text(token.span).to_string(),
            )
        };

        // primitive or composite type
        let ty = match kind {
            TokenType::Void => {
                self.bump();
                Type::Void
            }
            TokenType::Boolean => {
                self.bump();
                Type::Boolean
            }
            TokenType::Identifier | TokenType::TypeName => {
                return self.parse_named_type(&token_text, token_start);
            }
            TokenType::Function => self.parse_function_type()?,
            TokenType::Ref => self.parse_reference_type()?,
            TokenType::Vector => self.parse_vector_type()?,
            TokenType::Newtype => self.parse_newtype_type()?,
            TokenType::LessThan => {
                return self.parse_lifetime_signature_type();
            }
            TokenType::OpenParenthesis => self.parse_parenthesized_type()?,
            TokenType::OpenBracket => self.parse_array_type()?,
            TokenType::OpenBrace => {
                let (type_id, _, _) = self.parse_struct_type()?;

                return Ok(type_id);
            }
            _ => {
                return Err(ParseError::unexpected("type", kind, token_start));
            }
        };
        let copy = self.parsed_copy(&ty);

        self.intern_type(ty, copy)
    }

    /// Parse a named type form.
    fn parse_named_type(&mut self, name: &str, start: usize) -> ParseResult<TypeId> {
        if let Some(primitive) = Type::from_primitive_name(name) {
            self.bump();

            return self.intern_type(primitive, Copy::Yes);
        }

        // a type parameter in scope
        if let Some((index, parameter)) = self.generic_parameter(name) {
            let GenericParameterDomain::Type { .. } = parameter.domain else {
                return Err(ParseError::invalid("type parameter", start));
            };
            let copy = self.parameter_copy(parameter);
            self.bump();

            return self.intern_type(
                Type::Parameter {
                    index,
                    referent: false,
                },
                copy,
            );
        }

        let ty = match name {
            "fn" => self.parse_function_pointer_type()?,
            "ptr" => self.parse_pointer_type()?,
            "slice" => self.parse_slice_type()?,
            "dynamic" => self.parse_dynamic_type()?,
            "uninit" => self.parse_uninit_type()?,
            "manual" => self.parse_manual_type()?,
            "variant" => self.parse_variant_type()?,
            _ => {
                self.bump();
                let arguments = self.parse_identified_type_arguments()?;
                let base = self
                    .type_declaration_map
                    .get(name)
                    .copied()
                    .ok_or_else(|| {
                        ParseError::invalid(&format!("identified type '{name}'"), start)
                    })?;

                let base = self
                    .tree
                    .intern_type(Type::Declaration { declaration: base }, Copy::No);

                return self.apply_type_arguments(base, arguments);
            }
        };
        let copy = self.parsed_copy(&ty);

        self.intern_type(ty, copy)
    }

    /// Parse the generic arguments on an identified type.
    fn parse_identified_type_arguments(&mut self) -> ParseResult<Vec<GenericArgument>> {
        if !self.eat_token_if(TokenType::LessThan) {
            return Ok(Vec::new());
        }

        // parse the applied arguments in template order
        let mut arguments = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            arguments.push(self.parse_generic_argument()?);

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::GreaterThan)?;

        Ok(arguments)
    }

    /// Parse a region argument, with explicit storage after `&` when needed.
    pub(super) fn parse_region_argument(&mut self) -> ParseResult<GenericArgument> {
        let start = self.pos();
        let lifetime = self.parse_lifetime_union()?;
        let storage = if self.eat_token_if(TokenType::Ampersand) {
            self.parse_storage_argument()?
        } else {
            match lifetime.extents.as_slice() {
                [Extent::Parameter(index)] => Storage::Parameter(*index),
                _ => {
                    return Err(ParseError::invalid(
                        "region argument without storage",
                        start,
                    ));
                }
            }
        };

        Ok(GenericArgument::Region { lifetime, storage })
    }

    /// Parse one space, a join of spaces separated by pipes interning as one.
    fn parse_space_argument(&mut self) -> ParseResult<Space> {
        let mut spaces = vec![self.parse_space_atom()?];
        while self.eat_token_if(TokenType::Pipe) {
            spaces.push(self.parse_space_atom()?);
        }
        if let [space] = spaces.as_slice() {
            return Ok(*space);
        }

        Ok(self.tree.intern_space_join(spaces))
    }

    /// Parse one space by its name or by a space or region parameter in scope.
    fn parse_space_atom(&mut self) -> ParseResult<Space> {
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("region space", self.pos()))?;
        let text = self.tree.source_text(token.span).to_string();
        let start = token.start();
        if self.token_type(token) == TokenType::Lifetime {
            return match self.parse_extent()? {
                Extent::Parameter(index) => Ok(Space::Parameter(index)),
                _ => Err(ParseError::invalid("region space", start)),
            };
        }
        if let Some(space) = Space::from_name(&text) {
            self.bump();

            return Ok(space);
        }

        // read the space of one type's storage
        if text == "PlaceOf" {
            self.bump();
            self.eat_token(TokenType::LessThan)?;
            let (ty, _) = self.parse_type_use_part()?;
            self.eat_token(TokenType::GreaterThan)?;

            return Ok(Space::Of(ty));
        }
        if let Some((index, parameter)) = self.generic_parameter(&text)
            && matches!(
                parameter.domain,
                GenericParameterDomain::Space | GenericParameterDomain::Region { .. }
            )
        {
            self.bump();

            return Ok(Space::Parameter(index));
        }

        Err(ParseError::invalid("region space", start))
    }

    /// Parse a function pointer type.
    fn parse_function_pointer_type(&mut self) -> ParseResult<Type> {
        self.bump();
        let signature = self.parse_signature(Vec::new())?;

        Ok(Type::FunctionPointer { signature })
    }

    /// Parse a process-local machine pointer type.
    fn parse_pointer_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (pointee, _) = self.parse_type_use_part()?;

        // require the access exposed through the pointer
        self.eat_token(TokenType::Comma)?;
        let access = self
            .parse_access_if()
            .ok_or_else(|| ParseError::invalid("pointer access", self.pos()))?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Pointer { pointee, access })
    }

    /// Parse a slice type.
    fn parse_slice_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (element, _) = self.parse_type_use_part()?;
        let (kind, lifetime, storage, access) = self.parse_slice_qualifiers()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Slice {
            kind,
            lifetime,
            element,
            storage,
            access,
        })
    }

    /// Parse a dynamic erased value type.
    fn parse_dynamic_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (constraint, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers()?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(Type::Dynamic {
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            constraint,
            storage: qualifiers.storage,
            access: qualifiers.access,
        })
    }

    /// Parse a captured function value type.
    fn parse_function_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let signature = self.parse_signature(Vec::new())?;
        self.eat_token(TokenType::Comma)?;
        let multiplicity_token = self.eat_token(TokenType::Identifier)?;
        let multiplicity_start = multiplicity_token.start();
        let multiplicity_name = self.tree.source_text(multiplicity_token.span);
        let multiplicity = Multiplicity::from_name(multiplicity_name)
            .ok_or_else(|| ParseError::invalid("function multiplicity", multiplicity_start))?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers()?;
        self.eat_token(TokenType::GreaterThan)?;
        Ok(Type::Function {
            multiplicity,
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            signature,
            storage: qualifiers.storage,
            access: qualifiers.access,
        })
    }

    /// Parse a linear uninitialized allocation token type.
    fn parse_uninit_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (value, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Uninit { value })
    }

    /// Parse a manually dropped storage type.
    fn parse_manual_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (value, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::ManuallyDrop { value })
    }

    /// Parse a vector type.
    fn parse_vector_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (element, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let lanes = self.parse_length()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Vector { element, lanes })
    }

    /// Parse a newtype type.
    fn parse_newtype_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let (inner, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Newtype { inner })
    }

    /// Parse a tuple or function signature type.
    fn parse_parenthesized_type(&mut self) -> ParseResult<Type> {
        let parameters = self.parse_parenthesized_type_parameters()?;

        if self.eat_token_if(TokenType::FatArrow) {
            let (result, _) = self.parse_type_use_part()?;
            let mut lifetimes = Vec::new();
            self.parse_lifetime_where(&mut lifetimes)?;

            return Ok(Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            });
        }

        Ok(Type::Tuple {
            elements: parameters
                .into_iter()
                .map(|parameter| parameter.ty)
                .collect(),
        })
    }

    /// Parse an explicitly lifetime-polymorphic function signature type.
    fn parse_lifetime_signature_type(&mut self) -> ParseResult<TypeId> {
        self.parse_lifetime_scope(|parser, lifetimes| parser.parse_signature(lifetimes))
    }

    /// Parse type parameters enclosed in parentheses.
    pub(super) fn parse_parenthesized_type_parameters(
        &mut self,
    ) -> ParseResult<Vec<SignatureParameter>> {
        self.eat_token(TokenType::OpenParenthesis)?;
        let mut parameters = Vec::new();

        while !self.peek_is(TokenType::CloseParenthesis) {
            let (ty, _) = self.parse_type_use_part()?;
            parameters.push(SignatureParameter { ty });

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::CloseParenthesis)?;

        Ok(parameters)
    }

    /// Parse a function signature type.
    pub(super) fn parse_signature(
        &mut self,
        lifetimes: Vec<LifetimeParameter>,
    ) -> ParseResult<TypeId> {
        let parameters = self.parse_parenthesized_type_parameters()?;
        self.eat_token(TokenType::FatArrow)?;

        self.parse_signature_result(lifetimes, parameters)
    }

    /// Parse a function signature result after its parameter types.
    fn parse_signature_result(
        &mut self,
        mut lifetimes: Vec<LifetimeParameter>,
        parameters: Vec<SignatureParameter>,
    ) -> ParseResult<TypeId> {
        let (result, _) = self.parse_type_use_part()?;
        self.parse_lifetime_where(&mut lifetimes)?;
        self.intern_type(
            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
            },
            Copy::Yes,
        )
    }

    /// Parse a fixed array type.
    fn parse_array_type(&mut self) -> ParseResult<Type> {
        self.bump();
        let (element, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Semicolon)?;

        let length = self.parse_length()?;

        self.eat_token(TokenType::CloseBracket)?;

        Ok(Type::FixedArray { element, length })
    }

    /// Parse one struct type and retain field declaration spans.
    pub(super) fn parse_struct_type(
        &mut self,
    ) -> ParseResult<(TypeId, Vec<FieldSpan>, TypeDeclarationSpans)> {
        let open_brace_token = self.eat_token(TokenType::OpenBrace)?;
        let open_brace_start = open_brace_token.start();
        let open_brace_length = self.tree.source_text(open_brace_token.span).len();
        let open_brace_span = self.span_at(open_brace_start, open_brace_length);

        let mut fields = Vec::new();
        let mut field_spans = Vec::new();

        while !self.peek_is(TokenType::CloseBrace) {
            let field_start = self.pos();

            // field attributes
            let (attributes, attribute_spans) = self.parse_attributes()?;

            // read a named field, including keywords and generated names prefixed with @
            let mut name = None;
            let mut name_span = None;
            let mut type_anchor = None;
            let offset = usize::from(self.peek_is(TokenType::At));
            if self
                .peek_nth_token(offset)
                .is_some_and(|token| self.token_type(token).is_name())
                && self
                    .peek_nth_token(offset + 1)
                    .is_some_and(|token| self.token_type(token) == TokenType::Colon)
            {
                let start = self.pos();
                let prefix = if self.eat_token_if(TokenType::At) {
                    "@"
                } else {
                    ""
                };
                let token = self.peek().copied().expect("a field name was matched");
                self.bump();
                let text = format!("{prefix}{}", self.tree.source_text(token.span));
                name_span = Some(self.span_from_parse_start(start));
                type_anchor = Some(self.eat_token(TokenType::Colon)?);
                name = Some(self.strings.intern(&text));
            }

            // field type
            let is_line_hole = type_anchor
                .as_ref()
                .is_some_and(|token| self.has_line_break_after(token));
            let (ty, type_span) = if let Some(type_anchor) = type_anchor {
                self.parse_type_use_after(type_anchor, "field type")
            } else {
                self.parse_type_use_part()?
            };

            // field node
            let field = Field {
                name,
                ty,
                attributes,
            };
            fields.push(self.tree.intern_field(field));

            // field delimiter
            if self.eat_token_if(TokenType::Semicolon) || self.eat_token_if(TokenType::Comma) {
                let field_span = self.span_from_parse_start(field_start);
                field_spans.push(FieldSpan::new(
                    field_span,
                    attribute_spans,
                    name_span,
                    type_span,
                ));
                continue;
            }

            if is_line_hole {
                let field_span = self.span_from_parse_start(field_start);
                field_spans.push(FieldSpan::new(
                    field_span,
                    attribute_spans,
                    name_span,
                    type_span,
                ));
                continue;
            }

            if self.peek_is(TokenType::CloseBrace) {
                let field_span = self.span_from_parse_start(field_start);
                field_spans.push(FieldSpan::new(
                    field_span,
                    attribute_spans,
                    name_span,
                    type_span,
                ));
                break;
            }

            return Err(ParseError::invalid("';' or '}'", self.pos()));
        }

        let close_brace_token = self.eat_token(TokenType::CloseBrace)?;
        let close_brace_start = close_brace_token.start();
        let close_brace_length = self.tree.source_text(close_brace_token.span).len();
        let close_brace_span = self.span_at(close_brace_start, close_brace_length);

        let struct_type = Type::Struct { fields };
        let type_id = self.intern_type(struct_type, Copy::No)?;

        Ok((
            type_id,
            field_spans,
            TypeDeclarationSpans::new(None, Some(open_brace_span), Some(close_brace_span)),
        ))
    }

    /// Parse a reference type.
    fn parse_reference_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;

        let (pointee, _) = self.parse_type_use_part()?;
        self.eat_token(TokenType::Comma)?;
        let qualifiers = self.parse_reference_qualifiers()?;

        self.eat_token(TokenType::GreaterThan)?;

        Ok(Type::Reference {
            kind: qualifiers.kind,
            lifetime: qualifiers.lifetime,
            storage: qualifiers.storage,
            access: qualifiers.access,
            pointee,
        })
    }

    /// Parse a physical variant type.
    fn parse_variant_type(&mut self) -> ParseResult<Type> {
        self.bump();
        self.eat_token(TokenType::LessThan)?;
        let discriminant = self.parse_type()?;
        self.eat_token(TokenType::GreaterThan)?;
        self.eat_token(TokenType::OpenBrace)?;

        let mut cases = Vec::new();
        while !self.peek_is(TokenType::CloseBrace) {
            let discriminant = self.parse_constant_for_type(discriminant)?;
            self.eat_token(TokenType::Equal)?;
            let (ty, _) = self.parse_type_use_part()?;
            cases.push(VariantCase { discriminant, ty });

            if self.eat_token_if(TokenType::Semicolon) || self.eat_token_if(TokenType::Comma) {
                continue;
            }

            if self.peek_is(TokenType::CloseBrace) {
                break;
            }

            return Err(ParseError::invalid("';' or '}'", self.pos()));
        }

        self.eat_token(TokenType::CloseBrace)?;

        Ok(Type::Variant {
            discriminant,
            cases,
        })
    }

    /// Parse one fixed array length: a literal or a value parameter in scope.
    fn parse_length(&mut self) -> ParseResult<StaticId> {
        // read a value parameter in scope
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("array length", self.pos()))?;
        if self.token_type(token) == TokenType::Identifier {
            let name = self.tree.source_text(token.span).to_string();
            let Some((index, parameter)) = self.generic_parameter(&name) else {
                return Err(ParseError::invalid("array length", token.start()));
            };
            let GenericParameterDomain::Value { .. } = parameter.domain else {
                return Err(ParseError::invalid("array length", token.start()));
            };
            self.bump();

            return Ok(self.tree.intern_static(Static::Parameter(index)));
        }

        // read a literal length
        let length = self.parse_int_literal()?;
        let length =
            i64::try_from(length).map_err(|_| ParseError::invalid("array length", self.pos()))?;

        Ok(self.tree.intern_static(Static::Integer(length)))
    }

    /// Parse reference-like qualifiers after the pointee type.
    fn parse_reference_qualifiers(&mut self) -> ParseResult<ResolvedReferenceQualifiers> {
        let mut qualifiers = ReferenceQualifiers::new();
        self.parse_reference_qualifier(&mut qualifiers)?;

        while self.peek_is(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| self.token_type(token) == TokenType::OpenParenthesis)
            {
                break;
            }

            self.bump();
            self.parse_reference_qualifier(&mut qualifiers)?;
        }

        qualifiers.finish("reference kind", self.pos())
    }

    /// Parse optional trailing qualifiers for one slice type.
    fn parse_slice_qualifiers(&mut self) -> ParseResult<(Reference, Lifetime, Storage, Access)> {
        let mut qualifiers = ReferenceQualifiers::new();

        while self.peek_is(TokenType::Comma) {
            if self
                .peek_nth_token(1)
                .is_some_and(|token| self.token_type(token) == TokenType::OpenParenthesis)
            {
                break;
            }

            self.bump();
            self.parse_reference_qualifier(&mut qualifiers)?;
        }

        let qualifiers = qualifiers.finish("slice kind", self.pos())?;

        Ok((
            qualifiers.kind,
            qualifiers.lifetime,
            qualifiers.storage,
            qualifiers.access,
        ))
    }

    /// Parse one access: a keyword or an access parameter in scope.
    fn parse_access_if(&mut self) -> Option<Access> {
        // readonly arrives as its own token
        if self.eat_token_if(TokenType::Readonly) {
            return Some(Access::Readonly);
        }

        // read a closed access name or an access parameter in scope
        let token = self.peek()?;
        if self.token_type(token) != TokenType::Identifier {
            return None;
        }
        let text = self.tree.source_text(token.span).to_string();
        let access = match Access::from_name(&text) {
            Some(access) => access,
            None => match self.generic_parameter(&text) {
                Some((index, parameter))
                    if matches!(parameter.domain, GenericParameterDomain::Access) =>
                {
                    Access::Parameter(index)
                }
                _ => return None,
            },
        };
        self.bump();

        Some(access)
    }

    /// Parse one reference-like qualifier.
    fn parse_reference_qualifier(
        &mut self,
        qualifiers: &mut ReferenceQualifiers,
    ) -> ParseResult<()> {
        let pos = self.pos();
        if let Some(kind) = self.parse_reference_kind()? {
            if qualifiers.kind.replace(kind).is_some() {
                return Err(ParseError::invalid("duplicate reference kind", pos));
            }
        } else if let Some(access) = self.parse_access_if() {
            if qualifiers.access.replace(access).is_some() {
                return Err(ParseError::invalid("duplicate reference access", pos));
            }
        } else if self.peek_is(TokenType::Lifetime) {
            if qualifiers.has_lifetime {
                return Err(ParseError::invalid("duplicate reference lifetime", pos));
            }
            qualifiers.lifetime = self.parse_lifetime_union()?;
            qualifiers.has_lifetime = true;
            if self.eat_token_if(TokenType::Ampersand) {
                let storage = self.parse_storage_argument()?;
                if qualifiers.storage.replace(storage).is_some() {
                    return Err(ParseError::invalid("duplicate reference storage", pos));
                }
            }
        } else if let Some(storage) = self.parse_storage_if()? {
            if qualifiers.storage.replace(storage).is_some() {
                return Err(ParseError::invalid("duplicate reference storage", pos));
            }
        } else {
            return Err(ParseError::invalid("reference qualifier", pos));
        }

        Ok(())
    }

    /// Parse one reference ownership kind.
    fn parse_reference_kind(&mut self) -> ParseResult<Option<Reference>> {
        let Some(token) = self.peek() else {
            return Ok(None);
        };
        if !matches!(
            self.token_type(token),
            TokenType::Ownership | TokenType::Identifier
        ) {
            return Ok(None);
        }

        let kind = match self.tree.source_text(token.span) {
            "managed" => Reference::Managed,
            "unique" => Reference::Unique,
            "borrowed" => Reference::Borrowed,
            "raw" => Reference::Raw,
            _ => return Ok(None),
        };
        self.bump();

        Ok(Some(kind))
    }

    /// Parse one optional reference storage.
    fn parse_storage_if(&mut self) -> ParseResult<Option<Storage>> {
        let Some(token) = self.peek() else {
            return Ok(None);
        };
        let text = self.tree.source_text(token.span);
        let is_parameter = self.generic_parameter(text).is_some_and(|(_, parameter)| {
            matches!(
                parameter.domain,
                GenericParameterDomain::Space | GenericParameterDomain::Region { .. }
            )
        });
        let is_bound = self.token_type(token) == TokenType::Lifetime;
        if !matches!(text, "frame" | "heap" | "static")
            && Space::from_name(text).is_none()
            && !is_parameter
            && !is_bound
        {
            return Ok(None);
        }

        self.parse_storage_argument().map(Some)
    }

    /// Parse a set of possible storage locations.
    fn parse_storage_argument(&mut self) -> ParseResult<Storage> {
        let mut members = vec![self.parse_storage_atom()?];
        while self.eat_token_if(TokenType::Pipe) {
            members.push(self.parse_storage_atom()?);
        }

        if let [storage] = members.as_slice() {
            return Ok(*storage);
        }

        Ok(self.tree.intern_storage_join(members))
    }

    /// Parse one concrete storage location or region parameter.
    fn parse_storage_atom(&mut self) -> ParseResult<Storage> {
        // read concrete residences with an implicit local space
        if self.eat_name_if("frame") {
            return Ok(Storage::Frame);
        }
        if self.eat_name_if("static") {
            let space = if self.eat_token_if(TokenType::OpenParenthesis) {
                let space = self.parse_space_argument()?;
                self.eat_token(TokenType::CloseParenthesis)?;
                space
            } else {
                Space::Local
            };

            return Ok(Storage::Static(space));
        }
        if self.eat_name_if("heap") {
            self.eat_token(TokenType::OpenParenthesis)?;
            let space = self.parse_space_argument()?;
            self.eat_token(TokenType::CloseParenthesis)?;

            return Ok(Storage::heap(space));
        }

        // read the complete storage supplied for a region parameter, a bound one in its space
        if self.peek_is(TokenType::Lifetime) {
            return match self.parse_extent()? {
                Extent::Parameter(index) => Ok(Storage::Parameter(index)),
                Extent::Bound(bound) => {
                    self.eat_token(TokenType::Ampersand)?;
                    let space = self.parse_space_argument()?;

                    Ok(Storage::Bound { bound, space })
                }
                _ => Err(ParseError::invalid("storage parameter", self.pos())),
            };
        }
        if let Some(token) = self.peek()
            && let Some((index, parameter)) =
                self.generic_parameter(self.tree.source_text(token.span))
            && matches!(parameter.domain, GenericParameterDomain::Region { .. })
        {
            self.bump();

            return Ok(Storage::Parameter(index));
        }

        // read a concrete residence in the selected space
        let space = self.parse_space_atom()?;
        let storage = if self.eat_name_if("static") {
            Storage::Static(space)
        } else {
            Storage::heap(space)
        };

        Ok(storage)
    }

    /// Parse one tick lifetime union.
    pub(super) fn parse_lifetime_union(&mut self) -> ParseResult<Lifetime> {
        // read the erased extent as the wildcard
        if self
            .peek()
            .is_some_and(|token| self.tree.source_text(token.span) == "'_")
        {
            self.bump();

            return Ok(Lifetime::empty());
        }

        let mut extents = vec![self.parse_extent()?];
        while self.eat_token_if(TokenType::Pipe) {
            extents.push(self.parse_extent()?);
        }

        Ok(Lifetime::new(extents))
    }

    /// Parse one tick extent.
    fn parse_extent(&mut self) -> ParseResult<Extent> {
        let token = self.eat_token(TokenType::Lifetime)?;
        let name = self.tree.source_text(token.span);
        if name == "'static" {
            return Ok(Extent::Static);
        }
        if name == "'frame" {
            return Ok(Extent::Frame);
        }
        if name == "'managed" {
            return Ok(Extent::Managed);
        }

        let Some(term) = self.extent_of_name(name) else {
            return Err(ParseError::invalid_with_length(
                "lifetime name",
                token.start(),
                token.span.len() as usize,
            ));
        };

        Ok(term)
    }

    /// Return the copy decision one parsed shape takes, a declaration's set by its attribute.
    fn parsed_copy(&self, ty: &Type) -> Copy {
        match ty {
            Type::Reference { kind, .. }
            | Type::Slice { kind, .. }
            | Type::Dynamic { kind, .. }
            | Type::Function { kind, .. } => kind.copy(),
            Type::Uninit { .. } | Type::Struct { .. } | Type::Newtype { .. } | Type::Variant { .. } => {
                Copy::No
            }
            Type::ManuallyDrop { value } => self.tree.copy(*value),
            Type::FixedArray { element, .. } | Type::Vector { element, .. } => {
                self.tree.copy(*element)
            }
            Type::Tuple { elements } => match elements.iter().all(|element| self.tree.copy(*element).is_yes()) {
                true => Copy::Yes,
                false => Copy::No,
            },
            _ => Copy::Yes,
        }
    }

    /// Return the copy decision one type parameter's bounds write.
    pub(super) fn parameter_copy(&self, parameter: &GenericParameter) -> Copy {
        match &parameter.domain {
            GenericParameterDomain::Type { bounds }
                if bounds
                    .iter()
                    .any(|bound| LanguageItem::Copy.is_bound_by(&self.tree, *bound)) =>
            {
                Copy::Yes
            }
            _ => Copy::No,
        }
    }

    /// Return a canonical type id for the provided type shape.
    pub(super) fn intern_type(&mut self, ty: Type, copy: Copy) -> ParseResult<TypeId> {
        Ok(self.tree.intern_type(ty, copy))
    }

    /// Return the canonical parse-recovery type.
    pub(super) fn error_type(&mut self) -> TypeId {
        self.tree.intern_type(Type::Error, Copy::Yes)
    }
}
