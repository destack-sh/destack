use std::env;
use std::sync::OnceLock;

use destack_dir::{self as dir, SymbolSpace, SymbolType, Type};
use destack_source::{Edit, FileId, ModuleId, PackageId, PathExt, Uri};
use serde::{Deserialize, Serialize};

use super::context::{CompletionContext, ContextResult, detect_completion_context};
use super::fuzzy::score_completion;
use crate::format::format_local_type;
use crate::query::common::{
    MemberKind, MemberName, build_import_edits, dynamic_parameter_display_names,
    dynamic_parameter_names, get_canonical_symbol, get_module_by_file_id, resolve_type_members,
    search_importable_symbols, visible_symbols,
};
use crate::{Session, TokenAtCursor};

// sort order priorities (lower = higher priority in completion list)
const SORT_LOCAL_SYMBOL: u32 = 10;
const SORT_EXTENSION_METHOD: u32 = 15;
const SORT_BUILTIN: u32 = 20;
const SORT_IMPORT_STARTER: u32 = 50;
const SORT_DEFAULT: u32 = 100;
const SORT_KEYWORD: u32 = 700;
const SORT_AUTO_IMPORT: u32 = 500;

/// Weights for auto import ranking heuristics.
#[derive(Debug, Clone, Copy)]
struct AutoImportWeights {
    /// The bonus for the same folder.
    same_folder_bonus: u32,
    /// The bonus for the same package.
    same_package_bonus: u32,
    /// The penalty for a different package.
    other_package_penalty: u32,
    /// The multiplier for path distance.
    distance_multiplier: u32,
    /// The cap for the distance penalty.
    distance_cap: u32,
    /// The multiplier for depth difference.
    depth_multiplier: u32,
    /// The cap for the depth penalty.
    depth_cap: u32,
    /// The minimum auto import sort order.
    minimum_sort_order: u32,
}

const AUTO_IMPORT_WEIGHTS: AutoImportWeights = AutoImportWeights {
    same_folder_bonus: 100,
    same_package_bonus: 40,
    other_package_penalty: 80,
    distance_multiplier: 8,
    distance_cap: 280,
    depth_multiplier: 3,
    depth_cap: 60,
    minimum_sort_order: 340,
};

/// Get auto import weights, optionally overridden by the environment.
fn auto_import_weights() -> AutoImportWeights {
    static WEIGHTS: OnceLock<AutoImportWeights> = OnceLock::new();
    *WEIGHTS.get_or_init(|| parse_auto_import_weights_from_env().unwrap_or(AUTO_IMPORT_WEIGHTS))
}

/// Parse auto import weights from `DESTACK_AUTO_IMPORT_WEIGHTS`.
fn parse_auto_import_weights_from_env() -> Option<AutoImportWeights> {
    // read the raw environment override string
    let raw = env::var("DESTACK_AUTO_IMPORT_WEIGHTS").ok()?;

    // start from the default weights and override per entry
    let mut weights = AUTO_IMPORT_WEIGHTS;

    // parse comma separated key value entries
    for entry in raw.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        // parse the entry into a key and numeric value
        let mut parts = entry.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim().parse::<u32>().ok()?;

        // apply the override when the key matches a known weight
        match key {
            "same_folder_bonus" => weights.same_folder_bonus = value,
            "same_package_bonus" => weights.same_package_bonus = value,
            "other_package_penalty" => weights.other_package_penalty = value,
            "distance_multiplier" => weights.distance_multiplier = value,
            "distance_cap" => weights.distance_cap = value,
            "depth_multiplier" => weights.depth_multiplier = value,
            "depth_cap" => weights.depth_cap = value,
            "minimum_sort_order" => weights.minimum_sort_order = value,
            _ => {}
        }
    }

    Some(weights)
}

/// Check whether a symbol type participates in the type namespace.
fn is_type_symbol(symbol_type: SymbolType) -> bool {
    matches!(
        symbol_type,
        SymbolType::Class
            | SymbolType::Struct
            | SymbolType::Interface
            | SymbolType::Enum
            | SymbolType::TypeAlias
            | SymbolType::Newtype
    )
}

/// Check whether a symbol matches a completion filter.
fn matches_completion_filter(
    symbol_type: SymbolType,
    symbol_space: SymbolSpace,
    filter: Option<SymbolSpace>,
) -> bool {
    // allow everything when there is no filter
    let Some(filter) = filter else {
        return true;
    };

    // match filter semantics against symbol namespaces
    match filter {
        SymbolSpace::Type => is_type_symbol(symbol_type),
        SymbolSpace::Value => {
            symbol_space == SymbolSpace::Value || symbol_space == SymbolSpace::TypeValue
        }
        SymbolSpace::TypeValue => symbol_space == SymbolSpace::TypeValue,
        SymbolSpace::Label => symbol_space == SymbolSpace::Label,
    }
}

fn normalize_separators(path: &str) -> String {
    path.replace('\\', "/")
}

