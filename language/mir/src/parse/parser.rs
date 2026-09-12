use destack_core::{Blob, FxIndexMap, FxIndexSet, StringPool};
use destack_source::{
    DiagnosticCollection, DiagnosticCollector, DiagnosticSeverity, File, FileId, ModuleId,
    NodeSpanList, NodeSpanType, PackageId, Span,
};

use crate::source::{Lexer, Token, TokenType};
use crate::{
    Access, Block, DispatchTable, DropTable, EffectTable, Extent, Function, GenericArgument,
    GenericParameter, GenericParameterDomain, Global, LayoutTable, Lifetime, LifetimeParameter,
    Local, LocalNodeId, Node, ProfileTable, RegionBound, Space, Static, Storage, TargetLayout,
    Tree, Type, TypeDeclaration, Value,
};

use super::error::{ParseError, ParseResult};

/// The result of parsing one MIR source file.
#[derive(Debug)]
pub struct ParsedMir {
    /// The parsed MIR tree.
    pub tree: Tree,
    /// Target ABI layout.
    pub target_layout: TargetLayout,
    /// Canonical MIR layout table.
    pub layouts: LayoutTable,
    /// Canonical MIR dispatch table.
    pub dispatch: DispatchTable,
    /// Canonical MIR drop table.
    pub drops: DropTable,

    /// Function and call effect table.
    pub effects: EffectTable,
    /// Static profile counter table.
    pub profile: ProfileTable,
    /// The parsed string pool.
    pub strings: StringPool,
    /// The collected parse diagnostics.
    pub diagnostics: DiagnosticCollection,
}

impl ParsedMir {
    /// Return the parsed tree, strings, and diagnostics.
    pub fn into_parts(
        self,
    ) -> (
        Tree,
        TargetLayout,
        LayoutTable,
        DispatchTable,
        DropTable,
        EffectTable,
        ProfileTable,
        StringPool,
        DiagnosticCollection,
    ) {
        (
            self.tree,
            self.target_layout,
            self.layouts,
            self.dispatch,
            self.drops,
            self.effects,
            self.profile,
            self.strings,
            self.diagnostics,
        )
    }

    /// Return the parsed MIR when no parse errors were emitted.
    pub fn finish(self) -> Result<(Tree, StringPool), DiagnosticCollection> {
        let Self {
            tree,
            target_layout: _,
            layouts: _,
            dispatch: _,
            drops: _,
            effects: _,
            profile: _,
            strings,
            diagnostics,
        } = self;

        // fail strictly when parse diagnostics were emitted
        if diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            return Err(diagnostics);
        }

        Ok((tree, strings))
    }
}

/// Options for the MIR parser.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Pointer size in bytes.
    pub pointer_bytes: u8,
    /// The module the parsed declarations belong to.
    pub module: ModuleId,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            pointer_bytes: 8,
            module: ModuleId::new(PackageId::new(0), 0),
        }
    }
}

/// Parser for MIR text format.
#[derive(Debug)]
pub struct Parser {
    /// Current position in the tokens.
    pub(super) pos: usize,
    /// The tree being built.
    pub(super) tree: Tree,
    /// Target ABI layout.
    pub(super) target_layout: TargetLayout,
    /// Canonical MIR layout table.
    pub(super) layouts: LayoutTable,
    /// Canonical MIR dispatch table.
    pub(super) dispatch: DispatchTable,
    /// Canonical MIR drop table.
    pub(super) drops: DropTable,

