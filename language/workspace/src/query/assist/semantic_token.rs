use destack_dir as dir;
use destack_source::{FileId, NodeSpanType, Span};

use crate::Session;
use crate::query::common::get_module_by_file_id;

/// Semantic token type for LSP semantic highlighting.
///
/// Maps to LSP's SemanticTokenTypes. More granular than lexical highlighting
/// because we have resolution information from DIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
#[derive(Debug, Clone)]
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

/// Get semantic tokens for a file.
///
/// Returns tokens suitable for LSP textDocument/semanticTokens/full.
/// Tokens are in source order (not delta-encoded; the LSP layer handles that).
pub fn semantic_tokens(session: &Session, file: FileId) -> Vec<SemanticToken> {
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };

    let mut tokens = Vec::new();
    let module_guard = module.read();
    let dir_tree = module_guard.dir.tree.read();

    // collect declaration tokens (these are definition sites)
    for (decl_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        let descriptor = declaration.descriptor();

        // get the main span (identifier) for the declaration
        let ast_node_id = dir_tree.get_source(decl_id.id);
        let Some(main_span) = module_guard
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        // determine token type from declaration kind
        let token_type = match declaration {
            dir::Declaration::Function { .. } => SemanticTokenType::Function,
            dir::Declaration::Struct { .. } => SemanticTokenType::Struct,
            dir::Declaration::Class { .. } => SemanticTokenType::Class,
            dir::Declaration::Interface { .. } => SemanticTokenType::Interface,
            dir::Declaration::Enum { .. } => SemanticTokenType::Enum,
            dir::Declaration::Namespace { .. } => SemanticTokenType::Namespace,
            dir::Declaration::Type { .. } => SemanticTokenType::Type,
            dir::Declaration::Extension { .. } => SemanticTokenType::Type,
        };

        // build modifiers
        let mut modifiers = SemanticTokenModifiers::DECLARATION;
        if descriptor.export.is_some() {
            modifiers = modifiers.union(SemanticTokenModifiers::DEFINITION);
        }
        if descriptor.abstraction == dir::DeclarationAbstraction::Abstract {
            modifiers = modifiers.union(SemanticTokenModifiers::ABSTRACT);
        }

        // check for async on functions
        if let dir::Declaration::Function { signature, .. } = declaration
            && signature.asynchrony == dir::Asynchrony::Async
        {
            modifiers = modifiers.union(SemanticTokenModifiers::ASYNC);
        }

        tokens.push(SemanticToken::new(main_span, token_type).with_modifiers(modifiers));
    }

    // collect parameter tokens
    for (parameter_id, parameter) in dir_tree.iter_nodes_of_type::<dir::Parameter>() {
        let ast_node_id = dir_tree.get_source(parameter_id.id);
        let span = module_guard.ast.tree.get_span_by_id(ast_node_id);

        // for parameters, try to get just the name span if available
        let name_span = module_guard
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
            .unwrap_or(span);

        let mut modifiers = SemanticTokenModifiers::DECLARATION;

        // check for readonly via immutable mutability
        if let Some(binding_mod) = parameter.modifiers()
            && binding_mod.mutability == Some(dir::Mutability::Immutable)
        {
            modifiers = modifiers.union(SemanticTokenModifiers::READONLY);
        }

        tokens.push(
            SemanticToken::new(name_span, SemanticTokenType::Parameter).with_modifiers(modifiers),
        );
    }

    // collect local variable bindings (Pattern::Binding)
    for (pattern_id, pattern) in dir_tree.iter_nodes_of_type::<dir::Pattern>() {
        if let dir::Pattern::Binding { mutability, .. } = pattern {
            let ast_node_id = dir_tree.get_source(pattern_id.id);
            let Some(main_span) = module_guard
                .ast
                .tree
                .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
            else {
                continue;
            };

            let mut modifiers = SemanticTokenModifiers::DECLARATION;
            if *mutability == Some(dir::Mutability::Immutable) {
                modifiers = modifiers.union(SemanticTokenModifiers::READONLY);
            } else if *mutability == Some(dir::Mutability::Mutable) {
                modifiers = modifiers.union(SemanticTokenModifiers::MUTABLE);
            }

            tokens.push(
                SemanticToken::new(main_span, SemanticTokenType::Variable).with_modifiers(modifiers),
            );
        }
    }

    // collect pattern field bindings (destructuring)
    for (field_id, field) in dir_tree.iter_nodes_of_type::<dir::PatternField>() {
        let ast_node_id = dir_tree.get_source(field_id.id);
        let Some(main_span) = module_guard
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        let (token_type, modifiers) = match field {
            dir::PatternField::Named { mutability, .. } => {
                let mut mods = SemanticTokenModifiers::DECLARATION;
                if *mutability == Some(dir::Mutability::Immutable) {
                    mods = mods.union(SemanticTokenModifiers::READONLY);
                }
                (SemanticTokenType::Variable, mods)
            }
            dir::PatternField::Alias { mutability, .. } => {
                let mut mods = SemanticTokenModifiers::DECLARATION;
                if *mutability == Some(dir::Mutability::Immutable) {
                    mods = mods.union(SemanticTokenModifiers::READONLY);
                }
                (SemanticTokenType::Variable, mods)
            }
            dir::PatternField::Spread { mutability, .. } => {
                let mut mods = SemanticTokenModifiers::DECLARATION;
                if *mutability == Some(dir::Mutability::Immutable) {
                    mods = mods.union(SemanticTokenModifiers::READONLY);
                }
                (SemanticTokenType::Variable, mods)
            }
            dir::PatternField::Positional { .. } | dir::PatternField::Elision => continue,
        };

        tokens.push(SemanticToken::new(main_span, token_type).with_modifiers(modifiers));
    }

    // collect expression tokens (references, literals, etc.)
    for (expression_id, expression) in dir_tree.iter_nodes_of_type::<dir::Expression>() {
        let ast_node_id = dir_tree.get_source(expression_id.id);
        let span = module_guard.ast.tree.get_span_by_id(ast_node_id);

        match expression {
            // symbol references - look up the symbol to determine type
            dir::Expression::GlobalReference { target_symbol, .. }
            | dir::Expression::LocalReference { target_symbol, .. }
            | dir::Expression::ModuleReference { target_symbol, .. } => {
                let target_module = session.modules.get(target_symbol.module_id);
                let target_guard = target_module.read();
                let target_symbols = target_guard.dir.symbols.read();
                let symbol = target_symbols.get_symbol(target_symbol.local_id);

                let token_type = symbol_type_to_token_type(symbol.ty);
                tokens.push(SemanticToken::new(span, token_type));
            }

            // labelled statement - the label itself
            dir::Expression::Labelled { .. } => {
                if let Some(main_span) = module_guard
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                {
                    tokens.push(
                        SemanticToken::new(main_span, SemanticTokenType::Label)
                            .with_modifiers(SemanticTokenModifiers::DECLARATION),
                    );
                }
            }

            // literals
            dir::Expression::ScalarLiteral { value } => {
                let token_type = match value {
                    dir::ScalarLiteral::String(_)
                    | dir::ScalarLiteral::Character(_)
                    | dir::ScalarLiteral::RegexString { .. } => SemanticTokenType::String,
                    dir::ScalarLiteral::Integer(_)
                    | dir::ScalarLiteral::Float(_)
                    | dir::ScalarLiteral::Bigint(_)
                    | dir::ScalarLiteral::Boolean(_) => SemanticTokenType::Number,
                };
                tokens.push(SemanticToken::new(span, token_type));
            }

            dir::Expression::TypeLiteral { .. } => {
                tokens.push(SemanticToken::new(span, SemanticTokenType::Type));
            }

            dir::Expression::TemplateExpression { .. }
            | dir::Expression::TaggedTemplateExpression { .. } => {
                tokens.push(SemanticToken::new(span, SemanticTokenType::String));
            }

            // member access - the member name is a property
            dir::Expression::Member { .. } => {
                // try to get just the member name span
                if let Some(main_span) = module_guard
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                {
                    tokens.push(SemanticToken::new(main_span, SemanticTokenType::Property));
                }
            }

            _ => {}
        }
    }

    // collect member tokens (fields, methods)
    for (member_id, member) in dir_tree.iter_nodes_of_type::<dir::Member>() {
        let ast_node_id = dir_tree.get_source(member_id.id);

        // try to get the name span
        let Some(main_span) = module_guard
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        let (token_type, modifiers) = match member {
            dir::Member::Field { modifiers, .. } => {
                let mods = modifiers_from_binding(modifiers, true);
                (SemanticTokenType::Property, mods)
            }
            dir::Member::Method {
                modifiers,
                signature,
                ..
            } => {
                let mut mods = modifiers_from_binding(modifiers, true);
                if signature.asynchrony == dir::Asynchrony::Async {
                    mods = mods.union(SemanticTokenModifiers::ASYNC);
                }
                if matches!(
                    signature.abstraction,
                    dir::FunctionAbstraction::Abstract | dir::FunctionAbstraction::AbstractOverride
                ) {
                    mods = mods.union(SemanticTokenModifiers::ABSTRACT);
                }
                (SemanticTokenType::Method, mods)
            }
            dir::Member::Embed { .. } => continue,
            dir::Member::StaticBlock { .. } => continue,
        };

        tokens.push(SemanticToken::new(main_span, token_type).with_modifiers(modifiers));
    }

    // collect enum field tokens
    for (field_id, _field) in dir_tree.iter_nodes_of_type::<dir::EnumField>() {
        let ast_node_id = dir_tree.get_source(field_id.id);

        if let Some(main_span) = module_guard
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
        {
            tokens.push(
                SemanticToken::new(main_span, SemanticTokenType::EnumMember)
                    .with_modifiers(SemanticTokenModifiers::DECLARATION),
            );
        }
    }

    // collect type parameter tokens from declarations with generics
    for (_decl_id, declaration) in dir_tree.iter_nodes_of_type::<dir::Declaration>() {
        let generics = match declaration {
            dir::Declaration::Function { signature, .. } => signature.generics.as_ref(),
            dir::Declaration::Struct { generics, .. }
            | dir::Declaration::Class { generics, .. }
            | dir::Declaration::Interface { generics, .. }
            | dir::Declaration::Enum { generics, .. }
            | dir::Declaration::Namespace { generics, .. } => Some(generics),
            dir::Declaration::Type {
                static_parameters, ..
            } => {
                // type aliases have inline static parameters
                if let Some(params) = static_parameters {
                    for parameter_id in params {
                        let ast_node_id = dir_tree.get_source(parameter_id.id);
                        if let Some(main_span) = module_guard
                            .ast
                            .tree
                            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                        {
                            tokens.push(
                                SemanticToken::new(main_span, SemanticTokenType::TypeParameter)
                                    .with_modifiers(SemanticTokenModifiers::DECLARATION),
                            );
                        }
                    }
                }
                None
            }
            dir::Declaration::Extension { generics, .. } => Some(generics),
        };

        // collect type parameter tokens from static parameters
        if let Some(generics) = generics
            && let Some(static_params) = &generics.static_parameters
        {
            for parameter_id in static_params {
                let ast_node_id = dir_tree.get_source(parameter_id.id);
                if let Some(main_span) = module_guard
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                {
                    tokens.push(
                        SemanticToken::new(main_span, SemanticTokenType::TypeParameter)
                            .with_modifiers(SemanticTokenModifiers::DECLARATION),
                    );
                }
            }
        }
    }

    // collect annotation tokens (decorators, comments)
    for (annotation_id, annotation) in dir_tree.iter_nodes_of_type::<dir::Annotation>() {
        let ast_node_id = dir_tree.get_source(annotation_id.id);
        let span = module_guard.ast.tree.get_span_by_id(ast_node_id);

        match annotation {
            dir::Annotation::Decorator { .. } => {
                // for decorators, highlight the whole thing or just the name
                if let Some(main_span) = module_guard
                    .ast
                    .tree
                    .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
                {
                    tokens.push(SemanticToken::new(main_span, SemanticTokenType::Decorator));
                } else {
                    tokens.push(SemanticToken::new(span, SemanticTokenType::Decorator));
                }
            }
            dir::Annotation::Doc { .. } => {
                tokens.push(
                    SemanticToken::new(span, SemanticTokenType::Comment)
                        .with_modifiers(SemanticTokenModifiers::DOCUMENTATION),
                );
            }
            dir::Annotation::Comment { .. } => {
                tokens.push(SemanticToken::new(span, SemanticTokenType::Comment));
            }
        }
    }

    // collect dependency item tokens (imports/exports)
    for (item_id, item) in dir_tree.iter_nodes_of_type::<dir::DependencyItem>() {
        let ast_node_id = dir_tree.get_source(item_id.id);

        // get the local binding name span
        let Some(main_span) = module_guard
            .ast
            .tree
            .get_side_span_by_id(ast_node_id, NodeSpanType::Main)
        else {
            continue;
        };

        // determine token type based on what we're importing
        let token_type = match item {
            dir::DependencyItem::Local { target_symbol, .. }
            | dir::DependencyItem::Remote { target_symbol, .. } => {
                let target_module = session.modules.get(target_symbol.module_id);
                let target_guard = target_module.read();
                let target_symbols = target_guard.dir.symbols.read();
                let symbol = target_symbols.get_symbol(target_symbol.local_id);
                symbol_type_to_token_type(symbol.ty)
            }
            // unresolved items default to variable
            _ => SemanticTokenType::Variable,
        };

        tokens.push(
            SemanticToken::new(main_span, token_type)
                .with_modifiers(SemanticTokenModifiers::DECLARATION),
        );
    }

    // sort tokens by span start position
    tokens.sort_by_key(|t| t.span.start);

    // deduplicate overlapping tokens (prefer more specific ones which come later in our collection)
    deduplicate_tokens(&mut tokens);

    tokens
}

