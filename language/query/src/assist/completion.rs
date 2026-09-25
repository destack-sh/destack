use std::mem;

use serde::{Deserialize, Serialize};
use tspp_dir as dir;
use tspp_serde::Reflect;
use tspp_source::{FileId, Patch, Span};

use crate::complete::CompletionCollector;
use crate::source::ImportBinding;
use crate::{
    Formatter, ImportOrder, Module, ModuleQueryContext, ProgramQueryContext, QueryError,
    QueryPosition, QueryResult,
};

/// Maximum completion items returned in one response.
pub(crate) const MAX_COMPLETION_ITEMS: usize = 100;

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

impl From<dir::SymbolKind> for CompletionItemKind {
    /// Convert a symbol type into a completion kind.
    fn from(symbol_kind: dir::SymbolKind) -> Self {
        match symbol_kind {
            dir::SymbolKind::Variable => CompletionItemKind::Variable,
            dir::SymbolKind::Parameter => CompletionItemKind::ValueParameter,
            dir::SymbolKind::Label => CompletionItemKind::Label,
            dir::SymbolKind::AssociatedConst => CompletionItemKind::AssociatedConst,
            dir::SymbolKind::GenericLifetimeParameter => CompletionItemKind::ValueParameter,
            dir::SymbolKind::Class => CompletionItemKind::Class,
            dir::SymbolKind::Struct => CompletionItemKind::Struct,
            dir::SymbolKind::Interface => CompletionItemKind::Interface,
            dir::SymbolKind::NewtypeInterface => CompletionItemKind::NewtypeInterface,
            dir::SymbolKind::Enum => CompletionItemKind::Enum,
            dir::SymbolKind::Variant => CompletionItemKind::EnumMember,
            dir::SymbolKind::Function => CompletionItemKind::Function,
            dir::SymbolKind::Import | dir::SymbolKind::ExportAlias => CompletionItemKind::Reference,
            dir::SymbolKind::Extension => CompletionItemKind::Extension,
            dir::SymbolKind::TypeAlias => CompletionItemKind::TypeAlias,
            dir::SymbolKind::AssociatedType => CompletionItemKind::AssociatedType,
            dir::SymbolKind::GenericTypeParameter => CompletionItemKind::TypeParameter,
            dir::SymbolKind::GenericConstParameter => CompletionItemKind::Variable,
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

/// One entry in a completion list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionEntry {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionItemKind,
    /// The exact text shown directly after the label.
    pub label_suffix: Option<String>,
    /// The declaration owner or import source.
    pub description: Option<String>,
    /// The primary source edit.
    pub edit: CompletionEdit,
    /// Whether to preselect this item.
    pub preselect: bool,
    /// Whether this item is deprecated.
    pub is_deprecated: bool,
    /// Whether this completion inserts one auto import.
    pub is_auto_import: bool,
    /// Matched character positions in the label.
    pub match_positions: Vec<usize>,
    /// Expanded fields or the request that computes them.
    pub details: Option<CompletionEntryDetails>,
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

/// A completion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionRequest {
    /// The queried position.
    pub position: QueryPosition,
    /// The trigger that initiated completion.
    pub trigger: CompletionTrigger,
    /// Whether to include auto import completions.
    pub include_auto_imports: bool,
    /// When expanded fields are computed.
    pub details: CompletionDetailsMode,
}

/// When expanded completion fields are computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CompletionDetailsMode {
    /// Compute expanded fields in the completion list.
    Eager,
    /// Compute expanded fields after selecting an entry.
    Deferred,
}

/// A completion response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionResponse {
    /// Completion entries.
    pub entries: Vec<CompletionEntry>,
    /// Whether another request may produce more items.
    pub is_incomplete: bool,
}

/// Request for the expanded fields of one completion entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionDetailsRequest {
    /// The module that produced the entry.
    pub module: Module,
    /// The source file that produced the entry.
    pub file_id: FileId,
    /// The selected declaration when one exists.
    pub symbol: Option<dir::GlobalSymbolId>,
    /// Additional source edits applied with the selected entry.
    pub additional_edits: Vec<Patch>,
}

/// Expanded fields for one selected completion entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionDetailsResponse {
    /// The declaration shown for the selected entry.
    pub declaration: Option<String>,
    /// Documentation for the selected entry.
    pub documentation: Option<String>,
    /// Additional source edits applied with the selected entry.
    pub additional_edits: Vec<Patch>,
}

/// Expanded fields retained by one completion entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum CompletionEntryDetails {
    /// A request retained until the entry is selected.
    Deferred(CompletionDetailsRequest),
    /// Fields computed with the completion list.
    Eager(CompletionDetailsResponse),
}

