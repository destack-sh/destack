use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, NodeSpanList, NodeSpanRegion, NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::{Module, ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

/// Semantic token type for LSP semantic highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum SemanticTokenType {
    /// Namespace token.
    Namespace,
    /// Type token.
    Type,
    /// Class token.
    Class,
    /// Enum token.
    Enum,
    /// Interface token.
    Interface,
    /// Struct token.
    Struct,
    /// Type parameter token.
    TypeParameter,
    /// Parameter token.
    Parameter,
    /// Variable token.
    Variable,
    /// Property token.
    Property,
    /// Enum member token.
    EnumMember,
    /// Function token.
    Function,
    /// Method token.
    Method,
    /// Comment token.
    Comment,
    /// Decorator token.
    Decorator,
    /// Label token.
    Label,
}

impl SemanticTokenType {
    /// Return the token type for one declaration.
    fn declaration(declaration: &dir::Declaration) -> Self {
        match declaration {
            dir::Declaration::Global(_) | dir::Declaration::Module(_) => Self::Namespace,
            dir::Declaration::Function(_) => Self::Function,
            dir::Declaration::Struct(_) => Self::Struct,
            dir::Declaration::Class(_) => Self::Class,
            dir::Declaration::Interface(_) => Self::Interface,
            dir::Declaration::Enum(_) => Self::Enum,
            dir::Declaration::Type(_) | dir::Declaration::Extension(_) => Self::Type,
        }
    }
}

impl TryFrom<dir::SymbolKind> for SemanticTokenType {
    type Error = QueryError;

    /// Convert one declaration kind into its semantic token type.
    fn try_from(symbol_kind: dir::SymbolKind) -> Result<Self, Self::Error> {
        match symbol_kind {
            dir::SymbolKind::Variable
            | dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::GenericValueParameter => Ok(Self::Variable),
            dir::SymbolKind::Parameter => Ok(Self::Parameter),
            dir::SymbolKind::Class => Ok(Self::Class),
            dir::SymbolKind::Struct => Ok(Self::Struct),
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => Ok(Self::Interface),
            dir::SymbolKind::Enum => Ok(Self::Enum),
            dir::SymbolKind::Variant => Ok(Self::EnumMember),
            dir::SymbolKind::Function => Ok(Self::Function),
            dir::SymbolKind::Label => Ok(Self::Label),
            dir::SymbolKind::Import => Ok(Self::Variable),
            dir::SymbolKind::Extension => Ok(Self::Type),
            dir::SymbolKind::AssociatedType
            | dir::SymbolKind::TypeAlias
            | dir::SymbolKind::Newtype => Ok(Self::Type),
            dir::SymbolKind::GenericTypeParameter => Ok(Self::TypeParameter),
            dir::SymbolKind::ExportAlias => Err(QueryError::invalid(
                "export alias has no selected declaration classification",
            )),
        }
    }
}

/// Semantic token modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
#[serde(transparent)]
pub struct SemanticTokenModifiers(u32);

impl SemanticTokenModifiers {
    /// No modifiers.
    pub const NONE: Self = Self(0);
    /// Declaration modifier.
    pub const DECLARATION: Self = Self(1 << 0);
    /// Readonly modifier.
    pub const READONLY: Self = Self(1 << 1);
    /// Static modifier.
    pub const STATIC: Self = Self(1 << 2);
    /// Deprecated modifier.
    pub const DEPRECATED: Self = Self(1 << 3);
    /// Abstract modifier.
    pub const ABSTRACT: Self = Self(1 << 4);
    /// Async modifier.
    pub const ASYNC: Self = Self(1 << 5);
    /// Modification modifier.
    pub const MODIFICATION: Self = Self(1 << 6);
    /// Documentation modifier.
    pub const DOCUMENTATION: Self = Self(1 << 7);
    /// Default library modifier.
    pub const DEFAULT_LIBRARY: Self = Self(1 << 8);

    /// Return the union of two modifier sets.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Return the intersection of two modifier sets.
    fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// Return whether this modifier set contains another set.
    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Return the raw modifier bits.
    pub fn bits(self) -> u32 {
        self.0
    }

    /// Return modifiers from explicit member flags.
    fn member(is_declaration: bool, is_readonly: bool, is_static: bool, is_abstract: bool) -> Self {
        let mut modifiers = if is_declaration {
            Self::DECLARATION
        } else {
            Self::NONE
        };

        if is_readonly {
            modifiers = modifiers.union(Self::READONLY);
        }
        if is_static {
            modifiers = modifiers.union(Self::STATIC);
        }
        if is_abstract {
            modifiers = modifiers.union(Self::ABSTRACT);
        }

        modifiers
    }
}

/// A single semantic token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticToken {
    /// The span of the token.
    pub span: Span,
    /// The type of the token.
    pub token_type: SemanticTokenType,
    /// The modifiers of the token.
    pub modifiers: SemanticTokenModifiers,
}

impl SemanticToken {
    /// Create a semantic token.
    fn new(span: Span, token_type: SemanticTokenType, modifiers: SemanticTokenModifiers) -> Self {
        Self {
            span,
            token_type,
            modifiers,
        }
    }
}

/// A semantic tokens request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRequest {
    /// The queried module profile.
    pub module: Module,
    /// The queried source file.
    pub file_id: FileId,
}

