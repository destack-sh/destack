use destack_dir::{self as dir, SymbolSpace, SymbolType};
use destack_source::{FileId, ModuleId};

use super::context::{CompletionContext, ContextResult, detect_completion_context};
use crate::query::common::get_module_by_file_id;
use crate::{Session, TokenAtCursor};

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

impl From<SymbolType> for CompletionKind {
    fn from(ty: SymbolType) -> Self {
        match ty {
            SymbolType::Void => CompletionKind::Variable,
            SymbolType::Class => CompletionKind::Class,
            SymbolType::Struct => CompletionKind::Struct,
            SymbolType::Interface => CompletionKind::Interface,
            SymbolType::Enum => CompletionKind::Enum,
            SymbolType::Function => CompletionKind::Function,
            SymbolType::Extension => CompletionKind::Class,
            SymbolType::TypeAlias => CompletionKind::TypeParameter,
            SymbolType::Newtype => CompletionKind::TypeParameter,
        }
    }
}

/// A completion item.
#[derive(Debug, Clone)]
pub struct Completion {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionKind,
    /// Detail shown alongside the label.
    pub detail: Option<String>,
    /// Documentation for the item.
    pub documentation: Option<String>,
    /// Text to insert when selected (if different from label).
    pub insert_text: Option<String>,
    /// Whether the insert text is a snippet.
    pub is_snippet: bool,
    /// Sort priority (lower = higher priority).
    pub sort_order: u32,
    /// Whether to preselect this item.
    pub preselect: bool,
}

impl Completion {
    /// Create a simple completion.
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            is_snippet: false,
            sort_order: 100,
            preselect: false,
        }
    }

    /// Set the detail.
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the documentation.
    pub fn with_documentation(mut self, doc: impl Into<String>) -> Self {
        self.documentation = Some(doc.into());
        self
    }

    /// Set the insert text.
    pub fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insert_text = Some(text.into());
        self
    }

    /// Mark as a snippet.
    pub fn as_snippet(mut self) -> Self {
        self.is_snippet = true;
        self
    }

    /// Set sort order.
    pub fn with_sort_order(mut self, order: u32) -> Self {
        self.sort_order = order;
        self
    }

    /// Mark as preselected.
    pub fn preselected(mut self) -> Self {
        self.preselect = true;
        self
    }
}

/// Trigger character that caused the completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionTrigger {
    /// Invoked manually or automatically.
    Invoked,
    /// Triggered by a character (e.g., '.').
    Character(char),
    /// Re-triggered for incomplete results.
    Incomplete,
}

/// Get completions at the given position.
/// NOTE #Incomplete: improve completions queries
pub fn completions(
    session: &Session,
    file: FileId,
    offset: u32,
    trigger: CompletionTrigger,
) -> Vec<Completion> {
    // detect completion context
    let ContextResult { context, token } = detect_completion_context(session, file, offset);

    match context {
        CompletionContext::MemberAccess {
            receiver_symbol, ..
        } => complete_members(session, file, receiver_symbol, token.as_ref()),

        CompletionContext::TypePosition => complete_types(session, file, token.as_ref()),

        CompletionContext::ValuePosition { scope_id } => {
            complete_values(session, file, scope_id, token.as_ref(), trigger)
        }

        CompletionContext::ImportPath => {
            Vec::new() // #Incomplete: complete module paths
        }

        CompletionContext::ImportClause { target_module } => {
            complete_imports(session, target_module, token.as_ref())
        }

        CompletionContext::Unknown => complete_all(session, file, token.as_ref()),
    }
}

/// Complete members of a type (after `.`).
fn complete_members(
    session: &Session,
    _file: FileId,
    receiver_symbol: Option<dir::GlobalSymbolId>,
    _token: Option<&TokenAtCursor>,
) -> Vec<Completion> {
    let mut results = Vec::new();

    // if we have a receiver symbol, try to get its members
    if let Some(symbol_id) = receiver_symbol {
        let module = session.modules.get(symbol_id.module_id);
        let module_guard = module.read();
        let symbols = module_guard.dir.symbols.read();
        let _symbol = symbols.get_symbol(symbol_id.local_id);

        // get the scope owned by this symbol (for types like struct/class)
        if let Some(owned_scope_id) = get_symbol_owned_scope(&symbols, symbol_id.local_id) {
            let scope = symbols.get_scope_by_id(owned_scope_id);

            // add all named symbols in the scope as member completions
            for (key, member_id) in &scope.named_symbols {
                if let destack_dir::StaticKey::Name(name_id) = key {
                    let member_symbol = symbols.get_symbol(*member_id);
                    let name = module_guard.ast.strings.get(*name_id).to_string();
                    let kind = CompletionKind::from(member_symbol.ty);

                    let mut completion = Completion::new(name, kind).with_sort_order(10);

                    // add detail based on symbol type
                    if member_symbol.ty == SymbolType::Function {
                        completion = completion.with_detail("method");
                    }

                    results.push(completion);
                }
            }
        }

        drop(symbols);
        drop(module_guard);

        // also check for extension methods
        results.extend(complete_extension_methods(session, symbol_id));
    }

    // if no results or no receiver symbol, fall back to common properties
    if results.is_empty() {
        results.extend(common_member_completions());
    }

    results.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.label.cmp(&b.label)));
    results
}