fn normal_components(path: &std::path::Path) -> Vec<String> {
    use std::path::Component;

    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => {
                components.push(prefix.as_os_str().to_string_lossy().to_string());
            }
            Component::RootDir => {
                components.push("/".to_string());
            }
            Component::Normal(part) => {
                components.push(part.to_string_lossy().to_string());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                components.push("..".to_string());
            }
        }
    }
    components
}

fn relative_path(
    from_dir: &std::path::Path,
    to_path: &std::path::Path,
) -> Option<std::path::PathBuf> {
    let from_dir = from_dir.normalize();
    let to_path = to_path.normalize();

    let from_components = normal_components(&from_dir);
    let to_components = normal_components(&to_path);

    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    if common == 0 && from_dir.is_absolute() && to_path.is_absolute() {
        return None;
    }

    let mut relative = std::path::PathBuf::new();

    for _ in common..from_components.len() {
        relative.push("..");
    }

    for component in &to_components[common..] {
        relative.push(component);
    }

    Some(relative)
}

fn path_distance(from_dir: &std::path::Path, to_path: &std::path::Path) -> u32 {
    let from_dir = from_dir.normalize();
    let to_dir = to_path.parent().unwrap_or(to_path).normalize();

    let from_components = normal_components(&from_dir);
    let to_components = normal_components(&to_dir);

    let mut common = 0usize;
    while common < from_components.len()
        && common < to_components.len()
        && from_components[common] == to_components[common]
    {
        common += 1;
    }

    let ups = from_components.len().saturating_sub(common);
    let downs = to_components.len().saturating_sub(common);
    (ups + downs) as u32
}

fn auto_import_sort_order(
    session: &Session,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    target_module_id: ModuleId,
    module_path: &str,
) -> u32 {
    let mut sort_order = SORT_AUTO_IMPORT;
    let weights = auto_import_weights();

    let target_package_id = {
        let module = session.modules.get(target_module_id);
        module.read().package_id
    };

    if let Some(current_package_id) = current_package_id {
        if current_package_id == target_package_id {
            sort_order = sort_order.saturating_sub(weights.same_package_bonus);
        } else {
            sort_order = sort_order.saturating_add(weights.other_package_penalty);
        }
    }

    let source_file = session.files.get(file_id);
    let Some(source_path) = source_file.path.as_ref() else {
        return sort_order;
    };

    let Some(source_dir) = source_path.parent() else {
        return sort_order;
    };

    // compute path heuristics without IO
    let source_dir = source_dir.normalize();
    let target_path = std::path::Path::new(module_path).normalize();
    let target_dir = target_path.parent().unwrap_or(&target_path).to_path_buf();

    // same folder boost
    if source_dir == target_dir {
        sort_order = sort_order.saturating_sub(weights.same_folder_bonus);
    }

    // distance penalty
    let distance = path_distance(&source_dir, &target_path);
    let distance_penalty = distance
        .saturating_mul(weights.distance_multiplier)
        .min(weights.distance_cap);
    sort_order = sort_order.saturating_add(distance_penalty);

    // deeper paths are slightly less preferred
    let source_depth = normal_components(&source_dir).len() as u32;
    let target_depth = normal_components(&target_dir).len() as u32;
    let depth_penalty = target_depth
        .saturating_sub(source_depth)
        .saturating_mul(weights.depth_multiplier)
        .min(weights.depth_cap);
    sort_order = sort_order.saturating_add(depth_penalty);

    sort_order.max(weights.minimum_sort_order)
}

/// Kind of completion item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Sort text for LSP (if different from label).
    pub sort_text: Option<String>,
    /// Whether to preselect this item.
    pub preselect: bool,
    /// Whether the item is deprecated.
    pub deprecated: bool,
    /// Additional text edits to apply (e.g., auto-import).
    pub additional_text_edits: Vec<Edit>,
    /// Matched character positions in the label (for UI highlighting).
    pub match_positions: Vec<usize>,
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
            match_positions: Vec::new(),
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

    /// Add additional text edits (e.g., auto-import).
    pub fn with_additional_edits(mut self, edits: Vec<Edit>) -> Self {
        self.additional_text_edits = edits;
        self
    }

    /// Mark as deprecated.
    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }

    /// Set sort text (for LSP ordering).
    pub fn with_sort_text(mut self, text: impl Into<String>) -> Self {
        self.sort_text = Some(text.into());
        self
    }
}

/// Trigger character that caused the completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionTrigger {
    /// Invoked manually or automatically.
    Invoked,
    /// Triggered by a character (e.g., '.').
    Character(char),
    /// Re-triggered for incomplete results.
    Incomplete,
}

/// Request completion items at a cursor position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// The document URI.
    pub uri: Uri,
    /// The byte offset in the document.
    pub offset: u32,
    /// The trigger that initiated completion.
    pub trigger: CompletionTrigger,
    /// Whether to include auto import completions.
    pub include_imports: bool,
}

/// Response payload for completion queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Completion items.
    pub items: Vec<Completion>,
    /// Whether the results are incomplete.
    pub is_incomplete: bool,
}