impl ModuleQueryContext<'_> {
    /// Return completion entries at one position.
    pub fn completion(
        &self,
        request: CompletionRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<CompletionResponse> {
        let position = request.position;
        let details_mode = request.details;
        let file_id = position.file_id;
        let offset = position.offset;
        let cursor = self.cursor(file_id, offset)?;
        let Some(context) = cursor.classify_completion()? else {
            return Ok(CompletionResponse {
                entries: Vec::new(),
                is_incomplete: false,
            });
        };

        // exclude bindings declared by the pattern under initialization
        let initializing_pattern = cursor.initializing_pattern()?;

        // collect and rank candidates for the selected context
        let collector = CompletionCollector::new(self, program, initializing_pattern);
        let completions =
            collector.collect(request.trigger, &context, request.include_auto_imports)?;
        let completions = completions.rank(&context);
        let mut is_incomplete = completions.is_incomplete;

        // collect authored imports once for auto import planning
        let imports = completions
            .items
            .iter()
            .any(|completion| completion.import().is_some())
            .then(|| self.import_declarations(file_id))
            .transpose()?;

        // resolve only candidates returned to the editor
        let replacement_start = context
            .prefix
            .as_ref()
            .map_or(offset, |prefix| prefix.start);
        let replacement_end = context.prefix.as_ref().map_or(offset, |prefix| prefix.end);
        let replacement = Span::new(file_id, replacement_start, replacement_end);

        // retain the authored arguments of the completed call
        let has_arguments = cursor.has_call_arguments()?;

        // expand and render the ranked entries
        let capacity = completions.items.len().min(MAX_COMPLETION_ITEMS);
        let mut entries = Vec::with_capacity(capacity);
        let mut expanded = Vec::new();
        let mut has_preselected = false;
        'candidates: for completion in completions.items {
            if entries.len() == MAX_COMPLETION_ITEMS {
                is_incomplete = true;

                break;
            }
            collector.expand(completion, &mut expanded)?;

            for completion in expanded.drain(..) {
                if entries.len() == MAX_COMPLETION_ITEMS {
                    is_incomplete = true;

                    break 'candidates;
                }

                // omit candidates that cannot introduce their import binding
                let Some(mut entry) = collector.entry(
                    completion,
                    position,
                    replacement,
                    has_arguments,
                    imports.as_ref(),
                )?
                else {
                    continue;
                };

                // retain one preselected result after expansion and import planning
                if entry.preselect {
                    entry.preselect = !has_preselected;
                    has_preselected = true;
                }

                // compute expanded fields for clients without lazy resolution
                if details_mode == CompletionDetailsMode::Eager
                    && let Some(details) = entry.details.take()
                {
                    let CompletionEntryDetails::Deferred(request) = details else {
                        return Err(QueryError::invalid(
                            "completion details were already computed",
                        ));
                    };
                    let details = self.completion_details(request, program)?;
                    entry.details = Some(CompletionEntryDetails::Eager(details));
                }

                entries.push(entry);
            }
        }

        Ok(CompletionResponse {
            entries,
            is_incomplete,
        })
    }

    /// Return expanded fields for one selected completion entry.
    pub fn completion_details(
        &self,
        request: CompletionDetailsRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<CompletionDetailsResponse> {
        // render declaration text and authored documentation
        let (declaration, documentation) = match request.symbol {
            Some(symbol) => {
                let module = program.module(symbol.module_id)?;
                let formatter = Formatter::new(&module, program);
                let declaration = Some(formatter.symbol_signature(symbol)?);
                let documentation = program.symbol_documentation(symbol)?;

                (declaration, documentation)
            }
            None => (None, None),
        };

        Ok(CompletionDetailsResponse {
            declaration,
            documentation,
            additional_edits: request.additional_edits,
        })
    }
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

impl CompletionOrigin {
    /// Return the ranking order for this completion origin.
    pub(crate) fn order(self) -> u8 {
        match self {
            Self::Contextual => 0,
            Self::Member | Self::Local => 1,
            Self::Builtin => 2,
            Self::AutoImport => 3,
            Self::Keyword => 4,
        }
    }
}

/// One completion candidate before ranking and source edit construction.
#[derive(Debug, Clone)]
pub(crate) struct CompletionCandidate {
    /// The label shown in the completion list.
    pub(crate) label: String,
    /// The kind of completion.
    pub(crate) kind: CompletionItemKind,
    /// The exact text shown directly after the label.
    pub(crate) label_suffix: Option<String>,
    /// The declaration owner or import source.
    pub(crate) description: Option<String>,
    /// The insertion produced when this candidate is selected.
    pub(crate) insertion: CompletionInsertion,
    /// Stable text used to order otherwise equal candidates.
    pub(crate) ordering_text: Option<String>,
    /// Whether to preselect this item.
    pub(crate) preselect: bool,
    /// Whether this item is deprecated.
    pub(crate) is_deprecated: bool,
    /// Matched character positions in the label.
    pub(crate) match_positions: Vec<usize>,
    /// The origin bucket used for ranking.
    pub(crate) origin: CompletionOrigin,
    /// The structured import ordering key for ranking.
    pub(crate) import_order: Option<ImportOrder>,
    /// The exact value type when this candidate denotes one.
    pub(crate) type_id: Option<dir::GlobalTypeId>,
    /// The target declaration carried by this candidate.
    symbol: Option<dir::GlobalSymbolId>,
    /// The constructor family expanded after ranking.
    constructors: Option<ConstructorFamily>,
    /// The member completion deferred until after ranking.
    member: Option<CompletionMember>,
    /// The import inserted with this candidate.
    import: Option<CompletionImport>,
}

/// One member completion resolved after candidate ranking.
#[derive(Debug, Clone)]
pub(crate) enum CompletionMember {
    /// One exact member binding.
    Access {
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
}

/// One constructor family expanded after candidate ranking.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ConstructorFamily {
    /// The constructors declared by one class.
    Class,
    /// The constructors declared by one newtype.
    Newtype,
}

/// The insertion produced by one completion candidate.
#[derive(Debug, Clone)]
pub(crate) enum CompletionInsertion {
    /// Insert the candidate label.
    Label,
    /// Resolve a call after ranking.
    Call,
    /// Resolve one struct expression after ranking.
    StructExpression,
    /// Resolve one class constructor overload after ranking.
    ClassConstructor {
        /// The selected constructor type.
        type_id: dir::GlobalTypeId,
        /// The selected constructor declaration when one exists.
        call_symbol: Option<dir::GlobalSymbolId>,
    },
    /// Resolve one newtype constructor overload after ranking.
    NewtypeConstructor {
        /// The selected constructor type.
        type_id: dir::GlobalTypeId,
    },
    /// Insert exact text.
    Text(String),
    /// Insert an editor snippet.
    Snippet(String),
}

/// One import inserted with a completion entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionImport {
    /// The binding inserted into the current module.
    pub(crate) binding: ImportBinding,
    /// The module specifier inserted into the current module.
    pub(crate) specifier: String,
}

