use destack_dir::SymbolType;
use destack_source::FileId;

use crate::Session;
use crate::query::common::get_module_by_file_id;

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
pub fn completions(
    session: &Session,
    file: FileId,
    _offset: u32,
    _trigger: CompletionTrigger,
) -> Vec<Completion> {
    let mut results = Vec::new();

    // 1. get the module for this file
    let Some(module) = get_module_by_file_id(session, file) else {
        return results;
    };

    let module_guard = module.read();

    // 2. collect all named symbols from the module
    let symbols = module_guard.dir.symbols.read();
    for symbol in symbols.symbols() {
        // only include symbols with names
        let Some(string_id) = symbol.name() else {
            continue;
        };

        let name = module_guard.ast.strings.get(string_id).to_string();
        let kind = CompletionKind::from(symbol.ty);

        results.push(Completion::new(name, kind));
    }

    // 3. add language keywords
    results.extend(keyword_completions());

    // 4. sort by label
    results.sort_by(|a, b| a.label.cmp(&b.label));

    results
}

/// Get keyword completions.
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