/// Generate a snippet for a function call with parameter placeholders.
///
/// Returns `(snippet_text, is_snippet)`.
/// For no params: `"foo()"` with `is_snippet = false`.
/// For params: `"foo(${1:param1}, ${2:param2})$0"` with `is_snippet = true`.
fn generate_call_snippet(name: &str, param_names: &[String]) -> (String, bool) {
    if param_names.is_empty() {
        (format!("{name}()"), false)
    } else {
        let params_str: String = param_names
            .iter()
            .enumerate()
            .map(|(i, p)| format!("${{{}:{}}}", i + 1, p))
            .collect::<Vec<_>>()
            .join(", ");
        (format!("{name}({params_str})$0"), true)
    }
}

/// Get parameter names from a function symbol's declaration.
fn get_function_param_names(
    session: &Session,
    symbol_id: dir::GlobalSymbolId,
) -> Option<Vec<String>> {
    // resolve dynamic parameter names from the shared helper
    let param_names = dynamic_parameter_names(session, symbol_id)?;

    // drop empty parameter lists so callers can fall back
    if param_names.is_empty() {
        return None;
    }

    Some(param_names)
}

/// Generate auto-import completions for a prefix using indexed lookup.
///
/// Searches exported symbols from other modules and creates completions
/// with import edits.
fn complete_auto_imports(
    session: &Session,
    file_id: FileId,
    prefix: &str,
    space_filter: Option<SymbolSpace>,
) -> Vec<Completion> {
    // only suggest auto-imports with 2+ characters typed
    if prefix.len() < 2 {
        return Vec::new();
    }

    // get current module to exclude from results and rank by package
    let (current_module_id, current_package_id) =
        if let Some(module) = get_module_by_file_id(session, file_id) {
            let module = module.read();
            (Some(module.id), Some(module.package_id))
        } else {
            (None, None)
        };

    let mut results = Vec::new();
    let mut found_indexed = false;

    // use indexed lookup from all programs
    for program in session.programs.iter() {
        let program = program.value();

        // search by prefix using the index
        // NOTE #Incomplete: index is not yet populated during compilation
        let exports = program
            .index
            .exports
            .search_by_prefix(prefix, current_module_id);

        if !exports.is_empty() {
            found_indexed = true;
        }

        for export in exports {
            if !matches_completion_filter(export.symbol_type, export.space, space_filter) {
                continue;
            }

            let Some(module_path) = &export.module_path else {
                continue;
            };

            // convert the export into an auto import completion
            push_auto_import_completion(
                session,
                file_id,
                current_package_id,
                export.module_id,
                module_path,
                &export.name,
                export.symbol_type,
                &mut results,
            );
        }
    }

    if !found_indexed {
        let exports = search_importable_symbols(session, prefix, current_module_id);

        for export in exports {
            if !matches_completion_filter(export.kind, export.space, space_filter) {
                continue;
            }

            let Some(module_path) = &export.module_path else {
                continue;
            };

            // convert the export into an auto import completion
            push_auto_import_completion(
                session,
                file_id,
                current_package_id,
                export.module_id,
                module_path,
                &export.name,
                export.kind,
                &mut results,
            );
        }
    }

    results
}

/// Push a single auto import completion into the results list.
#[allow(clippy::too_many_arguments)]
fn push_auto_import_completion(
    session: &Session,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    module_id: ModuleId,
    module_path: &str,
    export_name: &str,
    symbol_type: SymbolType,
    results: &mut Vec<Completion>,
) {
    // build a display path relative to the current file
    let display_path = build_display_path(session, file_id, module_path);

    // build import edits and skip already imported symbols
    let import_edits = build_import_edits(session, file_id, export_name, &display_path);
    if import_edits.is_empty() {
        return;
    }

    // compute ranking information for this auto import
    let sort_order =
        auto_import_sort_order(session, file_id, current_package_id, module_id, module_path);
    let sort_text = format!("{sort_order:04}:{display_path}:{export_name}");

    // assemble the completion item and attach edits
    let kind = CompletionKind::from(symbol_type);
    let detail = format!("Auto import from {display_path}");
    let completion = Completion::new(export_name, kind)
        .with_detail(detail)
        .with_sort_order(sort_order)
        .with_sort_text(sort_text)
        .with_additional_edits(import_edits);

    results.push(completion);
}

/// Build a display path for an import.
///
/// Tries to compute a relative path from the current file to the target module.
fn build_display_path(session: &Session, file_id: FileId, module_path: &str) -> String {
    let source_file = session.files.get(file_id);
    let Some(source_path) = source_file.path.as_ref() else {
        return module_path.to_string();
    };
    let Some(source_dir) = source_path.parent() else {
        return module_path.to_string();
    };

    let target_path = std::path::Path::new(module_path);

    let relative =
        relative_path(source_dir, target_path).unwrap_or_else(|| target_path.normalize());
    let mut display_path = normalize_separators(&relative.to_string_lossy());
    if !display_path.starts_with("./") && !display_path.starts_with("../") {
        display_path = format!("./{display_path}");
    }

    for extension in [".ds", ".ts"] {
        if display_path.ends_with(extension) {
            display_path.truncate(display_path.len().saturating_sub(extension.len()));
        }
    }

    display_path
}