/// Get the scope owned by a symbol (for types that declare scopes).
fn get_symbol_owned_scope(
    symbols: &dir::SymbolTable,
    symbol_id: dir::LocalSymbolId,
) -> Option<dir::LocalScopeId> {
    // find a scope that has this symbol as its owner
    for scope in symbols.scopes() {
        if scope.owner_id == Some(symbol_id) {
            // return the scope id - we need to find it
            // this is a bit awkward, let's iterate with index
            break;
        }
    }

    // #Incomplete alternative: iterate scopes with their ids
    // for now, return None and rely on extension methods or other fallbacks
    None
}

/// Complete extension methods for a symbol.
fn complete_extension_methods(
    _session: &Session,
    _target_symbol: dir::GlobalSymbolId,
) -> Vec<Completion> {
    // NOTE #Incomplete: iterate through all extensions that match the target type
    Vec::new()
}

/// Common member completions (fallback).
fn common_member_completions() -> Vec<Completion> {
    vec![
        Completion::new("toString", CompletionKind::Method)
            .with_detail("(): string")
            .with_sort_order(50),
        Completion::new("valueOf", CompletionKind::Method)
            .with_detail("(): any")
            .with_sort_order(50),
    ]
}

/// Complete types (in type position).
fn complete_types(
    session: &Session,
    file: FileId,
    token: Option<&TokenAtCursor>,
) -> Vec<Completion> {
    let mut results = Vec::new();

    let Some(module) = get_module_by_file_id(session, file) else {
        return results;
    };

    let module_guard = module.read();
    let symbols = module_guard.dir.symbols.read();

    let prefix = token.map(|t| t.text.as_str()).unwrap_or("");

    // add type symbols from current module
    for symbol in symbols.symbols() {
        // only include type symbols
        if symbol.space != SymbolSpace::Type && symbol.space != SymbolSpace::TypeValue {
            continue;
        }

        let Some(string_id) = symbol.name() else {
            continue;
        };

        let name = module_guard.ast.strings.get(string_id).to_string();

        // filter by prefix if typing
        if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
            continue;
        }

        let kind = CompletionKind::from(symbol.ty);
        results.push(Completion::new(name, kind).with_sort_order(10));
    }

    drop(symbols);
    drop(module_guard);

    // add primitive types
    results.extend(primitive_type_completions(prefix));

    results.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.label.cmp(&b.label)));
    results
}

/// Primitive type completions.
fn primitive_type_completions(prefix: &str) -> Vec<Completion> {
    const PRIMITIVES: &[&str] = &[
        "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64", "float32",
        "float64", "bool", "string", "void", "never", "any", "unknown",
    ];

    PRIMITIVES
        .iter()
        .filter(|p| prefix.is_empty() || p.to_lowercase().starts_with(&prefix.to_lowercase()))
        .map(|p| Completion::new(*p, CompletionKind::TypeParameter).with_sort_order(20))
        .collect()
}

/// Complete values (in expression position).
fn complete_values(
    session: &Session,
    file: FileId,
    scope_id: Option<dir::LocalScopeId>,
    token: Option<&TokenAtCursor>,
    trigger: CompletionTrigger,
) -> Vec<Completion> {
    let mut results = Vec::new();

    let Some(module) = get_module_by_file_id(session, file) else {
        return results;
    };

    let module_guard = module.read();
    let symbols = module_guard.dir.symbols.read();

    let prefix = token.map(|t| t.text.as_str()).unwrap_or("");

    // if we have a scope, walk up from it to collect visible symbols
    if let Some(scope_id) = scope_id {
        let mut current_scope_id = Some(scope_id);

        while let Some(sid) = current_scope_id {
            let scope = symbols.get_scope_by_id(sid);

            for (key, symbol_id) in &scope.named_symbols {
                if let dir::StaticKey::Name(name_id) = key {
                    let symbol = symbols.get_symbol(*symbol_id);

                    // only include value symbols
                    if symbol.space != dir::SymbolSpace::Value
                        && symbol.space != dir::SymbolSpace::TypeValue
                    {
                        continue;
                    }

                    let name = module_guard.ast.strings.get(*name_id).to_string();

                    // filter by prefix
                    if !prefix.is_empty()
                        && !name.to_lowercase().starts_with(&prefix.to_lowercase())
                    {
                        continue;
                    }

                    let kind = CompletionKind::from(symbol.ty);
                    results.push(Completion::new(name, kind).with_sort_order(10));
                }
            }

            current_scope_id = scope.parent.map(|(id, _)| id);
        }
    } else {
        // no scope, add all module-level symbols
        for symbol in symbols.symbols() {
            let Some(string_id) = symbol.name() else {
                continue;
            };

            let name = module_guard.ast.strings.get(string_id).to_string();

            if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                continue;
            }

            let kind = CompletionKind::from(symbol.ty);
            results.push(Completion::new(name, kind).with_sort_order(10));
        }
    }

    drop(symbols);
    drop(module_guard);

    // add keywords if triggered manually or at statement position
    if matches!(trigger, CompletionTrigger::Invoked) || prefix.is_empty() {
        results.extend(keyword_completions_filtered(prefix));
    }

    results.sort_by(|a, b| a.sort_order.cmp(&b.sort_order).then(a.label.cmp(&b.label)));
    results
}

