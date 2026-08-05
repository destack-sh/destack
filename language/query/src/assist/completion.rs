use std::mem;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{FileId, Patch, Span};
use serde::{Deserialize, Serialize};

use crate::complete::{CompletionCollector, CompletionCursor, rank_completions};
use crate::source::ImportBinding;
use crate::{
    ImportOrder, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
};

/// Sort order for contextual candidates.
pub(crate) const SORT_CONTEXTUAL: u32 = 0;
/// Sort order for local declaration candidates.
pub(crate) const SORT_LOCAL_SYMBOL: u32 = 10;
/// Sort order for member candidates.
pub(crate) const SORT_MEMBER: u32 = 20;
/// Sort order for builtin candidates.
pub(crate) const SORT_BUILTIN: u32 = 30;
/// Sort order for default candidates.
pub(crate) const SORT_DEFAULT: u32 = 100;
/// Sort order for keyword candidates.
pub(crate) const SORT_KEYWORD: u32 = 700;

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum CompletionItemKind {
    /// Associated constant declaration.
    AssociatedConst,
    /// Associated type declaration.
    AssociatedType,
    /// Method declaration.
    Method,
    /// Function declaration.
    Function,
    /// Constructor declaration.
    Constructor,
    /// Field declaration.
    Field,
    /// Variable declaration.
    Variable,
    /// Class declaration.
    Class,
    /// Interface declaration.
    Interface,
    /// Nominal interface declaration.
    NewtypeInterface,
    /// Nominal type declaration.
    Newtype,
    /// Type alias declaration.
    TypeAlias,
    /// Extension declaration.
    Extension,
    /// Module declaration.
    Module,
    /// Property declaration.
    Property,
    /// Value expression.
    Value,
    /// Enum declaration.
    Enum,
    /// Language keyword.
    Keyword,
    /// Source file.
    File,
    /// Symbol reference.
    Reference,
    /// Label declaration.
    Label,
    /// Source folder.
    Folder,
    /// Enum member.
    EnumMember,
    /// Constant declaration.
    Constant,
    /// Struct declaration.
    Struct,
    /// Type parameter declaration.
    TypeParameter,
    /// Value parameter declaration.
    ValueParameter,
    /// Builtin type.
    BuiltinType,
}

/// The origin bucket for one completion candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub(crate) enum CompletionOrigin {
    /// A context-shaped completion, such as an expected object-literal field.
    Contextual,
    /// A local or in-scope declaration.
    Local,
    /// A builtin or ambient candidate.
    Builtin,
    /// A candidate that requires a new import.
    AutoImport,
    /// A language keyword candidate.
    Keyword,
    /// A member of the completed receiver.
    Member,
}

impl From<dir::SymbolKind> for CompletionItemKind {
    /// Convert a symbol type into a completion kind.
    fn from(symbol_kind: dir::SymbolKind) -> Self {
        match symbol_kind {
            dir::SymbolKind::Variable => CompletionItemKind::Variable,
            dir::SymbolKind::Parameter => CompletionItemKind::ValueParameter,
            dir::SymbolKind::AssociatedConst => CompletionItemKind::AssociatedConst,
            dir::SymbolKind::GenericValueParameter => CompletionItemKind::ValueParameter,
            dir::SymbolKind::Class => CompletionItemKind::Class,
            dir::SymbolKind::Struct => CompletionItemKind::Struct,
            dir::SymbolKind::Interface => CompletionItemKind::Interface,
            dir::SymbolKind::NewtypeInterface => CompletionItemKind::NewtypeInterface,
            dir::SymbolKind::Enum => CompletionItemKind::Enum,
            dir::SymbolKind::Variant => CompletionItemKind::EnumMember,
            dir::SymbolKind::Function => CompletionItemKind::Function,
            dir::SymbolKind::Label => CompletionItemKind::Label,
            dir::SymbolKind::Import => CompletionItemKind::Reference,
            dir::SymbolKind::Extension => CompletionItemKind::Extension,
            dir::SymbolKind::TypeAlias => CompletionItemKind::TypeAlias,
            dir::SymbolKind::AssociatedType => CompletionItemKind::AssociatedType,
            dir::SymbolKind::GenericTypeParameter => CompletionItemKind::TypeParameter,
            dir::SymbolKind::Newtype => CompletionItemKind::Newtype,
        }
    }
}

