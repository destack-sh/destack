use std::collections::{HashMap, HashSet};

use destack_base::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_fir::write;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::{
    AddressSpace, Block, Function, Global, Instruction, Local, LocalNodeId, Mutability, Node,
    NodeTree, NodeTreeImpl, ReferenceKind, Type, TypeAlias,
};

use super::types::format_type_expanded;

pub type MirFormatter<'a, 'buf> = Formatter<'buf, MirFormatContext<'a>>;

/// MIR format options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MirFormatOptions {
    /// Line ending style.
    pub line_ending: LineEnding,
    /// Indent style.
    pub indent_style: IndentStyle,
    /// Indent width.
    pub indent_width: u8,
    /// Maximum line width.
    pub line_width: u8,
    /// Whether to emit synthetic type aliases for readability.
    pub use_type_aliases: bool,
    /// Minimum number of uses before a type gets a synthetic alias.
    pub type_alias_min_uses: u8,
}

impl Default for MirFormatOptions {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::LineFeed,
            indent_style: IndentStyle::Space,
            indent_width: 4,
            line_width: 100,
            use_type_aliases: false,
            type_alias_min_uses: 2,
        }
    }
}

impl MirFormatOptions {
    /// Convert to print options.
    pub fn as_print_options(&self) -> PrintOptions {
        PrintOptions {
            line_ending: self.line_ending,
            line_width: self.line_width,
            indent_style: self.indent_style,
            indent_width: self.indent_width,
        }
    }

    /// Enable or disable synthetic type aliases.
    pub fn with_type_aliases(mut self, value: bool) -> Self {
        self.use_type_aliases = value;
        self
    }

    /// Set the minimum number of uses required for a synthetic alias.
    pub fn with_type_alias_min_uses(mut self, value: u8) -> Self {
        self.type_alias_min_uses = value;
        self
    }

}

impl FormatOptions for MirFormatOptions {
    fn indent_style(&self) -> IndentStyle {
        self.indent_style
    }

    fn indent_width(&self) -> u8 {
        self.indent_width
    }

    fn line_width(&self) -> u8 {
        self.line_width
    }

    fn as_print_options(&self) -> PrintOptions {
        self.as_print_options()
    }
}

/// MIR format context.
pub struct MirFormatContext<'a> {
    /// Format options.
    pub options: MirFormatOptions,
    /// The MIR tree.
    pub tree: &'a NodeTree,
    /// The strings.
    pub strings: &'a ImmutableStringPool,
    /// Dummy file for FIR compatibility.
    file: File,

    // local context (a little bit hacky but fine for now)
    /// Map from block ID to its index in the current function's block list.
    /// Used for formatting block references with stable indices.
    pub block_indices: HashMap<LocalNodeId<Block>, usize>,
    /// Map from local ID to its index in the current function's local list.
    pub local_indices: HashMap<LocalNodeId<Local>, usize>,
    /// Map from function ID to its unique display name.
    pub function_names: HashMap<LocalNodeId<Function>, String>,
    /// Map from global ID to its unique display name.
    pub global_names: HashMap<LocalNodeId<Global>, String>,
    /// Map from type ID to its alias name (if any).
    pub type_alias_by_type: HashMap<LocalNodeId<Type>, String>,
    /// Synthetic aliases generated for readability.
    pub synthetic_aliases: Vec<(LocalNodeId<Type>, String)>,
}

impl<'a> std::fmt::Debug for MirFormatContext<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MirFormatContext")
            .field("options", &self.options)
            .finish()
    }
}

impl<'a> MirFormatContext<'a> {
    /// Create a new format context.
    pub fn new(
        tree: &'a NodeTree,
        strings: &'a ImmutableStringPool,
        options: MirFormatOptions,
    ) -> Self {
        // collect explicit type aliases
        let type_alias_by_type: HashMap<_, _> = tree
            .iter_nodes::<TypeAlias>()
            .map(|(_, alias)| (alias.ty, strings.get(alias.name).to_string()))
            .collect();

        // assign unique function and global names
        let function_names = build_unique_function_names(tree, strings);
        let global_names = build_unique_global_names(tree, strings);

        // include synthetic aliases when configured
        let (type_alias_by_type, synthetic_aliases) = if options.use_type_aliases {
            // build synthetic aliases
            build_synthetic_aliases(
                tree,
                strings,
                type_alias_by_type,
                options.type_alias_min_uses,
            )
        } else {
            // skip synthetic aliases
            (type_alias_by_type, Vec::new())
        };

        // assemble the format context
        Self {
            options,
            tree,
            strings,
            file: File::empty_text(FileType::Destack),
            block_indices: HashMap::new(),
            local_indices: HashMap::new(),
            function_names,
            global_names,
            type_alias_by_type,
            synthetic_aliases,
        }
    }