/// Filter and rank completions using fuzzy matching.
fn filter_and_rank_completions(
    completions: Vec<Completion>,
    token: Option<&TokenAtCursor>,
) -> Vec<Completion> {
    use super::fuzzy::FuzzyMatch;

    let prefix = token.map(|t| t.text.as_str()).unwrap_or("");

    // score each completion and filter non-matches, capturing match positions
    let mut scored: Vec<(Completion, FuzzyMatch)> = completions
        .into_iter()
        .filter_map(|c| score_completion(&c.label, prefix).map(|m| (c, m)))
        .collect();

    // sort by match tier, then by sort_order, then by sort_text and score
    scored.sort_by(|a, b| {
        b.1.tier
            .cmp(&a.1.tier)
            .then(a.0.sort_order.cmp(&b.0.sort_order))
            .then_with(|| {
                let a_text = a.0.sort_text.as_deref().unwrap_or(&a.0.label);
                let b_text = b.0.sort_text.as_deref().unwrap_or(&b.0.label);
                a_text.cmp(b_text)
            })
            .then(b.1.score.cmp(&a.1.score))
    });

    // convert to results, populating match_positions
    let mut results: Vec<Completion> = scored
        .into_iter()
        .map(|(mut c, m)| {
            c.match_positions = m.matched_indices;
            c
        })
        .collect();

    // preselect if there's a clear winner (single completion or exact match)
    if results.len() == 1 {
        results[0].preselect = true;
    } else if !prefix.is_empty() && results.first().map(|c| c.label == prefix).unwrap_or(false) {
        // exact match gets preselected
        results[0].preselect = true;
    }

    results
}

/// Get completions at the given position.
pub fn completions(
    session: &Session,
    file: FileId,
    offset: u32,
    trigger: CompletionTrigger,
) -> Vec<Completion> {
    // detect completion context
    let ContextResult { context, token } = detect_completion_context(session, file, offset);
    let prefix = token.as_ref().map(|t| t.text.as_str()).unwrap_or("");

    let mut results = match &context {
        CompletionContext::MemberAccess {
            receiver_symbol,
            receiver_type,
            ..
        } => complete_members(session, file, *receiver_type, *receiver_symbol),

        CompletionContext::TypePosition {
            scope_id,
            scope_mark,
        } => complete_types(session, file, *scope_id, *scope_mark),

        CompletionContext::ValuePosition {
            scope_id,
            scope_mark,
        } => complete_values(session, file, *scope_id, *scope_mark, trigger),

        CompletionContext::ObjectLiteral {
            expected_type,
            existing_fields,
            scope_id,
            scope_mark,
            ..
        } => complete_object_literal(
            session,
            file,
            *expected_type,
            existing_fields,
            *scope_id,
            *scope_mark,
        ),

        CompletionContext::ImportPath { partial_path } => {
            complete_import_paths(session, file, partial_path)
        }

        CompletionContext::ImportClause {
            target_module,
            existing_names,
            space_filter,
        } => complete_imports(session, *target_module, existing_names, *space_filter),

        CompletionContext::Unknown => complete_all(session, file),
    };

    // add auto-import completions for value and type positions
    match context {
        CompletionContext::ValuePosition { .. } => {
            results.extend(complete_auto_imports(
                session,
                file,
                prefix,
                Some(SymbolSpace::Value),
            ));
        }
        CompletionContext::TypePosition { .. } => {
            results.extend(complete_auto_imports(
                session,
                file,
                prefix,
                Some(SymbolSpace::Type),
            ));
        }
        _ => {}
    }

    // apply fuzzy matching to filter and rank results
    filter_and_rank_completions(results, token.as_ref())
}

