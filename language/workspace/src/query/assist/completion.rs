use std::collections::HashSet;
use std::env;
use std::sync::OnceLock;

use destack_ast::Keyword;
use destack_dir::{self as dir, FloatType, IntType, SymbolSpace, SymbolType};
use destack_source::{Edit, FileId, FileType, ModuleId, PackageId, PathExt, Uri};
use serde::{Deserialize, Serialize};

use super::context::{CompletionContext, ContextResult, detect_completion_context};
use crate::format::format_local_type;
use crate::program::Loader;
use crate::query::common::{
    ImportEditMode, MemberInfo, MemberKind, MemberName, build_import_display_path,
    build_import_edits_with_mode, dynamic_parameter_names, ensure_program_export_index,
    get_canonical_symbol, get_module_by_file_id, matches_symbol_space_filter,
    module_name_from_path, owned_scope_for_symbol, path_component_count, path_distance,
    program_for_file, resolve_extension_members_for_symbol,
    resolve_nominal_symbol_from_initializer, resolve_reference_members, resolve_type_members,
    score_completion, search_importable_symbols_for_program, visible_symbols,
};
use crate::{Session, TokenAtCursor};

// sort order priorities (lower = higher priority in completion list)
const SORT_LOCAL_SYMBOL: u32 = 10;
const SORT_BUILTIN: u32 = 20;
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

