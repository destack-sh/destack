use crate::parse::{
    DeclarationHeader, PendingDecorators, is_declaration_keyword, is_declaration_prefix_keyword,
};
use crate::{Parser, ParserCheckpoint, ParserError, ParserResult, ParserSpanStart};
use destack_dir::{
    Asynchrony, Declaration, DependencyBinding, DependencyForm, DependencyItem, EnumKind,
    ExportKind, Expression, Keyword, LocalNodeId, PlaceModifier, TokenType, TypeKind,
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
        if !is_declaration_prefix_keyword(keyword) {
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
    #[inline(never)]
    pub(in crate::parse::expression) fn eat_keyword_declaration_expression(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        // parse direct binding expressions
        if keyword == Keyword::Let {
            return self.eat_let_from_keyword(start, header, keyword).map(Some);
        }
        if keyword == Keyword::Const {
            return self.eat_const_expression(start, header).map(Some);
        }
        if keyword == Keyword::Using {
            return self.try_eat_using_expression(start, header);
        }

        // parse declaration nodes
        let declaration = match keyword {
            // functions
            Keyword::Function | Keyword::Async | Keyword::Comptime => {
                self.try_eat_function_declaration(start, keyword, header)
            }

            // structs and classes
            Keyword::Struct => self.try_eat_struct_or_class_declaration(start, header, false),
            Keyword::Class => self.try_eat_struct_or_class_declaration(start, header, true),

            // enums
            Keyword::Enum => self.eat_enum_declaration(start, header).map(Some),

            // interfaces and extensions
            Keyword::Interface => self.try_eat_interface_declaration(start, header),
            Keyword::Extension => self.try_eat_extension_declaration(start, header),

            // type declarations
            Keyword::Newtype => self.try_eat_newtype_declaration(start, header),
            Keyword::Type | Keyword::Readonly => self.try_eat_type_alias_declaration(start, header),

            // not a declaration expression
            _ => Ok(None),
        }?;

        Ok(declaration.map(|declaration| self.declaration_expression(start, declaration)))
    }

    /// Parse a function declaration when the keyword owns a function head.
    #[inline(never)]
    fn try_eat_function_declaration(
        &mut self,
        start: &ParserSpanStart,
        keyword: Keyword,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        let has_function_head = if keyword == Keyword::Function {
            matches!(
                self.token_type_at_offset(1),
                TokenType::Identifier
                    | TokenType::Multiply
                    | TokenType::OpenParenthesis
                    | TokenType::LessThan
            )
        } else {
            self.next_keyword() == Some(Keyword::Function) && !self.next_token().is_on_new_line()
        };
        if !has_function_head {
            return Ok(None);
        }

        let declaration = self.eat_function(start, header)?;

        Ok(Some(declaration))
    }

    /// Parse a struct or class declaration when its nominal head follows.
    #[inline(never)]
    fn try_eat_struct_or_class_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
        is_class: bool,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        let has_nominal_head = matches!(
            self.token_type_at_offset(1),
            TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
        ) || self.keyword_at_offset(1) == Some(Keyword::Extends);
        if !has_nominal_head {
            return Ok(None);
        }

        let declaration = self.eat_struct_or_class(start, header, is_class)?;

        Ok(Some(declaration))
    }

    /// Parse an enum declaration.
    #[inline(never)]
    fn eat_enum_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Declaration>> {
        let has_enum_head = matches!(
            self.token_type_at_offset(1),
            TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
        );
        if !has_enum_head {
            return Err(ParserError::unexpected(self.peek()));
        }

        self.eat_enum(start, EnumKind::Enum, header)
    }

    /// Parse a const enum declaration or const binding expression.
    #[inline(never)]
    fn eat_const_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<LocalNodeId<Expression>> {
        let is_const_enum =
            self.next_keyword() == Some(Keyword::Enum) && !self.next_token().is_on_new_line();
        if !is_const_enum {
            return self.eat_let_from_keyword(start, header, Keyword::Const);
        }

        self.eat_keyword(Keyword::Const)?;
        let declaration = self.eat_enum(start, EnumKind::Const, header)?;

        Ok(self.declaration_expression(start, declaration))
    }

    /// Parse a structural interface declaration when its head follows.
    #[inline(never)]
    fn try_eat_interface_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        let has_interface_head = matches!(
            self.token_type_at_offset(1),
            TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
        );
        if !has_interface_head {
            return Ok(None);
        }

        let declaration = self.eat_interface(start, header, TypeKind::Structural)?;

        Ok(Some(declaration))
    }

    /// Parse an extension declaration when its head follows.
    #[inline(never)]
    fn try_eat_extension_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        let has_extension_head = matches!(
            self.token_type_at_offset(1),
            TokenType::Identifier | TokenType::LessThan | TokenType::OpenBrace
        );
        if !has_extension_head {
            return Ok(None);
        }

        let declaration = self.eat_extension(start, header)?;

        Ok(Some(declaration))
    }

    /// Parse a nominal interface or type alias declaration.
    #[inline(never)]
    fn try_eat_newtype_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        if self.next_keyword() != Some(Keyword::Interface) {
            return self.try_eat_type_alias_declaration(start, header);
        }

        self.eat_keyword(Keyword::Newtype)?;
        let declaration = self.eat_interface(start, header, TypeKind::Nominal)?;

        Ok(Some(declaration))
    }

    /// Parse a type alias declaration when the keyword owns the following head.
    #[inline(never)]
    fn try_eat_type_alias_declaration(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Declaration>>> {
        if !self.type_keyword_starts_alias_declaration() {
            return Ok(None);
        }

        let keyword = self.eat_keyword_in(&[Keyword::Type, Keyword::Readonly, Keyword::Newtype])?;
        let type_keyword = self.type_keyword_header(keyword)?;
        let declaration = self.eat_type_alias_declaration(start, header, type_keyword)?;

        Ok(Some(declaration))
    }

    /// Parse a using expression when its binding head follows.
    #[inline(never)]
    fn try_eat_using_expression(
        &mut self,
        start: &ParserSpanStart,
        header: DeclarationHeader,
    ) -> ParserResult<Option<LocalNodeId<Expression>>> {
        if !self.can_parse_using_declaration(&header, Asynchrony::Sync) {
            return Ok(None);
        }

        self.eat_using(start, header, Asynchrony::Sync).map(Some)
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

        let mut checkpoint =
            (!self.export_prefix_definitely_starts_declaration()).then(|| self.checkpoint());
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
            return Err(ParserError::unexpected(self.peek()));
        }

        // placement only applies to bindings and nominal declarations
        if header.place.is_some() && !self.current_declaration_allows_place_modifier(keyword) {
            return Err(ParserError::unexpected(self.peek()));
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
        let expression = self.eat_keyword_expression(start, keyword, header)?;
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
        checkpoint: Option<ParserCheckpoint>,
    ) -> ParserResult<()> {
        let Some(checkpoint) = checkpoint else {
            return Err(ParserError::unexpected(self.peek()));
        };

        self.restore(checkpoint);

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

            if declaration
                .keyword()
                .is_some_and(Self::is_place_modifier_keyword)
            {
                return self
                    .token_at_offset(3)
                    .keyword()
                    .is_some_and(Self::is_direct_export_declaration_keyword);
            }

            return declaration
                .keyword()
                .is_some_and(Self::is_direct_export_declaration_keyword);
        }

        if next_keyword == Some(Keyword::Async) {
            return self.token_at_offset(2).keyword() == Some(Keyword::Function);
        }

        if next_keyword.is_some_and(Self::is_place_modifier_keyword) {
            return self
                .token_at_offset(2)
                .keyword()
                .is_some_and(Self::is_direct_export_declaration_keyword);
        }

        next_keyword.is_some_and(Self::is_direct_export_declaration_keyword)
    }

    /// Return whether one keyword is a declaration placement modifier.
    fn is_place_modifier_keyword(keyword: Keyword) -> bool {
        matches!(keyword, Keyword::Local | Keyword::Shared)
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

    /// Return whether the current declaration accepts an explicit placement modifier.
    fn current_declaration_allows_place_modifier(&mut self, keyword: Keyword) -> bool {
        matches!(
            keyword,
            Keyword::Struct | Keyword::Class | Keyword::Enum | Keyword::Newtype | Keyword::Let
        ) || keyword == Keyword::Const
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
        let starts_comptime_function =
            keyword == Some(Keyword::Comptime) && self.next_keyword() == Some(Keyword::Function);
        let starts_placed_declaration = keyword.is_some_and(Self::is_place_modifier_keyword)
            && self
                .token_at_offset(1)
                .keyword()
                .is_some_and(Self::is_direct_export_declaration_keyword);

        keyword.is_some_and(is_declaration_keyword)
            || starts_async_function
            || starts_comptime_function
            || starts_placed_declaration
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
            Keyword::Final => {
                self.bump();
                header.is_final = true;

                Ok(true)
            }
            Keyword::Local => {
                if header.place.is_some() {
                    return Err(ParserError::unexpected(self.peek()));
                }

                self.bump();
                header.place = Some(PlaceModifier::Local);

                Ok(true)
            }
            Keyword::Shared => {
                if header.place.is_some() {
                    return Err(ParserError::unexpected(self.peek()));
                }

                self.bump();
                header.place = Some(PlaceModifier::Shared);

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

        // global declarations do not introduce local or shared storage
        if header.place.is_some() {
            return Err(ParserError::unexpected(self.peek()));
        }

        self.bump();
        let declaration = self.eat_global_body(start, header)?;
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
