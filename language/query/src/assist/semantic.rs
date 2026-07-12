use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::{Module, ModuleQueryContext, Range};

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
    /// Macro token.
    Macro,
    /// Keyword token.
    Keyword,
    /// Modifier token.
    Modifier,
    /// Comment token.
    Comment,
    /// String token.
    String,
    /// Number token.
    Number,
    /// Regular expression token.
    Regexp,
    /// Operator token.
    Operator,
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

    /// Return the token type for one symbol kind.
    fn symbol_kind(symbol_kind: dir::SymbolKind) -> Self {
        match symbol_kind {
            dir::SymbolKind::Variable
            | dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::GenericValueParameter => Self::Variable,
            dir::SymbolKind::Class => Self::Class,
            dir::SymbolKind::Struct => Self::Struct,
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => Self::Interface,
            dir::SymbolKind::Enum => Self::Enum,
            dir::SymbolKind::EnumField => Self::EnumMember,
            dir::SymbolKind::Function => Self::Function,
            dir::SymbolKind::Label => Self::Label,
            dir::SymbolKind::Import => Self::Variable,
            dir::SymbolKind::Extension => Self::Type,
            dir::SymbolKind::AssociatedType
            | dir::SymbolKind::TypeAlias
            | dir::SymbolKind::GenericTypeParameter
            | dir::SymbolKind::Newtype => Self::Type,
        }
    }

    /// Return the token type for one scalar literal.
    fn scalar(value: &dir::ScalarLiteral) -> Self {
        match value {
            dir::ScalarLiteral::Null | dir::ScalarLiteral::Undefined => Self::Keyword,
            dir::ScalarLiteral::String(_) | dir::ScalarLiteral::Character(_) => Self::String,
            dir::ScalarLiteral::RegexString { .. } => Self::Regexp,
            dir::ScalarLiteral::Integer(_)
            | dir::ScalarLiteral::Float(_)
            | dir::ScalarLiteral::Bigint(_)
            | dir::ScalarLiteral::Boolean(_) => Self::Number,
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
    /// Definition modifier.
    pub const DEFINITION: Self = Self(1 << 1);
    /// Readonly modifier.
    pub const READONLY: Self = Self(1 << 2);
    /// Static modifier.
    pub const STATIC: Self = Self(1 << 3);
    /// Deprecated modifier.
    pub const DEPRECATED: Self = Self(1 << 4);
    /// Abstract modifier.
    pub const ABSTRACT: Self = Self(1 << 5);
    /// Async modifier.
    pub const ASYNC: Self = Self(1 << 6);
    /// Modification modifier.
    pub const MODIFICATION: Self = Self(1 << 7);
    /// Documentation modifier.
    pub const DOCUMENTATION: Self = Self(1 << 8);
    /// Default library modifier.
    pub const DEFAULT_LIBRARY: Self = Self(1 << 9);
    /// Mutable modifier.
    pub const MUTABLE: Self = Self(1 << 10);

    /// Return the union of two modifier sets.
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
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
    pub fn new(
        span: Span,
        token_type: SemanticTokenType,
        modifiers: SemanticTokenModifiers,
    ) -> Self {
        Self {
            span,
            token_type,
            modifiers,
        }
    }

    /// Create a semantic token with no modifiers.
    pub fn plain(span: Span, token_type: SemanticTokenType) -> Self {
        Self::new(span, token_type, SemanticTokenModifiers::NONE)
    }

    /// Return whether this token overlaps another token.
    fn overlaps(&self, other: &Self) -> bool {
        other.span.start < self.span.end
    }
}

/// Semantic token collection for one module.
struct SemanticTokens<'owner, 'module> {
    /// The queried module.
    module: &'owner ModuleQueryContext<'module>,
    /// The collected tokens.
    tokens: Vec<SemanticToken>,
}

impl<'owner, 'module> SemanticTokens<'owner, 'module> {
    /// Collect semantic tokens for one module.
    fn collect(module: &'owner ModuleQueryContext<'module>) -> Vec<SemanticToken> {
        let mut semantic_tokens = Self {
            module,
            tokens: Vec::new(),
        };

        // collect all token families
        semantic_tokens.collect_declarations();
        semantic_tokens.collect_parameters();
        semantic_tokens.collect_pattern_bindings();
        semantic_tokens.collect_pattern_fields();
        semantic_tokens.collect_expressions();
        semantic_tokens.collect_members();
        semantic_tokens.collect_enum_fields();
        semantic_tokens.collect_type_parameters();
        semantic_tokens.collect_decorators();
        semantic_tokens.collect_documentation_comments();
        semantic_tokens.collect_dependency_items();

        semantic_tokens.finish()
    }

    /// Finish collection with filtering, sorting, and deduplication.
    fn finish(mut self) -> Vec<SemanticToken> {
        // remove synthetic and internal zero-width spans
        self.tokens
            .retain(|token| token.span.start < token.span.end);

        // order tokens by source position
        self.tokens.sort_by_key(|token| token.span.start);

        self.deduplicate();

        self.tokens
    }

    /// Collect declaration tokens.
    fn collect_declarations(&mut self) {
        let view = self.module.view();

        // collect definition-site declaration names
        for (declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            let source_node_id = view.get_source(declaration_id);
            let Some(main_span) = self.main_span(source_node_id) else {
                continue;
            };

            let mut modifiers = SemanticTokenModifiers::DECLARATION;
            if declaration.export().is_some() {
                modifiers = modifiers.union(SemanticTokenModifiers::DEFINITION);
            }
            if declaration.is_abstract() {
                modifiers = modifiers.union(SemanticTokenModifiers::ABSTRACT);
            }
            if let dir::Declaration::Function(declaration) = declaration {
                if declaration.signature.asynchrony == dir::Asynchrony::Async {
                    modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
                }
            }

            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::declaration(declaration),
                modifiers,
            ));
        }
    }

    /// Collect parameter tokens.
    fn collect_parameters(&mut self) {
        let view = self.module.view();

        // collect parameter declarations
        for (parameter_id, _) in view.iter_nodes_of_type::<dir::Parameter>() {
            let source_node_id = view.get_source(parameter_id);
            let Some(name_span) = self.main_span(source_node_id) else {
                continue;
            };

            self.tokens.push(SemanticToken::new(
                name_span,
                SemanticTokenType::Parameter,
                SemanticTokenModifiers::DECLARATION,
            ));
        }
    }

    /// Collect local variable binding tokens.
    fn collect_pattern_bindings(&mut self) {
        let view = self.module.view();

        // collect binding patterns
        for (pattern_id, pattern) in view.iter_nodes_of_type::<dir::Pattern>() {
            if !matches!(pattern, dir::Pattern::Binding { .. }) {
                continue;
            }

            let source_node_id = view.get_source(pattern_id);
            let Some(main_span) = self.main_span(source_node_id) else {
                continue;
            };

            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::Variable,
                SemanticTokenModifiers::DECLARATION,
            ));
        }
    }

    /// Collect destructuring pattern field tokens.
    fn collect_pattern_fields(&mut self) {
        let view = self.module.view();

        // collect named and spread destructuring bindings
        for (field_id, field) in view.iter_nodes_of_type::<dir::PatternField>() {
            let token_type = match field {
                dir::PatternField::Named { .. } | dir::PatternField::Rest { .. } => {
                    SemanticTokenType::Variable
                }
                dir::PatternField::Positional { .. }
                | dir::PatternField::Computed { .. }
                | dir::PatternField::Elision => continue,
            };

            let source_node_id = view.get_source(field_id);
            let Some(main_span) = self.main_span(source_node_id) else {
                continue;
            };

            self.tokens.push(SemanticToken::new(
                main_span,
                token_type,
                SemanticTokenModifiers::DECLARATION,
            ));
        }
    }

    /// Collect expression tokens.
    fn collect_expressions(&mut self) {
        let view = self.module.view();

        // collect expression literals and visible reference segments
        for (expression_id, expression) in view.iter_nodes_of_type::<dir::Expression>() {
            let source_node_id = view.get_source(expression_id);
            let Some(span) = self.module.tree().get_span_by_id(source_node_id) else {
                continue;
            };

            match expression {
                dir::Expression::Label { .. } => {
                    if let Some(main_span) = self.main_span(source_node_id) {
                        self.tokens.push(SemanticToken::new(
                            main_span,
                            SemanticTokenType::Label,
                            SemanticTokenModifiers::DECLARATION,
                        ));
                    }
                }
                dir::Expression::ScalarLiteral(value) => {
                    self.tokens
                        .push(SemanticToken::plain(span, SemanticTokenType::scalar(value)));
                }
                dir::Expression::TemplateExpression { .. }
                | dir::Expression::TaggedTemplateExpression { .. } => {
                    self.tokens
                        .push(SemanticToken::plain(span, SemanticTokenType::String));
                }
                dir::Expression::Member { .. } => {
                    if let Some(main_span) = self.main_span(source_node_id) {
                        self.tokens
                            .push(SemanticToken::plain(main_span, SemanticTokenType::Property));
                    }
                }
                _ => {}
            }
        }
    }

    /// Collect member tokens.
    fn collect_members(&mut self) {
        let view = self.module.view();

        // collect fields, methods, and associated items
        for (member_id, member) in view.iter_nodes_of_type::<dir::Member>() {
            let source_node_id = view.get_source(member_id);
            let Some(main_span) = self.main_span(source_node_id) else {
                continue;
            };

            let Some((token_type, modifiers)) = Self::member_token(member) else {
                continue;
            };
            self.tokens
                .push(SemanticToken::new(main_span, token_type, modifiers));
        }
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
            dir::Member::Error => panic!("error member reached semantic tokens"),
        }
    }

    /// Collect enum field tokens.
    fn collect_enum_fields(&mut self) {
        let view = self.module.view();

        // collect enum member declarations
        for (field_id, _) in view.iter_nodes_of_type::<dir::EnumField>() {
            let source_node_id = view.get_source(field_id);
            let Some(main_span) = self.main_span(source_node_id) else {
                continue;
            };

            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::EnumMember,
                SemanticTokenModifiers::DECLARATION,
            ));
        }
    }

    /// Collect generic type parameter tokens.
    fn collect_type_parameters(&mut self) {
        let view = self.module.view();

        // collect declaration generic parameter names
        for (_declaration_id, declaration) in view.iter_nodes_of_type::<dir::Declaration>() {
            let Some(generic_parameters) = declaration.generic_parameters() else {
                continue;
            };

            for &parameter_id in generic_parameters {
                let source_node_id = view.get_source(parameter_id);
                let Some(main_span) = self.main_span(source_node_id) else {
                    continue;
                };

                self.tokens.push(SemanticToken::new(
                    main_span,
                    SemanticTokenType::TypeParameter,
                    SemanticTokenModifiers::DECLARATION,
                ));
            }
        }
    }

    /// Collect decorator tokens.
    fn collect_decorators(&mut self) {
        let view = self.module.view();

        // collect decorator names
        for (decorator_id, _) in view.iter_nodes_of_type::<dir::Decorator>() {
            let source_node_id = view.get_source(decorator_id);
            let Some(span) = self.module.tree().get_span_by_id(source_node_id) else {
                continue;
            };

            // prefer the decorator name when a main span exists
            if let Some(main_span) = self.main_span(source_node_id) {
                self.tokens.push(SemanticToken::plain(
                    main_span,
                    SemanticTokenType::Decorator,
                ));
            } else {
                self.tokens
                    .push(SemanticToken::plain(span, SemanticTokenType::Decorator));
            }
        }
    }

    /// Collect documentation comments.
    fn collect_documentation_comments(&mut self) {
        let file = self.module.source_file();

        // collect doc line and block comments
        for comment in self.module.comments().iter().copied() {
            let raw_text = file.span_str(comment.span).trim_start();
            if !raw_text.starts_with("///") && !raw_text.starts_with("/**") {
                continue;
            }

            self.tokens.push(SemanticToken::new(
                comment.span,
                SemanticTokenType::Comment,
                SemanticTokenModifiers::DOCUMENTATION,
            ));
        }
    }

    /// Collect dependency item tokens.
    fn collect_dependency_items(&mut self) {
        let view = self.module.view();

        // collect imported and exported binding names
        for (item_id, _) in view.iter_nodes_of_type::<dir::DependencyItem>() {
            let source_node_id = view.get_source(item_id);
            let Some(main_span) = self.main_span(source_node_id) else {
                continue;
            };

            let target_symbol = self
                .module
                .dependency_symbol_target(item_id)
                .unwrap_or_else(|| panic!("missing dependency target for {item_id:?}"));
            let target_module = self.module.module_context(target_symbol.module_id);
            let target_symbol_table = target_module.symbols();
            let symbol = target_symbol_table.get_symbol(target_symbol.local_id);

            self.tokens.push(SemanticToken::new(
                main_span,
                SemanticTokenType::symbol_kind(symbol.kind),
                SemanticTokenModifiers::DECLARATION,
            ));
        }
    }

    /// Deduplicate overlapping tokens.
    fn deduplicate(&mut self) {
        if self.tokens.len() <= 1 {
            return;
        }

        let mut write_index = 0;
        for read_index in 1..self.tokens.len() {
            if self.tokens[write_index].overlaps(&self.tokens[read_index]) {
                self.tokens[write_index] = self.tokens[read_index].clone();
            } else {
                write_index += 1;
                if write_index != read_index {
                    self.tokens[write_index] = self.tokens[read_index].clone();
                }
            }
        }

        self.tokens.truncate(write_index + 1);
    }

    /// Return the main span for a source node.
    fn main_span(&self, source_node_id: u32) -> Option<Span> {
        self.module
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
    }
}

/// Request semantic tokens for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRequest {
    /// The queried module.
    pub module: Module,
}

/// Request semantic tokens for a document range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensRangeRequest {
    /// The queried range.
    pub range: Range,
}

/// Response payload for semantic tokens queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct SemanticTokensResponse {
    /// Semantic tokens.
    pub tokens: Vec<SemanticToken>,
}

impl ModuleQueryContext<'_> {
    /// Return semantic tokens for a module file.
    pub fn semantic_tokens(&self) -> Vec<SemanticToken> {
        SemanticTokens::collect(self)
    }

    /// Return semantic tokens for a range in a module file.
    pub fn semantic_tokens_range(&self, range: Span) -> Vec<SemanticToken> {
        self.semantic_tokens()
            .into_iter()
            .filter(|token| token.span.start >= range.start && token.span.end <= range.end)
            .collect()
    }
}