    /// Function and call effect table.
    pub(super) effects: EffectTable,
    /// Static profile counter table.
    pub(super) profile: ProfileTable,
    /// The string pool.
    pub(super) strings: StringPool,
    /// The source file id for spans.
    pub(super) file_id: FileId,
    /// The exact source Blob for diagnostics.
    pub(super) blob: Blob,
    /// The diagnostics produced while parsing.
    pub(super) diagnostics: DiagnosticCollector,
    /// Map from function names with their generic arguments to their ids for forward references.
    pub(super) function_map: FxIndexMap<(String, Vec<GenericArgument>), LocalNodeId<Function>>,
    /// Map from global names to their ids (for forward references).
    pub(super) global_map: FxIndexMap<String, LocalNodeId<Global>>,
    /// Map from type declaration names to their ids (for references).
    pub(super) type_declaration_map: FxIndexMap<String, LocalNodeId<TypeDeclaration>>,
    /// Set of type declarations that have been defined.
    pub(super) type_declaration_definitions: FxIndexSet<String>,
    /// Map from symbolic block names to their predeclared block ids.
    pub(super) block_name_map: FxIndexMap<String, LocalNodeId<Block>>,
    /// Blocks predeclared for the current function body in source order.
    pub(super) predeclared_blocks: Vec<LocalNodeId<Block>>,
    /// SSA values defined in the current function.
    pub(super) defined_values: FxIndexSet<Value>,
    /// Map from symbolic local names to their local ids.
    pub(super) local_name_map: FxIndexMap<String, LocalNodeId<Local>>,
    /// The function currently being parsed.
    pub(super) current_function: Option<LocalNodeId<Function>>,
    /// SSA value types for the current function.
    pub(super) value_types: Vec<Option<LocalNodeId<Type>>>,
    /// The next SSA value id for the current function.
    pub(super) next_value_id: u32,
    /// The number of blocks parsed in the current function so far.
    pub(super) parsed_block_count: usize,
    /// Lifetime names visible in the current signature/type body.
    pub(super) lifetime_scopes: Vec<Vec<(String, Extent)>>,
    /// Generic parameter names visible in the current signature/type body.
    pub(super) generic_scopes: Vec<Vec<GenericParameter>>,
    /// The module the parsed declarations belong to.
    pub(super) module: ModuleId,
}

impl Parser {
    /// Create a new parser for a specific file.
    pub fn new(file: &File, options: ParseOptions) -> ParseResult<Self> {
        if !file.is_text() {
            return Err(ParseError::new("MIR parser requires text content", 0));
        }

        let content = file.text();

        let tree = Tree::with_parsed_source(content.to_string(), Lexer::lex(file.id, content));
        let target_layout = TargetLayout::for_pointer_bytes(options.pointer_bytes);

        Ok(Self {
            module: options.module,
            pos: 0,
            tree,
            target_layout,
            layouts: LayoutTable::default(),
            dispatch: DispatchTable::default(),
            drops: DropTable::default(),
            effects: EffectTable::default(),
            profile: ProfileTable::default(),
            strings: StringPool::new(),
            file_id: file.id,
            blob: file.blob(),
            diagnostics: DiagnosticCollector::new(),
            function_map: FxIndexMap::default(),
            global_map: FxIndexMap::default(),
            type_declaration_map: FxIndexMap::default(),
            type_declaration_definitions: FxIndexSet::default(),
            block_name_map: FxIndexMap::default(),
            predeclared_blocks: Vec::new(),
            defined_values: FxIndexSet::default(),
            local_name_map: FxIndexMap::default(),
            current_function: None,
            value_types: Vec::new(),
            next_value_id: 0,
            parsed_block_count: 0,
            lifetime_scopes: Vec::new(),
            generic_scopes: Vec::new(),
        })
    }

    /// Parse MIR text and return the parsed source bundle.
    pub fn parse(file: &File, options: ParseOptions) -> ParseResult<ParsedMir> {
        let mut parser = Parser::new(file, options)?;

        // parse the semantic MIR
        parser.parse_module();

        // attach source comments after the node graph exists
        parser.attach_comment_ownership();

        Ok(ParsedMir {
            tree: parser.tree,
            target_layout: parser.target_layout,
            layouts: parser.layouts,
            dispatch: parser.dispatch,
            drops: parser.drops,
            effects: parser.effects,
            profile: parser.profile,
            strings: parser.strings,
            diagnostics: parser.diagnostics.take_collection(),
        })
    }