impl From<dir::MemberKind> for CompletionItemKind {
    /// Convert one member kind into a completion kind.
    fn from(kind: dir::MemberKind) -> Self {
        match kind {
            dir::MemberKind::Field | dir::MemberKind::IndexSignature => Self::Field,
            dir::MemberKind::Property => Self::Property,
            dir::MemberKind::Method | dir::MemberKind::CallSignature => Self::Method,
            dir::MemberKind::Constructor | dir::MemberKind::ConstructSignature => Self::Constructor,
            dir::MemberKind::AssociatedType => Self::AssociatedType,
            dir::MemberKind::AssociatedConst => Self::AssociatedConst,
            dir::MemberKind::Variant => Self::EnumMember,
        }
    }
}

impl From<&dir::Symbol> for CompletionItemKind {
    /// Convert one DIR symbol into its exact completion kind.
    fn from(symbol: &dir::Symbol) -> Self {
        if symbol.kind == dir::SymbolKind::Variable
            && symbol.binding_mutability == Some(dir::Mutability::Immutable)
        {
            Self::Constant
        } else {
            symbol.kind.into()
        }
    }
}

/// The primary source edit applied by one completion item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionEdit {
    /// The source range replaced by the completion.
    pub span: Span,
    /// The inserted text or snippet.
    pub new_text: String,
    /// Whether the inserted text is a snippet.
    pub is_snippet: bool,
}

/// A completion item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionItem {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionItemKind,
    /// Detail shown alongside the label.
    pub detail: Option<String>,
    /// Documentation for the item.
    pub documentation: Option<String>,
    /// The primary source edit.
    pub edit: CompletionEdit,
    /// Whether to preselect this item.
    pub preselect: bool,
    /// Whether this item is deprecated.
    pub is_deprecated: bool,
    /// Additional text edits to apply, for example auto imports.
    pub additional_edits: Vec<Patch>,
    /// Whether this completion inserts one auto import.
    pub is_auto_import: bool,
    /// Matched character positions in the label.
    pub match_positions: Vec<usize>,
}

/// One completion candidate before ranking and source edit construction.
#[derive(Debug)]
pub(crate) struct CompletionCandidate {
    /// The label shown in the completion list.
    pub(crate) label: String,
    /// The kind of completion.
    pub(crate) kind: CompletionItemKind,
    /// Detail shown alongside the label.
    pub(crate) detail: Option<String>,
    /// Documentation for the item.
    pub(crate) documentation: Option<String>,
    /// The insertion produced when this candidate is selected.
    insertion: CompletionInsertion,
    /// The producer ordering bucket.
    pub(crate) producer_order: u32,
    /// Stable text used to order otherwise equal candidates.
    pub(crate) ordering_text: Option<String>,
    /// Whether to preselect this item.
    pub(crate) preselect: bool,
    /// Whether this item is deprecated.
    pub(crate) is_deprecated: bool,
    /// Additional source edits to apply.
    pub(crate) additional_edits: Vec<Patch>,
    /// Matched character positions in the label.
    pub(crate) match_positions: Vec<usize>,
    /// The origin bucket used for ranking.
    pub(crate) origin: CompletionOrigin,
    /// The structured import ordering key for ranking.
    pub(crate) import_order: Option<ImportOrder>,
    /// The exact value type when this candidate denotes one.
    pub(crate) type_id: Option<dir::GlobalTypeId>,
    /// The canonical declaration carried by this candidate.
    symbol: Option<dir::GlobalSymbolId>,
    /// The specialized resolution selected for this candidate.
    resolution: CompletionResolution,
}

/// Specialized work deferred until one completion candidate survives ranking.
#[derive(Debug)]
pub(crate) enum CompletionResolution {
    /// No specialized resolution.
    None,
    /// One struct expression.
    Struct,
    /// One class constructor overload.
    ClassConstructor {
        /// The selected constructor type.
        type_id: dir::GlobalTypeId,
        /// The selected constructor declaration when one exists.
        call_symbol: Option<dir::GlobalSymbolId>,
    },
    /// One newtype constructor overload.
    NewtypeConstructor {
        /// The selected constructor type.
        type_id: dir::GlobalTypeId,
    },
    /// One exact member binding.
    Member {
        /// The DIR member lookup site.
        site: dir::MemberSite,
        /// The selected member key.
        key: dir::StaticKey,
    },
    /// One contextual object field.
    ObjectField {
        /// The object literal member lookup site.
        site: dir::MemberSite,
        /// The selected field key.
        key: dir::StaticKey,
    },
    /// One declaration imported through an exact module specifier.
    AutoImport {
        /// The binding inserted into the current module.
        binding: ImportBinding,
        /// The module specifier inserted into the current module.
        specifier: String,
    },
}