/// Complete members of a type (after `.`).
fn complete_members(
    session: &Session,
    file: FileId,
    receiver_type: Option<dir::LocalTypeId>,
    receiver_symbol: Option<dir::GlobalSymbolId>,
) -> Vec<Completion> {
    let mut results = Vec::new();

    // get the module for context
    let Some(module) = get_module_by_file_id(session, file) else {
        return common_member_completions();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return common_member_completions();
    };
    let types = ctx.types();
    let symbols = ctx.symbols();
    let current_module_id = ctx.module_id;

    // primary path: use receiver type for type-aware completions
    if let Some(type_id) = receiver_type {
        // resolve members from the type
        let members = resolve_type_members(&types, &symbols, type_id, session);

        for member in members {
            // skip non-displayable members
            let MemberName::String(name) = member.name else {
                continue;
            };
            let kind = match member.kind {
                MemberKind::Method => CompletionKind::Method,
                MemberKind::Field => CompletionKind::Field,
                MemberKind::CallSignature => CompletionKind::Function,
                MemberKind::ConstructSignature => CompletionKind::Constructor,
                MemberKind::IndexSignature => CompletionKind::Property,
                MemberKind::EnumMember => CompletionKind::EnumMember,
            };

            let mut completion = Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL);

            // add type detail
            if let Some(member_type_id) = member.type_id {
                let type_text =
                    format_local_type(member_type_id, &types, &session.modules, &session.strings);
                completion = completion.with_detail(type_text);
            }

            results.push(completion);
        }

        // also check for extension methods if the type is nominal
        let extension_symbol = {
            let ty = types.get_type(type_id);
            if let Type::Reference { symbol, .. } = ty {
                Some(*symbol)
            } else {
                None
            }
        };

        drop(types);
        drop(symbols);
        drop(module);

        if let Some(symbol) = extension_symbol {
            results.extend(complete_extension_methods(
                session,
                symbol,
                current_module_id,
            ));
        }
    }
    // fallback path: use receiver symbol (for cases where type inference hasn't run)
    else if let Some(symbol_id) = receiver_symbol {
        drop(types);
        drop(symbols);
        drop(module);

        // load the symbol's module
        let symbol_module = session.modules.get(symbol_id.module_id);
        let symbol_module = symbol_module.read();
        let Some(symbol_ctx) = session.query_context(&symbol_module) else {
            return common_member_completions();
        };
        let symbols = symbol_ctx.symbols();
        let types = symbol_ctx.types();

        // get the scope owned by this symbol (for types like struct/class)
        if let Some(owned_scope_id) = get_symbol_owned_scope(&symbols, symbol_id.local_id) {
            let scope = symbols.get_scope_by_id(owned_scope_id);

            // add all named symbols in the scope as member completions
            for (key, member_id) in symbols.active_named_symbols(scope) {
                if let destack_dir::StaticKey::Name(name_id) = key {
                    let member_symbol = symbols.get_symbol(member_id);
                    let name = session.strings.get(name_id).to_string();
                    let kind = CompletionKind::from(member_symbol.ty);

                    let mut completion =
                        Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL);

                    // add type detail from primary declaration
                    if let Some(declaration) = member_symbol.primary_declaration
                        && let Some(type_id) = types.get_declared_or_inferred_type_id(declaration)
                    {
                        let type_text =
                            format_local_type(type_id, &types, &session.modules, &session.strings);
                        completion = completion.with_detail(type_text);
                    }

                    // fall back to generic detail for functions
                    if completion.detail.is_none() && member_symbol.ty == SymbolType::Function {
                        completion = completion.with_detail("method");
                    }

                    results.push(completion);
                }
            }
        }

        drop(types);
        drop(symbols);
        drop(symbol_module);

        // also check for extension methods
        results.extend(complete_extension_methods(
            session,
            symbol_id,
            current_module_id,
        ));
    }

    // if no results, fall back to common properties
    if results.is_empty() {
        results.extend(common_member_completions());
    }

    results
}

/// Get the scope owned by a symbol (for types that declare scopes).
fn get_symbol_owned_scope(
    symbols: &dir::SymbolTable,
    symbol_id: dir::LocalSymbolId,
) -> Option<dir::LocalScopeId> {
    // iterate through scopes to find the one owned by this symbol
    for (idx, scope) in symbols.scopes().enumerate() {
        if scope.owner_id == Some(symbol_id) {
            return Some(dir::LocalScopeId::new(idx as u32));
        }
    }
    None
}

/// Complete extension methods for a symbol.
///
/// NOTE #Architecture: extension methods should be part of resolve_type_members output,
/// not a separate lookup. The caller shouldn't need to know about extensions as a
/// separate concept.
fn complete_extension_methods(
    session: &Session,
    target_symbol: dir::GlobalSymbolId,
    current_module_id: ModuleId,
) -> Vec<Completion> {
    let mut results = Vec::new();
    let canonical_target = get_canonical_symbol(session, target_symbol);

    for module in session.modules.iter() {
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            continue;
        };
        let types = ctx.types();
        let symbols = ctx.symbols();

        let Some(extension_ids) = types.get_extensions_for_target(canonical_target) else {
            continue;
        };

        for extension_id in extension_ids {
            let extension = types.get_extension(*extension_id);
            let visible = match extension.kind {
                dir::ExtensionKind::Inherent => true,
                dir::ExtensionKind::Local => extension.symbol.module_id == current_module_id,
                dir::ExtensionKind::Nominal => true,
            };
            if !visible {
                continue;
            }

            let ext_symbol = symbols.get_symbol(extension.symbol.local_id);
            let Some(ext_decl_id) = ext_symbol.primary_declaration else {
                continue;
            };
            let Ok(local_decl_id): Result<dir::LocalNodeId<dir::Declaration>, _> =
                ext_decl_id.try_into()
            else {
                continue;
            };
            let tree = ctx.tree();
            let ext_decl: &dir::Declaration = tree.get(local_decl_id);

            let Some(member_ids) = ext_decl.member_ids() else {
                continue;
            };

            for member_node_id in member_ids {
                let member_node: &dir::Member = tree.get(*member_node_id);

                // get the member's name from its key
                let Some(key) = member_node.key() else {
                    continue;
                };
                let name_id = match key {
                    dir::DynamicKey::Name(name_id) => *name_id,
                    dir::DynamicKey::Number(name_id) => *name_id,
                    _ => continue,
                };
                let name = session.strings.get(name_id).to_string();

                // determine completion kind from member type
                let (kind, is_method) = match member_node {
                    dir::Member::Method { .. } => (CompletionKind::Method, true),
                    dir::Member::Field { .. } => (CompletionKind::Field, false),
                    dir::Member::Type { .. } => (CompletionKind::Class, false),
                    _ => (CompletionKind::Property, false),
                };

                // slightly lower priority than direct members
                let mut completion =
                    Completion::new(&name, kind).with_sort_order(SORT_EXTENSION_METHOD);

                // get the member's symbol for type info (may be Void if not fully resolved)
                let member_symbol_id = member_node.symbol();
                let member = symbols.get_symbol(member_symbol_id);

                // add type info
                if let Some(decl) = member.primary_declaration
                    && let Some(type_id) = types.get_declared_or_inferred_type_id(decl)
                {
                    let type_text =
                        format_local_type(type_id, &types, &session.modules, &session.strings);
                    completion = completion.with_detail(type_text);
                }

                // for methods, try to generate snippet with parameter placeholders
                if is_method {
                    // try to get param names from the method signature in the Member node
                    if let dir::Member::Method { signature, .. } = member_node {
                        let param_names = dynamic_parameter_display_names(
                            session,
                            &tree,
                            &signature.dynamic_parameters,
                        );

                        if !param_names.is_empty() {
                            let (snippet, is_snippet) = generate_call_snippet(&name, &param_names);
                            completion = completion.with_insert_text(snippet);
                            if is_snippet {
                                completion = completion.as_snippet();
                            }
                        } else {
                            // no params, just add ()
                            completion = completion.with_insert_text(format!("{name}()"));
                        }
                    }
                }

                results.push(completion);
            }
        }
    }

    results
}