    /// Apply one ordered segment span list to one MIR node.
    pub(super) fn set_segment_spans<T>(
        &mut self,
        id: LocalNodeId<T>,
        segment_spans: &[Span],
    ) -> ParseResult<()>
    where
        T: Node,
    {
        // preserve source order
        for (index, span) in segment_spans.iter().copied().enumerate() {
            let segment_index = u16::try_from(index).map_err(|_| {
                ParseError::new(
                    format!("too many segment spans for node {}", id.id),
                    self.pos(),
                )
            })?;
            self.tree.set_side_span(
                id,
                NodeSpanType::ListItem(NodeSpanList::Segment, segment_index),
                span,
            );
        }

        Ok(())
    }

    /// Parse one declaration name.
    pub(super) fn parse_symbol_name(&mut self) -> ParseResult<(String, usize)> {
        let name_token = self.eat_token(TokenType::Identifier)?;
        let start = name_token.start();
        let name = self.tree.source_text(name_token.span).to_string();

        Ok((name, start))
    }

    /// Parse the generic arguments applied to one function reference.
    pub(super) fn parse_function_arguments(&mut self) -> ParseResult<Vec<GenericArgument>> {
        if !self.eat_token_if(TokenType::LessThan) {
            return Ok(Vec::new());
        }

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

    /// Parse one generic argument: a region, a space, an access, a value, or a type.
    pub(super) fn parse_generic_argument(&mut self) -> ParseResult<GenericArgument> {
        if self.peek_is(TokenType::Lifetime) {
            return self.parse_region_argument();
        }
        let token = self
            .peek()
            .ok_or_else(|| ParseError::unexpected_end("generic argument", self.pos()))?;
        let kind = self.token_type(token);
        let text = self.tree.source_text(token.span).to_string();
        let start = token.start();

        // read a parameter in scope by its domain
        let parameter = self
            .generic_parameter(&text)
            .map(|(index, parameter)| (index, parameter.domain.clone()));
        if let Some((index, domain)) = parameter {
            self.bump();

            return Ok(match domain {
                GenericParameterDomain::Type { .. } => {
                    GenericArgument::Type(self.intern_type(Type::Parameter { index })?)
                }
                GenericParameterDomain::Region { .. } => GenericArgument::Region {
                    lifetime: Lifetime::new([Extent::Parameter(index)]),
                    storage: Storage::Parameter(index),
                },
                GenericParameterDomain::Space => GenericArgument::Space(Space::Parameter(index)),
                GenericParameterDomain::Access => GenericArgument::Access(Access::Parameter(index)),
                GenericParameterDomain::Value { .. } => {
                    GenericArgument::Value(self.tree.intern_static(Static::Parameter(index)))
                }
            });
        }

        match kind {
            // closed spaces and accesses by their keywords
            TokenType::Identifier if let Some(space) = Space::from_name(&text) => {
                self.bump();

                Ok(GenericArgument::Space(space))
            }
            TokenType::Identifier if let Some(access) = Access::from_name(&text) => {
                self.bump();

                Ok(GenericArgument::Access(access))
            }
            TokenType::Readonly => {
                self.bump();

                Ok(GenericArgument::Access(Access::Readonly))
            }
            // literal values
            TokenType::BooleanLiteral
            | TokenType::Integer
            | TokenType::Float
            | TokenType::Character
            | TokenType::String
            | TokenType::Regex => Ok(GenericArgument::Value(self.parse_static()?)),
            TokenType::Identifier if matches!(text.as_str(), "null" | "undefined") => {
                Ok(GenericArgument::Value(self.parse_static()?))
            }
            // aggregate values and nominal values by their literal content
            _ if self.peek_static_aggregate() || self.peek_static_nominal(kind) => {
                Ok(GenericArgument::Value(self.parse_static()?))
            }
            // an explicit type argument
            TokenType::Type => {
                self.bump();
                let (ty, _) = self.parse_type_use_part()?;

                Ok(GenericArgument::Type(ty))
            }
            // every other argument is a type
            _ if self.peek_type(kind) => {
                let (ty, _) = self.parse_type_use_part()?;

                Ok(GenericArgument::Type(ty))
            }
            _ => Err(ParseError::unexpected("generic argument", kind, start)),
        }
    }

    /// Return whether the next tokens open a static array, tuple, or object value.
    fn peek_static_aggregate(&self) -> bool {
        let Some(open) = self.peek() else {
            return false;
        };
        let content = match self.token_type(open) {
            TokenType::OpenBracket | TokenType::OpenParenthesis => self.peek_nth_token(1),
            TokenType::OpenBrace => self.peek_nth_token(3),
            _ => return false,
        };

        content.is_some_and(|token| self.peek_static_literal(token))
    }

    /// Return whether the next tokens name a declared type followed by its static value.
    fn peek_static_nominal(&self, kind: TokenType) -> bool {
        kind == TokenType::Identifier
            && self.peek_nth_token(1).is_some_and(|token| {
                matches!(
                    self.token_type(token),
                    TokenType::OpenParenthesis | TokenType::OpenBrace
                )
            })
    }

    /// Return whether one token starts a static literal value.
    fn peek_static_literal(&self, token: &Token) -> bool {
        match self.token_type(token) {
            TokenType::BooleanLiteral
            | TokenType::Integer
            | TokenType::Float
            | TokenType::Character
            | TokenType::String
            | TokenType::Regex
            | TokenType::CloseBracket
            | TokenType::CloseParenthesis
            | TokenType::CloseBrace => true,
            TokenType::Identifier => matches!(
                self.tree.source_text(token.span),
                "null" | "undefined" | "NaN" | "Infinity" | "-Infinity"
            ),
            _ => false,
        }
    }

    /// Parse the generic arguments, generic parameters, and lifetime binders of a declaration.
    pub(super) fn parse_declaration_parameters(
        &mut self,
        regions_as_generics: bool,
    ) -> ParseResult<(
        Vec<GenericArgument>,
        Vec<GenericParameter>,
        Vec<LifetimeParameter>,
    )> {
        // keep one ordered scope for all generic parameter domains
        let scope = self.generic_scopes.len();
        self.generic_scopes.push(Vec::new());
        self.lifetime_scopes.push(Vec::new());
        if !self.eat_token_if(TokenType::LessThan) {
            return Ok((Vec::new(), Vec::new(), Vec::new()));
        }

        let mut arguments = Vec::new();
        let mut lifetimes = Vec::new();
        let mut outlives_names: Vec<(usize, Vec<(String, usize)>)> = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            // bind regions by generic index in types and by lifetime slot in functions
            if self.peek_is(TokenType::Lifetime) {
                let token = self.eat_token(TokenType::Lifetime)?;
                let name = self.tree.source_text(token.span).to_string();
                if self.lifetime_scopes[scope]
                    .iter()
                    .any(|(candidate, _)| *candidate == name)
                {
                    return Err(ParseError::invalid(
                        "duplicate lifetime parameter",
                        token.start(),
                    ));
                }
                if regions_as_generics {
                    let index = self.generic_scopes[scope].len();
                    self.lifetime_scopes[scope]
                        .push((name.clone(), Extent::Parameter(index as u32)));
                    self.generic_scopes[scope].push(GenericParameter {
                        name: self.strings.intern(&name),
                        domain: GenericParameterDomain::Region {
                            outlives: Vec::new(),
                        },
                    });

                    // retain forward lifetime bounds until every region is named
                    if self.eat_token_if(TokenType::Colon) {
                        let mut outlived = Vec::new();
                        loop {
                            let token = self.eat_token(TokenType::Lifetime)?;
                            let name = self.tree.source_text(token.span).to_string();
                            outlived.push((name, token.start()));
                            if !self.eat_token_if(TokenType::Ampersand) {
                                break;
                            }
                        }
                        outlives_names.push((index, outlived));
                    }
                } else {
                    let slot = RegionBound::new(lifetimes.len() as u32);
                    self.lifetime_scopes[scope].push((name.clone(), Extent::Bound(slot)));
                    lifetimes.push(LifetimeParameter::new(Some(self.strings.intern(&name))));
                }
            } else if let Some(parameter) = self.parse_generic_parameter_if()? {
                self.generic_scopes[scope].push(parameter);
            } else {
                arguments.push(self.parse_generic_argument()?);
            }

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }
        self.eat_token(TokenType::GreaterThan)?;

        // resolve the forward region bounds in declaration order
        for (index, outlived) in outlives_names {
            for (name, position) in outlived {
                let term = self.lifetime_scopes[scope]
                    .iter()
                    .find(|(candidate, _)| *candidate == name)
                    .map(|(_, term)| *term);
                let Some(Extent::Parameter(outlived_index)) = term else {
                    return Err(ParseError::invalid("region parameter", position));
                };
                let GenericParameterDomain::Region { outlives } =
                    &mut self.generic_scopes[scope][index].domain
                else {
                    unreachable!("a region parameter has region bounds");
                };
                outlives.push(outlived_index);
            }
        }

        Ok((arguments, self.generic_scopes[scope].clone(), lifetimes))
    }

