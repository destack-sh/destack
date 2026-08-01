use crate::parse::context::{ExpressionContext, StatementPosition};
use crate::parse::lookahead::DelimiterDepth;
use crate::parse::{DeclarationHeader, TypeKeywordHeader};
use crate::{ParseStart, Parser, ParserError, ParserResult, TokenProbe};
use destack_dir::{
    Asynchrony, Declaration, EnumKind, ExportKind, Expression, Keyword, LocalNodeId, PlaceModifier,
    TokenType, TypeKind,
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
        context: ExpressionContext,
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
            Some(
                Keyword::Declare
                    | Keyword::Abstract
                    | Keyword::Final
                    | Keyword::Local
                    | Keyword::Shared
            )
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
        if context.statement == StatementPosition::Direct
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
        if matches!(keyword, Keyword::Async | Keyword::Comptime) {
            probe.bump();

            return probe.peek_keyword() == Some(Keyword::Function)
                && !probe.peek_token().is_on_new_line();
        }

        // classify declaration nouns from their required continuation
        match keyword {
            Keyword::Let | Keyword::Const | Keyword::Using => true,
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
        context: ExpressionContext,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let header = self.parse_declaration_header(DeclarationHeader::default())?;
        let decorators = self.parse_decorators(context.function);
        let header = self.parse_declaration_header(header)?;

        // parse contextual global after declaration modifiers
        if self.peek_identifier_is("global") && self.peek_next_token_type() == TokenType::OpenBrace
        {
            let main_range = self.peek_token().range();
            self.bump();
            let declaration = self.parse_global(start, main_range, header, context.function)?;

            return Ok(self.insert_declaration_expression(start, declaration));
        }

        let Some(keyword) = self.peek_keyword() else {
            return Err(ParserError::unexpected(self.peek_token_span()));
        };

        let expression = match keyword {
            Keyword::Let | Keyword::Const => {
                if keyword == Keyword::Const && self.peek_next_keyword() == Some(Keyword::Enum) {
                    self.bump();
                    let declaration =
                        self.parse_enum(start, EnumKind::Const, header, context.function)?;
                    self.insert_declaration_expression(start, declaration)
                } else {
                    self.parse_let(start, header, context.function)?
                }
            }
            Keyword::Using => {
                self.parse_using(start, header, Asynchrony::Sync, context.function)?
            }
            Keyword::Function | Keyword::Async | Keyword::Comptime => {
                let declaration = self.parse_function(start, header, context)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Struct | Keyword::Class => {
                let is_anonymous =
                    keyword == Keyword::Class && context.statement != StatementPosition::Direct;
                let declaration =
                    self.parse_struct_or_class(start, header, is_anonymous, context.function)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Enum => {
                let declaration =
                    self.parse_enum(start, EnumKind::Enum, header, context.function)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Interface => {
                let declaration =
                    self.parse_interface(start, header, TypeKind::Structural, context.function)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Extension => {
                let declaration = self.parse_extension(start, header, context.function)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Newtype if self.peek_next_keyword() == Some(Keyword::Interface) => {
                self.bump();
                let declaration =
                    self.parse_interface(start, header, TypeKind::Nominal, context.function)?;
                self.insert_declaration_expression(start, declaration)
            }
            Keyword::Type | Keyword::Readonly | Keyword::Newtype
                if self.peek_type_keyword_alias() =>
            {
                let keyword =
                    self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
                let type_keyword = TypeKeywordHeader::from_keyword(keyword)
                    .ok_or_else(|| ParserError::unexpected(self.peek_token().range()))?;
                let declaration =
                    self.parse_type_alias(start, header, type_keyword, context.function)?;
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
                Keyword::Export => {
                    self.bump();
                    header.export = Some(if self.peek_is_keyword(Keyword::Default) {
                        self.bump();
                        ExportKind::Default
                    } else {
                        ExportKind::Named
                    });
                }
                Keyword::Declare => {
                    let span = self.eat().span;
                    header.is_ambient = true;
                    header.declare_range = Some(span.range());
                }
                Keyword::Abstract => {
                    self.bump();
                    header.is_abstract = true;
                }
                Keyword::Final => {
                    self.bump();
                    header.is_final = true;
                }
                Keyword::Local => {
                    self.bump();
                    header.place = Some(PlaceModifier::Local);
                }
                Keyword::Shared => {
                    self.bump();
                    header.place = Some(PlaceModifier::Shared);
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
