use std::mem;

use destack_dir as dir;
use destack_serde::Reflect;
use destack_source::{Patch, Span};
use serde::{Deserialize, Serialize};

use crate::complete::{CompletionCollector, CompletionCursor};
use crate::source::ImportBinding;
use crate::{
    ImportOrder, ModuleQueryContext, ProgramQueryContext, QueryError, QueryPosition, QueryResult,
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

/// A completion item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct CompletionItem {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionItemKind,
    /// The exact text shown directly after the label.
    pub label_suffix: Option<String>,
    /// The declaration shown in completion details.
    pub declaration: Option<String>,
    /// The declaration owner or import source.
    pub description: Option<String>,
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
}

/// A completion response.
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
        request: CompletionRequest,
        program: &ProgramQueryContext<'_>,
    ) -> QueryResult<CompletionResponse> {
        let position = request.position;
        let file_id = position.file_id;
        let offset = position.offset;
        let Some(CompletionCursor { context, token }) =
            self.classify_completion(file_id, offset)?
        else {
            return Ok(CompletionResponse {
                items: Vec::new(),
                is_incomplete: false,
            });
        };

        // exclude bindings declared by the pattern under initialization
        let initializing_pattern = self.initializing_pattern_at_offset(file_id, offset)?;

        // collect and rank candidates for the selected context
        let collector = CompletionCollector::new(self, program, file_id, initializing_pattern);
        let completions = collector.collect(
            request.trigger,
            &context,
            token.as_ref(),
            request.include_auto_imports,
        )?;
        let completions = completions.rank(&context, token.as_ref());
        let mut is_incomplete = completions.is_incomplete;

        // resolve only candidates returned to the editor
        let replacement_start = token.as_ref().map_or(offset, |token| token.start);
        let replacement_end = token.as_ref().map_or(offset, |token| token.end);
        let replacement = Span::new(file_id, replacement_start, replacement_end);
        let capacity = completions.items.len().min(MAX_COMPLETION_ITEMS);
        let mut items = Vec::with_capacity(capacity);
        let mut expanded = Vec::new();
        let mut has_preselected = false;
        'candidates: for completion in completions.items {
            if items.len() == MAX_COMPLETION_ITEMS {
                is_incomplete = true;

                break;
            }
            collector.expand(completion, &mut expanded)?;

            for completion in expanded.drain(..) {
                if items.len() == MAX_COMPLETION_ITEMS {
                    is_incomplete = true;

                    break 'candidates;
                }

                // omit candidates that cannot introduce their import binding
                let Some(mut item) = collector.resolve(completion, replacement)? else {
                    continue;
                };

                // retain one preselected result after expansion and import planning
                if item.preselect {
                    item.preselect = !has_preselected;
                    has_preselected = true;
                }

                items.push(item);
            }
        }

        Ok(CompletionResponse {
            items,
            is_incomplete,
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
    /// The declaration shown in completion details.
    pub(crate) declaration: Option<String>,
    /// The declaration owner or import source.
    pub(crate) description: Option<String>,
    /// Documentation for the item.
    pub(crate) documentation: Option<String>,
    /// The insertion produced when this candidate is selected.
    insertion: CompletionInsertion,
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

/// One import inserted with a completion candidate.
#[derive(Debug, Clone)]
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
            declaration: None,
            description: None,
            documentation: None,
            insertion: CompletionInsertion::Label,
            ordering_text: None,
            preselect: false,
            is_deprecated: false,
            additional_edits: Vec::new(),
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

    /// Set the declaration shown in completion details.
    pub(crate) fn with_declaration(mut self, declaration: impl Into<String>) -> Self {
        self.declaration = Some(declaration.into());

        self
    }

    /// Set the item description.
    pub(crate) fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());

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

    /// Take the import inserted with this candidate.
    pub(crate) fn take_import(&mut self) -> Option<CompletionImport> {
        self.import.take()
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
    pub(crate) fn into_item(self, span: Span) -> QueryResult<CompletionItem> {
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

        Ok(CompletionItem {
            label: self.label,
            kind: self.kind,
            label_suffix: self.label_suffix,
            declaration: self.declaration,
            description: self.description,
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

    /// Return whether this item is constructable with `new`.
    pub(crate) fn is_constructable(self) -> bool {
        matches!(self, Self::Class | Self::Struct)
    }
}