    /// Parse one generic parameter declaration when the next tokens declare one.
    fn parse_generic_parameter_if(&mut self) -> ParseResult<Option<GenericParameter>> {
        let Some(token) = self.peek().copied() else {
            return Ok(None);
        };
        let kind = self.token_type(&token);
        let text = self.tree.source_text(token.span).to_string();
        let start = token.start();

        // read the domain keyword for a space, access, or value parameter
        let domain = match (kind, text.as_str()) {
            (TokenType::Identifier, "space") => Some(GenericParameterDomain::Space),
            (TokenType::Identifier, "access") => Some(GenericParameterDomain::Access),
            (TokenType::Const, _) => None,
            (TokenType::Identifier, _) if self.peek_declares_type_parameter(&text) => {
                Some(GenericParameterDomain::Type { bounds: Vec::new() })
            }
            _ => return Ok(None),
        };
        self.bump();

        // read the parameter name
        let name_token = match domain {
            Some(GenericParameterDomain::Type { .. }) => token,
            _ => self.eat_token(TokenType::Identifier)?,
        };
        let name = self.tree.source_text(name_token.span).to_string();
        if self.generic_scopes.last().is_some_and(|scope| {
            scope
                .iter()
                .any(|parameter| self.strings.get(parameter.name) == name)
        }) {
            return Err(ParseError::invalid("duplicate generic parameter", start));
        }

        // read the bounds or the value type
        let domain = match domain {
            Some(GenericParameterDomain::Type { .. }) => {
                let mut bounds = Vec::new();
                if self.eat_token_if(TokenType::Colon) {
                    loop {
                        let (bound, _) = self.parse_type_use_part()?;
                        bounds.push(bound);
                        if !self.eat_token_if(TokenType::Ampersand) {
                            break;
                        }
                    }
                }

                GenericParameterDomain::Type { bounds }
            }
            Some(domain) => domain,
            None => {
                self.eat_token(TokenType::Colon)?;
                let (ty, _) = self.parse_type_use_part()?;

                GenericParameterDomain::Value { ty }
            }
        };

        Ok(Some(GenericParameter {
            name: self.strings.intern(&name),
            domain,
        }))
    }