/// Compute a sort order for an auto import candidate.
fn auto_import_sort_order(
    session: &Session,
    file_id: FileId,
    current_package_id: Option<PackageId>,
    target_module_id: ModuleId,
    module_path: &str,
) -> u32 {
    // start with the base sort order and weights
    let mut sort_order = SORT_AUTO_IMPORT;
    let weights = auto_import_weights();

    // resolve the target package id
    let target_package_id = {
        let module = session.modules.get(target_module_id);
        module.read().package_id
    };

    // adjust for same or different packages
    if let Some(current_package_id) = current_package_id {
        if current_package_id == target_package_id {
            sort_order = sort_order.saturating_sub(weights.same_package_bonus);
        } else {
            sort_order = sort_order.saturating_add(weights.other_package_penalty);
        }
    }

    // resolve the source path and directory
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
    let source_depth = path_component_count(&source_dir);
    let target_depth = path_component_count(&target_dir);
    let depth_penalty = target_depth
        .saturating_sub(source_depth)
        .saturating_mul(weights.depth_multiplier)
        .min(weights.depth_cap);
    sort_order = sort_order.saturating_add(depth_penalty);

    // return the final sort order with minimum enforced
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
    /// Convert a symbol type into a completion kind.
    fn from(ty: SymbolType) -> Self {
        // map symbol types to completion kinds
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
    /// Additional text edits to apply (e.g., auto import).
    pub additional_text_edits: Vec<Edit>,
    /// Whether this completion inserts an auto import.
    #[serde(default)]
    pub is_auto_import: bool,
    /// Matched character positions in the label (for UI highlighting).
    pub match_positions: Vec<usize>,
}

impl Completion {
    /// Create a simple completion.
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        // build a completion with defaults
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

    /// Mark as an auto import.
    pub fn as_auto_import(mut self) -> Self {
        self.is_auto_import = true;
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

    /// Add additional text edits (e.g., auto import).
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
    /// Retriggered for incomplete results.
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
    // return a plain call when there are no params
    if param_names.is_empty() {
        (format!("{name}()"), false)
    } else {
        // build a snippet with parameter placeholders
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

/// Format a type detail string for a symbol's declared or inferred type.
fn format_symbol_type_detail(session: &Session, symbol_id: dir::GlobalSymbolId) -> Option<String> {
    // read the symbol's module and build query context
    let module = session.modules.get(symbol_id.module_id);
    let module = module.read();
    let ctx = session.query_context(&module)?;

    // load symbol and type tables
    let symbols = ctx.symbols();
    let types = ctx.types();

    // resolve the declared or inferred type from the primary declaration
    let symbol = symbols.get_symbol(symbol_id.local_id);
    let declaration = symbol.primary_declaration?;
    let type_id = types.get_declared_or_inferred_type_id(declaration)?;

    // format the type using the symbol's module type table
    Some(format_local_type(
        type_id,
        &types,
        &session.modules,
        &session.strings,
    ))
}

/// Build a completion item from a resolved member entry.
fn completion_for_member(
    session: &Session,
    member: MemberInfo,
    types: Option<&dir::TypeTable>,
) -> Option<Completion> {
    // resolve a string name for the member
    let MemberName::String(name) = member.name else {
        return None;
    };

    // map member kinds to completion kinds
    let kind = match member.kind {
        MemberKind::Method => CompletionKind::Method,
        MemberKind::Field => CompletionKind::Field,
        MemberKind::CallSignature => CompletionKind::Function,
        MemberKind::ConstructSignature => CompletionKind::Constructor,
        MemberKind::IndexSignature => CompletionKind::Property,
        MemberKind::EnumMember => CompletionKind::EnumMember,
    };

    // start with a base completion entry
    let mut completion = Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL);

    // add type detail from the resolved member type
    if let Some(member_type_id) = member.type_id {
        if let Some(types) = types {
            let type_text =
                format_local_type(member_type_id, types, &session.modules, &session.strings);
            completion = completion.with_detail(type_text);
        }
    } else {
        let symbol_id = member.symbol_id;
        if let Some(symbol_id) = symbol_id {
            let type_text = format_symbol_type_detail(session, symbol_id);
            if let Some(type_text) = type_text {
                completion = completion.with_detail(type_text);
            }
        }
    }

    // add call snippet for method members with symbols
    if member.kind == MemberKind::Method {
        let symbol_id = member.symbol_id;
        if let Some(symbol_id) = symbol_id {
            if let Some(param_names) = get_function_param_names(session, symbol_id) {
                let (snippet, is_snippet) = generate_call_snippet(&completion.label, &param_names);
                completion = completion.with_insert_text(snippet);
                if is_snippet {
                    completion = completion.as_snippet();
                }
            } else {
                let label = completion.label.clone();
                completion = completion.with_insert_text(format!("{label}()"));
            }
        }
    }

    Some(completion)
}

// auto import completion thresholds
const AUTO_IMPORT_MIN_PREFIX: usize = 2;
const AUTO_IMPORT_SHORT_PREFIX_LIMIT: usize = 50;

/// Generate auto import completions for a prefix using indexed lookup.
///
/// Searches exported symbols from other modules and creates completions
/// with import edits.
fn complete_auto_imports(
    session: &Session,
    file_id: FileId,
    prefix: &str,
    space_filter: Option<SymbolSpace>,
    allow_short_prefix: bool,
) -> Vec<Completion> {
    // avoid scanning the full index for empty prefixes
    if prefix.is_empty() {
        return Vec::new();
    }

    // only suggest auto imports with 2 or more characters typed
    if prefix.len() < AUTO_IMPORT_MIN_PREFIX && !allow_short_prefix {
        return Vec::new();
    }

    // resolve the import mode from the space filter
    let import_mode = import_mode_for_space(space_filter);

    // get current module to exclude from results and rank by package
    let (current_module_id, current_package_id) =
        if let Some(module) = get_module_by_file_id(session, file_id) {
            let module = module.read();
            (Some(module.id), Some(module.package_id))
        } else {
            (None, None)
        };

    // prepare result storage and dedupe tracking
    let mut results = Vec::new();
    let mut found_indexed = false;
    let mut seen: HashSet<(ModuleId, dir::LocalSymbolId)> = HashSet::new();

    // resolve the program for this file
    let program = program_for_file(session, file_id);

    // ensure the export index is populated for the current program
    ensure_program_export_index(session, &program);

    // search by prefix using the index
    let exports = program
        .index
        .exports
        .search_by_prefix(prefix, current_module_id);
    if !exports.is_empty() {
        found_indexed = true;
    }

    for export in exports {
        // filter exports by completion scope
        if !matches_symbol_space_filter(export.symbol_type, export.space, space_filter) {
            continue;
        }

        // skip exports without a module path
        let Some(module_path) = &export.module_path else {
            continue;
        };

        // dedupe across modules
        let key = (export.module_id, export.symbol_id);
        if !seen.insert(key) {
            continue;
        }

        // convert the export into an auto import completion
        push_auto_import_completion(
            session,
            file_id,
            current_package_id,
            export.module_id,
            module_path,
            &export.name,
            export.symbol_type,
            import_mode,
            &mut results,
        );
    }

    if !found_indexed {
        // fall back to direct module scanning when no indexes are available
        let exports =
            search_importable_symbols_for_program(session, &program, prefix, current_module_id);

        for export in exports {
            // filter exports by completion scope
            if !matches_symbol_space_filter(export.kind, export.space, space_filter) {
                continue;
            }

            // skip exports without a module path
            let Some(module_path) = &export.module_path else {
                continue;
            };

            // dedupe across modules
            let key = (export.module_id, export.local_id);
            if !seen.insert(key) {
                continue;
            }

            // convert the export into an auto import completion
            push_auto_import_completion(
                session,
                file_id,
                current_package_id,
                export.module_id,
                module_path,
                &export.name,
                export.kind,
                import_mode,
                &mut results,
            );
        }
    }

    if allow_short_prefix && prefix.len() < AUTO_IMPORT_MIN_PREFIX {
        sort_auto_imports_for_short_prefix(&mut results);
        results.truncate(AUTO_IMPORT_SHORT_PREFIX_LIMIT);
    }

    results
}

/// Generate auto import completions and remove visible duplicates.
fn complete_auto_imports_with_visibility(
    session: &Session,
    file_id: FileId,
    prefix: &str,
    space_filter: Option<SymbolSpace>,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
    allow_short_prefix: bool,
) -> Vec<Completion> {
    // build auto import completions
    let mut completions =
        complete_auto_imports(session, file_id, prefix, space_filter, allow_short_prefix);

    // filter out completions that are already visible
    if let Some(visible_names) =
        collect_visible_names(session, file_id, scope_id, scope_mark, space_filter)
    {
        completions.retain(|item| !visible_names.contains(item.label.as_str()));
    }

    completions
}

/// Collect visible symbol names for a scope and space.
fn collect_visible_names(
    session: &Session,
    file_id: FileId,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
    space_filter: Option<SymbolSpace>,
) -> Option<HashSet<String>> {
    let scope_id = scope_id?;

    // resolve the module and query context
    let module = get_module_by_file_id(session, file_id)?;
    let module = module.read();
    let ctx = session.query_context(&module)?;
    let symbols = ctx.symbols();

    // collect names from visible symbols when scope is available
    let mut names = HashSet::new();
    let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

    // walk visible symbols in scope order
    for visible in visible_symbols(&symbols, scope_id, mark, space_filter) {
        // resolve the symbol name key
        let dir::StaticKey::Name(name_id) = visible.key else {
            continue;
        };

        // insert the name
        let name = session.strings.get(name_id).to_string();
        names.insert(name);
    }

    Some(names)
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
    import_mode: ImportEditMode,
    results: &mut Vec<Completion>,
) {
    // build a display path relative to the current file
    let display_path = build_import_display_path(session, file_id, module_path);

    // build import edits and skip already imported symbols
    let import_edits =
        build_import_edits_with_mode(session, file_id, export_name, &display_path, import_mode);
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
        .with_additional_edits(import_edits)
        .as_auto_import();

    results.push(completion);
}

/// Sort auto import completions for short prefixes.
fn sort_auto_imports_for_short_prefix(completions: &mut [Completion]) {
    // sort by sort order and label fallback
    completions.sort_by(|left, right| {
        let left_key = (
            left.sort_order,
            left.sort_text.as_deref().unwrap_or(&left.label),
            left.label.as_str(),
        );
        let right_key = (
            right.sort_order,
            right.sort_text.as_deref().unwrap_or(&right.label),
            right.label.as_str(),
        );
        left_key.cmp(&right_key)
    });
}

/// Resolve the import mode based on a completion space filter.
fn import_mode_for_space(space_filter: Option<SymbolSpace>) -> ImportEditMode {
    // prefer type imports for type space completions
    match space_filter {
        Some(SymbolSpace::Type) => ImportEditMode::Type,
        _ => ImportEditMode::Value,
    }
}

/// Filter and rank completions using fuzzy matching.
fn filter_and_rank_completions(
    completions: Vec<Completion>,
    token: Option<&TokenAtCursor>,
) -> Vec<Completion> {
    // import fuzzy matching helpers
    use crate::query::common::FuzzyMatch;

    // resolve the prefix for matching
    let prefix = token.map(|t| t.text.as_str()).unwrap_or("");

    // score each completion and filter non matches, capturing match positions
    let mut scored: Vec<(Completion, FuzzyMatch)> = completions
        .into_iter()
        .filter_map(|c| score_completion(&c.label, prefix).map(|m| (c, m)))
        .collect();

    // sort by completion group, then match tier, then sort_order, sort_text and score
    scored.sort_by(|a, b| {
        let left_group = completion_group_rank(&a.0);
        let right_group = completion_group_rank(&b.0);
        let left_auto = is_auto_import_completion(&a.0);
        let right_auto = is_auto_import_completion(&b.0);

        let base_order = left_group
            .cmp(&right_group)
            .then(b.1.tier.cmp(&a.1.tier))
            .then(a.0.sort_order.cmp(&b.0.sort_order));

        if left_auto && right_auto {
            base_order
                .then_with(|| {
                    let a_text = a.0.sort_text.as_deref().unwrap_or(&a.0.label);
                    let b_text = b.0.sort_text.as_deref().unwrap_or(&b.0.label);
                    a_text.cmp(b_text)
                })
                .then(b.1.score.cmp(&a.1.score))
        } else {
            base_order.then(b.1.score.cmp(&a.1.score)).then_with(|| {
                let a_text = a.0.sort_text.as_deref().unwrap_or(&a.0.label);
                let b_text = b.0.sort_text.as_deref().unwrap_or(&b.0.label);
                a_text.cmp(b_text)
            })
        }
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

    // return the sorted results
    results
}

/// Compute a sort rank for completion group ordering.
fn completion_group_rank(completion: &Completion) -> u8 {
    // place keywords last
    if completion.kind == CompletionKind::Keyword {
        return 2;
    }

    // place auto imports after local symbols
    if is_auto_import_completion(completion) {
        return 1;
    }

    // return the default group rank
    0
}

/// Check whether a completion entry represents an auto import.
fn is_auto_import_completion(completion: &Completion) -> bool {
    completion.is_auto_import
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
    let allow_short_prefix = matches!(trigger, CompletionTrigger::Invoked);

    // resolve base completions for the detected context
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
        } => complete_values(session, file, *scope_id, *scope_mark, false),

        CompletionContext::StatementPosition {
            scope_id,
            scope_mark,
        } => complete_values(
            session,
            file,
            *scope_id,
            *scope_mark,
            matches!(trigger, CompletionTrigger::Invoked),
        ),

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
        CompletionContext::ObjectLiteralValue {
            scope_id,
            scope_mark,
        } => complete_values(session, file, *scope_id, *scope_mark, false),
        CompletionContext::CallArgument {
            scope_id,
            scope_mark,
        } => complete_values(session, file, *scope_id, *scope_mark, false),
        CompletionContext::NewExpression {
            scope_id,
            scope_mark,
        } => complete_new_expression(session, file, *scope_id, *scope_mark),

        CompletionContext::ImportPath { partial_path } => {
            complete_import_paths(session, file, partial_path)
        }

        CompletionContext::ImportClause {
            target_module,
            existing_names,
            space_filter,
        } => complete_imports(session, *target_module, existing_names, *space_filter),

        CompletionContext::Unknown => {
            complete_all(session, file, matches!(trigger, CompletionTrigger::Invoked))
        }
    };

    // add auto import completions for relevant positions
    match context {
        CompletionContext::ValuePosition {
            scope_id,
            scope_mark,
        } => {
            let auto_imports = complete_auto_imports_with_visibility(
                session,
                file,
                prefix,
                Some(SymbolSpace::Value),
                scope_id,
                scope_mark,
                allow_short_prefix,
            );
            results.extend(auto_imports);
        }
        CompletionContext::StatementPosition {
            scope_id,
            scope_mark,
        } => {
            let auto_imports = complete_auto_imports_with_visibility(
                session,
                file,
                prefix,
                Some(SymbolSpace::Value),
                scope_id,
                scope_mark,
                allow_short_prefix,
            );
            results.extend(auto_imports);
        }
        CompletionContext::TypePosition {
            scope_id,
            scope_mark,
        } => {
            let auto_imports = complete_auto_imports_with_visibility(
                session,
                file,
                prefix,
                Some(SymbolSpace::Type),
                scope_id,
                scope_mark,
                allow_short_prefix,
            );
            results.extend(auto_imports);
        }
        CompletionContext::ObjectLiteralValue {
            scope_id,
            scope_mark,
        } => {
            let auto_imports = complete_auto_imports_with_visibility(
                session,
                file,
                prefix,
                Some(SymbolSpace::Value),
                scope_id,
                scope_mark,
                allow_short_prefix,
            );
            results.extend(auto_imports);
        }
        CompletionContext::CallArgument {
            scope_id,
            scope_mark,
        } => {
            let auto_imports = complete_auto_imports_with_visibility(
                session,
                file,
                prefix,
                Some(SymbolSpace::Value),
                scope_id,
                scope_mark,
                allow_short_prefix,
            );
            results.extend(auto_imports);
        }
        CompletionContext::NewExpression {
            scope_id,
            scope_mark,
        } => {
            let mut auto_imports = complete_auto_imports_with_visibility(
                session,
                file,
                prefix,
                Some(SymbolSpace::Value),
                scope_id,
                scope_mark,
                allow_short_prefix,
            );

            // filter to constructable auto import entries
            auto_imports.retain(is_constructable_completion);
            results.extend(auto_imports);
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
    // prepare the completion buffer
    let mut results = Vec::new();

    // get the module for context
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let types = ctx.types();
    let symbols = ctx.symbols();
    let current_module_id = ctx.module_id;

    // primary path: use receiver type for type aware completions
    if let Some(type_id) = receiver_type {
        // resolve members from the type
        let members = resolve_type_members(&types, &symbols, type_id, session, current_module_id);

        for member in members {
            let Some(completion) = completion_for_member(session, member, Some(&types)) else {
                continue;
            };

            results.push(completion);
        }

        return results;
    }

    // fallback path: resolve nominal type from the receiver initializer
    if let Some(symbol_id) = receiver_symbol {
        let type_symbol = if symbol_id.module_id == ctx.module_id {
            resolve_nominal_symbol_from_initializer(session, &ctx, symbol_id)
        } else {
            let symbol_module = session.modules.get(symbol_id.module_id);
            let symbol_module = symbol_module.read();
            let symbol_ctx = session.query_context(&symbol_module);
            symbol_ctx.and_then(|symbol_ctx| {
                resolve_nominal_symbol_from_initializer(session, &symbol_ctx, symbol_id)
            })
        };

        if let Some(type_symbol) = type_symbol {
            drop(types);
            drop(symbols);
            drop(module);

            let members = resolve_reference_members(type_symbol, session, current_module_id);
            for member in members {
                let Some(completion) = completion_for_member(session, member, None) else {
                    continue;
                };

                results.push(completion);
            }

            return results;
        }
    }

    // fallback path: use receiver symbol (for cases where type inference hasn't run)
    if let Some(symbol_id) = receiver_symbol {
        // release the current module state before switching contexts
        drop(types);
        drop(symbols);
        drop(module);

        // load the symbol's module
        let symbol_module = session.modules.get(symbol_id.module_id);
        let symbol_module = symbol_module.read();
        let Some(symbol_ctx) = session.query_context(&symbol_module) else {
            return Vec::new();
        };
        let symbols = symbol_ctx.symbols();
        let types = symbol_ctx.types();

        // get the scope owned by this symbol (for types like struct/class)
        if let Some(owned_scope_id) = owned_scope_for_symbol(&symbols, symbol_id.local_id) {
            let scope = symbols.get_scope_by_id(owned_scope_id);

            // add all named symbols in the scope as member completions
            for (key, member_id) in symbols.active_named_symbols(scope) {
                if let dir::StaticKey::Name(name_id) = key {
                    let member_symbol = symbols.get_symbol(member_id);
                    let name = session.strings.get(name_id).to_string();
                    let kind = CompletionKind::from(member_symbol.ty);

                    let mut completion =
                        Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL);

                    // add type detail from primary declaration
                    let declaration = member_symbol.primary_declaration;
                    if let Some(declaration) = declaration {
                        let type_id = types.get_declared_or_inferred_type_id(declaration);
                        if let Some(type_id) = type_id {
                            let type_text = format_local_type(
                                type_id,
                                &types,
                                &session.modules,
                                &session.strings,
                            );
                            completion = completion.with_detail(type_text);
                        }
                    }

                    // fall back to generic detail for functions
                    if completion.detail.is_none() && member_symbol.ty == SymbolType::Function {
                        completion = completion.with_detail("method");
                    }

                    results.push(completion);
                }
            }
        }

        // release the borrowed symbol module state
        drop(types);
        drop(symbols);
        drop(symbol_module);

        // merge extension members for this symbol
        let extension_members =
            resolve_extension_members_for_symbol(session, symbol_id, current_module_id);
        let mut seen_names: HashSet<String> = results
            .iter()
            .map(|completion| completion.label.clone())
            .collect();

        for member in extension_members {
            let Some(completion) = completion_for_member(session, member, None) else {
                continue;
            };

            if !seen_names.insert(completion.label.clone()) {
                continue;
            }

            results.push(completion);
        }
    }

    // return the final member completions
    results
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
    // prepare the completion buffer and scope name tracking
    let mut results = Vec::new();
    let mut seen_names = HashSet::new();

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
        let current_module_id = ctx.module_id;

        // resolve type members
        let members = resolve_type_members(&types, &symbols, type_id, session, current_module_id);

        for member in members {
            // skip non field members (methods, call signatures, etc.)
            if member.kind != MemberKind::Field {
                continue;
            }

            // skip non displayable members
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
            seen_names.insert(name);
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
        let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

        for visible in visible_symbols(&symbols, scope_id, mark, Some(SymbolSpace::Value)) {
            let dir::StaticKey::Name(name_id) = visible.key else {
                continue;
            };

            let name = session.strings.get(name_id).to_string();

            // dedupe against names already emitted from expected type fields
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

    // return the object literal completions
    results
}

/// Complete types (in type position).
fn complete_types(
    session: &Session,
    file: FileId,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
) -> Vec<Completion> {
    if scope_id.is_none() {
        return primitive_type_completions();
    }

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

    // prepare the completion buffer
    let mut results = Vec::new();
    let mut seen_names = HashSet::new();

    // resolve symbol types across re exports
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

    // walk up from the scope to collect visible types
    let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

    for visible in visible_symbols(
        &symbols,
        scope_id.expect("scope_id is required for type completions"),
        mark,
        Some(SymbolSpace::Type),
    ) {
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

    // include type imports and re exports from dependencies
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

    // return the type completions
    results
}

/// Primitive type completions.
fn primitive_type_completions() -> Vec<Completion> {
    // base literal keywords
    let mut names = vec![
        "any".to_string(),
        "unknown".to_string(),
        "never".to_string(),
        "void".to_string(),
        "null".to_string(),
        "undefined".to_string(),
        "object".to_string(),
        "boolean".to_string(),
        "character".to_string(),
        "string".to_string(),
        "bigint".to_string(),
        "number".to_string(),
        "symbol".to_string(),
        "unique symbol".to_string(),
        "int".to_string(),
        "uint".to_string(),
        "float".to_string(),
    ];

    // add integer type variants
    let int_types = [
        IntType::Int8,
        IntType::Int16,
        IntType::Int32,
        IntType::Int64,
        IntType::Int128,
        IntType::Int256,
        IntType::Isize,
        IntType::Uint8,
        IntType::Uint16,
        IntType::Uint32,
        IntType::Uint64,
        IntType::Uint128,
        IntType::Uint256,
        IntType::Usize,
    ];
    for int_type in int_types {
        names.push(int_type.as_str());
    }

    // add float type variants
    let float_types = [FloatType::Float32, FloatType::Float64];
    for float_type in float_types {
        names.push(float_type.as_str());
    }

    // dedupe while preserving order
    let mut seen: HashSet<String> = HashSet::new();
    let mut completions = Vec::new();
    for name in names {
        if !seen.insert(name.clone()) {
            continue;
        }

        completions.push(
            Completion::new(name, CompletionKind::TypeParameter).with_sort_order(SORT_BUILTIN),
        );
    }

    completions
}

/// Complete values (in expression position).
fn complete_values(
    session: &Session,
    file: FileId,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
    include_keywords: bool,
) -> Vec<Completion> {
    if scope_id.is_none() {
        return if include_keywords {
            keyword_completions()
        } else {
            Vec::new()
        };
    }

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

    // initialize completion buffers
    let mut results = Vec::new();

    // collect symbols to process (to avoid holding symbols lock while generating snippets)
    let mut symbols_to_process: Vec<(dir::LocalSymbolId, String, SymbolType)> = Vec::new();

    // walk visible symbols in scope order
    let mut seen_names = HashSet::new();
    let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

    for visible in visible_symbols(
        &symbols,
        scope_id.expect("scope_id is required for value completions"),
        mark,
        Some(SymbolSpace::Value),
    ) {
        let dir::StaticKey::Name(name_id) = visible.key else {
            continue;
        };

        let name = session.strings.get(name_id).to_string();
        if !seen_names.insert(name.clone()) {
            continue;
        }

        symbols_to_process.push((visible.id, name, visible.symbol.ty));
    }

    // release symbol and module handles before formatting snippets
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

    // add keywords if requested
    if include_keywords {
        results.extend(keyword_completions());
    }

    // return the value completions
    results
}

/// Complete constructable symbols for a new expression.
fn complete_new_expression(
    session: &Session,
    file: FileId,
    scope_id: Option<dir::LocalScopeId>,
    scope_mark: Option<dir::LocalScopeMark>,
) -> Vec<Completion> {
    if scope_id.is_none() {
        return Vec::new();
    }

    // resolve the module and query context
    let Some(module) = get_module_by_file_id(session, file) else {
        return Vec::new();
    };
    let module = module.read();
    let Some(ctx) = session.query_context(&module) else {
        return Vec::new();
    };
    let symbols = ctx.symbols();

    // prepare result containers
    let mut results = Vec::new();
    let mut seen = HashSet::new();
    let mark = scope_mark.unwrap_or(dir::LocalScopeMark::end());

    // prefer scoped symbol lookup when possible
    for visible in visible_symbols(
        &symbols,
        scope_id.expect("scope_id is required for new expression completions"),
        mark,
        Some(SymbolSpace::Value),
    ) {
        // skip symbols that are not constructable
        if !is_constructable_symbol(visible.symbol.ty) {
            continue;
        }

        // resolve the symbol name key
        let dir::StaticKey::Name(name_id) = visible.key else {
            continue;
        };

        // skip duplicate names
        let name = session.strings.get(name_id).to_string();
        if !seen.insert(name.clone()) {
            continue;
        }

        // push a completion entry
        let kind = CompletionKind::from(visible.symbol.ty);
        results.push(Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL));
    }

    // return the new expression completions
    results
}

/// Check whether a symbol type is constructable with new.
fn is_constructable_symbol(symbol_type: SymbolType) -> bool {
    // return whether the symbol is constructable
    matches!(symbol_type, SymbolType::Class | SymbolType::Struct)
}

/// Check whether a completion entry is constructable with new.
fn is_constructable_completion(completion: &Completion) -> bool {
    // return whether the completion kind supports new
    matches!(
        completion.kind,
        CompletionKind::Class | CompletionKind::Struct
    )
}

/// Complete imports from a module.
fn complete_imports(
    session: &Session,
    target_module: Option<ModuleId>,
    existing_names: &[String],
    space_filter: Option<SymbolSpace>,
) -> Vec<Completion> {
    // resolve the target module id
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

    // prepare the completion buffer
    let mut results = Vec::new();

    // normalize existing names for filtering
    let existing_names: HashSet<&str> = existing_names.iter().map(String::as_str).collect();

    // add all exported symbols
    for symbol in symbols.symbols() {
        if symbol.export.is_none() {
            continue;
        }

        let Some(string_id) = symbol.name() else {
            continue;
        };

        if !matches_symbol_space_filter(symbol.ty, symbol.space, space_filter) {
            continue;
        }

        let name = session.strings.get(string_id).to_string();
        if existing_names.contains(name.as_str()) {
            continue;
        }

        let kind = CompletionKind::from(symbol.ty);
        results.push(Completion::new(name, kind).with_sort_order(SORT_LOCAL_SYMBOL));
    }

    // return the collected completions
    results
}

/// Complete all symbols (fallback for unknown context).
fn complete_all(_session: &Session, _file: FileId, include_keywords: bool) -> Vec<Completion> {
    if include_keywords {
        keyword_completions()
    } else {
        Vec::new()
    }
}

/// Get keyword completions.
fn keyword_completions() -> Vec<Completion> {
    // define the keyword list
    let keywords = [
        Keyword::Public,
        Keyword::Protected,
        Keyword::Private,
        Keyword::Readonly,
        Keyword::Static,
        Keyword::Final,
        Keyword::Accessor,
        Keyword::Default,
        Keyword::Self_,
        Keyword::This,
        Keyword::Super,
        Keyword::Package,
        Keyword::Import,
        Keyword::Export,
        Keyword::From,
        Keyword::Const,
        Keyword::Let,
        Keyword::Var,
        Keyword::Namespace,
        Keyword::Type,
        Keyword::Newtype,
        Keyword::Struct,
        Keyword::Class,
        Keyword::Enum,
        Keyword::Union,
        Keyword::Interface,
        Keyword::Function,
        Keyword::Extension,
        Keyword::Declare,
        Keyword::New,
        Keyword::Delete,
        Keyword::Constructor,
        Keyword::Asserts,
        Keyword::Extends,
        Keyword::Implements,
        Keyword::Satisfies,
        Keyword::Abstract,
        Keyword::Override,
        Keyword::InstanceOf,
        Keyword::Where,
        Keyword::Typeof,
        Keyword::Keyof,
        Keyword::Infer,
        Keyword::Any,
        Keyword::Never,
        Keyword::As,
        Keyword::Is,
        Keyword::In,
        Keyword::Of,
        Keyword::Using,
        Keyword::Provides,
        Keyword::Comptime,
        Keyword::If,
        Keyword::Else,
        Keyword::Match,
        Keyword::Switch,
        Keyword::Case,
        Keyword::Do,
        Keyword::While,
        Keyword::For,
        Keyword::Loop,
        Keyword::Assert,
        Keyword::Break,
        Keyword::Continue,
        Keyword::Debugger,
        Keyword::Return,
        Keyword::Yield,
        Keyword::Goto,
        Keyword::Try,
        Keyword::Catch,
        Keyword::Throw,
        Keyword::Finally,
        Keyword::Async,
        Keyword::Await,
        Keyword::Get,
        Keyword::Set,
        Keyword::Move,
        Keyword::With,
    ];

    // add keyword completions
    let mut seen = HashSet::new();
    let mut completions = Vec::new();
    for keyword in keywords {
        let label = keyword.as_str();
        if !seen.insert(label) {
            continue;
        }

        let mut completion =
            Completion::new(label, CompletionKind::Keyword).with_sort_order(SORT_KEYWORD);
        if let Some(snippet) = keyword_snippet(keyword) {
            completion = completion.with_insert_text(snippet).as_snippet();
        }

        completions.push(completion);
    }

    // add literal keywords that are not in the enum
    for literal in ["true", "false", "null", "undefined"] {
        if !seen.insert(literal) {
            continue;
        }
        completions
            .push(Completion::new(literal, CompletionKind::Keyword).with_sort_order(SORT_KEYWORD));
    }

    completions
}

/// Resolve a snippet template for control-flow keywords.
fn keyword_snippet(keyword: Keyword) -> Option<&'static str> {
    match keyword {
        Keyword::If => Some("if (${1:condition}) {\n    $0\n}"),
        Keyword::For => Some("for (${1:item} in ${2:items}) {\n    $0\n}"),
        Keyword::While => Some("while (${1:condition}) {\n    $0\n}"),
        Keyword::Switch => {
            Some("switch (${1:value}) {\n    case ${2:pattern}:\n        $0\n    default:\n}")
        }
        Keyword::Try => Some("try {\n    $1\n} catch (${2:error}) {\n    $0\n}"),
        _ => None,
    }
}

/// Complete import paths (relative paths or package names).
fn complete_import_paths(session: &Session, file: FileId, partial: &str) -> Vec<Completion> {
    // prepare the completion buffer
    let mut results = Vec::new();

    // select completion strategy based on the partial path
    if partial.starts_with("./") || partial.starts_with("../") {
        // relative path: list directory contents
        let source_file = session.files.get(file);
        let path = source_file.path.as_ref();
        if let Some(path) = path {
            let base_dir = path.parent();
            if let Some(base_dir) = base_dir {
                results.extend(complete_relative_path(session, base_dir, partial));
            }
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

    // return the import path completions
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

    // read directory entries for completion
    let Ok(entries) = session.fs.read_dir(&dir_to_list) else {
        return vec![];
    };

    // map entries into completion items
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
            } else if let Some(file_type) = FileType::from_path(entry) {
                let loader = Loader::from_file_type(file_type);
                if !loader.is_code() {
                    return None;
                }

                let module_name = module_name_from_path(entry)?;
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

/// Resolve a module specifier for an importable file path.
/// Complete package names from the registry.
fn complete_package_names(session: &Session, prefix: &str) -> Vec<Completion> {
    // prepare the completion buffer
    let mut results = Vec::new();

    // iterate packages in the registry
    for pkg_ref in session.packages.iter() {
        let pkg = pkg_ref.read();
        let Some(name) = pkg.name.as_ref() else {
            continue;
        };

        // filter by prefix
        if !prefix.is_empty() && !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
            continue;
        }

        // build the completion entry
        let mut completion =
            Completion::new(name.clone(), CompletionKind::Module).with_sort_order(SORT_BUILTIN);

        // add version detail if available
        if let Some(v) = &pkg.version {
            completion = completion.with_detail(format!("v{v}"));
        }

        results.push(completion);
    }

    // return package completions
    results
}

#[cfg(test)]
mod tests {
    use super::{generate_call_snippet, keyword_snippet};
    use destack_ast::Keyword;

    /// Build plain call text for functions without parameters.
    #[test]
    fn test_generate_call_snippet_without_parameters() {
        let (snippet, is_snippet) = generate_call_snippet("run", &[]);

        assert_eq!(snippet, "run()");
        assert!(!is_snippet);
    }

    /// Build placeholder snippets for parameterized function calls.
    #[test]
    fn test_generate_call_snippet_with_parameters() {
        let parameters = vec!["value".to_string(), "count".to_string()];
        let (snippet, is_snippet) = generate_call_snippet("run", &parameters);

        assert_eq!(snippet, "run(${1:value}, ${2:count})$0");
        assert!(is_snippet);
    }

    /// Provide structured snippets for control flow keywords.
    #[test]
    fn test_keyword_snippet_for_control_flow() {
        let snippet = keyword_snippet(Keyword::If);

        assert_eq!(snippet, Some("if (${1:condition}) {\n    $0\n}"));
    }
}
