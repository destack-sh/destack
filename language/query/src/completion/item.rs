use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::Patch;
use serde::{Deserialize, Serialize};

use crate::{ImportOrder, Position};

/// Sort order for local semantic candidates.
pub(super) const SORT_LOCAL_SYMBOL: u32 = 10;
/// Sort order for builtin candidates.
pub(super) const SORT_BUILTIN: u32 = 20;
/// Sort order for default candidates.
pub(super) const SORT_DEFAULT: u32 = 100;
/// Sort order for keyword candidates.
pub(super) const SORT_KEYWORD: u32 = 700;

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// The semantic origin bucket for one completion candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub(super) enum CompletionOrigin {
    /// A base completion candidate.
    #[default]
    Base,
    /// A context-shaped completion, such as an expected object-literal field.
    Contextual,
    /// A local or in-scope semantic candidate.
    Local,
    /// A builtin or ambient candidate.
    Builtin,
    /// A candidate that requires a new import.
    AutoImport,
    /// A language keyword candidate.
    Keyword,
}

/// The callable and constructable shape of one completion value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub(super) struct CompletionValueShape {
    /// Whether the completion can be called like one function.
    pub(super) is_callable: bool,
    /// Whether the completion can be used as one constructor target.
    pub(super) is_constructable: bool,
}

impl From<dir::SymbolKind> for CompletionKind {
    /// Convert a symbol type into a completion kind.
    fn from(symbol_kind: dir::SymbolKind) -> Self {
        match symbol_kind {
            dir::SymbolKind::Variable
            | dir::SymbolKind::AssociatedConst
            | dir::SymbolKind::GenericValueParameter => CompletionKind::Variable,
            dir::SymbolKind::Class => CompletionKind::Class,
            dir::SymbolKind::Struct => CompletionKind::Struct,
            dir::SymbolKind::Interface | dir::SymbolKind::NewtypeInterface => {
                CompletionKind::Interface
            }
            dir::SymbolKind::Enum => CompletionKind::Enum,
            dir::SymbolKind::EnumField => CompletionKind::EnumMember,
            dir::SymbolKind::Function => CompletionKind::Function,
            dir::SymbolKind::Label => CompletionKind::Reference,
            dir::SymbolKind::Import => CompletionKind::Reference,
            dir::SymbolKind::Extension => CompletionKind::Class,
            dir::SymbolKind::TypeAlias
            | dir::SymbolKind::AssociatedType
            | dir::SymbolKind::GenericTypeParameter => CompletionKind::TypeParameter,
            dir::SymbolKind::Newtype => CompletionKind::TypeParameter,
        }
    }
}