    /// Return whether one identifier at a declaration position declares a type parameter.
    fn peek_declares_type_parameter(&self, text: &str) -> bool {
        // treat a declared name or literal as an argument and a bound name as a parameter
        let is_declared = Type::from_primitive_name(text).is_some()
            || self.type_declaration_map.contains_key(text)
            || Space::from_name(text).is_some()
            || Access::from_name(text).is_some()
            || matches!(
                text,
                "null" | "undefined" | "NaN" | "Infinity" | "-Infinity"
            );
        let is_bound = self
            .peek_nth_token(1)
            .is_some_and(|token| self.token_type(token) == TokenType::Colon);

        is_bound || !is_declared
    }

    /// Return one generic parameter visible in the current scope with its index.
    pub(super) fn generic_parameter(&self, name: &str) -> Option<(u32, &GenericParameter)> {
        self.generic_scopes.iter().rev().find_map(|scope| {
            scope
                .iter()
                .enumerate()
                .find(|(_, parameter)| self.strings.get(parameter.name) == name)
                .map(|(index, parameter)| (index as u32, parameter))
        })
    }

    /// Parse optional lifetime parameters after a declaration name.
    pub(super) fn parse_lifetime_parameters(&mut self) -> ParseResult<Vec<LifetimeParameter>> {
        self.generic_scopes.push(Vec::new());
        if !self.eat_token_if(TokenType::LessThan) {
            self.lifetime_scopes.push(Vec::new());
            return Ok(Vec::new());
        }

        let mut lifetimes = Vec::new();
        let mut scope = Vec::new();
        while !self.peek_is(TokenType::GreaterThan) {
            let name_token = self.eat_token(TokenType::Lifetime)?;
            let name = self.tree.source_text(name_token.span).to_string();
            if scope.iter().any(|(candidate, _)| candidate == &name) {
                return Err(ParseError::invalid(
                    "duplicate lifetime parameter",
                    name_token.start(),
                ));
            }
            let slot = RegionBound::new(scope.len() as u32);
            scope.push((name.clone(), Extent::Bound(slot)));
            lifetimes.push(LifetimeParameter::new(Some(self.strings.intern(&name))));

            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        self.eat_token(TokenType::GreaterThan)?;
        self.lifetime_scopes.push(scope);

        Ok(lifetimes)
    }

    /// Parse one optional trailing lifetime where clause.
    pub(super) fn parse_lifetime_where(
        &mut self,
        lifetimes: &mut [LifetimeParameter],
    ) -> ParseResult<()> {
        if !self.eat_name_if("where") {
            return Ok(());
        }

        loop {
            let left = self.parse_scope_lifetime()?;
            self.eat_token(TokenType::Colon)?;
            let right = self.parse_lifetime_union()?;
            if left.depth != 0 || left.index as usize >= lifetimes.len() {
                return Err(ParseError::invalid(
                    "outlives parameter outside the current binder",
                    self.pos(),
                ));
            }
            let previous = &lifetimes[left.index as usize].outlives;
            lifetimes[left.index as usize].outlives =
                Lifetime::new(previous.extents.iter().chain(&right.extents).copied());
            if !self.eat_token_if(TokenType::Comma) {
                break;
            }
        }

        Ok(())
    }

    /// Parse one lifetime binder name against the active function scope.
    pub(super) fn parse_scope_lifetime(&mut self) -> ParseResult<RegionBound> {
        let token = self.eat_token(TokenType::Lifetime)?;
        let name = self.tree.source_text(token.span);
        let term = self.extent_of_name(name);
        let Some(Extent::Bound(slot)) = term else {
            return Err(ParseError::invalid("lifetime parameter", token.start()));
        };

        Ok(slot)
    }

    /// Parse a lifetime parameter scope around one parser operation.
    pub(super) fn parse_lifetime_scope<T>(
        &mut self,
        parse: impl FnOnce(&mut Self, Vec<LifetimeParameter>) -> ParseResult<T>,
    ) -> ParseResult<T> {
        let lifetime_scope_count = self.lifetime_scopes.len();
        let lifetimes = self.parse_lifetime_parameters();
        let result = lifetimes.and_then(|lifetimes| parse(self, lifetimes));
        self.restore_lifetime_scopes(lifetime_scope_count);

        result
    }

    /// Leave the current lifetime and generic parameter scope.
    pub(super) fn pop_lifetime_scope(&mut self) {
        self.lifetime_scopes.pop();
        self.generic_scopes.pop();
    }

    /// Restore the lifetime and generic scope stacks to a previous depth.
    pub(super) fn restore_lifetime_scopes(&mut self, count: usize) {
        self.lifetime_scopes.truncate(count);
        self.generic_scopes.truncate(count);
    }

    /// Resolve one named lifetime in the visible lifetime scopes.
    pub(super) fn extent_of_name(&self, name: &str) -> Option<Extent> {
        let mut depth = 0;
        for scope in self.lifetime_scopes.iter().rev() {
            for (candidate, term) in scope {
                if candidate == name {
                    return Some(match term {
                        Extent::Bound(bound) => Extent::Bound(RegionBound {
                            depth,
                            index: bound.index,
                        }),
                        term => *term,
                    });
                }
            }
            if scope
                .iter()
                .any(|(_, extent)| matches!(extent, Extent::Bound(_)))
            {
                depth += 1;
            }
        }

        None
    }

    /// Record the type for a value in the current function.
    pub(super) fn record_value_type(
        &mut self,
        value: Value,
        ty: LocalNodeId<Type>,
    ) -> ParseResult<()> {
        if self.current_function.is_some() {
            let existing = self.value_types.get(value.0 as usize).copied().flatten();
            if let Some(existing) = existing {
                if existing != ty {
                    return Err(ParseError::new(
                        format!("value {value:?} has mismatched types {existing:?} and {ty:?}"),
                        self.pos(),
                    ));
                }
                return Ok(());
            }

            self.resize_value_slots(value);
            self.value_types[value.0 as usize] = Some(ty);
        }

        Ok(())
    }

    /// Resize current function SSA side tables for one value.
    pub(super) fn resize_value_slots(&mut self, value: Value) {
        let index = value.0 as usize;
        let value_count = index + 1;

        if self.value_types.len() < value_count {
            self.value_types.resize(value_count, None);
        }
    }

    /// Reset per-function parse state.
    pub(super) fn reset_function_parse_state(&mut self) {
        self.block_name_map.clear();
        self.predeclared_blocks.clear();
        self.defined_values.clear();
        self.local_name_map.clear();
        self.value_types.clear();
        self.next_value_id = 0;
        self.parsed_block_count = 0;
    }

    /// Return whether the current token starts a value definition.
    pub(super) fn is_value_definition_start(&self) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        matches!(
            self.peek_nth_token(1).map(|token| self.token_type(token)),
            Some(TokenType::Colon)
        )
    }

