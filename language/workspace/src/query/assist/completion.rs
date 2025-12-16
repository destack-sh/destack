use destack_source::FileId;

use crate::Session;

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
    _session: &Session,
    _file: FileId,
    _offset: u32,
    _trigger: CompletionTrigger,
) -> Vec<Completion> {
    // 1. determine context (after '.', in type position, etc.)
    // 2. based on context, collect candidates:
    //    - after '.': fields and methods of the receiver type
    //    - in type position: types in scope
    //    - in expression position: variables, functions, types in scope
    //    - in import: exported members
    // 3. filter and sort
    todo!("#Incomplete: completions")
}