/// A semantic tokens response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensResponse {
    /// Semantic tokens.
    pub tokens: Vec<SemanticToken>,
}

impl ModuleQueryContext<'_> {
    /// Return semantic tokens for a module file.
    pub fn semantic_tokens(
        &self,
        request: SemanticTokensRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<SemanticTokensResponse> {
        let tokens = SemanticTokens::collect(self, program, request.file_id)?;

        Ok(SemanticTokensResponse { tokens })
    }
}

/// Semantic token collection for one module.
struct SemanticTokens<'owner, 'module, 'program> {
    /// The queried module.
    module: &'owner ModuleQueryContext<'module>,
    /// The shared program context.
    program: &'owner ProgramQueryContext<'program>,
    /// The queried source file.
    file_id: FileId,
    /// The collected tokens.
    tokens: Vec<SemanticToken>,
}

impl<'owner, 'module, 'program> SemanticTokens<'owner, 'module, 'program> {
    /// Collect semantic tokens for one source file.
    fn collect(
        module: &'owner ModuleQueryContext<'module>,
        program: &'owner ProgramQueryContext<'program>,
        file_id: FileId,
    ) -> QueryResult<Vec<SemanticToken>> {
        let mut semantic_tokens = Self {
            module,
            program,
            file_id,
            tokens: Vec::new(),
        };

        // collect all token families
        semantic_tokens.collect_declarations()?;
        semantic_tokens.collect_parameters()?;
        semantic_tokens.collect_pattern_bindings()?;
        semantic_tokens.collect_pattern_fields()?;
        semantic_tokens.collect_expressions()?;
        semantic_tokens.collect_modifications()?;
        semantic_tokens.collect_members()?;
        semantic_tokens.collect_type_members()?;
        semantic_tokens.collect_enum_fields()?;
        semantic_tokens.collect_type_parameters()?;
        semantic_tokens.collect_type_references()?;
        semantic_tokens.collect_decorators()?;
        semantic_tokens.collect_documentation_comments()?;
        semantic_tokens.collect_dependency_items()?;

        semantic_tokens.finish()
    }

    /// Finish collection with filtering, ordering, and exact modifier merging.
    fn finish(mut self) -> QueryResult<Vec<SemanticToken>> {
        // retain authored nonempty spans in the queried file
        self.tokens
            .retain(|token| token.span.file == self.file_id && token.span.start < token.span.end);

        // order tokens by complete source span
        self.tokens
            .sort_by_key(|token| (token.span.start, token.span.end));
        let mut tokens: Vec<SemanticToken> = Vec::with_capacity(self.tokens.len());
        for token in self.tokens {
            let Some(previous) = tokens.last_mut() else {
                tokens.push(token);
                continue;
            };
            if previous.span == token.span && previous.token_type == token.token_type {
                previous.modifiers = previous.modifiers.union(token.modifiers);
            } else if token.span.start < previous.span.end {
                return Err(QueryError::conflict(format!(
                    "semantic tokens: {:?}, {:?}",
                    previous.span, token.span
                )));
            } else {
                tokens.push(token);
            }
        }

        Ok(tokens)
    }