/// Get semantic tokens for a range in a file.
///
/// Returns tokens suitable for LSP textDocument/semanticTokens/range.
pub fn semantic_tokens_range(session: &Session, file: FileId, range: Span) -> Vec<SemanticToken> {
    semantic_tokens(session, file)
        .into_iter()
        .filter(|token| token.span.start >= range.start && token.span.end <= range.end)
        .collect()
}

/// Extract modifiers from a BindingModifier.
fn modifiers_from_binding(
    binding: &Option<dir::BindingModifier>,
    is_declaration: bool,
) -> SemanticTokenModifiers {
    let mut modifiers = if is_declaration {
        SemanticTokenModifiers::DECLARATION
    } else {
        SemanticTokenModifiers::NONE
    };

    if let Some(binding) = binding {
        if binding.mutability == Some(dir::Mutability::Immutable) {
            modifiers = modifiers.union(SemanticTokenModifiers::READONLY);
        }
        if binding.anchor == Some(dir::BindingAnchor::Static) {
            modifiers = modifiers.union(SemanticTokenModifiers::STATIC);
        }
    }

    modifiers
}

/// Map SymbolType to SemanticTokenType.
fn symbol_type_to_token_type(symbol_type: dir::SymbolType) -> SemanticTokenType {
    match symbol_type {
        dir::SymbolType::Void => SemanticTokenType::Variable,
        dir::SymbolType::Class => SemanticTokenType::Class,
        dir::SymbolType::Struct => SemanticTokenType::Struct,
        dir::SymbolType::Interface => SemanticTokenType::Interface,
        dir::SymbolType::Enum => SemanticTokenType::Enum,
        dir::SymbolType::Function => SemanticTokenType::Function,
        dir::SymbolType::Extension => SemanticTokenType::Type,
        dir::SymbolType::TypeAlias => SemanticTokenType::Type,
        dir::SymbolType::Newtype => SemanticTokenType::Type,
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
