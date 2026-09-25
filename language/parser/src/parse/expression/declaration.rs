use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{DeclarationHeader, ExpressionPosition, TypeKeywordHeader};
use crate::{ParseStart, Parser, ParserError, ParserResult, TokenProbe};
use tspp_dir::{
    Asynchrony, Declaration, ExportKind, Expression, Keyword, LocalNodeId, TokenType, TypeKind,
};

impl Parser {
    /// Insert a value expression for one declaration.
    pub(in crate::parse::expression) fn insert_declaration_expression(
        &mut self,
        start: &ParseStart,
        declaration: LocalNodeId<Declaration>,
    ) -> LocalNodeId<Expression> {
        self.insert_node(
            Expression::Declaration(declaration),
            self.range_since(start),
        )
    }

    /// Return whether the current tokens start a declaration expression.
    pub(in crate::parse::expression) fn peek_declaration_primary(
        &self,
        position: ExpressionPosition,
    ) -> bool {
        let mut probe = self.cursor.probe(&self.file);

        // exclude standalone export clauses and expressions
        if probe.peek_keyword() == Some(Keyword::Export) {
            probe.bump();
            if matches!(
                probe.peek_token_type(),
                TokenType::Assign | TokenType::OpenBrace | TokenType::Multiply
            ) {
                return false;
            }
            if probe.peek_keyword() == Some(Keyword::Type) {
                probe.bump();
                if matches!(
                    probe.peek_token_type(),
                    TokenType::OpenBrace | TokenType::Multiply | TokenType::End
                ) {
                    return false;
                }

                return probe.scan_type_alias();
            }
            if probe.peek_keyword() == Some(Keyword::Default) {
                probe.bump();
            }
            if probe.peek_token_type() == TokenType::At {
                return true;
            }
        }

        // consume declaration modifiers without changing parser state
        while matches!(
            probe.peek_keyword(),
            Some(Keyword::Declare | Keyword::Abstract | Keyword::Final | Keyword::Shared)
        ) {
            probe.bump();
            if probe.peek_token().is_on_new_line() {
                return false;
            }
        }

        // classify the declaration noun after its modifiers
        // ambient global blocks use an identifier-shaped declaration noun
        if probe.peek_identifier_is("global") {
            probe.bump();

            return probe.peek_token_type() == TokenType::OpenBrace;
        }

        let Some(keyword) = probe.peek_keyword() else {
            return false;
        };

        // direct declaration nouns own malformed statement heads
        if position.is_statement()
            && matches!(
                keyword,
                Keyword::Struct
                    | Keyword::Class
                    | Keyword::Enum
                    | Keyword::Function
                    | Keyword::Extension
                    | Keyword::Interface
                    | Keyword::Newtype
            )
        {
            return true;
        }

        // modified functions keep their modifier in the function parser
        if keyword == Keyword::Async {
            probe.bump();

            return probe.peek_keyword() == Some(Keyword::Function)
                && !probe.peek_token().is_on_new_line();
        }

        // await heads an asynchronous using
        if keyword == Keyword::Await {
            probe.bump();

            return probe.peek_keyword() == Some(Keyword::Using)
                && !probe.peek_token().is_on_new_line();
        }

        // classify declaration nouns from their required continuation
        match keyword {
            Keyword::Let | Keyword::Using => true,
            Keyword::Const => probe.scan_const_declaration(!position.is_nested_statement()),
            Keyword::Struct | Keyword::Enum | Keyword::Interface => {
                probe.bump();

                matches!(
                    probe.peek_token_type(),
                    TokenType::Identifier | TokenType::Literal | TokenType::OpenBrace
                )
            }
            Keyword::Class => {
                probe.bump();

                matches!(
                    probe.peek_token_type(),
                    TokenType::Identifier | TokenType::Literal | TokenType::OpenBrace
                ) || probe.peek_keyword() == Some(Keyword::Extends)
            }
            Keyword::Function => {
                probe.bump();
                if probe.peek_token_type() == TokenType::Multiply {
                    probe.bump();
                }

                probe.peek_token_type() == TokenType::Identifier
            }
            Keyword::Extension => probe.scan_extension(),
            Keyword::Newtype => {
                probe.bump();
                probe.peek_keyword() == Some(Keyword::Interface) || probe.scan_type_alias()
            }
            Keyword::Type | Keyword::Readonly => {
                probe.bump();
                probe.scan_type_alias()
            }
            _ => false,
        }
    }