/// A completion item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Completion {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionKind,
    /// Detail shown alongside the label.
    pub detail: Option<String>,
    /// Documentation for the item.
    pub documentation: Option<String>,
    /// Text to insert when selected if different from the label.
    pub insert_text: Option<String>,
    /// Whether the insert text is one snippet.
    pub is_snippet: bool,
    /// Sort priority: lower = higher priority.
    pub sort_order: u32,
    /// Sort text for LSP if different from the label.
    pub sort_text: Option<String>,
    /// Whether to preselect this item.
    pub preselect: bool,
    /// Whether the item is deprecated.
    pub deprecated: bool,
    /// Additional text edits to apply, for example auto imports.
    pub additional_text_edits: Vec<Patch>,
    /// Whether this completion inserts one auto import.
    #[serde(default)]
    pub is_auto_import: bool,
    /// Matched character positions in the label.
    pub match_positions: Vec<usize>,
    /// The semantic origin bucket for ranking.
    #[serde(skip)]
    pub(super) origin: CompletionOrigin,
    /// The structured import ordering key for ranking.
    #[serde(skip)]
    pub(super) import_order: Option<ImportOrder>,
    /// Whether this member comes from one extension lookup.
    #[serde(skip)]
    pub(super) is_extension_member: bool,
    /// The direct nominal symbol for semantic ranking when available.
    #[serde(skip)]
    pub(super) nominal_symbol: Option<dir::GlobalSymbolId>,
    /// Related nominal symbols for semantic ranking.
    #[serde(skip)]
    pub(super) related_nominals: Vec<dir::GlobalSymbolId>,
    /// The callable and constructable shape for semantic ranking.
    #[serde(skip)]
    pub(super) value_shape: CompletionValueShape,
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
            sort_order: SORT_DEFAULT,
            sort_text: None,
            preselect: false,
            deprecated: false,
            additional_text_edits: Vec::new(),
            is_auto_import: false,
            match_positions: Vec::new(),
            origin: CompletionOrigin::Base,
            import_order: None,
            is_extension_member: false,
            nominal_symbol: None,
            related_nominals: Vec::new(),
            value_shape: CompletionValueShape::default(),
        }
    }

    /// Set the detail text.
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

    /// Mark the item as one snippet.
    pub fn as_snippet(mut self) -> Self {
        self.is_snippet = true;
        self
    }

    /// Mark the item as contextual.
    pub fn as_contextual(mut self) -> Self {
        self.origin = CompletionOrigin::Contextual;
        self
    }

    /// Mark the item as local.
    pub fn as_local(mut self) -> Self {
        self.origin = CompletionOrigin::Local;
        self
    }

    /// Mark the item as builtin.
    pub fn as_builtin(mut self) -> Self {
        self.origin = CompletionOrigin::Builtin;
        self
    }

    /// Mark the item as one auto import.
    pub fn as_auto_import(mut self) -> Self {
        self.is_auto_import = true;
        self.origin = CompletionOrigin::AutoImport;
        self
    }

    /// Set the sort order.
    pub fn with_sort_order(mut self, order: u32) -> Self {
        self.sort_order = order;
        self
    }

    /// Mark the item as preselected.
    pub fn preselected(mut self) -> Self {
        self.preselect = true;
        self
    }

    /// Add additional edits.
    pub fn with_additional_edits(mut self, edits: Vec<Patch>) -> Self {
        self.additional_text_edits = edits;
        self
    }

    /// Mark the item as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Mark the item as one keyword completion.
    pub fn as_keyword(mut self) -> Self {
        self.origin = CompletionOrigin::Keyword;
        self
    }

    /// Set the sort text.
    pub fn with_sort_text(mut self, text: impl Into<String>) -> Self {
        self.sort_text = Some(text.into());
        self
    }

    /// Set the structured import sort key.
    pub(super) fn with_import_order(mut self, order: ImportOrder) -> Self {
        self.import_order = Some(order);
        self
    }

    /// Mark the item as one extension member.
    pub(super) fn with_extension_member(mut self) -> Self {
        self.is_extension_member = true;
        self
    }

    /// Set the direct nominal symbol.
    pub(super) fn with_nominal_symbol(mut self, symbol_id: dir::GlobalSymbolId) -> Self {
        self.nominal_symbol = Some(symbol_id);
        self
    }

    /// Set the related nominal symbols.
    pub(super) fn with_related_nominals(mut self, symbols: Vec<dir::GlobalSymbolId>) -> Self {
        self.related_nominals = symbols;
        self
    }

    /// Set the callable and constructable value shape.
    pub(super) fn with_value_shape(mut self, value_shape: CompletionValueShape) -> Self {
        self.value_shape = value_shape;
        self
    }

    /// Return the stable ordering text for one completion.
    pub(super) fn ordering_text(&self) -> &str {
        self.sort_text
            .as_deref()
            .map_or(self.label.as_str(), |text| text)
    }
}

/// Trigger character that caused the completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CompletionTrigger {
    /// Invoked manually or automatically.
    Invoked,
    /// Triggered by one character, for example `.`.
    Character(char),
    /// Retriggered for incomplete results.
    Incomplete,
}

/// Request completion items at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionRequest {
    /// The queried position.
    pub position: Position,
    /// The trigger that initiated completion.
    pub trigger: CompletionTrigger,
    /// Whether to include auto import completions.
    pub include_imports: bool,
}

/// Response payload for completion queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionResponse {
    /// Completion items.
    pub items: Vec<Completion>,
    /// Whether the results are incomplete.
    pub is_incomplete: bool,
}

impl CompletionValueShape {
    /// Build one coarse callable and constructable shape from one symbol type.
    pub(super) fn symbol_kind(symbol_kind: dir::SymbolKind) -> Self {
        match symbol_kind {
            dir::SymbolKind::Function => Self {
                is_callable: true,
                is_constructable: false,
            },
            dir::SymbolKind::Class | dir::SymbolKind::Struct => Self {
                is_callable: false,
                is_constructable: true,
            },
            _ => Self::default(),
        }
    }
}