/// Common member completions (fallback).
fn common_member_completions() -> Vec<Completion> {
    vec![
        Completion::new("toString", CompletionKind::Method)
            .with_detail("(): string")
            .with_sort_order(SORT_IMPORT_STARTER),
        Completion::new("valueOf", CompletionKind::Method)
            .with_detail("(): any")
            .with_sort_order(SORT_IMPORT_STARTER),
    ]
}

/// Complete fields inside an object literal.
fn complete_object_literal(
    session: &Session,
    file: FileId,
    expected_type: Option<dir::LocalTypeId>,
    existing_fields: &[String],
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
) -> Vec<Completion> {
    let mut results = Vec::new();

    // if we have an expected type, suggest its fields
    if let Some(type_id) = expected_type {
        let Some(module) = get_module_by_file_id(session, file) else {
            return results;
        };
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            return results;
        };
        let types = ctx.types();
        let symbols = ctx.symbols();

        // resolve type members
        let members = resolve_type_members(&types, &symbols, type_id, session);

        for member in members {
            // skip non-field members (methods, call signatures, etc.)
            if member.kind != MemberKind::Field {
                continue;
            }

            // skip non-displayable members
            let MemberName::String(name) = member.name else {
                continue;
            };

            // skip already present fields
            if existing_fields.contains(&name) {
                continue;
            }

            // build completion with snippet for field value
            let mut completion = Completion::new(&name, CompletionKind::Field)
                .with_insert_text(format!("{name}: $0"))
                .as_snippet()
                .with_sort_order(5); // high priority

            // add type detail
            if let Some(member_type_id) = member.type_id {
                let type_text =
                    format_local_type(member_type_id, &types, &session.modules, &session.strings);
                completion = completion.with_detail(type_text);
            }

            results.push(completion);
        }
    }

    // also add value completions from scope (for shorthand syntax)
    if let Some(scope_id) = scope_id {
        let Some(module) = get_module_by_file_id(session, file) else {
            return results;
        };
        let module = module.read();
        let Some(ctx) = session.query_context(&module) else {
            return results;
        };
        let symbols = ctx.symbols();
        let mut seen_names = std::collections::HashSet::new();
        let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

        for visible in visible_symbols(&symbols, scope_id, mark, Some(SymbolSpace::Value)) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = session.strings.get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            // skip already present fields
            if existing_fields.contains(&name) {
                continue;
            }

            let kind = CompletionKind::from(visible.symbol.ty);

            // lower priority than expected type fields
            results.push(Completion::new(name, kind).with_sort_order(SORT_BUILTIN));
        }
    }

    results
}