/// Completion candidates and whether collection was truncated.
pub(crate) struct CompletionCandidates {
    /// The collected candidates.
    pub(crate) items: Vec<CompletionCandidate>,
    /// Whether another request may produce more candidates.
    pub(crate) is_incomplete: bool,
}

impl CompletionCandidate {
    /// Create a simple completion.
    pub(crate) fn new(
        label: impl Into<String>,
        kind: CompletionItemKind,
        origin: CompletionOrigin,
    ) -> Self {
        Self {
            label: label.into(),
            kind,
            label_suffix: None,
            description: None,
            insertion: CompletionInsertion::Label,
            ordering_text: None,
            preselect: false,
            is_deprecated: false,
            match_positions: Vec::new(),
            origin,
            import_order: None,
            type_id: None,
            symbol: None,
            constructors: None,
            member: None,
            import: None,
        }
    }

    /// Defer call insertion until after ranking.
    pub(crate) fn with_call(mut self) -> Self {
        self.insertion = CompletionInsertion::Call;

        self
    }

    /// Set the exact suffix shown after the label.
    pub(crate) fn with_label_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.label_suffix = Some(suffix.into());

        self
    }

    /// Set the item description.
    pub(crate) fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());

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

    /// Set the target declaration.
    pub(crate) fn with_symbol(mut self, symbol: dir::GlobalSymbolId) -> Self {
        self.symbol = Some(symbol);

        self
    }

    /// Set one exact member lookup.
    pub(crate) fn with_member(mut self, site: dir::MemberSite, key: dir::StaticKey) -> Self {
        self.member = Some(CompletionMember::Access { site, key });

        self
    }

    /// Set one contextual object field lookup.
    pub(crate) fn with_object_field(mut self, site: dir::MemberSite, key: dir::StaticKey) -> Self {
        self.member = Some(CompletionMember::ObjectField { site, key });

        self
    }

    /// Set struct expression insertion.
    pub(crate) fn with_struct(mut self, symbol: dir::GlobalSymbolId) -> Self {
        self.symbol = Some(symbol);
        self.insertion = CompletionInsertion::StructExpression;

        self
    }

    /// Set class constructor expansion.
    pub(crate) fn with_class_constructors(mut self, symbol: dir::GlobalSymbolId) -> Self {
        self.symbol = Some(symbol);
        self.constructors = Some(ConstructorFamily::Class);

        self
    }

    /// Set one class constructor overload.
    pub(crate) fn with_class_constructor(
        mut self,
        symbol: dir::GlobalSymbolId,
        type_id: dir::GlobalTypeId,
        call_symbol: Option<dir::GlobalSymbolId>,
    ) -> Self {
        self.symbol = Some(symbol);
        self.insertion = CompletionInsertion::ClassConstructor {
            type_id,
            call_symbol,
        };

        self
    }

    /// Set newtype constructor expansion.
    pub(crate) fn with_newtype_constructors(mut self, symbol: dir::GlobalSymbolId) -> Self {
        self.symbol = Some(symbol);
        self.constructors = Some(ConstructorFamily::Newtype);

        self
    }

    /// Set one newtype constructor overload.
    pub(crate) fn with_newtype_constructor(
        mut self,
        symbol: dir::GlobalSymbolId,
        type_id: dir::GlobalTypeId,
    ) -> Self {
        self.symbol = Some(symbol);
        self.insertion = CompletionInsertion::NewtypeConstructor { type_id };

        self
    }

    /// Set one exact import.
    pub(crate) fn with_import(mut self, binding: ImportBinding, specifier: String) -> Self {
        self.import = Some(CompletionImport { binding, specifier });

        self
    }

    /// Take the constructor family selected for this candidate.
    pub(crate) fn take_constructors(&mut self) -> Option<ConstructorFamily> {
        self.constructors.take()
    }

    /// Take the member completion selected for this candidate.
    pub(crate) fn take_member(&mut self) -> Option<CompletionMember> {
        self.member.take()
    }

    /// Take the insertion selected for this candidate.
    pub(crate) fn take_insertion(&mut self) -> CompletionInsertion {
        mem::replace(&mut self.insertion, CompletionInsertion::Label)
    }

    /// Return the import inserted with this candidate.
    pub(crate) fn import(&self) -> Option<&CompletionImport> {
        self.import.as_ref()
    }

    /// Return the target declaration carried by this candidate.
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
    pub(crate) fn into_entry(
        self,
        position: QueryPosition,
        span: Span,
        additional_edits: Vec<Patch>,
    ) -> QueryResult<CompletionEntry> {
        let (new_text, is_snippet) = match self.insertion {
            CompletionInsertion::Label => (self.label.clone(), false),
            CompletionInsertion::Text(text) => (text, false),
            CompletionInsertion::Snippet(text) => (text, true),
            CompletionInsertion::Call
            | CompletionInsertion::StructExpression
            | CompletionInsertion::ClassConstructor { .. }
            | CompletionInsertion::NewtypeConstructor { .. } => {
                return Err(QueryError::invalid("completion insertion was not rendered"));
            }
        };

        let details = if self.symbol.is_some() || !additional_edits.is_empty() {
            Some(CompletionEntryDetails::Deferred(CompletionDetailsRequest {
                module: position.module,
                file_id: position.file_id,
                symbol: self.symbol,
                additional_edits,
            }))
        } else {
            None
        };

        Ok(CompletionEntry {
            label: self.label,
            kind: self.kind,
            label_suffix: self.label_suffix,
            description: self.description,
            edit: CompletionEdit {
                span,
                new_text,
                is_snippet,
            },
            preselect: self.preselect,
            is_deprecated: self.is_deprecated,
            is_auto_import: self.origin == CompletionOrigin::AutoImport,
            match_positions: self.match_positions,
            details,
        })
    }
}

impl CompletionItemKind {
    /// Return whether this item can insert a call.
    pub(crate) fn is_callable(self) -> bool {
        matches!(self, Self::Constructor | Self::Function | Self::Method)
    }

    /// Return whether this item shows a value type after its label.
    pub(crate) fn has_value_suffix(self) -> bool {
        matches!(
            self,
            Self::AssociatedConst
                | Self::Constant
                | Self::EnumMember
                | Self::Field
                | Self::Function
                | Self::Method
                | Self::Property
                | Self::Value
                | Self::ValueParameter
                | Self::Variable
        )
    }

    /// Return whether this item shows generic parameters after its label.
    pub(crate) fn has_generic_suffix(self) -> bool {
        matches!(
            self,
            Self::Class
                | Self::Enum
                | Self::Extension
                | Self::Interface
                | Self::Newtype
                | Self::NewtypeInterface
                | Self::Struct
                | Self::TypeAlias
        )
    }
}