/// The insertion produced by one completion candidate.
#[derive(Debug)]
enum CompletionInsertion {
    /// Insert the candidate label.
    Label,
    /// Resolve a call after ranking.
    Call,
    /// Insert exact text.
    Text(String),
    /// Insert an editor snippet.
    Snippet(String),
}

/// Completion candidates and whether collection was truncated.
pub(crate) struct CompletionCandidates {
    /// The collected candidates.
    pub(crate) items: Vec<CompletionCandidate>,
    /// Whether another request may produce more candidates.
    pub(crate) is_incomplete: bool,
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
    pub position: QueryPosition,
    /// The trigger that initiated completion.
    pub trigger: CompletionTrigger,
    /// Whether to include auto import completions.
    pub include_auto_imports: bool,
}

/// Response payload for completion queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionResponse {
    /// Completion items.
    pub items: Vec<CompletionItem>,
    /// Whether another request may produce more items.
    pub is_incomplete: bool,
}

impl ModuleQueryContext<'_> {
    /// Return completion items at one position.
    pub fn completion(
        &self,
        program: &ProgramQueryContext<'_>,
        file_id: FileId,
        offset: u32,
        trigger: CompletionTrigger,
        include_auto_imports: bool,
    ) -> QueryResult<CompletionResponse> {
        let Some(CompletionCursor { context, token }) =
            self.classify_completion(file_id, offset)?
        else {
            return Ok(CompletionResponse {
                items: Vec::new(),
                is_incomplete: false,
            });
        };

        // collect and rank candidates for the selected context
        let collector = CompletionCollector::new(self, program, file_id);
        let completions =
            collector.collect(trigger, &context, token.as_ref(), include_auto_imports)?;
        let is_incomplete = completions.is_incomplete;
        let completions = rank_completions(completions.items, &context, token.as_ref());

        // resolve only candidates returned to the editor
        let replacement_start = token.as_ref().map_or(offset, |token| token.start);
        let replacement_end = token.as_ref().map_or(offset, |token| token.end);
        let replacement = Span::new(file_id, replacement_start, replacement_end);
        let mut items = Vec::with_capacity(completions.len());
        for completion in completions {
            let completion = collector.resolve(completion)?;
            items.push(completion.into_item(replacement)?);
        }

        Ok(CompletionResponse {
            items,
            is_incomplete,
        })
    }
}