    /// Get the index of a block in the current function.
    pub fn block_index(&self, id: LocalNodeId<Block>) -> usize {
        // resolve the cached block index when available
        if let Some(index) = self.block_indices.get(&id) {
            return *index;
        }

        // fall back to the local id
        id.id as usize
    }

    /// Get the index of a local in the current function.
    pub fn local_index(&self, id: LocalNodeId<Local>) -> usize {
        // resolve the cached local index when available
        if let Some(index) = self.local_indices.get(&id) {
            return *index;
        }

        // fall back to the local id
        id.id as usize
    }

    /// Get the unique function display name.
    pub fn function_name(&self, id: LocalNodeId<Function>) -> &str {
        // resolve cached unique name
        if let Some(name) = self.function_names.get(&id) {
            return name.as_str();
        }

        // fall back to stored name
        let function = self.tree.get(id);
        self.strings.get(function.name)
    }

    /// Get the unique global display name.
    pub fn global_name(&self, id: LocalNodeId<Global>) -> &str {
        // resolve cached unique name
        if let Some(name) = self.global_names.get(&id) {
            return name.as_str();
        }

        // fall back to stored name
        let global = self.tree.get(id);
        self.strings.get(global.name)
    }

    /// Get the alias name for a type, if one exists.
    pub fn type_alias_name(&self, ty: LocalNodeId<Type>) -> Option<&str> {
        // resolve the alias name when present
        self.type_alias_by_type.get(&ty).map(|name| name.as_str())
    }
}

