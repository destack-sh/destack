use destack_dir as dir;
use destack_source::{NodeSpanType, Span};
use serde::{Deserialize, Serialize};

use crate::core::{ModuleQueryContext, QueryModule, QueryRange};
use crate::dir::{
    declaration_export, declaration_is_abstract, dependency_symbol_target, expression_symbol_target,
};

/// Semantic token type for LSP semantic highlighting.
///
/// Maps to LSP's SemanticTokenTypes. More granular than lexical highlighting
/// because we have resolution information from DIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SemanticTokenType {
    Namespace,
    Type,
    Class,
    Enum,
    Interface,
    Struct,
    TypeParameter,
    Parameter,
    Variable,
    Property,
    EnumMember,
    Function,
    Method,
    Macro,
    Keyword,
    Modifier,
    Comment,
    String,
    Number,
    Regexp,
    Operator,
    Decorator,
    Label,
}

/// Semantic token modifiers (can be combined as a bitset).
///
/// Maps to LSP's SemanticTokenModifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SemanticTokenModifiers(u32);

impl SemanticTokenModifiers {
    pub const NONE: Self = Self(0);
    pub const DECLARATION: Self = Self(1 << 0);
    pub const DEFINITION: Self = Self(1 << 1);
    pub const READONLY: Self = Self(1 << 2);
    pub const STATIC: Self = Self(1 << 3);
    pub const DEPRECATED: Self = Self(1 << 4);
    pub const ABSTRACT: Self = Self(1 << 5);
    pub const ASYNC: Self = Self(1 << 6);
    pub const MODIFICATION: Self = Self(1 << 7);
    pub const DOCUMENTATION: Self = Self(1 << 8);
    pub const DEFAULT_LIBRARY: Self = Self(1 << 9);
    pub const MUTABLE: Self = Self(1 << 10);

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn bits(self) -> u32 {
        self.0
    }
}

/// A single semantic token.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticToken {
    /// The span of the token.
    pub span: Span,
    /// The type of the token.
    pub token_type: SemanticTokenType,
    /// The modifiers of the token.
    pub modifiers: SemanticTokenModifiers,
}

impl SemanticToken {
    pub fn new(span: Span, token_type: SemanticTokenType) -> Self {
        Self {
            span,
            token_type,
            modifiers: SemanticTokenModifiers::NONE,
        }
    }

    pub fn with_modifiers(mut self, modifiers: SemanticTokenModifiers) -> Self {
        self.modifiers = self.modifiers.union(modifiers);
        self
    }
}

/// Request semantic tokens for a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTokensRequest {
    /// The queried module.
    pub module: QueryModule,
}

/// Request semantic tokens for a document range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTokensRangeRequest {
    /// The queried range.
    pub range: QueryRange,
}

/// Response payload for semantic tokens queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticTokensResponse {
    /// Semantic tokens.
    pub tokens: Vec<SemanticToken>,
}

