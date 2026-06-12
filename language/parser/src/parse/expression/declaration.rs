use crate::parse::{DeclarationHeader, PendingDecorators, is_declaration_keyword};
use crate::{Parser, ParserCheckpoint, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{
    Asynchrony, Declaration, DependencyBinding, DependencyForm, DependencyItem, EnumKind,
    ExportKind, Expression, Keyword, LocalNodeId, TokenType, TypeKind,
};

/// The outcome of parsing an `export` declaration prefix.
enum ExportPrefix {
    /// The prefix belongs to a following declaration.
    Declaration,
    /// The prefix starts a standalone export expression.
    Expression,
    /// The prefix ended before a declaration head.
    LineBreak,
}

impl Parser {
    /// Build a declaration expression from an already parsed declaration.
    pub(in crate::parse::expression) fn declaration_expression(
        &mut self,
        start: &ParserSpanStart,
        declaration: LocalNodeId<Declaration>,
    ) -> LocalNodeId<Expression> {
        self.insert_node(
            Expression::Declaration(declaration),
            self.get_span_from(start),
        )
    }

    /// Eat a declaration prefix primary when present.
    ///
    /// Examples:
    /// ```ds
    /// export function value() {}
    /// declare class Value {}
    /// @sealed export class Value {}
    /// ```
    pub(in crate::parse::expression) fn eat_declaration_prefix_primary(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        let is_declaration_prefix = matches!(
            keyword,
            Keyword::Export | Keyword::Declare | Keyword::Abstract | Keyword::Final
        ) || self.language.is_destack() && keyword == Keyword::Shared;
        if !is_declaration_prefix {
            return Ok(None);
        }

        self.eat_declaration_prefixed_expression(start)
    }

    /// Parse declaration shaped keyword expressions.
    ///
    /// Examples:
    /// ```ds
    /// class Value {}
    /// function value() {}
    /// interface Shape {}
    /// ```
    pub(in crate::parse::expression) fn eat_keyword_declaration_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        // functions
        if keyword == Keyword::Function
            && matches!(
                self.token_type_at_offset(1),
                TokenType::Identifier
                    | TokenType::Multiply
                    | TokenType::OpenParenthesis
                    | TokenType::LessThan
            )
        {
            let declaration = self.eat_function(start, header)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // structs and classes
        else if matches!(keyword, Keyword::Struct | Keyword::Class)
            && (matches!(
                self.token_type_at_offset(1),
                TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
            ) || self.keyword_at_offset(1) == Some(Keyword::Extends))
        {
            let is_class = keyword == Keyword::Class;
            let declaration = self.eat_struct_or_class(start, header, is_class)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // enum declarations
        else if keyword == Keyword::Enum {
            if !matches!(
                self.token_type_at_offset(1),
                TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
            ) {
                return Err(ParserError::unexpected(self.peek()?.span));
            }

            let declaration = self.eat_enum(start, EnumKind::Enum, header)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // const enum declarations
        else if keyword == Keyword::Const
            && self.next_keyword() == Some(Keyword::Enum)
            && !self.next_token().is_on_new_line()
        {
            self.eat_keyword(Keyword::Const)?;
            let declaration = self.eat_enum(start, EnumKind::Const, header)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // interface declarations
        else if keyword == Keyword::Interface
            && matches!(
                self.token_type_at_offset(1),
                TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
            )
        {
            let declaration = self.eat_interface(start, header, TypeKind::Structural)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // extension declarations
        else if keyword == Keyword::Extension
            && matches!(
                self.token_type_at_offset(1),
                TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
            )
        {
            let declaration = self.eat_extension(start, header)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // binding declarations
        else if matches!(keyword, Keyword::Let | Keyword::Const) {
            self.eat_let_from_keyword(start, header, keyword).map(Some)
        }
        // nominal interfaces
        else if keyword == Keyword::Newtype && self.next_keyword() == Some(Keyword::Interface) {
            self.eat_keyword(Keyword::Newtype)?;
            let declaration = self.eat_interface(start, header, TypeKind::Nominal)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // type alias declarations
        else if matches!(
            keyword,
            Keyword::Type | Keyword::Newtype | Keyword::Readonly
        ) && self.type_keyword_starts_alias_declaration()
        {
            let keyword =
                self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
            let type_keyword = self.type_keyword_header(keyword)?;
            let declaration = self.eat_type_alias_declaration(start, header, type_keyword)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // using declarations
        else if keyword == Keyword::Using
            && self.can_parse_using_declaration(&header, Asynchrony::Sync)
        {
            self.eat_using(start, header, Asynchrony::Sync).map(Some)
        }
        // async and comptime function declarations
        else if (keyword == Keyword::Async
            || keyword == Keyword::Comptime && self.language.is_destack())
            && self.next_keyword() == Some(Keyword::Function)
            && !self.next_token().is_on_new_line()
        {
            let declaration = self.eat_function(start, header)?;

            Ok(Some(self.declaration_expression(start, declaration)))
        }
        // not a declaration expression
        else {
            Ok(None)
        }
    }

    /// Parse declaration prefix modifiers before a keyword expression.
    ///
    /// Examples:
    /// ```ds
    /// export default value
    /// declare export class Value {}
    /// @sealed export default class Value {}
    /// ```
    fn eat_declaration_prefixed_expression(
        &mut self,
        start: &ParserSpanStart,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if self.current_token_starts_export_clause() {
            return self.eat_export().map(Some);
        }

        let mut checkpoint = (!self.export_prefix_definitely_starts_declaration())
            .then(|| (self.checkpoint(), self.tree.next_id()));
        let mut header = DeclarationHeader::default();

        // export prefix
        match self.eat_export_prefix(&mut header)? {
            ExportPrefix::Declaration => {}
            ExportPrefix::Expression => {
                self.restore_export_checkpoint(checkpoint.take())?;
                let expression = self.eat_export()?;

                return Ok(Some(expression));
            }
            ExportPrefix::LineBreak => return Ok(None),
        }

        // decorators and modifiers
        let mut decorators = if self.peek_is(TokenType::At) {
            self.eat_decorators_maybe()?
        } else {
            PendingDecorators::new()
        };
        self.eat_declaration_prefix_modifiers(&mut header)?;

        // modifier line boundary
        if header.has_modifier() && self.current_token_is_on_new_line() {
            self.restore_export_checkpoint(checkpoint.take())?;
            return Ok(None);
        }

        // declaration keyword
        let Some(keyword) = self.current_keyword() else {
            if let Some(expression) = self.eat_prefixed_global_expression(start, header)? {
                return Ok(Some(expression));
            }

            self.restore_export_checkpoint(checkpoint.take())?;
            return Ok(None);
        };

        // invalid default enum
        if keyword == Keyword::Enum && header.export == Some(ExportKind::Default) {
            return Err(ParserError::unexpected(self.peek()?.span));
        }

        // ambient enum split by newline
        if keyword == Keyword::Enum
            && header.declare_span.is_some()
            && self.token_at_offset(1).is_on_new_line()
        {
            self.restore_export_checkpoint(checkpoint.take())?;
            return Ok(None);
        }

        // keyword declaration
        let expression = self.eat_keyword_expression_with_header(start, keyword, header)?;
        if let Some(expression_id) = expression {
            self.attach_pending_decorators_to_expression(&mut decorators, expression_id);

            let wraps_default_export = header.export == Some(ExportKind::Default)
                && !matches!(self.tree.get(expression_id), Expression::Declaration(_));
            if wraps_default_export {
                return Ok(Some(
                    self.wrap_default_export_expression(start, expression_id),
                ));
            }

            return Ok(Some(expression_id));
        }

        self.restore_export_checkpoint(checkpoint.take())?;

        Ok(None)
    }

    /// Restore a speculative export prefix checkpoint.
    fn restore_export_checkpoint(
        &mut self,
        checkpoint: Option<(ParserCheckpoint, u32)>,
    ) -> ParserResult<()> {
        let Some((checkpoint, mark)) = checkpoint else {
            return Err(ParserError::unexpected(self.peek()?.span));
        };

        self.restore(checkpoint, mark);

        Ok(())
    }

    /// Parse an optional export prefix before a declaration.
    ///
    /// Examples:
    /// ```ds
    /// export default
    /// export type
    /// export
    /// ```
    fn eat_export_prefix(&mut self, header: &mut DeclarationHeader) -> ParserResult<ExportPrefix> {
        if self.current_keyword() != Some(Keyword::Export) {
            return Ok(ExportPrefix::Declaration);
        }

        // export kind
        self.bump();
        if self.current_keyword() == Some(Keyword::Default) {
            self.bump();
            header.export = Some(ExportKind::Default);
        } else if self.peek_is(TokenType::Assign) {
            return Ok(ExportPrefix::Expression);
        } else {
            header.export = Some(ExportKind::Named);
        }

        // export clause
        if self.current_export_type_starts_clause() {
            return Ok(ExportPrefix::Expression);
        }

        // declaration head
        if self.current_token_starts_exported_declaration() {
            return Ok(ExportPrefix::Declaration);
        }

        // line boundary
        if self.current_token_is_on_new_line() {
            return Ok(ExportPrefix::LineBreak);
        }

        Ok(ExportPrefix::Expression)
    }

    /// Return whether the current cursor starts a standalone export clause.
    fn current_token_starts_export_clause(&mut self) -> bool {
        if self.current_keyword() != Some(Keyword::Export) {
            return false;
        }

        let next_token = self.token_at_offset(1);
        if matches!(
            next_token.ty(),
            TokenType::OpenBrace | TokenType::Multiply | TokenType::Assign
        ) {
            return true;
        }

        if next_token.keyword() != Some(Keyword::Type) {
            return false;
        }

        let after_type = self.token_at_offset(2);

        matches!(
            after_type.ty(),
            TokenType::OpenBrace | TokenType::Multiply | TokenType::End
        )
    }

    /// Return whether the current export prefix can only start a declaration.
    fn export_prefix_definitely_starts_declaration(&mut self) -> bool {
        if self.current_keyword() != Some(Keyword::Export) {
            return false;
        }

        let next_token = self.token_at_offset(1);
        if next_token.is_on_new_line() {
            return false;
        }

        if matches!(
            next_token.ty(),
            TokenType::OpenBrace | TokenType::Multiply | TokenType::Assign
        ) {
            return false;
        }

        let next_keyword = next_token.keyword();
        if next_keyword == Some(Keyword::Default) {
            let declaration = self.token_at_offset(2);
            if declaration.is_on_new_line() {
                return false;
            }

            return declaration
                .keyword()
                .is_some_and(Self::is_direct_export_declaration_keyword);
        }

        if next_keyword == Some(Keyword::Async) {
            return self.token_at_offset(2).keyword() == Some(Keyword::Function);
        }

        next_keyword.is_some_and(Self::is_direct_export_declaration_keyword)
    }

    /// Return whether one keyword directly starts an exported declaration.
    fn is_direct_export_declaration_keyword(keyword: Keyword) -> bool {
        matches!(
            keyword,
            Keyword::Struct
                | Keyword::Class
                | Keyword::Enum
                | Keyword::Function
                | Keyword::Interface
                | Keyword::Type
                | Keyword::Newtype
                | Keyword::Const
                | Keyword::Let
                | Keyword::Using
        )
    }

    /// Return whether `export type` starts an export clause.
    fn current_export_type_starts_clause(&mut self) -> bool {
        self.current_keyword() == Some(Keyword::Type)
            && matches!(
                self.token_type_at_offset(1),
                TokenType::OpenBrace | TokenType::Multiply | TokenType::End
            )
    }

    /// Return whether the current cursor starts an exported declaration.
    fn current_token_starts_exported_declaration(&mut self) -> bool {
        let keyword = self.current_keyword();
        let starts_async_function = (keyword == Some(Keyword::Async)
            || self.current_identifier_str_is("async"))
            && self.next_keyword() == Some(Keyword::Function);
        let starts_comptime_function = self.language.is_destack()
            && keyword == Some(Keyword::Comptime)
            && self.next_keyword() == Some(Keyword::Function);

        keyword.is_some_and(is_declaration_keyword)
            || starts_async_function
            || starts_comptime_function
            || self.peek_is(TokenType::At)
    }

    /// Parse declaration modifiers after export and decorators.
    ///
    /// Examples:
    /// ```ds
    /// declare abstract
    /// export default
    /// async function
    /// ```
    fn eat_declaration_prefix_modifiers(
        &mut self,
        header: &mut DeclarationHeader,
    ) -> ParserResult<()> {
        while let Some(keyword) = self.current_keyword() {
            if !self.eat_declaration_prefix_modifier(header, keyword)? {
                break;
            }
        }

        Ok(())
    }

    /// Parse one declaration prefix modifier.
    ///
    /// Examples:
    /// ```ds
    /// declare
    /// export
    /// async
    /// ```
    fn eat_declaration_prefix_modifier(
        &mut self,
        header: &mut DeclarationHeader,
        keyword: Keyword,
    ) -> ParserResult<bool> {
        match keyword {
            Keyword::Declare => {
                let span = self.eat_keyword(Keyword::Declare)?.span;
                header.is_ambient = true;
                header.declare_span = Some(span);

                Ok(true)
            }
            Keyword::Abstract => {
                self.bump();
                header.is_abstract = true;

                Ok(true)
            }
            Keyword::Final if self.language.is_destack() => {
                self.bump();
                header.is_final = true;

                Ok(true)
            }
            Keyword::Shared if self.language.is_destack() => {
                self.bump();
                header.is_shared = true;

                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Parse a prefixed global declaration expression.
    ///
    /// Examples:
    /// ```ds
    /// declare global {}
    /// global {}
    /// export global {}
    /// ```
    fn eat_prefixed_global_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if !self.is_global_identifier() || self.next_token_type() != TokenType::OpenBrace {
            return Ok(None);
        }

        let declaration = self.eat_global(start, header)?;
        let expression = self.declaration_expression(start, declaration);

        Ok(Some(expression))
    }

    /// Wrap an expression in an `export default` dependency expression.
    fn wrap_default_export_expression(
        &mut self,
        start: &ParserSpanStart,
        value: LocalNodeId<Expression>,
    ) -> LocalNodeId<Expression> {
        let item = self.insert_node(
            DependencyItem::Binding {
                binding: DependencyBinding::Default,
                form: Some(DependencyForm::Plain),
                name: None,
                alias: None,
                value: Some(value),
            },
            self.get_span_from(start),
        );

        self.insert_node(
            Expression::Export {
                form: DependencyForm::Plain,
                target: None,
                items: vec![item],
                attributes: None,
            },
            self.get_span_from(start),
        )
    }
}