    /// Return whether the current token starts a value reference.
    pub(super) fn is_value_reference_start(&self) -> bool {
        self.peek_is(TokenType::Identifier)
    }

    /// Return whether the current token starts a block label.
    pub(super) fn is_block_label_start(&self) -> bool {
        let Some(token) = self.peek() else {
            return false;
        };

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        if !self.is_token_at_line_start(token) {
            return false;
        }

        let Some(raw_index) = self
            .tree
            .tokens()
            .iter()
            .enumerate()
            .skip(self.pos)
            .find_map(|(index, token)| (!token.is_trivia()).then_some(index))
        else {
            return false;
        };

        let mut saw_colon = false;
        for token in self.tree.tokens().iter().skip(raw_index + 1) {
            match self.token_type(token) {
                TokenType::Newline | TokenType::End => break,
                TokenType::Equal => return false,
                TokenType::Colon => saw_colon = true,
                _ => {}
            }
        }

        saw_colon
    }

    /// Return whether the token at one raw index starts a line-local block label.
    pub(super) fn is_block_label_token(&self, token_index: usize) -> bool {
        let tokens = self.tree.tokens();

        let Some(token) = tokens.get(token_index) else {
            return false;
        };

        if self.token_type(token) != TokenType::Identifier {
            return false;
        }

        if !self.is_token_at_line_start(token) {
            return false;
        }

        if tokens[..token_index]
            .iter()
            .rev()
            .take_while(|token| self.token_type(token) != TokenType::Newline)
            .any(|token| !token.is_trivia())
        {
            return false;
        }

        let mut saw_colon = false;
        let mut next_index = token_index + 1;
        while let Some(next_token) = tokens.get(next_index) {
            match self.token_type(next_token) {
                TokenType::Newline | TokenType::End => break,
                TokenType::Equal => return false,
                TokenType::Colon => saw_colon = true,
                _ => {}
            }

            next_index += 1;
        }

        saw_colon
    }
}