    /// Parse one declaration expression after its head has been classified.
    pub(in crate::parse::expression) fn parse_declaration_primary(
        &mut self,
        start: &ParseStart,
        position: ExpressionPosition,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let header = self.parse_declaration_header(DeclarationHeader::default())?;
        let decorators = self.parse_decorators();
        let header = self.parse_declaration_header(header)?;

        // parse contextual global after declaration modifiers
        if self.peek_identifier_is("global") && self.peek_next_token_type() == TokenType::OpenBrace
        {
            let main_range = self.peek_token().range();
            self.bump();
            let declaration = self.parse_global(start, main_range, header)?;

            return Ok(self.insert_declaration_expression(start, declaration));
        }

        let Some(keyword) = self.peek_keyword() else {
            return Err(ParserError::unexpected(self.peek_token_span()));
        };

        let expression = match keyword {
            Keyword::Let | Keyword::Const => {
                let const_noun = if keyword == Keyword::Const {
                    self.peek_next_keyword()
                } else {
                    None
                };
                match const_noun {
                    // const heads the const function form
                    Some(Keyword::Function) => {
                        let declaration = self.parse_function(start, header, position)?;
                        self.insert_declaration_expression(start, declaration)
                    }
                    _ => self.parse_let(start, header)?,
                }
            }
            Keyword::Using => self.parse_using(start, header, Asynchrony::Sync)?,
            Keyword::Await => self.parse_using(start, header, Asynchrony::Async)?,
            Keyword::Function | Keyword::Async => {
                let declaration = self.parse_function(start, header, position)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Struct | Keyword::Class => {
                let is_anonymous = keyword == Keyword::Class && !position.is_statement();
                let declaration = self.parse_struct_or_class(start, header, is_anonymous)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Enum => {
                let declaration = self.parse_enum(start, header)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Interface => {
                let declaration = self.parse_interface(start, header, TypeKind::Structural)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Extension => {
                let declaration = self.parse_extension(start, header)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Newtype if self.peek_next_keyword() == Some(Keyword::Interface) => {
                self.bump();
                let declaration = self.parse_interface(start, header, TypeKind::Nominal)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Type | Keyword::Readonly | Keyword::Newtype
                if self.peek_type_keyword_alias() =>
            {
                let keyword =
                    self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
                let type_keyword = TypeKeywordHeader::from_keyword(keyword)
                    .ok_or_else(|| ParserError::unexpected(self.peek_token().range()))?;
                let declaration = self.parse_type_alias(start, header, type_keyword)?;
                self.insert_declaration_expression(start, declaration)
            }
            _ => return Err(ParserError::unexpected(self.peek_token_span())),
        };

        // attach decorators to the exact declaration owner
        let owner = match self.tree.get(expression) {
            Expression::Declaration(declaration) => declaration.id,
            _ => expression.id,
        };
        self.attach_decorators(owner, decorators);

        Ok(expression)
    }

    /// Parse declaration modifiers into one declaration header.
    fn parse_declaration_header(
        &mut self,
        mut header: DeclarationHeader,
    ) -> ParserResult<DeclarationHeader> {
        while let Some(keyword) = self.peek_keyword() {
            let is_newline_allowed = keyword == Keyword::Export;

            match keyword {
                Keyword::Export if header.export.is_none() => {
                    self.bump();
                    header.export = Some(if self.peek_is_keyword(Keyword::Default) {
                        self.bump();
                        ExportKind::Default
                    } else {
                        ExportKind::Named
                    });
                }
                Keyword::Declare if !header.is_ambient => {
                    let span = self.eat().span;
                    header.is_ambient = true;
                    header.declare_range = Some(span.range());
                }
                Keyword::Abstract if !header.is_abstract => {
                    self.bump();
                    header.is_abstract = true;
                }
                Keyword::Final if !header.is_final => {
                    self.bump();
                    header.is_final = true;
                }
                Keyword::Shared if !header.is_shared => {
                    self.bump();
                    header.is_shared = true;
                }
                // report a repeated modifier and skip it
                Keyword::Export
                | Keyword::Declare
                | Keyword::Abstract
                | Keyword::Final
                | Keyword::Shared => {
                    let range = self.peek_token().range();
                    self.report_error(ParserError::unexpected(range));
                    self.bump();
                }
                _ => break,
            }

            if !is_newline_allowed && self.peek_is_on_new_line() {
                return Err(ParserError::unexpected(self.peek_token_span()));
            }
        }

        Ok(header)
    }
}

impl TokenProbe<'_> {
    /// Advance through declaration decorators and modifiers and return whether it is exported.
    pub(in crate::parse) fn scan_declaration_prefixes(&mut self) -> Option<bool> {
        let mut is_exported = false;

        // scan prefixes owned by the outer expression
        if !self.scan_decorators() {
            return None;
        }

        // scan prefixes owned by the declaration
        if !self.scan_declaration_header(&mut is_exported) {
            return None;
        }
        if !self.scan_decorators() {
            return None;
        }
        if !self.scan_declaration_header(&mut is_exported) {
            return None;
        }

        Some(is_exported)
    }

    /// Advance through one declaration header.
    fn scan_declaration_header(&mut self, is_exported: &mut bool) -> bool {
        while let Some(keyword) = self.peek_keyword() {
            let is_newline_allowed = keyword == Keyword::Export;

            // scan one recognized modifier
            match keyword {
                Keyword::Export => {
                    self.bump();
                    *is_exported = true;
                    if self.peek_keyword() == Some(Keyword::Default) {
                        self.bump();
                    }
                }
                Keyword::Declare | Keyword::Abstract | Keyword::Final | Keyword::Shared => {
                    self.bump()
                }
                _ => break,
            }

            // non-export modifiers bind to the declaration on the same line
            if !is_newline_allowed && self.peek_token().is_on_new_line() {
                return false;
            }
        }

        true
    }

    /// Return whether a const keyword heads a declaration.
    fn scan_const_declaration(&mut self, is_statement: bool) -> bool {
        self.bump();

        // const functions declare their own noun
        if self.peek_keyword() == Some(Keyword::Function) && !self.peek_token().is_on_new_line() {
            return true;
        }

        // a brace group binds only when a binding continuation follows it
        if self.peek_token_type() == TokenType::OpenBrace {
            if !self.scan_delimiter_group(TokenType::OpenBrace, TokenType::CloseBrace) {
                return true;
            }

            return matches!(self.peek_token_type(), TokenType::Assign | TokenType::Colon);
        }

        // statement position owns every other head, expressions evaluate
        is_statement
    }

    /// Return whether an extension keyword is followed by an extension declaration head.
    fn scan_extension(&mut self) -> bool {
        self.bump();
        if self.peek_keyword() == Some(Keyword::Of)
            || matches!(
                self.peek_token_type(),
                TokenType::LessThan | TokenType::ShiftLeft
            )
        {
            return true;
        }
        if self.peek_token_type() != TokenType::Identifier {
            return false;
        }

        self.bump();

        self.peek_keyword() == Some(Keyword::Of)
            || matches!(
                self.peek_token_type(),
                TokenType::LessThan | TokenType::ShiftLeft
            )
    }

    /// Return whether a consumed type keyword is followed by an alias declaration.
    fn scan_type_alias(&mut self) -> bool {
        if self.peek_token_type() != TokenType::Identifier || self.peek_token().is_on_new_line() {
            return false;
        }

        // accept a direct alias assignment
        self.bump();
        if self.peek_token_type() == TokenType::Assign {
            return true;
        }
        if self.peek_token_type() != TokenType::LessThan {
            return false;
        }

        // find the end of a generic parameter list
        let mut delimiters = DelimiterDepth::type_expression();
        loop {
            let token_type = self.peek_token_type();
            if token_type == TokenType::End {
                return true;
            }
            if !delimiters.advance(token_type) {
                return false;
            }
            self.bump();

            if delimiters.is_top_level() {
                return self.peek_token_type() == TokenType::Assign;
            }
        }
    }
}