/// Complete types (in type position).
fn complete_types(
    session: &Session,
    file: FileId,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
) -> Vec<Completion> {
    // get module AST/DIR
    let Some(module) = get_module_by_file_id(session, file) else {
        return primitive_type_completions();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return primitive_type_completions();
    };
    let symbols = ctx.symbols();
    let dir_tree = ctx.tree();

    let mut results = Vec::new();
    let mut seen_names = std::collections::HashSet::new();
    let resolve_symbol_type = |symbol_id: dir::LocalSymbolId, symbol: &dir::Symbol| {
        if symbol.ty != SymbolType::Void {
            return symbol.ty;
        }

        let global_id = dir::GlobalSymbolId {
            module_id: ctx.module_id,
            local_id: symbol_id,
        };
        let canonical_id = get_canonical_symbol(session, global_id);
        if canonical_id == global_id {
            return symbol.ty;
        }

        let module = session.modules.get(canonical_id.module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(ctx.profile_id) else {
            return symbol.ty;
        };
        let symbols = dir.symbols.read();
        let canonical_symbol = symbols.get_symbol(canonical_id.local_id);
        canonical_symbol.ty
    };

    // if we have a scope, walk up from it to collect visible types
    if let Some(scope_id) = scope_id {
        let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

        for visible in visible_symbols(&symbols, scope_id, mark, Some(SymbolSpace::Type)) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = session.strings.get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            let kind = CompletionKind::from(resolve_symbol_type(visible.id, visible.symbol));
            results.push(Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL));
        }
    } else {
        // no scope, add all module-level type symbols
        for (idx, symbol) in symbols.symbols().enumerate() {
            if !symbol.is_active() {
                continue;
            }

            // only include type-space symbols
            if symbol.space != dir::SymbolSpace::Type && symbol.space != dir::SymbolSpace::TypeValue
            {
                continue;
            }

            let Some(string_id) = symbol.name() else {
                continue;
            };

            let name = session.strings.get(string_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }
            let local_id = dir::LocalSymbolId::new(idx as u32);
            let kind = CompletionKind::from(resolve_symbol_type(local_id, symbol));
            results.push(Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL));
        }
    }

    for (_, item) in dir_tree.iter_nodes_of_type::<dir::DependencyItem>() {
        let Some(symbol_id) = item.symbol() else {
            continue;
        };

        let symbol = symbols.get_symbol(symbol_id);
        if symbol.space != dir::SymbolSpace::Type && symbol.space != dir::SymbolSpace::TypeValue {
            continue;
        }

        let Some(name_id) = symbol.name() else {
            continue;
        };

        let name = session.strings.get(name_id).to_string();
        if !seen_names.insert(name.clone()) {
            continue;
        }

        let kind = CompletionKind::from(resolve_symbol_type(symbol_id, symbol));
        results.push(Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL));
    }

    // add primitive types
    results.extend(primitive_type_completions());

    results
}

/// Primitive type completions.
fn primitive_type_completions() -> Vec<Completion> {
    const PRIMITIVES: &[&str] = &[
        "int8", "int16", "int32", "int64", "uint8", "uint16", "uint32", "uint64", "float32",
        "float64", "bool", "string", "void", "never", "any", "unknown",
    ];

    PRIMITIVES
        .iter()
        .map(|p| Completion::new(*p, CompletionKind::TypeParameter).with_sort_order(SORT_BUILTIN))
        .collect()
}

/// Complete values (in expression position).
fn complete_values(
    session: &Session,
    file: FileId,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
    trigger: CompletionTrigger,
) -> Vec<Completion> {
    // get module AST/DIR
    let Some(module) = get_module_by_file_id(session, file) else {
        return keyword_completions();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return keyword_completions();
    };
    let symbols = ctx.symbols();
    let module_id = ctx.module_id;

    let mut results = Vec::new();

    // collect symbols to process (to avoid holding symbols lock while generating snippets)
    let mut symbols_to_process: Vec<(dir::LocalSymbolId, String, SymbolType)> = Vec::new();

    // if we have a scope, walk visible symbols in scope order
    if let Some(scope_id) = scope_id {
        let mut seen_names = std::collections::HashSet::new();
        let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

        for visible in visible_symbols(&symbols, scope_id, mark, Some(SymbolSpace::Value)) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = session.strings.get(name_id).to_string();
            if !seen_names.insert(name.clone()) {
                continue;
            }

            symbols_to_process.push((visible.id, name, visible.symbol.ty));
        }
    } else {
        // no scope, fall back to any active value symbols
        let mut seen = std::collections::HashSet::new();

        for (idx, symbol) in symbols.symbols().enumerate() {
            if !symbol.is_active() {
                continue;
            }
            if symbol.space != dir::SymbolSpace::Value
                && symbol.space != dir::SymbolSpace::TypeValue
            {
                continue;
            }

            let Some(string_id) = symbol.name() else {
                continue;
            };

            let name = session.strings.get(string_id).to_string();
            if !seen.insert(name.clone()) {
                continue;
            }

            let local_id = dir::LocalSymbolId::new(idx as u32);
            symbols_to_process.push((local_id, name, symbol.ty));
        }
    }

    drop(symbols);
    drop(module);

    // build completions with snippets for functions
    for (local_id, name, symbol_type) in symbols_to_process {
        let kind = CompletionKind::from(symbol_type);
        let mut completion = Completion::new(&name, kind).with_sort_order(SORT_LOCAL_SYMBOL);

        // for functions, generate snippet with parameter placeholders
        if symbol_type == SymbolType::Function {
            let global_id = dir::GlobalSymbolId {
                module_id,
                local_id,
            };
            if let Some(param_names) = get_function_param_names(session, global_id) {
                let (snippet, is_snippet) = generate_call_snippet(&name, &param_names);
                completion = completion.with_insert_text(snippet);
                if is_snippet {
                    completion = completion.as_snippet();
                }
            }
        }

        results.push(completion);
    }

    // add keywords if triggered manually
    if matches!(trigger, CompletionTrigger::Invoked) {
        results.extend(keyword_completions());
    }

    results
}