/// Get semantic tokens for a query context file.
///
/// Returns tokens suitable for LSP textDocument/semanticTokens/full.
/// Tokens are in source order (not delta-encoded; the LSP layer handles that).
pub fn semantic_tokens(ctx: &ModuleQueryContext<'_>) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let dir_tree = ctx.dir().view();

    // collect declaration tokens (these are definition sites)
    for (decl_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        // get the main span (identifier) for the declaration
        let source_node_id = dir_tree.get_source(decl_id);
        let Some(main_span) = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        // determine token type from declaration kind
        let token_type = match declaration {
            dir::Declaration::Global(_) => SemanticTokenType::Namespace,
            dir::Declaration::Module(_) => SemanticTokenType::Namespace,
            dir::Declaration::Function(_) => SemanticTokenType::Function,
            dir::Declaration::Struct(_) => SemanticTokenType::Struct,
            dir::Declaration::Class(_) => SemanticTokenType::Class,
            dir::Declaration::Interface(_) => SemanticTokenType::Interface,
            dir::Declaration::Enum(_) => SemanticTokenType::Enum,
            dir::Declaration::Type(_) => SemanticTokenType::Type,
            dir::Declaration::Extension(_) => SemanticTokenType::Type,
        };

        // build modifiers
        let mut modifiers = SemanticTokenModifiers::DECLARATION;
        if declaration_export(declaration).is_some() {
            modifiers = modifiers.union(SemanticTokenModifiers::DEFINITION);
        }
        if declaration_is_abstract(declaration) {
            modifiers = modifiers.union(SemanticTokenModifiers::ABSTRACT);
        }

        // check for async on functions
        if let dir::Declaration::Function(declaration) = declaration
            && declaration.signature.asynchrony == dir::Asynchrony::Async
        {
            modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
        }

        tokens.push(SemanticToken::new(main_span, token_type).with_modifiers(modifiers));
    }

    // collect parameter tokens
    for (parameter_id, parameter) in dir_tree.iter_nodes_of_type::<dir::Parameter>() {
        let source_node_id = dir_tree.get_source(parameter_id);
        let Some(span) = ctx.dir().tree().get_span_by_id(source_node_id) else {
            continue;
        };

        // for parameters, try to get just the name span if available
        let name_span = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
            .unwrap_or(span);

        let mut modifiers = SemanticTokenModifiers::DECLARATION;

        // mark readonly parameters directly from their final fields
        if parameter_is_readonly(parameter) {
            modifiers = modifiers.union(SemanticTokenModifiers::READONLY);
        }

        tokens.push(
            SemanticToken::new(name_span, SemanticTokenType::Parameter).with_modifiers(modifiers),
        );
    }

    // collect local variable bindings (Pattern::Binding)
    let mut saw_pattern_bindings = false;
    for (pattern_id, pattern) in dir_tree.iter_nodes_of_type::<dir::Pattern>() {
        if let dir::Pattern::Binding { .. } = pattern {
            saw_pattern_bindings = true;
            let source_node_id = dir_tree.get_source(pattern_id);
            let Some(main_span) = ctx
                .dir()
                .tree()
                .get_side_span_by_id(source_node_id, NodeSpanType::Main)
            else {
                continue;
            };

            let modifiers = SemanticTokenModifiers::DECLARATION;

            tokens.push(
                SemanticToken::new(main_span, SemanticTokenType::Variable)
                    .with_modifiers(modifiers),
            );
        }
    }

    // collect declarator bindings when pattern nodes are absent
    if !saw_pattern_bindings {
        for (_declarator_id, declarator) in dir_tree.iter_nodes_of_type::<dir::Declarator>() {
            let pattern = dir_tree.get::<dir::Pattern>(declarator.pattern);
            let dir::Pattern::Binding { .. } = pattern else {
                continue;
            };

            let source_node_id = dir_tree.get_source(declarator.pattern);
            let Some(main_span) = ctx
                .dir()
                .tree()
                .get_side_span_by_id(source_node_id, NodeSpanType::Main)
            else {
                continue;
            };

            let modifiers = SemanticTokenModifiers::DECLARATION;

            tokens.push(
                SemanticToken::new(main_span, SemanticTokenType::Variable)
                    .with_modifiers(modifiers),
            );
        }
    }

    // collect pattern field bindings (destructuring)
    for (field_id, field) in dir_tree.iter_nodes_of_type::<dir::PatternField>() {
        let source_node_id = dir_tree.get_source(field_id);
        let Some(main_span) = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        let (token_type, modifiers) = match field {
            dir::PatternField::Named { .. } | dir::PatternField::Spread { .. } => (
                SemanticTokenType::Variable,
                SemanticTokenModifiers::DECLARATION,
            ),
            dir::PatternField::Positional { .. }
            | dir::PatternField::Computed { .. }
            | dir::PatternField::Elision => continue,
        };

        tokens.push(SemanticToken::new(main_span, token_type).with_modifiers(modifiers));
    }

    // collect expression tokens (references, literals, etc.)
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let source_node_id = dir_tree.get_source(expression_id);
        let Some(span) = ctx.dir().tree().get_span_by_id(source_node_id) else {
            continue;
        };

        match expression {
            // symbol references - look up the symbol to determine type
            dir::Expression::QualifiedReference { .. } => {
                let Some(target_symbol) = expression_symbol_target(ctx.dir(), expression_id) else {
                    continue;
                };
                let Some(target_ctx) = ctx.module_context(target_symbol.module_id) else {
                    continue;
                };
                let target_symbols = target_ctx.dir().symbols();
                let symbol = target_symbols.get_symbol(target_symbol.local_id);

                let token_type = symbol_form_to_token_type(symbol.form);
                tokens.push(SemanticToken::new(span, token_type));
            }

            // labelled statement - the label itself
            dir::Expression::Label { .. } => {
                if let Some(main_span) = ctx
                    .dir()
                    .tree()
                    .get_side_span_by_id(source_node_id, NodeSpanType::Main)
                {
                    tokens.push(
                        SemanticToken::new(main_span, SemanticTokenType::Label)
                            .with_modifiers(SemanticTokenModifiers::DECLARATION),
                    );
                }
            }

            // literals
            dir::Expression::ScalarLiteral(value) => {
                let token_type = match value {
                    dir::ScalarLiteral::Null => SemanticTokenType::Keyword,
                    dir::ScalarLiteral::String(_) | dir::ScalarLiteral::Character(_) => {
                        SemanticTokenType::String
                    }
                    dir::ScalarLiteral::RegexString { .. } => SemanticTokenType::Regexp,
                    dir::ScalarLiteral::Integer(_)
                    | dir::ScalarLiteral::Float(_)
                    | dir::ScalarLiteral::Bigint(_)
                    | dir::ScalarLiteral::Boolean(_) => SemanticTokenType::Number,
                };
                tokens.push(SemanticToken::new(span, token_type));
            }

            dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. } => {
                tokens.push(SemanticToken::new(span, SemanticTokenType::String));
            }

            // member access - the member name is a property
            dir::Expression::Member { .. } => {
                // try to get just the member name span
                if let Some(main_span) = ctx
                    .dir()
                    .tree()
                    .get_side_span_by_id(source_node_id, NodeSpanType::Main)
                {
                    tokens.push(SemanticToken::new(main_span, SemanticTokenType::Property));
                }
            }

            _ => {}
        }
    }

    // collect member tokens (fields, methods)
    for (member_id, member) in dir_tree.iter_nodes_of_type::<dir::Member>() {
        let source_node_id = dir_tree.get_source(member_id);

        // try to get the name span
        let Some(main_span) = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        let (token_type, modifiers) = match member {
            dir::Member::AssociatedType {
                is_static,
                is_abstract,
                ..
            } => {
                let mods = modifiers_from_member_flags(true, false, *is_static, *is_abstract);
                (SemanticTokenType::Type, mods)
            }
            dir::Member::AssociatedConst { is_static, .. } => {
                let mut mods = modifiers_from_member_flags(true, true, *is_static, false);
                mods = mods.union(SemanticTokenModifiers::READONLY);
                (SemanticTokenType::Property, mods)
            }
            dir::Member::Field {
                is_readonly,
                mutability,
                is_static,
                is_abstract,
                ..
            } => {
                let is_readonly = *is_readonly || *mutability == Some(dir::Mutability::Immutable);
                let mods = modifiers_from_member_flags(true, is_readonly, *is_static, *is_abstract);
                (SemanticTokenType::Property, mods)
            }
            dir::Member::Method {
                signature,
                is_static,
                ..
            } => {
                let mut mods =
                    modifiers_from_member_flags(true, false, *is_static, signature.is_abstract);
                if signature.asynchrony == dir::Asynchrony::Async {
                    mods = mods.union(SemanticTokenModifiers::ASYNC);
                }
                (SemanticTokenType::Method, mods)
            }
            dir::Member::StaticBlock { .. } => continue,
            dir::Member::ComptimeBlock { .. } => continue,
            dir::Member::Error => continue,
        };

        tokens.push(SemanticToken::new(main_span, token_type).with_modifiers(modifiers));
    }

    // collect enum field tokens
    for (field_id, _field) in dir_tree.iter_nodes_of_type::<dir::EnumField>() {
        let source_node_id = dir_tree.get_source(field_id);

        if let Some(main_span) = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
        {
            tokens.push(
                SemanticToken::new(main_span, SemanticTokenType::EnumMember)
                    .with_modifiers(SemanticTokenModifiers::DECLARATION),
            );
        }
    }

    // collect type parameter tokens from declarations with generics
    for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        if let Some(generic_parameters) = declaration.generic_parameters() {
            for &parameter_id in generic_parameters {
                let source_node_id = dir_tree.get_source(parameter_id);
                if let Some(main_span) = ctx
                    .dir()
                    .tree()
                    .get_side_span_by_id(source_node_id, NodeSpanType::Main)
                {
                    tokens.push(
                        SemanticToken::new(main_span, SemanticTokenType::TypeParameter)
                            .with_modifiers(SemanticTokenModifiers::DECLARATION),
                    );
                }
            }
        }
    }

    // collect decorator tokens
    for (decorator_id, _decorator) in dir_tree.iter_nodes_of_type::<dir::Decorator>() {
        let source_node_id = dir_tree.get_source(decorator_id);
        let Some(span) = ctx.dir().tree().get_span_by_id(source_node_id) else {
            continue;
        };

        // for decorators, highlight the whole thing or just the name
        if let Some(main_span) = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
        {
            tokens.push(SemanticToken::new(main_span, SemanticTokenType::Decorator));
        } else {
            tokens.push(SemanticToken::new(span, SemanticTokenType::Decorator));
        }
    }

    // collect documentation comment tokens
    let Some(file) = ctx
        .repository()
        .file(ctx.revision(), ctx.file_id())
        .ok()
        .flatten()
    else {
        return tokens;
    };
    for comment in ctx.dir().tree().comments().iter().copied() {
        let raw_text = file.span_str(comment.span);
        let raw_text = raw_text.trim_start();
        if !raw_text.starts_with("///") && !raw_text.starts_with("/**") {
            continue;
        }

        tokens.push(
            SemanticToken::new(comment.span, SemanticTokenType::Comment)
                .with_modifiers(SemanticTokenModifiers::DOCUMENTATION),
        );
    }

    // collect dependency item tokens (imports/exports)
    for (item_id, _) in dir_tree.iter_nodes_of_type::<dir::DependencyItem>() {
        let source_node_id = dir_tree.get_source(item_id);

        // get the local binding name span
        let Some(main_span) = ctx
            .dir()
            .tree()
            .get_side_span_by_id(source_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        // determine token type based on what we're importing
        let token_type = if let Some(target_symbol) = dependency_symbol_target(ctx.dir(), item_id) {
            let Some(target_ctx) = ctx.module_context(target_symbol.module_id) else {
                continue;
            };
            let target_symbols = target_ctx.dir().symbols();
            let symbol = target_symbols.get_symbol(target_symbol.local_id);
            symbol_form_to_token_type(symbol.form)
        } else {
            SemanticTokenType::Variable
        };

        tokens.push(
            SemanticToken::new(main_span, token_type)
                .with_modifiers(SemanticTokenModifiers::DECLARATION),
        );
    }

    // filter out zero-width spans (synthetic/internal nodes with no visible source text)
    tokens.retain(|t| t.span.start < t.span.end);

    // sort tokens by span start position
    tokens.sort_by_key(|t| t.span.start);

    // deduplicate overlapping tokens (prefer more specific ones which come later in our collection)
    deduplicate_tokens(&mut tokens);

    tokens
}