    /// Collect declaration tokens.
    fn collect_declarations(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect definition-site declaration names
        for (declaration_id, declaration) in view.iter_nodes::<dir::Declaration>() {
            if declaration.name().is_none() {
                continue;
            }

            let Some(main_span) = self.main_span(declaration_id.into_any())? else {
                continue;
            };

            let mut modifiers = SemanticTokenModifiers::DECLARATION;
            if declaration.is_abstract() {
                modifiers = modifiers.union(SemanticTokenModifiers::ABSTRACT);
            }
            if let dir::Declaration::Function(declaration) = declaration
                && declaration.signature.asynchrony == dir::Asynchrony::Async
            {
                modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
            }
            modifiers = modifiers.union(self.node_symbol_modifiers(declaration_id.into_any())?);

            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::declaration(declaration),
                modifiers,
            ));
        }

        Ok(())
    }

    /// Collect parameter tokens.
    fn collect_parameters(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect parameter declarations
        for (parameter_id, _) in view.iter_nodes::<dir::Parameter>() {
            let Some(name_span) = self.main_span(parameter_id.into_any())? else {
                continue;
            };
            let modifiers = SemanticTokenModifiers::DECLARATION
                .union(self.node_symbol_modifiers(parameter_id.into_any())?);

            self.tokens.push(SemanticToken::new(
                name_span,
                SemanticTokenType::Parameter,
                modifiers,
            ));
        }

        Ok(())
    }

    /// Collect local variable binding tokens.
    fn collect_pattern_bindings(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect binding patterns
        for (pattern_id, pattern) in view.iter_nodes::<dir::Pattern>() {
            if !matches!(pattern, dir::Pattern::Binding { .. }) {
                continue;
            }

            let Some(main_span) = self.main_span(pattern_id.into_any())? else {
                continue;
            };

            let modifiers = self.binding_modifiers(pattern_id.into_any())?;
            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::Variable,
                modifiers.union(SemanticTokenModifiers::DECLARATION),
            ));
        }

        Ok(())
    }

    /// Collect destructuring pattern field tokens.
    fn collect_pattern_fields(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect destructuring names from their exact roles
        for (field_id, field) in view.iter_nodes::<dir::PatternField>() {
            let dir::PatternField::Named { pattern, .. } = field else {
                continue;
            };

            let Some(main_span) = self.main_span(field_id.into_any())? else {
                continue;
            };

            // shorthand names introduce bindings
            if pattern.is_none() {
                let modifiers = self.binding_modifiers(field_id.into_any())?;
                self.tokens.push(SemanticToken::new(
                    main_span,
                    SemanticTokenType::Variable,
                    modifiers.union(SemanticTokenModifiers::DECLARATION),
                ));

                continue;
            }

            // explicit names select the recorded field projection
            let parent = view
                .get_parent_for(field_id)
                .ok_or(QueryError::missing(format!(
                    "semantic token pattern field parent: {:?}",
                    field_id.into_global_any(self.module.module_id())
                )))?;
            if parent.ty != dir::NodeType::Pattern {
                return Err(QueryError::invalid(format!(
                    "semantic token pattern field parent: {:?}, {parent:?}",
                    field_id.into_global_any(self.module.module_id())
                )));
            }
            let field = self.pattern_field_resolution(parent, field_id)?;
            let Some((token_type, modifiers)) = self.field_projection_token(&field.projection)?
            else {
                continue;
            };

            self.tokens
                .push(SemanticToken::new(main_span, token_type, modifiers));
        }

        Ok(())
    }

    /// Return the recorded projection for one destructuring field.
    fn pattern_field_resolution(
        &self,
        pattern: dir::LocalNodeIdAny,
        field: dir::LocalNodeId<dir::PatternField>,
    ) -> QueryResult<&dir::PatternFieldResolution> {
        let pattern = pattern.into_global(self.module.module_id());
        let resolution = self
            .module
            .resolutions()?
            .pattern_resolution(pattern)
            .ok_or(QueryError::missing(format!(
                "semantic token pattern: {pattern:?}"
            )))?;
        let fields = match resolution {
            dir::PatternResolution::Variant(resolution) => &resolution.fields,
            dir::PatternResolution::Destructure(resolution) => match resolution.as_ref() {
                dir::PatternDestructureResolution::Object(resolution) => &resolution.fields,
                dir::PatternDestructureResolution::Nominal(resolution) => &resolution.fields,
                dir::PatternDestructureResolution::Tuple(_)
                | dir::PatternDestructureResolution::Sequence(_) => {
                    return Err(QueryError::invalid(format!(
                        "semantic token named pattern: {pattern:?}"
                    )));
                }
            },
            _ => {
                return Err(QueryError::invalid(format!(
                    "semantic token named pattern: {pattern:?}"
                )));
            }
        };
        let field = field.into_global_any(self.module.module_id());

        fields
            .iter()
            .find(|resolution| resolution.source == field)
            .ok_or(QueryError::missing(format!(
                "semantic token pattern field: {field:?}"
            )))
    }

    /// Return the semantic token selected by one field projection.
    fn field_projection_token(
        &self,
        resolution: &dir::ProjectionResolution,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        let mut token: Option<(SemanticTokenType, SemanticTokenModifiers)> = None;
        for projection in resolution.iter() {
            let candidate = match projection {
                dir::Projection::Absent { .. } => {
                    Some((SemanticTokenType::Property, SemanticTokenModifiers::NONE))
                }
                dir::Projection::Field(field) => match field.target {
                    dir::FieldTarget::Structural { .. } => {
                        Some((SemanticTokenType::Property, SemanticTokenModifiers::NONE))
                    }
                    dir::FieldTarget::Member { symbol, .. } => self.symbol_token(symbol)?,
                },
                dir::Projection::Member(access) => {
                    let mut symbols = Vec::new();
                    access.target.collect_symbols(&mut symbols);
                    if symbols.is_empty() {
                        Some((SemanticTokenType::Property, SemanticTokenModifiers::NONE))
                    } else {
                        self.symbol_targets_token(&symbols)?
                    }
                }
                dir::Projection::Call(call) => match call.target.symbol() {
                    Some(symbol) => self.symbol_token(symbol)?,
                    None => Some((SemanticTokenType::Property, SemanticTokenModifiers::NONE)),
                },
                dir::Projection::Subscript(_)
                | dir::Projection::ObjectRest { .. }
                | dir::Projection::SliceLength { .. }
                | dir::Projection::DynamicPayload { .. }
                | dir::Projection::DynamicType { .. }
                | dir::Projection::VariantTag { .. }
                | dir::Projection::VariantPayload { .. }
                | dir::Projection::NewtypePayload { .. }
                | dir::Projection::Borrow { .. }
                | dir::Projection::Move { .. }
                | dir::Projection::Dereference(_)
                | dir::Projection::Copy { .. } => {
                    return Err(QueryError::invalid(format!(
                        "semantic token field projection: {projection:?}"
                    )));
                }
            };
            let Some(candidate) = candidate else {
                return Ok(None);
            };
            if token.is_some_and(|(token_type, _)| token_type != candidate.0) {
                return Ok(None);
            }

            token = Some(match token {
                Some((token_type, modifiers)) => (token_type, modifiers.intersection(candidate.1)),
                None => candidate,
            });
        }

        Ok(token)
    }

    /// Collect expression tokens.
    fn collect_expressions(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect labels and visible reference segments
        for (expression_id, expression) in view.iter_nodes::<dir::Expression>() {
            if self.is_decorator_name(expression_id)? {
                continue;
            }

            match expression {
                dir::Expression::Label { .. } => {
                    if let Some(main_span) = self.main_span(expression_id.into_any())? {
                        self.tokens.push(SemanticToken::new(
                            main_span,
                            SemanticTokenType::Label,
                            SemanticTokenModifiers::DECLARATION,
                        ));
                    }
                }
                dir::Expression::Identifier { .. } | dir::Expression::Member { .. } => {
                    let node_id = expression_id.into_global_any(self.module.module_id());
                    let Some((token_type, modifiers)) = self.reference_token(node_id)? else {
                        continue;
                    };
                    let Some(main_span) = self.main_span(expression_id.into_any())? else {
                        continue;
                    };

                    self.tokens
                        .push(SemanticToken::new(main_span, token_type, modifiers));
                }
                dir::Expression::Break { label: Some(_), .. }
                | dir::Expression::Continue { label: Some(_) } => {
                    let node_id = expression_id.into_global_any(self.module.module_id());
                    let (token_type, modifiers) = self
                        .reference_token(node_id)?
                        .ok_or(QueryError::missing(format!("label target: {node_id:?}")))?;
                    let Some(main_span) = self.main_span(expression_id.into_any())? else {
                        continue;
                    };

                    self.tokens
                        .push(SemanticToken::new(main_span, token_type, modifiers));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Collect writable place tokens selected during checking.
    fn collect_modifications(&mut self) -> QueryResult<()> {
        // collect each checked writable place
        for (target, write) in self.module.writable_places()? {
            self.push_modification(target, write)?;
        }

        Ok(())
    }

    /// Push one exact writable place token.
    fn push_modification(
        &mut self,
        source: dir::GlobalNodeIdAny,
        write: &dir::WriteResolution,
    ) -> QueryResult<()> {
        let Some((token_type, modifiers)) = self.write_token(write)? else {
            return Ok(());
        };
        if source.module_id != self.module.module_id() {
            return Err(QueryError::invalid(format!(
                "semantic token source: {:?}, {:?}",
                source,
                self.module.module_id()
            )));
        }

        // map the checked node through the visible tree to its authored span
        let source_id = self.module.view()?.get_source_any(source.local_id);
        let span = self
            .module
            .source_index()?
            .get_main(source_id)
            .ok_or(QueryError::missing(format!(
                "semantic token span: {source:?}"
            )))?;
        let modifiers = modifiers.union(SemanticTokenModifiers::MODIFICATION);
        self.tokens
            .push(SemanticToken::new(span, token_type, modifiers));

        Ok(())
    }

    /// Return the semantic token selected by one writable place.
    fn write_token(
        &self,
        write: &dir::WriteResolution,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        match write {
            dir::WriteResolution::Binding { symbol, .. } => self.symbol_token(*symbol),
            dir::WriteResolution::Member(member) => self.member_resolution_token(member),
            dir::WriteResolution::Subscript(_) | dir::WriteResolution::Dereference(_) => Ok(None),
        }
    }

    /// Return the semantic token selected by one member resolution.
    fn member_resolution_token(
        &self,
        resolution: &dir::MemberResolution,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        let symbols = resolution.target_symbols();
        if !symbols.is_empty() {
            return self.symbol_targets_token(&symbols);
        }

        // symbol-free selections tokenize structural members as properties
        let structural = resolution.iter().any(|access| {
            matches!(
                access.target,
                dir::MemberTarget::Field(_) | dir::MemberTarget::Projection { .. }
            )
        });
        if structural {
            return Ok(Some((
                SemanticTokenType::Property,
                SemanticTokenModifiers::NONE,
            )));
        }

        Ok(None)
    }

    /// Return the exact token type recorded for one expression reference.
    fn reference_token(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        // explicit label transfers carry local symbol identity directly
        if self
            .module
            .resolutions()?
            .label_resolution(node_id)
            .is_some()
        {
            return Ok(Some((
                SemanticTokenType::Label,
                SemanticTokenModifiers::NONE,
            )));
        }

        // namespace values have module rather than symbol identity
        if matches!(
            self.module.resolved()?.references.get(node_id),
            Some(dir::Reference::Namespace(_))
        ) {
            return Ok(Some((
                SemanticTokenType::Namespace,
                SemanticTokenModifiers::NONE,
            )));
        }

        // use the complete checked member selection when present
        if let Some(resolution) = self.module.resolutions()?.member_resolution(node_id) {
            return self.member_resolution_token(resolution);
        }

        // classify remaining references from their recorded symbol identities
        let Some(symbols) = self.module.symbol_targets(node_id)? else {
            return Ok(None);
        };

        self.symbol_targets_token(&symbols)
    }

    /// Return one classification shared by every selected symbol.
    fn symbol_targets_token(
        &self,
        symbols: &[dir::GlobalSymbolId],
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        let mut token: Option<(SemanticTokenType, SemanticTokenModifiers)> = None;
        for symbol_id in symbols {
            for symbol_id in self.program.symbol_targets(*symbol_id)? {
                let Some(candidate) = self.target_symbol_token(symbol_id)? else {
                    return Ok(None);
                };
                if token.is_some_and(|(token_type, _)| token_type != candidate.0) {
                    return Ok(None);
                }

                token = Some(match token {
                    Some((token_type, modifiers)) => {
                        (token_type, modifiers.intersection(candidate.1))
                    }
                    None => candidate,
                });
            }
        }

        Ok(token)
    }

    /// Return the token classification for one symbol target.
    fn symbol_token(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        self.symbol_targets_token(&[symbol_id])
    }

    /// Return the token classification for one target symbol.
    fn target_symbol_token(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        let symbol_module = self.program.module(symbol_id.module_id)?;
        let symbols = symbol_module.bindings()?;
        let symbol = symbols.get_symbol(symbol_id.local_id);
        let symbol_modifiers = self.symbol_modifiers(symbol_id)?;

        // classify member and parameter declarations by their exact source role
        if let Some(declaration) = symbol.declaration {
            match declaration.local_id.ty {
                dir::NodeType::Member => {
                    let member_id = dir::LocalNodeId::<dir::Member>::new(declaration.local_id.id);
                    let member = symbol_module.view()?.get(member_id);
                    let token_type = match symbol.kind {
                        dir::SymbolKind::Function => SemanticTokenType::Method,
                        dir::SymbolKind::AssociatedType => SemanticTokenType::Type,
                        dir::SymbolKind::Variant => SemanticTokenType::EnumMember,
                        _ => SemanticTokenType::Property,
                    };
                    let modifiers =
                        Self::member_reference_modifiers(member).union(symbol_modifiers);

                    return Ok(Some((token_type, modifiers)));
                }
                dir::NodeType::TypeMember => {
                    let token_type = match symbol.kind {
                        dir::SymbolKind::Function => SemanticTokenType::Method,
                        dir::SymbolKind::AssociatedType => SemanticTokenType::Type,
                        dir::SymbolKind::Variant => SemanticTokenType::EnumMember,
                        _ => SemanticTokenType::Property,
                    };

                    return Ok(Some((token_type, symbol_modifiers)));
                }
                dir::NodeType::Parameter => {
                    return Ok(Some((SemanticTokenType::Parameter, symbol_modifiers)));
                }
                _ => {}
            }
        }

        let token_type = SemanticTokenType::try_from(symbol.kind)?;
        let source_modifiers = match symbol.declaration {
            Some(declaration) => {
                Self::declaration_reference_modifiers(&symbol_module, declaration)?
            }
            None => SemanticTokenModifiers::NONE,
        };
        let modifiers = match symbol.kind {
            dir::SymbolKind::GenericValueParameter => SemanticTokenModifiers::READONLY,
            dir::SymbolKind::Variable
                if symbol.binding_mutability == Some(dir::Mutability::Immutable) =>
            {
                SemanticTokenModifiers::READONLY
            }
            _ => SemanticTokenModifiers::NONE,
        };

        Ok(Some((
            token_type,
            modifiers.union(source_modifiers).union(symbol_modifiers),
        )))
    }

    /// Return source modifiers carried from one declaration to its references.
    fn declaration_reference_modifiers(
        module: &ModuleQueryContext<'_>,
        declaration: dir::GlobalNodeIdAny,
    ) -> QueryResult<SemanticTokenModifiers> {
        if declaration.local_id.ty != dir::NodeType::Declaration {
            return Ok(SemanticTokenModifiers::NONE);
        }

        let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(declaration.local_id.id);
        let modifiers = match module.view()?.get(declaration_id) {
            dir::Declaration::Function(function)
                if function.signature.asynchrony == dir::Asynchrony::Async =>
            {
                SemanticTokenModifiers::ASYNC
            }
            dir::Declaration::Class(class) if class.is_abstract => SemanticTokenModifiers::ABSTRACT,
            _ => SemanticTokenModifiers::NONE,
        };

        Ok(modifiers)
    }

    /// Return modifiers recorded on one exact symbol.
    fn symbol_modifiers(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<SemanticTokenModifiers> {
        let mut modifiers = SemanticTokenModifiers::NONE;

        // record decorator state
        if self.program.symbol_is_deprecated(symbol_id)? {
            modifiers = modifiers.union(SemanticTokenModifiers::DEPRECATED);
        }

        // record package ownership
        if self.program.symbol_is_default_library(symbol_id)? {
            modifiers = modifiers.union(SemanticTokenModifiers::DEFAULT_LIBRARY);
        }

        Ok(modifiers)
    }

    /// Return modifiers recorded on the symbol declared by one node.
    fn node_symbol_modifiers(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<SemanticTokenModifiers> {
        let symbol_id = self
            .module
            .global_node_symbol(node_id)?
            .ok_or(QueryError::missing(format!(
                "semantic token declaration symbol: {:?}",
                node_id.into_global(self.module.module_id())
            )))?;

        self.symbol_modifiers(symbol_id)
    }

    /// Return modifiers carried by one member reference.
    fn member_reference_modifiers(member: &dir::Member) -> SemanticTokenModifiers {
        match member {
            dir::Member::AssociatedType { is_abstract, .. } => {
                SemanticTokenModifiers::member(false, false, false, *is_abstract)
            }
            dir::Member::AssociatedConst { is_abstract, .. } => {
                SemanticTokenModifiers::member(false, true, false, *is_abstract)
            }
            dir::Member::Field {
                is_readonly,
                mutability,
                is_static,
                is_abstract,
                ..
            } => {
                let is_readonly = *is_readonly || *mutability == Some(dir::Mutability::Immutable);

                SemanticTokenModifiers::member(false, is_readonly, *is_static, *is_abstract)
            }
            dir::Member::Method {
                signature,
                is_static,
                ..
            } => {
                let mut modifiers =
                    SemanticTokenModifiers::member(false, false, *is_static, signature.is_abstract);
                if signature.asynchrony == dir::Asynchrony::Async {
                    modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
                }

                modifiers
            }
            dir::Member::StaticBlock { .. }
            | dir::Member::ComptimeBlock { .. }
            | dir::Member::Error => SemanticTokenModifiers::NONE,
        }
    }

    /// Return modifiers recorded for one binding declaration.
    fn binding_modifiers(
        &self,
        declaration: dir::LocalNodeIdAny,
    ) -> QueryResult<SemanticTokenModifiers> {
        let symbol_id = self
            .module
            .global_node_symbol(declaration)?
            .ok_or(QueryError::missing(format!(
                "semantic token symbol: {:?}",
                declaration.into_global(self.module.module_id())
            )))?;
        let symbol = self.module.bindings()?.get_symbol(symbol_id.local_id);

        let modifiers = if symbol.binding_mutability == Some(dir::Mutability::Immutable) {
            SemanticTokenModifiers::READONLY
        } else {
            SemanticTokenModifiers::NONE
        };

        Ok(modifiers)
    }

    /// Collect member tokens.
    fn collect_members(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect fields, methods, and associated items
        for (member_id, member) in view.iter_nodes::<dir::Member>() {
            let Some((token_type, modifiers)) = Self::member_token(member) else {
                continue;
            };
            let Some(main_span) = self.main_span(member_id.into_any())? else {
                continue;
            };
            let modifiers = modifiers.union(self.node_symbol_modifiers(member_id.into_any())?);
            self.tokens
                .push(SemanticToken::new(main_span, token_type, modifiers));
        }

        Ok(())
    }

    /// Return the token classification for one member.
    fn member_token(member: &dir::Member) -> Option<(SemanticTokenType, SemanticTokenModifiers)> {
        match member {
            dir::Member::AssociatedType { is_abstract, .. } => Some((
                SemanticTokenType::Type,
                SemanticTokenModifiers::member(true, false, false, *is_abstract),
            )),
            dir::Member::AssociatedConst { is_abstract, .. } => {
                let modifiers = SemanticTokenModifiers::member(true, true, false, *is_abstract)
                    .union(SemanticTokenModifiers::READONLY);

                Some((SemanticTokenType::Property, modifiers))
            }
            dir::Member::Field {
                is_readonly,
                mutability,
                is_static,
                is_abstract,
                ..
            } => {
                let is_readonly = *is_readonly || *mutability == Some(dir::Mutability::Immutable);
                let modifiers =
                    SemanticTokenModifiers::member(true, is_readonly, *is_static, *is_abstract);

                Some((SemanticTokenType::Property, modifiers))
            }
            dir::Member::Method {
                signature,
                is_static,
                ..
            } => {
                let mut modifiers =
                    SemanticTokenModifiers::member(true, false, *is_static, signature.is_abstract);
                if signature.asynchrony == dir::Asynchrony::Async {
                    modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
                }

                Some((SemanticTokenType::Method, modifiers))
            }
            dir::Member::StaticBlock { .. } | dir::Member::ComptimeBlock { .. } => None,
            dir::Member::Error => None,
        }
    }

    /// Collect type member tokens.
    fn collect_type_members(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect interface and shape member declarations
        for (member_id, member) in view.iter_nodes::<dir::TypeMember>() {
            let Some((token_type, modifiers)) = self.type_member_token(member_id, member)? else {
                continue;
            };
            let Some(main_span) = self.main_span(member_id.into_any())? else {
                continue;
            };
            let modifiers = modifiers.union(self.node_symbol_modifiers(member_id.into_any())?);

            self.tokens
                .push(SemanticToken::new(main_span, token_type, modifiers));
        }

        Ok(())
    }

    /// Return the token classification for one type member.
    fn type_member_token(
        &self,
        member_id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
    ) -> QueryResult<Option<(SemanticTokenType, SemanticTokenModifiers)>> {
        let declaration = SemanticTokenModifiers::DECLARATION;

        let token = match member {
            dir::TypeMember::Field {
                is_static,
                is_readonly,
                ..
            } => {
                let modifiers = declaration.union(SemanticTokenModifiers::member(
                    false,
                    *is_readonly,
                    *is_static,
                    false,
                ));

                Some((SemanticTokenType::Property, modifiers))
            }
            dir::TypeMember::Method {
                signature,
                is_static,
                ..
            } => {
                let symbol_id = self
                    .module
                    .global_node_symbol(member_id.into_any())?
                    .ok_or(QueryError::missing(format!(
                        "semantic token member symbol: {:?}",
                        member_id.into_global_any(self.module.module_id())
                    )))?;
                let (_, _, definition) = self
                    .module
                    .definition_member(self.program, symbol_id)?
                    .ok_or(QueryError::missing(format!(
                        "semantic token member definition: {symbol_id:?}"
                    )))?;
                let dir::DefinitionMember::Method(definition) = definition else {
                    return Err(QueryError::invalid(format!(
                        "semantic token member definition: {symbol_id:?}"
                    )));
                };
                let is_abstract = definition.implementation == dir::MethodImplementation::Required;
                let mut modifiers = declaration.union(SemanticTokenModifiers::member(
                    false,
                    false,
                    *is_static,
                    is_abstract,
                ));
                if signature.asynchrony == dir::Asynchrony::Async {
                    modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
                }

                Some((SemanticTokenType::Method, modifiers))
            }
            dir::TypeMember::AssociatedType { is_abstract, .. } => Some((
                SemanticTokenType::Type,
                declaration.union(SemanticTokenModifiers::member(
                    false,
                    false,
                    false,
                    *is_abstract,
                )),
            )),
            dir::TypeMember::AssociatedConst { is_abstract, .. } => Some((
                SemanticTokenType::Property,
                declaration.union(SemanticTokenModifiers::member(
                    false,
                    true,
                    false,
                    *is_abstract,
                )),
            )),
            dir::TypeMember::CallSignature { .. }
            | dir::TypeMember::ConstructSignature { .. }
            | dir::TypeMember::IndexSignature { .. }
            | dir::TypeMember::Error => None,
        };

        Ok(token)
    }

    /// Collect enum field tokens.
    fn collect_enum_fields(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect enum member declarations
        for (field_id, _) in view.iter_nodes::<dir::EnumField>() {
            let Some(main_span) = self.main_span(field_id.into_any())? else {
                continue;
            };
            let modifiers = SemanticTokenModifiers::DECLARATION
                .union(SemanticTokenModifiers::READONLY)
                .union(self.node_symbol_modifiers(field_id.into_any())?);

            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::EnumMember,
                modifiers,
            ));
        }

        Ok(())
    }

    /// Collect generic type parameter tokens.
    fn collect_type_parameters(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect declaration generic parameter names
        for (_declaration_id, declaration) in view.iter_nodes::<dir::Declaration>() {
            let Some(generic_parameters) = declaration.generic_parameters() else {
                continue;
            };

            for &parameter_id in generic_parameters {
                let Some(main_span) = self.main_span(parameter_id.into_any())? else {
                    continue;
                };

                let parameter = view.get::<dir::GenericParameter>(parameter_id);
                let (token_type, modifiers) = match parameter {
                    dir::GenericParameter::Type { .. }
                    | dir::GenericParameter::VariadicType { .. } => (
                        SemanticTokenType::TypeParameter,
                        SemanticTokenModifiers::DECLARATION,
                    ),
                    dir::GenericParameter::Lifetime { .. }
                    | dir::GenericParameter::Value { .. }
                    | dir::GenericParameter::VariadicValue { .. } => (
                        SemanticTokenType::Variable,
                        SemanticTokenModifiers::DECLARATION.union(SemanticTokenModifiers::READONLY),
                    ),
                    dir::GenericParameter::Error => continue,
                };
                let modifiers =
                    modifiers.union(self.node_symbol_modifiers(parameter_id.into_any())?);
                self.tokens
                    .push(SemanticToken::new(main_span, token_type, modifiers));
            }
        }

        Ok(())
    }

    /// Collect type reference tokens with exact recorded identities.
    fn collect_type_references(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // visit authored names owned by type reference nodes
        for (type_id, type_expression) in view.iter_nodes::<dir::TypeExpression>() {
            let source_id = view.get_source(type_id);
            if self.module.source_index()?.try_get(source_id).is_none() {
                continue;
            }

            let mut spans = Vec::new();
            match type_expression {
                dir::TypeExpression::Reference { path, .. } if path.segments.len() > 1 => {
                    let node = type_id.into_any().into_global(self.module.module_id());

                    for (index, _) in path.segments.iter().enumerate() {
                        let index = u16::try_from(index).map_err(|_| {
                            QueryError::invalid(format!("semantic token path: {node:?}"))
                        })?;
                        let span_type = NodeSpanType::ListItem(NodeSpanList::Segment, index);
                        let span = self
                            .module
                            .source_index()?
                            .get_side(source_id, span_type)
                            .ok_or(QueryError::missing(format!(
                                "semantic token span: {node:?}"
                            )))?;
                        spans.push(span);
                    }
                }
                dir::TypeExpression::Reference { .. }
                | dir::TypeExpression::Member { .. }
                | dir::TypeExpression::Lifetime { .. }
                | dir::TypeExpression::This => {
                    if let Some(span) = self.main_span(type_id.into_any())? {
                        spans.push(span);
                    }
                }
                _ => continue,
            }

            // classify only segments carrying exact recorded symbol targets
            for span in spans {
                let Some(occurrence) =
                    self.module
                        .symbol_at_offset(self.program, span.file, span.start)?
                else {
                    continue;
                };
                if occurrence.span != span {
                    continue;
                }
                let Some((token_type, modifiers)) =
                    self.symbol_targets_token(&occurrence.symbols)?
                else {
                    continue;
                };

                self.tokens
                    .push(SemanticToken::new(span, token_type, modifiers));
            }
        }

        Ok(())
    }

    /// Collect decorator tokens.
    fn collect_decorators(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect decorator names
        for (decorator_id, decorator) in view.iter_nodes::<dir::Decorator>() {
            let name_id = Self::decorator_name_expression(view, decorator);
            let decorator_node = decorator_id.into_any().into_global(self.module.module_id());
            let span = self
                .main_span(name_id.into_any())?
                .ok_or(QueryError::missing(format!(
                    "semantic token span: {decorator_node:?}"
                )))?;
            let application = self
                .module
                .decorators()?
                .application_for_decorator(decorator_id)
                .ok_or(QueryError::missing(format!(
                    "semantic token decorator: {decorator_node:?}"
                )))?;
            let modifiers = match application.resolution.target {
                dir::DecoratorTarget::LanguageItem { symbol, .. }
                | dir::DecoratorTarget::Symbol { symbol } => self.symbol_modifiers(symbol),
            };
            let modifiers = modifiers?;

            self.tokens.push(SemanticToken::new(
                span,
                SemanticTokenType::Decorator,
                modifiers,
            ));
        }

        Ok(())
    }

    /// Return the expression that carries one decorator's source name.
    fn decorator_name_expression(
        view: dir::View<'_>,
        decorator: &dir::Decorator,
    ) -> dir::LocalNodeId<dir::Expression> {
        let mut expression_id = decorator.expression;

        // unwrap decorator calls to their named callee
        while let dir::Expression::Call { left, .. } = view.get(expression_id) {
            expression_id = *left;
        }

        expression_id
    }

    /// Return whether one expression is the exact name of an attached decorator.
    fn is_decorator_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> QueryResult<bool> {
        let view = self.module.view()?;
        let mut current_id = expression_id;

        // ascend through decorator call callee slots
        while let Some(parent_id) = view.get_parent_for(current_id) {
            if parent_id.ty == dir::NodeType::Decorator {
                let decorator_id = dir::LocalNodeId::<dir::Decorator>::new(parent_id.id);
                let decorator = view.get(decorator_id);

                let is_name = Self::decorator_name_expression(view, decorator) == expression_id;

                return Ok(is_name);
            }
            if parent_id.ty != dir::NodeType::Expression {
                return Ok(false);
            }

            let parent_id = dir::LocalNodeId::<dir::Expression>::new(parent_id.id);
            let dir::Expression::Call { left, .. } = view.get(parent_id) else {
                return Ok(false);
            };
            if *left != current_id {
                return Ok(false);
            }

            current_id = parent_id;
        }

        Ok(false)
    }

    /// Collect documentation comments.
    fn collect_documentation_comments(&mut self) -> QueryResult<()> {
        let file = self.module.read_file(self.file_id)?;

        // collect doc line and block comments
        for comment in self.module.comments(self.file_id)?.iter().copied() {
            let raw_text = file
                .get_span_str(comment.span)
                .ok_or(QueryError::invalid(format!(
                    "source span: {:?}",
                    comment.span
                )))?
                .trim_start();
            if !raw_text.starts_with("///") && !raw_text.starts_with("/**") {
                continue;
            }

            self.tokens.push(SemanticToken::new(
                comment.span,
                SemanticTokenType::Comment,
                SemanticTokenModifiers::DOCUMENTATION,
            ));
        }

        Ok(())
    }

    /// Collect dependency item tokens.
    fn collect_dependency_items(&mut self) -> QueryResult<()> {
        let view = self.module.view()?;

        // collect imported and exported binding names
        for (item_id, item) in view.iter_nodes::<dir::DependencyItem>() {
            if item.local_string_key().is_none() {
                continue;
            }

            let source_node_id = view.get_source(item_id);
            let Some(main_span) = self.main_span(item_id.into_any())? else {
                continue;
            };

            let targets = self.module.dependency_targets(item_id)?;
            if targets.is_empty() {
                continue;
            }

            // require every overload target to share one classification
            let mut token: Option<(SemanticTokenType, SemanticTokenModifiers)> = None;
            for target in targets {
                let candidate = match target {
                    dir::ReferenceTarget::Namespace(_) => {
                        (SemanticTokenType::Namespace, SemanticTokenModifiers::NONE)
                    }
                    dir::ReferenceTarget::Symbol(symbol_id) => {
                        let Some(token) = self.symbol_token(symbol_id)? else {
                            token = None;
                            break;
                        };

                        token
                    }
                };
                if token.is_some_and(|(token_type, _)| token_type != candidate.0) {
                    token = None;
                    break;
                }
                token = Some(match token {
                    Some((token_type, modifiers)) => {
                        (token_type, modifiers.intersection(candidate.1))
                    }
                    None => candidate,
                });
            }
            let Some((token_type, target_modifiers)) = token else {
                continue;
            };

            // emit the imported name independently from its local alias
            let imported_name = NodeSpanType::Region(NodeSpanRegion::Type);
            if let Some(imported_span) = self
                .module
                .source_index()?
                .get_side(source_node_id, imported_name)
                && imported_span != main_span
            {
                self.tokens.push(SemanticToken::new(
                    imported_span,
                    token_type,
                    target_modifiers,
                ));
            }

            self.tokens.push(SemanticToken::new(
                main_span,
                token_type,
                target_modifiers.union(SemanticTokenModifiers::DECLARATION),
            ));
        }

        Ok(())
    }

    /// Return the authored main span for one DIR node.
    fn main_span(&self, node_id: dir::LocalNodeIdAny) -> QueryResult<Option<Span>> {
        let source_node_id = self.module.view()?.get_source_any(node_id);
        if self
            .module
            .source_index()?
            .try_get(source_node_id)
            .is_none()
        {
            return Ok(None);
        }

        let node_id = node_id.into_global(self.module.module_id());
        let span =
            self.module
                .source_index()?
                .get_main(source_node_id)
                .ok_or(QueryError::missing(format!(
                    "semantic token main span: {node_id:?}, source={source_node_id}"
                )))?;

        Ok(Some(span))
    }
}