/// Complete imports from a module.
fn complete_imports(
    session: &Session,
    target_module: Option<ModuleId>,
    existing_names: &[String],
    space_filter: Option<SymbolSpace>,
) -> Vec<Completion> {
    let Some(module_id) = target_module else {
        return Vec::new();
    };

    // get module AST/DIR
    let module = session.modules.get(module_id);
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let symbols = ctx.symbols();

    let mut results = Vec::new();

    let existing_names: std::collections::HashSet<&str> =
        existing_names.iter().map(String::as_str).collect();

    // add all exported symbols
    for symbol in symbols.symbols() {
        if symbol.export.is_none() {
            continue;
        }

        let Some(string_id) = symbol.name() else {
            continue;
        };

        if !matches_completion_filter(symbol.ty, symbol.space, space_filter) {
            continue;
        }

        let name = session.strings.get(string_id).to_string();
        if existing_names.contains(name.as_str()) {
            continue;
        }

        let kind = CompletionKind::from(symbol.ty);
        results.push(Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL));
    }

    results
}

/// Complete all symbols (fallback for unknown context).
fn complete_all(session: &Session, file: FileId) -> Vec<Completion> {
    // get module AST/DIR
    let Some(module) = get_module_by_file_id(session, file) else {
        return keyword_completions();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return keyword_completions();
    };
    let symbols = ctx.symbols();

    let mut results = Vec::new();

    for symbol in symbols.symbols() {
        let Some(string_id) = symbol.name() else {
            continue;
        };

        let name = session.strings.get(string_id).to_string();
        let kind = CompletionKind::from(symbol.ty);
        results.push(Completion::new(name, kind));
    }

    drop(symbols);
    drop(module);

    results.extend(keyword_completions());
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
        .map(|kw| Completion::new(*kw, CompletionKind::Keyword).with_sort_order(SORT_KEYWORD))
        .collect()
}

/// Complete import paths (relative paths or package names).
fn complete_import_paths(session: &Session, file: FileId, partial: &str) -> Vec<Completion> {
    let mut results = Vec::new();

    if partial.starts_with("./") || partial.starts_with("../") {
        // relative path: list directory contents
        let source_file = session.files.get(file);
        if let Some(ref path) = source_file.path
            && let Some(base_dir) = path.parent()
        {
            results.extend(complete_relative_path(session, base_dir, partial));
        }
    } else if partial.is_empty() {
        // empty: suggest starters
        results.push(
            Completion::new("./", CompletionKind::Folder)
                .with_detail("relative")
                .with_sort_order(5),
        );
        results.push(
            Completion::new("../", CompletionKind::Folder)
                .with_detail("parent")
                .with_sort_order(6),
        );
        results.extend(complete_package_names(session, ""));
    } else {
        // package name prefix
        results.extend(complete_package_names(session, partial));
    }

    results
}

/// Complete relative paths by listing directory contents.
fn complete_relative_path(
    session: &Session,
    base_dir: &std::path::Path,
    partial: &str,
) -> Vec<Completion> {
    // resolve directory to list
    let (dir_to_list, prefix) = if let Some(slash) = partial.rfind('/') {
        (base_dir.join(&partial[..=slash]), &partial[slash + 1..])
    } else {
        (base_dir.to_path_buf(), partial)
    };

    let Ok(entries) = session.fs.read_dir(&dir_to_list) else {
        return vec![];
    };

    entries
        .iter()
        .filter_map(|entry| {
            let file_name = entry.file_name()?;
            let name = file_name.to_string_lossy().to_string();

            // skip hidden files
            if name.starts_with('.') {
                return None;
            }

            // filter by prefix
            if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                return None;
            }

            let meta = session.fs.metadata(entry).ok()?;
            if meta.is_directory {
                Some(
                    Completion::new(format!("{name}/"), CompletionKind::Folder)
                        .with_sort_order(SORT_LOCAL_SYMBOL),
                )
            } else if name.ends_with(".ds") {
                let module_name = name.strip_suffix(".ds")?;
                Some(
                    Completion::new(module_name, CompletionKind::Module)
                        .with_sort_order(SORT_LOCAL_SYMBOL),
                )
            } else {
                None
            }
        })
        .collect()
}

/// Complete package names from the registry.
fn complete_package_names(session: &Session, prefix: &str) -> Vec<Completion> {
    session
        .packages
        .iter()
        .filter_map(|pkg_ref| {
            let pkg = pkg_ref.read();
            let name = pkg.name.as_ref()?;

            // filter by prefix
            if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                return None;
            }

            let mut completion =
                Completion::new(name.clone(), CompletionKind::Module).with_sort_order(SORT_BUILTIN);

            // add version detail if available
            if let Some(v) = &pkg.version {
                completion = completion.with_detail(format!("v{v}"));
            }

            Some(completion)
        })
        .collect()
}