/// Get semantic tokens for a range in a query context file.
///
/// Returns tokens suitable for LSP textDocument/semanticTokens/range.
pub fn semantic_tokens_range(ctx: &ModuleQueryContext<'_>, range: Span) -> Vec<SemanticToken> {
    semantic_tokens(ctx)
        .into_iter()
        .filter(|token| token.span.start >= range.start && token.span.end <= range.end)
        .collect()
}

/// Extract modifiers from explicit member flags.
fn modifiers_from_member_flags(
    is_declaration: bool,
    is_readonly: bool,
    is_static: bool,
    is_abstract: bool,
) -> SemanticTokenModifiers {
    let mut modifiers = if is_declaration {
        SemanticTokenModifiers::DECLARATION
    } else {
        SemanticTokenModifiers::NONE
    };

    if is_readonly {
        modifiers = modifiers.union(SemanticTokenModifiers::READONLY);
    }
    if is_static {
        modifiers = modifiers.union(SemanticTokenModifiers::STATIC);
    }
    if is_abstract {
        modifiers = modifiers.union(SemanticTokenModifiers::ABSTRACT);
    }

    modifiers
}

/// Return whether a parameter is readonly.
fn parameter_is_readonly(parameter: &dir::Parameter) -> bool {
    match parameter {
        dir::Parameter::Named { is_readonly, .. }
        | dir::Parameter::VariadicNamed { is_readonly, .. } => *is_readonly,
        dir::Parameter::Pattern { .. }
        | dir::Parameter::VariadicPattern { .. }
        | dir::Parameter::Error => false,
    }
}