/// Complete imports from a module.
fn complete_imports(
    session: &Session,
    target_module: Option<ModuleId>,
    _token: Option<&TokenAtCursor>,
) -> Vec<Completion> {
    let mut results = Vec::new();

    let Some(module_id) = target_module else {
        return results;
    };

    let module = session.modules.get(module_id);
    let module_guard = module.read();
    let symbols = module_guard.dir.symbols.read();

    // add all exported symbols
    for symbol in symbols.symbols() {
        if symbol.export.is_none() {
            continue;
        }

        let Some(string_id) = symbol.name() else {
            continue;
        };

        let name = module_guard.ast.strings.get(string_id).to_string();
        let kind = CompletionKind::from(symbol.ty);
        results.push(Completion::new(name, kind).with_sort_order(10));
    }

    results.sort_by(|a, b| a.label.cmp(&b.label));
    results
}

/// Complete all symbols (fallback for unknown context).
fn complete_all(session: &Session, file: FileId, token: Option<&TokenAtCursor>) -> Vec<Completion> {
    let mut results = Vec::new();

    let Some(module) = get_module_by_file_id(session, file) else {
        return results;
    };

    let module_guard = module.read();
    let symbols = module_guard.dir.symbols.read();

    let prefix = token.map(|t| t.text.as_str()).unwrap_or("");

    for symbol in symbols.symbols() {
        let Some(string_id) = symbol.name() else {
            continue;
        };

        let name = module_guard.ast.strings.get(string_id).to_string();

        if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
            continue;
        }

        let kind = CompletionKind::from(symbol.ty);
        results.push(Completion::new(name, kind));
    }

    drop(symbols);
    drop(module_guard);

    results.extend(keyword_completions_filtered(prefix));
    results.sort_by(|a, b| a.label.cmp(&b.label));
    results
}

/// Get keyword completions.
#[allow(dead_code)]
fn keyword_completions() -> Vec<Completion> {
    const KEYWORDS: &[&str] = &[
        "as",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "implements",
        "import",
        "in",
        "interface",
        "is",
        "let",
        "match",
        "namespace",
        "new",
        "null",
        "of",
        "private",
        "protected",
        "public",
        "readonly",
        "return",
        "static",
        "struct",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typeof",
        "var",
        "void",
        "where",
        "while",
        "yield",
    ];

    KEYWORDS
        .iter()
        .map(|kw| Completion::new(*kw, CompletionKind::Keyword).with_sort_order(200))
        .collect()
}

/// Get keyword completions filtered by prefix.
fn keyword_completions_filtered(prefix: &str) -> Vec<Completion> {
    const KEYWORDS: &[&str] = &[
        "as",
        "async",
        "await",
        "break",
        "case",
        "catch",
        "class",
        "const",
        "continue",
        "default",
        "do",
        "else",
        "enum",
        "export",
        "extends",
        "false",
        "finally",
        "for",
        "function",
        "if",
        "implements",
        "import",
        "in",
        "interface",
        "is",
        "let",
        "match",
        "namespace",
        "new",
        "null",
        "of",
        "private",
        "protected",
        "public",
        "readonly",
        "return",
        "static",
        "struct",
        "super",
        "switch",
        "this",
        "throw",
        "true",
        "try",
        "type",
        "typeof",
        "var",
        "void",
        "where",
        "while",
        "yield",
    ];

    KEYWORDS
        .iter()
        .filter(|kw| prefix.is_empty() || kw.starts_with(prefix))
        .map(|kw| Completion::new(*kw, CompletionKind::Keyword).with_sort_order(200))
        .collect()
}