impl CompletionCandidate {
    /// Create a simple completion.
    pub(crate) fn new(
        label: impl Into<String>,
        kind: CompletionItemKind,
        origin: CompletionOrigin,
        producer_order: u32,
    ) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            documentation: None,
            insertion: CompletionInsertion::Label,
            producer_order,
            ordering_text: None,
            preselect: false,
            is_deprecated: false,
            additional_edits: Vec::new(),
            match_positions: Vec::new(),
            origin,
            import_order: None,
            type_id: None,
            symbol: None,
            resolution: CompletionResolution::None,
        }
    }

    /// Defer call insertion until after ranking.
    pub(crate) fn with_call(mut self) -> Self {
        self.insertion = CompletionInsertion::Call;

        self
    }

    /// Return whether this candidate still needs call rendering.
    pub(crate) fn is_call(&self) -> bool {
        matches!(self.insertion, CompletionInsertion::Call)
    }

    /// Set the detail text.
    pub(crate) fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// Set the documentation.
    pub(crate) fn with_documentation(mut self, documentation: impl Into<String>) -> Self {
        self.documentation = Some(documentation.into());
        self
    }

    /// Set the insert text.
    pub(crate) fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insertion = CompletionInsertion::Text(text.into());

        self
    }

    /// Set the insertion snippet.
    pub(crate) fn with_snippet(mut self, text: impl Into<String>) -> Self {
        self.insertion = CompletionInsertion::Snippet(text.into());

        self
    }

    /// Add additional edits.
    pub(crate) fn with_additional_edits(mut self, edits: Vec<Patch>) -> Self {
        self.additional_edits = edits;
        self
    }

    /// Set the stable ordering text.
    pub(crate) fn with_ordering_text(mut self, text: impl Into<String>) -> Self {
        self.ordering_text = Some(text.into());
        self
    }

    /// Mark this item as deprecated.
    pub(crate) fn with_deprecated(mut self) -> Self {
        self.is_deprecated = true;
        self
    }

    /// Set the structured import sort key.
    pub(crate) fn with_import_order(mut self, order: ImportOrder) -> Self {
        self.import_order = Some(order);
        self
    }

    /// Set the exact value type.
    pub(crate) fn with_type_id(mut self, type_id: dir::GlobalTypeId) -> Self {
        self.type_id = Some(type_id);
        self
    }

    /// Set the canonical declaration.
    pub(crate) fn with_symbol(mut self, symbol: dir::GlobalSymbolId) -> Self {
        self.symbol = Some(symbol);

        self
    }

    /// Bind this candidate to one exact member lookup.
    pub(crate) fn with_member(mut self, site: dir::MemberSite, key: dir::StaticKey) -> Self {
        self.resolution = CompletionResolution::Member { site, key };

        self
    }

    /// Bind this candidate to one contextual object field.
    pub(crate) fn with_object_field(mut self, site: dir::MemberSite, key: dir::StaticKey) -> Self {
        self.resolution = CompletionResolution::ObjectField { site, key };

        self
    }

    /// Bind this candidate to one struct expression.
    pub(crate) fn with_struct(mut self, symbol: dir::GlobalSymbolId) -> Self {
        self.symbol = Some(symbol);
        self.resolution = CompletionResolution::Struct;

        self
    }

    /// Bind this candidate to one class constructor overload.
    pub(crate) fn with_class_constructor(
        mut self,
        symbol: dir::GlobalSymbolId,
        type_id: dir::GlobalTypeId,
        call_symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        self.symbol = Some(symbol);
        self.resolution = CompletionResolution::ClassConstructor {
            type_id,
            call_symbol,
        };

        self
    }

    /// Bind this candidate to one newtype constructor overload.
    pub(crate) fn with_newtype_constructor(
        mut self,
        symbol: dir::GlobalSymbolId,
        type_id: dir::GlobalTypeId,
    ) -> Self {
        self.symbol = Some(symbol);
        self.resolution = CompletionResolution::NewtypeConstructor { type_id };

        self
    }

    /// Bind this candidate to one exact auto import.
    pub(crate) fn with_auto_import(mut self, binding: ImportBinding, specifier: String) -> Self {
        self.resolution = CompletionResolution::AutoImport { binding, specifier };

        self
    }

    /// Take the specialized resolution selected for this candidate.
    pub(crate) fn take_resolution(&mut self) -> CompletionResolution {
        mem::replace(&mut self.resolution, CompletionResolution::None)
    }

    /// Return the canonical declaration carried by this candidate.
    pub(crate) fn symbol(&self) -> Option<dir::GlobalSymbolId> {
        self.symbol
    }

    /// Return the stable ordering text for one completion.
    pub(crate) fn ordering_text(&self) -> &str {
        self.ordering_text
            .as_deref()
            .map_or(self.label.as_str(), |text| text)
    }

    /// Build the public completion item for one exact replacement range.
    fn into_item(self, span: Span) -> QueryResult<CompletionItem> {
        let (new_text, is_snippet) = match self.insertion {
            CompletionInsertion::Label => (self.label.clone(), false),
            CompletionInsertion::Text(text) => (text, false),
            CompletionInsertion::Snippet(text) => (text, true),
            CompletionInsertion::Call => {
                return Err(QueryError::invalid(
                    "completion call insertion was not rendered",
                ));
            }
        };

        Ok(CompletionItem {
            label: self.label,
            kind: self.kind,
            detail: self.detail,
            documentation: self.documentation,
            edit: CompletionEdit {
                span,
                new_text,
                is_snippet,
            },
            preselect: self.preselect,
            is_deprecated: self.is_deprecated,
            additional_edits: self.additional_edits,
            is_auto_import: self.origin == CompletionOrigin::AutoImport,
            match_positions: self.match_positions,
        })
    }
}

impl CompletionItemKind {
    /// Return whether this item can insert a call.
    pub(crate) fn is_callable(self) -> bool {
        matches!(self, Self::Constructor | Self::Function | Self::Method)
    }
}