/// Build unique display names for functions.
fn build_unique_function_names(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> HashMap<LocalNodeId<Function>, String> {
    // collect function names
    let names = tree
        .iter_nodes::<Function>()
        .map(|(id, function)| (id, strings.get(function.name).to_string()));

    // build stable unique names
    build_unique_names(names)
}

/// Build unique display names for globals.
fn build_unique_global_names(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
) -> HashMap<LocalNodeId<Global>, String> {
    // collect global names
    let names = tree
        .iter_nodes::<Global>()
        .map(|(id, global)| (id, strings.get(global.name).to_string()));

    // build stable unique names
    build_unique_names(names)
}

/// Build unique display names for nodes.
fn build_unique_names<T>(
    items: impl Iterator<Item = (LocalNodeId<T>, String)>,
) -> HashMap<LocalNodeId<T>, String>
where
    T: Node,
{
    // track used names and suffixes
    let mut used_names = HashSet::new();
    let mut next_suffix: HashMap<String, usize> = HashMap::new();
    let mut names_by_id = HashMap::new();

    // assign stable unique names
    for (id, base) in items {
        // resolve the base name or suffix
        let name = if used_names.contains(&base) {
            // use a suffixed variant
            // seed the suffix counter
            let entry = next_suffix.entry(base.clone()).or_insert(1);

            // find the next available suffix
            loop {
                let candidate = format!("{base}#{entry}");
                *entry += 1;
                if !used_names.contains(&candidate) {
                    break candidate;
                }
            }
        } else {
            // use the base name
            base.clone()
        };

        // record the final name
        used_names.insert(name.clone());
        names_by_id.insert(id, name);
    }

    // return the final name map
    names_by_id
}

/// Build synthetic type aliases based on usage counts.
#[allow(clippy::type_complexity)]
fn build_synthetic_aliases(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    mut type_alias_by_type: HashMap<LocalNodeId<Type>, String>,
    min_uses: u8,
) -> (
    HashMap<LocalNodeId<Type>, String>,
    Vec<(LocalNodeId<Type>, String)>,
) {
    // collect how often types appear in formatted output
    let type_uses = collect_type_uses(tree);

    // seed alias names with explicit aliases
    let mut alias_names: HashSet<String> = type_alias_by_type.values().cloned().collect();
    let mut synthetic_aliases = Vec::new();
    let mut next_alias_indices: HashMap<String, usize> = HashMap::new();
    let mut candidates: HashMap<String, AliasCandidateGroup> = HashMap::new();

    // collect candidates by structural key
    for (type_id, ty) in tree.iter_nodes::<Type>() {
        // skip types that already have aliases
        if type_alias_by_type.contains_key(&type_id) {
            continue;
        }

        // filter by aliasable shapes
        if !should_alias_type(ty) {
            continue;
        }

        // require enough uses to justify an alias
        let uses = type_uses.get(&type_id).copied().unwrap_or(0);
        if uses == 0 {
            continue;
        }

        // group candidates by structure
        let key = type_key_for_alias(tree, strings, type_id);
        let entry = candidates.entry(key).or_default();
        entry.total_uses += uses;
        entry.type_ids.push(type_id);

        // track metadata names when available
        if let Some(name) = metadata_name_for_type(tree, strings, type_id) {
            entry.metadata_names.insert(name);
        }
    }

    // order keys for deterministic naming
    let mut ordered_keys: Vec<_> = candidates.keys().cloned().collect();
    ordered_keys.sort();

    // assign aliases in stable order
    for key in ordered_keys {
        // select the next alias name
        let candidate = candidates
            .remove(&key)
            .unwrap_or_else(|| panic!("missing alias candidates for {key}"));
        if candidate.total_uses < u32::from(min_uses) {
            continue;
        }

        // reserve the alias name
        let alias_name =
            alias_name_for_candidate(tree, &candidate, &mut next_alias_indices, &alias_names);
        alias_names.insert(alias_name.clone());

        // assign the alias to all matching types
        if let Some((first, rest)) = candidate.type_ids.split_first() {
            type_alias_by_type.insert(*first, alias_name.clone());
            synthetic_aliases.push((*first, alias_name.clone()));
            for type_id in rest {
                type_alias_by_type.insert(*type_id, alias_name.clone());
            }
        }
    }

    (type_alias_by_type, synthetic_aliases)
}

/// Check whether a type is eligible for synthetic aliasing.
fn should_alias_type(ty: &Type) -> bool {
    // allow aliasing for common aggregate shapes
    matches!(ty, Type::Struct { .. } | Type::Tuple { .. })
}

/// Choose an alias name for a candidate group.
fn alias_name_for_candidate(
    tree: &NodeTree,
    candidate: &AliasCandidateGroup,
    next_alias_indices: &mut HashMap<String, usize>,
    alias_names: &HashSet<String>,
) -> String {
    // prefer a single metadata name when available
    if candidate.metadata_names.len() == 1 {
        let name = candidate
            .metadata_names
            .iter()
            .next()
            .unwrap_or_else(|| panic!("missing metadata name for alias candidate"));
        return unique_alias_name(name, alias_names);
    }

    // fall back to a type based prefix
    let first_id = candidate
        .type_ids
        .first()
        .copied()
        .unwrap_or_else(|| panic!("missing type id for alias candidate"));
    let prefix = type_alias_prefix(tree.get(first_id));
    next_available_alias_name(prefix, next_alias_indices, alias_names)
}

/// Return a unique alias name based on the preferred base.
fn unique_alias_name(base: &str, alias_names: &HashSet<String>) -> String {
    // fast path when unused
    if !alias_names.contains(base) {
        return base.to_string();
    }

    // add a suffix for uniqueness
    let mut suffix = 1;
    loop {
        let candidate = format!("{base}#{suffix}");
        if !alias_names.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

/// Return the next available alias name for a prefix.
fn next_available_alias_name(
    prefix: &str,
    next_alias_indices: &mut HashMap<String, usize>,
    alias_names: &HashSet<String>,
) -> String {
    // seed the prefix counter
    let entry = next_alias_indices.entry(prefix.to_string()).or_insert(0);

    // advance prefix counters until unused
    loop {
        let candidate = format!("{prefix}{entry}");
        *entry += 1;
        if !alias_names.contains(&candidate) {
            return candidate;
        }
    }
}

/// Return the default alias prefix for a type.
fn type_alias_prefix(ty: &Type) -> &'static str {
    // select a prefix based on the type shape
    match ty {
        Type::Struct { .. } => "Struct",
        Type::Tuple { .. } => "Tuple",
        Type::Array { .. } => "Array",
        Type::Reference { .. } => "Ref",
        Type::FunctionPointer { .. } => "Fn",
        _ => "Type",
    }
}

/// Read the metadata name for a type, if any.
fn metadata_name_for_type(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    ty: LocalNodeId<Type>,
) -> Option<String> {
    // read the metadata name when available
    tree.type_table
        .type_metadata(ty)
        .and_then(|metadata| metadata.name)
        .map(|name_id| strings.get(name_id).to_string())
}

/// Alias candidates grouped by structural key.
#[derive(Default)]
struct AliasCandidateGroup {
    /// Total uses across matching types.
    total_uses: u32,
    /// Type ids that share the same structural key.
    type_ids: Vec<LocalNodeId<Type>>,
    /// Metadata names seen for the group.
    metadata_names: HashSet<String>,
}

/// Build a structural key used for alias grouping.
fn type_key_for_alias(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    ty: LocalNodeId<Type>,
) -> String {
    // format a stable structural key
    match tree.get(ty) {
        Type::Void => "void".to_string(),
        Type::Boolean => "bool".to_string(),
        Type::Int { width, is_signed } => {
            // use signedness prefix plus width
            let prefix = if *is_signed { "i" } else { "u" };
            format!("{prefix}{width}")
        }
        Type::Isize => "isize".to_string(),
        Type::Usize => "usize".to_string(),
        Type::Float { width } => format!("f{width}"),
        Type::Type => "type".to_string(),
        Type::Reference {
            kind,
            address_space,
            mutability,
            pointee,
            is_nullable,
        } => {
            // start with reference header
            let mut result = String::new();

            // include the nullable marker when needed
            if *is_nullable {
                result.push_str("ref?<");
            }
            // include the non nullable marker otherwise
            else {
                result.push_str("ref<");
            }
            result.push_str(match kind {
                ReferenceKind::Managed => "managed",
                ReferenceKind::Owned => "owned",
                ReferenceKind::Borrowed => "borrowed",
                ReferenceKind::Raw => "raw",
            });

            // append address space when explicit
            if !address_space.is_generic() {
                let addrspace = match address_space.keyword() {
                    Some(name) => format!("addrspace({name})"),
                    None => match address_space {
                        AddressSpace::Target(id) => format!("addrspace({id})"),
                        _ => "addrspace(unknown)".to_string(),
                    },
                };
                result.push(' ');
                result.push_str(&addrspace);
            }

            // append mutability when required
            if *mutability == Mutability::Mutable {
                result.push_str(" mut");
            }

            // append the pointee key
            result.push(' ');
            result.push_str(&type_key_for_alias(tree, strings, *pointee));
            result.push('>');
            result
        }
        Type::Array {
            element, length, ..
        } => {
            // format array keys with element and length
            format!(
                "[{}; {length}]",
                type_key_for_alias(tree, strings, *element)
            )
        }
        Type::Tuple { elements, .. } => {
            // join tuple element keys
            let elements = elements
                .iter()
                .map(|element| type_key_for_alias(tree, strings, *element))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({elements})")
        }
        Type::Struct { fields, .. } => {
            // join struct field keys
            let fields = fields
                .iter()
                .map(|field_id| {
                    let field = tree.get(*field_id);
                    let field_type = type_key_for_alias(tree, strings, field.ty);
                    match field.name {
                        Some(name) => format!("{}: {field_type}", strings.get(name)),
                        None => field_type,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        Type::FunctionPointer { parameters, result } => {
            // join parameter and result keys
            let params = parameters
                .iter()
                .map(|param| type_key_for_alias(tree, strings, *param))
                .collect::<Vec<_>>()
                .join(", ");
            let result = type_key_for_alias(tree, strings, *result);
            format!("fn({params}) -> {result}")
        }
    }
}

/// Collect type usage counts for formatting.
fn collect_type_uses(tree: &NodeTree) -> HashMap<LocalNodeId<Type>, u32> {
    // initialize usage counts
    let mut counts = HashMap::new();

    // record global types
    for (_, global) in tree.iter_nodes::<Global>() {
        record_type_use(tree, global.ty, &mut counts);
    }

    // record function signatures
    for (_, function) in tree.iter_nodes::<Function>() {
        record_type_use(tree, function.return_type, &mut counts);
        for parameter in &function.parameters {
            record_type_use(tree, parameter.ty, &mut counts);
        }
    }

    // record local types
    for (_, local) in tree.iter_nodes::<Local>() {
        record_type_use(tree, local.ty, &mut counts);
    }

    // record block parameter types
    for (_, block) in tree.iter_nodes::<Block>() {
        for parameter in &block.parameters {
            record_type_use(tree, parameter.ty, &mut counts);
        }
    }

    // record instruction types
    for (_, instruction) in tree.iter_nodes::<Instruction>() {
        match instruction {
            Instruction::Cast { to_type, .. } => {
                record_type_use(tree, *to_type, &mut counts);
            }
            Instruction::LocalAddr { result_type, .. } => {
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::GlobalAddr { result_type, .. } => {
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::Load { result_type, .. } => {
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::FieldAddr { result_type, .. } => {
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::ElementAddr { result_type, .. } => {
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::Struct { ty, .. } => {
                record_type_use(tree, *ty, &mut counts);
            }
            Instruction::Tuple { ty, .. } => {
                record_type_use(tree, *ty, &mut counts);
            }
            Instruction::Array { ty, .. } => {
                record_type_use(tree, *ty, &mut counts);
            }
            Instruction::Call { signature, .. } => {
                record_type_use(tree, *signature, &mut counts);
            }
            Instruction::CallVirtual {
                declaring_type,
                signature,
                ..
            } => {
                record_type_use(tree, *declaring_type, &mut counts);
                record_type_use(tree, *signature, &mut counts);
            }
            Instruction::CallInterface {
                declaring_type,
                signature,
                ..
            } => {
                record_type_use(tree, *declaring_type, &mut counts);
                record_type_use(tree, *signature, &mut counts);
            }
            Instruction::CallIndirect { signature, .. } => {
                record_type_use(tree, *signature, &mut counts);
            }
            Instruction::ManagedAlloc {
                layout,
                result_type,
                ..
            } => {
                record_type_use(tree, *layout, &mut counts);
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::ManagedAllocArray {
                element,
                result_type,
                ..
            } => {
                record_type_use(tree, *element, &mut counts);
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::RawAlloc {
                layout,
                result_type,
                ..
            } => {
                record_type_use(tree, *layout, &mut counts);
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::StackAlloc {
                layout,
                result_type,
                ..
            } => {
                record_type_use(tree, *layout, &mut counts);
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::Const { .. }
            | Instruction::Binary { .. }
            | Instruction::Unary { .. }
            | Instruction::Select { .. }
            | Instruction::LocalGet { .. }
            | Instruction::LocalSet { .. }
            | Instruction::GlobalConst { .. }
            | Instruction::Store { .. }
            | Instruction::FieldGet { .. }
            | Instruction::FieldSet { .. }
            | Instruction::ElementGet { .. }
            | Instruction::ElementSet { .. }
            | Instruction::RawFree { .. }
            | Instruction::RawDrop { .. }
            | Instruction::StackDrop { .. }
            | Instruction::Assume { .. }
            | Instruction::Intrinsic { .. } => {}
        }
    }

    // return usage counts
    counts
}

/// Record usage of a type and its nested types.
fn record_type_use(
    tree: &NodeTree,
    ty: LocalNodeId<Type>,
    counts: &mut HashMap<LocalNodeId<Type>, u32>,
) {
    // track visited types for this traversal
    let mut visited = HashSet::new();

    // walk the type graph once
    record_type_use_inner(tree, ty, counts, &mut visited);
}

/// Record usage of a type once per traversal.
fn record_type_use_inner(
    tree: &NodeTree,
    ty: LocalNodeId<Type>,
    counts: &mut HashMap<LocalNodeId<Type>, u32>,
    visited: &mut HashSet<LocalNodeId<Type>>,
) {
    // guard against cycles
    if !visited.insert(ty) {
        return;
    }

    // increment the usage count
    *counts.entry(ty).or_insert(0) += 1;

    // record nested types
    match tree.get(ty) {
        Type::Reference { pointee, .. } => {
            record_type_use_inner(tree, *pointee, counts, visited);
        }
        Type::Array { element, .. } => {
            record_type_use_inner(tree, *element, counts, visited);
        }
        Type::Tuple { elements, .. } => {
            // record tuple element types
            for element_id in elements {
                record_type_use_inner(tree, *element_id, counts, visited);
            }
        }
        Type::Struct { fields, .. } => {
            // record struct field types
            for field_id in fields {
                let field = tree.get(*field_id);
                record_type_use_inner(tree, field.ty, counts, visited);
            }
        }
        Type::FunctionPointer { parameters, result } => {
            // record function pointer types
            for parameter_id in parameters {
                record_type_use_inner(tree, *parameter_id, counts, visited);
            }
            record_type_use_inner(tree, *result, counts, visited);
        }
        Type::Void
        | Type::Boolean
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float { .. }
        | Type::Type => {}
    }
}

impl<'a> FormatContext for MirFormatContext<'a> {
    type Options = MirFormatOptions;

    fn options(&self) -> &Self::Options {
        &self.options
    }

    fn file(&self) -> &File {
        &self.file
    }
}

/// Trait for formatting MIR nodes.
pub trait FormatMirNode<'a, T: Node> {
    /// Format a node.
    fn format_node(&self, id: LocalNodeId<T>, f: &mut MirFormatter<'a, '_>) -> FormatResult<()>;
}

/// Implement Format for LocalNodeId<T> where T implements FormatMirNode.
impl<'a, T: Node + Clone> Format<MirFormatContext<'a>> for LocalNodeId<T>
where
    NodeTree: NodeTreeImpl<T>,
    T: FormatMirNode<'a, T>,
{
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let node = f.context().tree.get(*self);
        node.format_node(*self, f)
    }
}

/// Format a MIR tree to a string.
pub fn format_mir(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    options: MirFormatOptions,
) -> String {
    let context = MirFormatContext::new(tree, strings, options);

    // format all globals and functions
    let formatted = destack_fir::format!(context, [FormatAllItems]);
    match formatted {
        Ok(doc) => match doc.print() {
            Ok(printed) => printed.as_str().to_string(),
            Err(_) => "<print error>".to_string(),
        },
        Err(_) => "<format error>".to_string(),
    }
}

/// Helper to format all module items (globals and functions).
struct FormatAllItems;

impl<'a> Format<MirFormatContext<'a>> for FormatAllItems {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let tree = f.context().tree;
        let type_alias_ids: Vec<_> = tree.iter_nodes::<TypeAlias>().map(|(id, _)| id).collect();
        let global_ids: Vec<_> = tree.iter_nodes::<Global>().map(|(id, _)| id).collect();
        let function_ids: Vec<_> = tree.iter_nodes::<Function>().map(|(id, _)| id).collect();
        let mut first = true;

        // format synthetic type aliases first
        let synthetic_aliases = f.context().synthetic_aliases.clone();
        for (type_id, name) in &synthetic_aliases {
            if !first {
                write!(f, [hard_line_break()])?;
            }
            first = false;
            write!(
                f,
                [
                    token("type"),
                    space(),
                    token("@"),
                    text(name),
                    space(),
                    token("="),
                    space()
                ]
            )?;
            let ty = f.context().tree.get(*type_id);
            format_type_expanded(f, *type_id, ty)?;
            write!(f, [hard_line_break()])?;
        }

        // format explicit type aliases next
        for type_alias_id in &type_alias_ids {
            if !first {
                write!(f, [hard_line_break()])?;
            }
            first = false;
            write!(f, [type_alias_id, hard_line_break()])?;
        }

        // format globals first
        for global_id in &global_ids {
            if !first {
                write!(f, [hard_line_break()])?;
            }
            first = false;
            write!(f, [global_id, hard_line_break()])?;
        }

        // format functions
        for function_id in &function_ids {
            if !first {
                write!(f, [hard_line_break()])?;
            }
            first = false;
            write!(f, [function_id])?;
        }
        Ok(())
    }
}