/// Map SymbolForm to SemanticTokenType.
fn symbol_form_to_token_type(symbol_form: dir::SymbolForm) -> SemanticTokenType {
    match symbol_form {
        dir::SymbolForm::Variable => SemanticTokenType::Variable,
        dir::SymbolForm::Class => SemanticTokenType::Class,
        dir::SymbolForm::Struct => SemanticTokenType::Struct,
        dir::SymbolForm::Interface | dir::SymbolForm::NewtypeInterface => {
            SemanticTokenType::Interface
        }
        dir::SymbolForm::Enum => SemanticTokenType::Enum,
        dir::SymbolForm::Function => SemanticTokenType::Function,
        dir::SymbolForm::Label => SemanticTokenType::Label,
        dir::SymbolForm::Import => SemanticTokenType::Variable,
        dir::SymbolForm::Extension => SemanticTokenType::Type,
        dir::SymbolForm::TypeAlias => SemanticTokenType::Type,
        dir::SymbolForm::Newtype => SemanticTokenType::Type,
    }
}

/// Remove duplicate tokens, preferring later ones (which are more specific).
fn deduplicate_tokens(tokens: &mut Vec<SemanticToken>) {
    if tokens.len() <= 1 {
        return;
    }

    let mut write_idx = 0;
    for read_idx in 1..tokens.len() {
        // if this token overlaps with the previous one, skip the previous
        if tokens[read_idx].span.start < tokens[write_idx].span.end {
            // keep the one with more specific type (later in our iteration order)
            tokens[write_idx] = tokens[read_idx].clone();
        } else {
            write_idx += 1;
            if write_idx != read_idx {
                tokens[write_idx] = tokens[read_idx].clone();
            }
        }
    }
    tokens.truncate(write_idx + 1);
}
