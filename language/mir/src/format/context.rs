use std::collections::{BTreeSet, HashMap, HashSet};

use destack_core::ImmutableStringPool;
use destack_fir::format::{Format, FormatContext, FormatOptions, FormatResult, Formatter};
use destack_fir::prelude::*;
use destack_fir::print::PrintOptions;
use destack_fir::write;
use destack_source::{File, FileType, IndentStyle, LineEnding};

use crate::parse::TokenType;
use crate::{
    Block, Function, Global, Instruction, Local, LocalNodeId, Mutability, Node, NodeTree,
    NodeTreeImpl, NodeType, ReferenceKind, TensorDimension, TensorLayout, Terminator, Type,
    TypeAlias, TypeReference, Value, function_signature_parts,
};

use super::r#type::format_type_declaration;

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
    /// Whether to render metadata names without module prefixes.
    pub use_local_names: bool,
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
            use_local_names: false,
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
            trim_trailing_whitespace: false,
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

    /// Enable or disable local name formatting for metadata.
    pub fn with_local_names(mut self, value: bool) -> Self {
        self.use_local_names = value;
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
    /// The function currently being formatted.
    pub current_function: Option<LocalNodeId<Function>>,
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
            .filter_map(|(_, alias)| {
                let TypeReference::Type(ty) = alias.ty else {
                    return None;
                };

                let name = strings.get(alias.name);
                let name = if options.use_local_names {
                    local_name_from_metadata(name)
                } else {
                    name.to_string()
                };
                Some((ty, name))
            })
            .collect();

        // assign unique function and global names
        let function_names = build_unique_function_names(tree, strings, options.use_local_names);
        let global_names = build_unique_global_names(tree, strings, options.use_local_names);

        // include synthetic aliases when configured
        let (type_alias_by_type, synthetic_aliases) = if options.use_type_aliases {
            // build synthetic aliases
            build_synthetic_aliases(
                tree,
                strings,
                type_alias_by_type,
                options.type_alias_min_uses,
                options.use_local_names,
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
            local_indices: HashMap::new(),
            function_names,
            global_names,
            type_alias_by_type,
            synthetic_aliases,
            current_function: None,
        }
    }

    /// Format a type alias name with the configured naming policy.
    pub fn format_alias_name(&self, name: &str) -> String {
        // use local names when configured
        if self.options.use_local_names {
            return local_name_from_metadata(name);
        }

        name.to_string()
    }

    /// Get the display name of a block in the current function.
    pub fn block_name(&self, id: LocalNodeId<Block>) -> String {
        let block = self.tree.get(id);
        let name = block
            .name
            .unwrap_or_else(|| panic!("missing MIR block name for {id:?}"));
        self.strings.get(name).to_string()
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

    /// Get the display name for an SSA value in the current function.
    pub fn value_name(&self, value: Value) -> String {
        let function_id = self
            .current_function
            .unwrap_or_else(|| panic!("missing current function while formatting {value:?}"));
        let function = self.tree.get(function_id);
        let name = function
            .value_name(value)
            .unwrap_or_else(|| panic!("missing MIR value name for {value:?}"));

        self.strings.get(name).to_string()
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

    /// Get the type for a value in the current function.
    pub fn value_type(&self, value: Value) -> Option<LocalNodeId<Type>> {
        let function_id = self.current_function?;
        let function = self.tree.get(function_id);
        function.value_type(value)
    }
}

/// Build unique display names for functions.
fn build_unique_function_names(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    use_local_names: bool,
) -> HashMap<LocalNodeId<Function>, String> {
    // collect function names
    let names = tree
        .iter_nodes::<Function>()
        .map(|(id, function)| (id, strings.get(function.name).to_string()));

    // build stable unique names
    if use_local_names {
        build_unique_names(names.map(|(id, name)| (id, local_name_from_metadata(&name))))
    } else {
        build_unique_names(names)
    }
}

/// Build unique display names for globals.
fn build_unique_global_names(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    use_local_names: bool,
) -> HashMap<LocalNodeId<Global>, String> {
    // collect global names
    let names = tree
        .iter_nodes::<Global>()
        .map(|(id, global)| (id, strings.get(global.name).to_string()));

    // build stable unique names
    if use_local_names {
        build_unique_names(names.map(|(id, name)| (id, local_name_from_metadata(&name))))
    } else {
        build_unique_names(names)
    }
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
    use_local_names: bool,
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
        if let Some(name) = metadata_name_for_type(tree, strings, type_id, use_local_names) {
            if entry.preferred_metadata_name.is_none() {
                entry.preferred_metadata_name = Some(name.clone());
            }
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
    matches!(
        ty,
        Type::Struct { .. } | Type::Tuple { .. } | Type::Callable { .. }
    )
}

/// Choose an alias name for a candidate group.
fn alias_name_for_candidate(
    tree: &NodeTree,
    candidate: &AliasCandidateGroup,
    next_alias_indices: &mut HashMap<String, usize>,
    alias_names: &HashSet<String>,
) -> String {
    // prefer the stable earliest metadata name when available
    if let Some(name) = &candidate.preferred_metadata_name {
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
        Type::Slice { .. } => "Slice",
        Type::Reference { .. } => "Ref",
        Type::FunctionPointer { .. } => "Function",
        Type::Callable { .. } => "Callable",
        _ => "Type",
    }
}

/// Read the metadata name for a type, if any.
fn metadata_name_for_type(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    ty: LocalNodeId<Type>,
    use_local_names: bool,
) -> Option<String> {
    // read the metadata name when available
    tree.metadata
        .layout
        .display_name(ty)
        .map(|name_id| strings.get(name_id).to_string())
        .map(|name| {
            if use_local_names {
                local_name_from_metadata(&name)
            } else {
                name
            }
        })
}

/// Strip module prefixes from metadata names.
fn local_name_from_metadata(name: &str) -> String {
    let Some((prefix, suffix)) = name.split_once(':') else {
        return name.to_string();
    };

    if prefix.contains('/') {
        return suffix.to_string();
    }

    name.to_string()
}

/// Alias candidates grouped by structural key.
#[derive(Default)]
struct AliasCandidateGroup {
    /// Total uses across matching types.
    total_uses: u32,
    /// Type ids that share the same structural key.
    type_ids: Vec<LocalNodeId<Type>>,
    /// The first metadata name seen for the group in type order.
    preferred_metadata_name: Option<String>,
    /// Metadata names seen for the group.
    metadata_names: HashSet<String>,
}

/// Build a structural key used for alias grouping.
fn type_key_for_alias(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    ty: LocalNodeId<Type>,
) -> String {
    let mut active_types = HashSet::new();
    type_key_for_alias_inner(tree, strings, ty, &mut active_types)
}

/// Build one structural key from a type reference.
fn type_key_for_alias_reference(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    ty: TypeReference,
    active_types: &mut HashSet<LocalNodeId<Type>>,
) -> String {
    match ty {
        TypeReference::Type(ty) => type_key_for_alias_inner(tree, strings, ty, active_types),
        TypeReference::Missing => "<missing>".to_string(),
        TypeReference::Error => "<error>".to_string(),
    }
}

/// Build a structural key used for alias grouping.
fn type_key_for_alias_inner(
    tree: &NodeTree,
    strings: &ImmutableStringPool,
    ty: LocalNodeId<Type>,
    active_types: &mut HashSet<LocalNodeId<Type>>,
) -> String {
    // stop when the traversal hits a recursive cycle
    if !active_types.insert(ty) {
        return format!("recursiveType{}", ty.id);
    }

    // format a stable structural key
    let key = match tree.get(ty) {
        Type::Void => "void".to_string(),
        Type::Boolean => "boolean".to_string(),
        Type::Int { width, is_signed } => {
            // use canonical integer names
            let prefix = if *is_signed { "int" } else { "uint" };
            format!("{prefix}{width}")
        }
        Type::Isize => "isize".to_string(),
        Type::Usize => "usize".to_string(),
        Type::Float { width } => format!("float{width}"),
        Type::TypeDescriptor => "typeDescriptor".to_string(),
        Type::TypeId => "typeId".to_string(),
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
            // append the pointee key first
            let pointee_key = type_key_for_alias_reference(tree, strings, *pointee, active_types);
            result.push_str(&pointee_key);

            // append the reference kind
            result.push_str(", ");
            result.push_str(match kind {
                ReferenceKind::Managed => "managed",
                ReferenceKind::Owned => "owned",
                ReferenceKind::Borrowed => "borrowed",
                ReferenceKind::Raw => "raw",
            });

            // append readonly when required
            if *mutability == Mutability::Immutable {
                result.push_str(", readonly");
            }

            // append address space when explicit
            if !address_space.is_local() {
                let addrspace = format!("space({})", address_space.label());
                result.push_str(", ");
                result.push_str(&addrspace);
            }
            result.push('>');
            result
        }
        Type::Array {
            element, length, ..
        } => {
            // format array keys with element and length
            let element_key = type_key_for_alias_reference(tree, strings, *element, active_types);
            format!("{element_key}[{length}]")
        }
        Type::Slice {
            kind,
            element,
            address_space,
            mutability,
        } => {
            // format slice keys with element type and qualifiers
            let element_key = type_key_for_alias_reference(tree, strings, *element, active_types);
            let mut result = format!("slice<{element_key}");
            match kind {
                ReferenceKind::Managed => {}
                ReferenceKind::Owned => result.push_str(", owned"),
                ReferenceKind::Borrowed => result.push_str(", borrowed"),
                ReferenceKind::Raw => result.push_str(", raw"),
            }
            if *mutability == Mutability::Immutable {
                result.push_str(", readonly");
            }
            if !address_space.is_local() {
                let address_space = format!("space({})", address_space.label());
                result.push_str(", ");
                result.push_str(&address_space);
            }
            result.push('>');
            result
        }
        Type::Tuple { elements, .. } => {
            // join tuple element keys
            let elements = elements
                .iter()
                .map(|element| type_key_for_alias_reference(tree, strings, *element, active_types))
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
                    let field_type =
                        type_key_for_alias_reference(tree, strings, field.ty, active_types);
                    match field.name {
                        Some(name) => format!("{}: {field_type}", strings.get(name)),
                        None => field_type,
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        Type::Newtype { inner, .. } => {
            let inner_key = type_key_for_alias_reference(tree, strings, *inner, active_types);
            format!("newtype<{inner_key}>")
        }
        Type::Vector { element, lanes, .. } => {
            // format vector keys with element and lane count
            let element_key = type_key_for_alias_reference(tree, strings, *element, active_types);
            format!("vector<{element_key}, {lanes}>")
        }
        Type::Tensor {
            element,
            shape,
            layout,
            ..
        } => {
            // format tensor keys with element, shape, and layout
            let element_key = type_key_for_alias_reference(tree, strings, *element, active_types);
            let shape_key = format_shape_key(shape);
            let layout_key = format_tensor_layout_key(layout);
            format!("tensor<{element_key}, {shape_key}, {layout_key}>")
        }
        Type::TensorView {
            kind,
            address_space,
            mutability,
            element,
            shape,
            layout,
            is_nullable,
        } => {
            // format tensor reference keys with reference header, shape, and layout
            let mut result = String::new();
            if *is_nullable {
                result.push_str("tensorView?<");
            } else {
                result.push_str("tensorView<");
            }
            let element_key = type_key_for_alias_reference(tree, strings, *element, active_types);
            result.push_str(&element_key);
            result.push_str(", ");
            result.push_str(match kind {
                ReferenceKind::Managed => "managed",
                ReferenceKind::Owned => "owned",
                ReferenceKind::Borrowed => "borrowed",
                ReferenceKind::Raw => "raw",
            });
            if *mutability == Mutability::Immutable {
                result.push_str(", readonly");
            }
            if !address_space.is_local() {
                let addrspace = format!("space({})", address_space.label());
                result.push_str(", ");
                result.push_str(&addrspace);
            }
            result.push_str(", ");
            result.push_str(&format_shape_key(shape));
            result.push_str(", ");
            result.push_str(&format_tensor_layout_key(layout));
            result.push('>');
            result
        }
        Type::FunctionSignature { parameters, result } => {
            // join parameter and result keys
            let params = parameters
                .iter()
                .map(|param| type_key_for_alias_reference(tree, strings, *param, active_types))
                .collect::<Vec<_>>()
                .join(", ");
            let result = type_key_for_alias_reference(tree, strings, *result, active_types);
            format!("sig({params}) -> {result}")
        }
        Type::FunctionPointer { signature } | Type::Callable { signature } => {
            let TypeReference::Type(signature) = *signature else {
                return type_key_for_alias_reference(tree, strings, *signature, active_types);
            };
            let Some((parameters, result)) = function_signature_parts(tree.get(signature)) else {
                panic!("callable type key expects a function signature");
            };
            let params = parameters
                .iter()
                .map(|param| type_key_for_alias_reference(tree, strings, *param, active_types))
                .collect::<Vec<_>>()
                .join(", ");
            let result = type_key_for_alias_reference(tree, strings, result, active_types);
            match tree.get(ty) {
                Type::FunctionPointer { .. } => format!("({params}) -> {result}"),
                Type::Callable { .. } => format!("({params}) => {result}"),
                _ => unreachable!(),
            }
        }
    };

    active_types.remove(&ty);
    key
}

/// Format a shape key for a tensor or vector.
fn format_shape_key(shape: &[TensorDimension]) -> String {
    // build a stable shape string
    let mut result = String::new();
    result.push('(');
    for (i, dim) in shape.iter().enumerate() {
        if i > 0 {
            result.push_str(", ");
        }
        match dim {
            TensorDimension::Static(value) => {
                result.push_str(&value.to_string());
            }
            TensorDimension::Dynamic => {
                result.push_str("dynamic");
            }
        }
    }
    result.push(')');
    result
}

/// Format a tensor layout key for a tensor or vector.
fn format_tensor_layout_key(layout: &TensorLayout) -> String {
    // encode layout in the structural key
    match layout {
        TensorLayout::RowMajor => "layout(rowMajor)".to_string(),
        TensorLayout::ColumnMajor => "layout(columnMajor)".to_string(),
        TensorLayout::Strided { strides } => {
            let stride_shape = format_shape_key(strides);
            format!("layout(strided({stride_shape}))")
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
        let terminator = tree.get(block.terminator);

        for parameter in &block.parameters {
            record_type_use(tree, parameter.ty, &mut counts);
        }

        match terminator {
            Terminator::Invoke { call, .. }
            | Terminator::InvokeIndirect { call, .. }
            | Terminator::InvokeVirtual {
                declaring_type: _,
                call,
                ..
            }
            | Terminator::InvokeInterface {
                declaring_type: _,
                call,
                ..
            }
            | Terminator::TailCall { call, .. }
            | Terminator::TailCallIndirect { call, .. }
            | Terminator::TailCallVirtual {
                declaring_type: _,
                call,
                ..
            }
            | Terminator::TailCallInterface {
                declaring_type: _,
                call,
                ..
            } => {
                record_type_use(tree, call.signature, &mut counts);
            }
            _ => {}
        }

        match terminator {
            Terminator::InvokeVirtual { declaring_type, .. }
            | Terminator::InvokeInterface { declaring_type, .. }
            | Terminator::TailCallVirtual { declaring_type, .. }
            | Terminator::TailCallInterface { declaring_type, .. } => {
                record_type_use(tree, *declaring_type, &mut counts);
            }
            _ => {}
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
            Instruction::Call { call, .. } => {
                record_type_use(tree, call.signature, &mut counts);
            }
            Instruction::CallVirtual {
                declaring_type,
                call,
                ..
            } => {
                record_type_use(tree, *declaring_type, &mut counts);
                record_type_use(tree, call.signature, &mut counts);
            }
            Instruction::CallInterface {
                declaring_type,
                call,
                ..
            } => {
                record_type_use(tree, *declaring_type, &mut counts);
                record_type_use(tree, call.signature, &mut counts);
            }
            Instruction::CallIndirect { call, .. } => {
                record_type_use(tree, call.signature, &mut counts);
            }
            Instruction::New {
                layout,
                result_type,
                ..
            } => {
                record_type_use(tree, *layout, &mut counts);
                record_type_use(tree, *result_type, &mut counts);
            }
            Instruction::NewSlice {
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
            | Instruction::Store { .. }
            | Instruction::FieldGet { .. }
            | Instruction::FieldSet { .. }
            | Instruction::ElementGet { .. }
            | Instruction::ElementSet { .. }
            | Instruction::RawFree { .. }
            | Instruction::Dispose { .. }
            | Instruction::AsyncDispose { .. }
            | Instruction::Pin { .. }
            | Instruction::Unpin { .. }
            | Instruction::Drop { .. }
            | Instruction::Assume { .. }
            | Instruction::Intrinsic { .. } => {}
            _ => {}
        }
    }

    // return usage counts
    counts
}

/// Record usage of a type and its nested types.
fn record_type_use(
    tree: &NodeTree,
    ty: TypeReference,
    counts: &mut HashMap<LocalNodeId<Type>, u32>,
) {
    let TypeReference::Type(ty) = ty else {
        return;
    };

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
            let TypeReference::Type(pointee) = *pointee else {
                return;
            };
            record_type_use_inner(tree, pointee, counts, visited);
        }
        Type::Array { element, .. } | Type::Slice { element, .. } => {
            let TypeReference::Type(element) = *element else {
                return;
            };
            record_type_use_inner(tree, element, counts, visited);
        }
        Type::Tuple { elements, .. } => {
            // record tuple element types
            for element_id in elements {
                let TypeReference::Type(element_id) = *element_id else {
                    continue;
                };
                record_type_use_inner(tree, element_id, counts, visited);
            }
        }
        Type::Struct { fields, .. } => {
            // record struct field types
            for field_id in fields {
                let field = tree.get(*field_id);
                let TypeReference::Type(field_ty) = field.ty else {
                    continue;
                };
                record_type_use_inner(tree, field_ty, counts, visited);
            }
        }
        Type::Newtype { inner, .. } => {
            let TypeReference::Type(inner) = *inner else {
                return;
            };
            record_type_use_inner(tree, inner, counts, visited);
        }
        Type::Vector { element, .. } => {
            let TypeReference::Type(element) = *element else {
                return;
            };
            record_type_use_inner(tree, element, counts, visited);
        }
        Type::Tensor { element, .. } => {
            let TypeReference::Type(element) = *element else {
                return;
            };
            record_type_use_inner(tree, element, counts, visited);
        }
        Type::TensorView { element, .. } => {
            let TypeReference::Type(element) = *element else {
                return;
            };
            record_type_use_inner(tree, element, counts, visited);
        }
        Type::FunctionSignature { parameters, result } => {
            // record function signature types
            for parameter_id in parameters {
                let TypeReference::Type(parameter_id) = *parameter_id else {
                    continue;
                };
                record_type_use_inner(tree, parameter_id, counts, visited);
            }
            let TypeReference::Type(result) = *result else {
                return;
            };
            record_type_use_inner(tree, result, counts, visited);
        }
        Type::FunctionPointer { signature } | Type::Callable { signature } => {
            let TypeReference::Type(signature) = *signature else {
                return;
            };
            record_type_use_inner(tree, signature, counts, visited);
        }
        Type::Void
        | Type::Boolean
        | Type::Int { .. }
        | Type::Isize
        | Type::Usize
        | Type::Float { .. }
        | Type::TypeDescriptor
        | Type::TypeId => {}
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
    let document = destack_fir::format!(context, [FormatAllItems])
        .unwrap_or_else(|error| panic!("failed to format MIR: {error:?}"));

    // print the formatted document
    let printed = document
        .print()
        .unwrap_or_else(|error| panic!("failed to print MIR: {error:?}"));

    printed.as_str().to_string()
}

/// One normalized comment line.
#[derive(Debug, Clone)]
struct CommentLine {
    /// The number of blank lines before this comment.
    blank_lines_before: usize,
    /// The exact comment text.
    text: String,
}

/// One normalized comment block.
struct CommentBlock {
    /// The comment lines.
    lines: Vec<CommentLine>,
    /// The blank lines after the final comment.
    trailing_blank_lines: usize,
}

/// Return one node's leading trivia byte bounds.
fn leading_trivia_bounds<T>(tree: &NodeTree, id: LocalNodeId<T>) -> Option<(u32, u32)>
where
    T: Node,
{
    let span = tree.leading_comment_span(id)?;
    Some((span.start, span.end))
}

/// Return the source start used to order one top level item.
fn top_level_item_start<T>(tree: &NodeTree, id: LocalNodeId<T>) -> u32
where
    T: Node,
{
    // prefer leading trivia so leading comments stay attached to the item order
    if let Some(span) = tree.leading_comment_span(id) {
        return span.start;
    }

    // fall back to the node enclosing span
    tree.get_span(id).map(|span| span.start).unwrap_or(u32::MAX)
}

/// Collect normalized comments between byte offsets.
fn collect_comments_between(tree: &NodeTree, start: u32, end: u32) -> CommentBlock {
    let mut comments = Vec::new();
    let mut newline_count = 0usize;

    for token in tree.tokens() {
        if token.span.end <= start {
            continue;
        }

        if token.span.start >= end {
            break;
        }

        match token.ty {
            TokenType::Comment => {
                comments.push(CommentLine {
                    blank_lines_before: newline_count.saturating_sub(1),
                    text: tree.source_text(token.span).to_string(),
                });
                newline_count = 0;
            }
            TokenType::Newline => {
                newline_count += 1;
            }
            TokenType::Whitespace => {}
            _ => {}
        }
    }

    CommentBlock {
        lines: comments,
        trailing_blank_lines: newline_count.saturating_sub(1),
    }
}

/// Write one normalized blank-line gap.
fn write_blank_line_gap<'a>(count: usize, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
    for _ in 0..count {
        write!(f, [empty_line()])?;
    }

    Ok(())
}

/// Write one normalized comment block.
fn write_comment_block<'a>(
    block: &CommentBlock,
    first_blank_lines_to_skip: usize,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    if block.lines.is_empty() {
        return Ok(false);
    }

    for (index, comment) in block.lines.iter().enumerate() {
        let blank_lines_before = if index == 0 {
            comment
                .blank_lines_before
                .saturating_sub(first_blank_lines_to_skip)
        } else {
            comment.blank_lines_before
        };

        if index > 0 {
            if blank_lines_before > 0 {
                write_blank_line_gap(blank_lines_before, f)?;
            } else {
                write!(f, [hard_line_break()])?;
            }
        } else if blank_lines_before > 0 {
            write_blank_line_gap(blank_lines_before, f)?;
        }

        write!(f, [text(&comment.text)])?;
    }

    if block.trailing_blank_lines > 0 {
        write_blank_line_gap(block.trailing_blank_lines, f)?;
    } else {
        write!(f, [hard_line_break()])?;
    }

    Ok(true)
}

/// Write comment lines after one canonical separator.
fn write_comments_after_separator<'a>(
    tree: &NodeTree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let comments = collect_comments_between(tree, start, end);
    write_comment_block(&comments, 1, f)
}

/// Write comment lines between byte offsets as standalone lines.
pub(crate) fn write_comments_before<'a>(
    tree: &NodeTree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let comments = collect_comments_between(tree, start, end);
    write_comment_block(&comments, 0, f)
}

/// Write comments after one anchor, keeping inline comments inline.
pub(crate) fn write_inline_comment_after<'a>(
    tree: &NodeTree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let inline_comment = tree.inline_comment_between(start, end);

    if let Some(comment) = inline_comment {
        write!(f, [space(), text(&comment.text)])?;
        return Ok(true);
    }

    Ok(false)
}

/// Write comments after one anchor, keeping inline comments inline.
pub(crate) fn write_comments_after<'a>(
    tree: &NodeTree,
    start: u32,
    end: u32,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool> {
    let mut block_start = None;
    let mut saw_newline = false;
    let wrote_inline_comment = write_inline_comment_after(tree, start, end, f)?;

    for token in tree.tokens() {
        if token.span.end <= start {
            continue;
        }

        if token.span.start >= end {
            break;
        }

        if token.ty == TokenType::Newline && !saw_newline {
            saw_newline = true;
            block_start = Some(token.span.start);
        }
    }

    let mut wrote_comment = wrote_inline_comment;

    if let Some(block_start) = block_start
        && write_comments_before(tree, block_start, end, f)?
    {
        wrote_comment = true;
    }

    Ok(wrote_comment)
}

/// Write leading comments for one node.
pub(crate) fn write_node_leading_comments<'a, T>(
    tree: &NodeTree,
    id: LocalNodeId<T>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some((start, end)) = leading_trivia_bounds(tree, id) else {
        return Ok(false);
    };

    write_comments_before(tree, start, end, f)
}

/// Write leading comments after one canonical sibling separator.
pub(crate) fn write_node_leading_comments_after_separator<'a, T>(
    tree: &NodeTree,
    id: LocalNodeId<T>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some((start, end)) = leading_trivia_bounds(tree, id) else {
        return Ok(false);
    };

    write_comments_after_separator(tree, start, end, f)
}

/// Write trailing comments after the final top level item.
fn write_top_level_comments_after<'a, T>(
    tree: &NodeTree,
    id: LocalNodeId<T>,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<bool>
where
    T: Node,
{
    let Some(span) = tree.get_span(id) else {
        return Ok(false);
    };

    write!(f, [hard_line_break()])?;

    let scope_end = tree
        .tokens()
        .last()
        .map(|token| token.span.end)
        .unwrap_or(span.end);
    write_comments_after_separator(tree, span.end, scope_end, f)
}

/// One synthetic type alias entry.
struct SyntheticAliasEntry {
    /// The aliased type id.
    type_id: LocalNodeId<Type>,
    /// The generated alias name.
    name: String,
}

/// Order alias entries so referenced aliases appear first.
fn order_alias_entries(tree: &NodeTree, entries: &[SyntheticAliasEntry]) -> Vec<usize> {
    if entries.len() <= 1 {
        return (0..entries.len()).collect();
    }

    let alias_types: HashSet<_> = entries.iter().map(|entry| entry.type_id).collect();
    let index_by_type: HashMap<_, _> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.type_id, index))
        .collect();

    let mut edges = vec![Vec::new(); entries.len()];
    let mut in_degree = vec![0usize; entries.len()];

    for (index, entry) in entries.iter().enumerate() {
        let dependencies = collect_alias_dependencies(tree, entry.type_id, &alias_types);
        for dependency in dependencies {
            let Some(&dependency_index) = index_by_type.get(&dependency) else {
                continue;
            };
            if dependency_index == index {
                continue;
            }
            edges[dependency_index].push(index);
            in_degree[index] += 1;
        }
    }

    let mut available = BTreeSet::new();
    for (index, degree) in in_degree.iter().enumerate() {
        if *degree == 0 {
            available.insert(index);
        }
    }

    let mut ordered = Vec::with_capacity(entries.len());
    while let Some(&index) = available.iter().next() {
        available.remove(&index);
        ordered.push(index);
        for dependent in &edges[index] {
            let degree = &mut in_degree[*dependent];
            *degree = degree.saturating_sub(1);
            if *degree == 0 {
                available.insert(*dependent);
            }
        }
    }

    if ordered.len() == entries.len() {
        return ordered;
    }

    let mut seen = HashSet::new();
    for index in &ordered {
        seen.insert(*index);
    }
    for index in 0..entries.len() {
        if !seen.contains(&index) {
            ordered.push(index);
        }
    }

    ordered
}

/// Collect alias dependencies for a type id.
fn collect_alias_dependencies(
    tree: &NodeTree,
    root: LocalNodeId<Type>,
    alias_types: &HashSet<LocalNodeId<Type>>,
) -> HashSet<LocalNodeId<Type>> {
    let mut dependencies = HashSet::new();
    let mut stack = vec![root];
    let mut visited = HashSet::new();

    while let Some(type_id) = stack.pop() {
        if !visited.insert(type_id) {
            continue;
        }

        let ty = tree.get(type_id);
        match ty {
            Type::Reference { pointee, .. } => {
                record_dependency(*pointee, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::Array { element, .. } | Type::Slice { element, .. } => {
                record_dependency(*element, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::Tuple { elements, .. } => {
                for element in elements {
                    record_dependency(*element, root, alias_types, &mut dependencies, &mut stack);
                }
            }
            Type::Struct { fields, .. } => {
                for field_id in fields {
                    let field = tree.get(*field_id);
                    record_dependency(field.ty, root, alias_types, &mut dependencies, &mut stack);
                }
            }
            Type::Newtype { inner, .. } => {
                record_dependency(*inner, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::Vector { element, .. } => {
                record_dependency(*element, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::Tensor { element, .. } => {
                record_dependency(*element, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::TensorView { element, .. } => {
                record_dependency(*element, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::FunctionSignature { parameters, result } => {
                for parameter in parameters {
                    record_dependency(*parameter, root, alias_types, &mut dependencies, &mut stack);
                }
                record_dependency(*result, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::FunctionPointer { signature } | Type::Callable { signature } => {
                record_dependency(*signature, root, alias_types, &mut dependencies, &mut stack);
            }
            Type::Void
            | Type::Boolean
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId => {}
        }
    }

    dependencies
}

/// Record a dependency and continue traversal.
fn record_dependency(
    type_id: TypeReference,
    root: LocalNodeId<Type>,
    alias_types: &HashSet<LocalNodeId<Type>>,
    dependencies: &mut HashSet<LocalNodeId<Type>>,
    stack: &mut Vec<LocalNodeId<Type>>,
) {
    let TypeReference::Type(type_id) = type_id else {
        return;
    };

    if type_id != root && alias_types.contains(&type_id) {
        dependencies.insert(type_id);
    }
    stack.push(type_id);
}

/// Helper to format all module items.
struct FormatAllItems;

impl<'a> Format<MirFormatContext<'a>> for FormatAllItems {
    fn format(&self, f: &mut MirFormatter<'a, '_>) -> FormatResult<()> {
        let tree = f.context().tree;
        let mut has_output = false;

        // synthetic aliases
        let mut alias_entries = Vec::new();
        for (type_id, name) in &f.context().synthetic_aliases {
            alias_entries.push(SyntheticAliasEntry {
                type_id: *type_id,
                name: name.clone(),
            });
        }

        if !f.context().synthetic_aliases.is_empty() {
            let alias_order = order_alias_entries(tree, &alias_entries);

            for index in alias_order {
                let entry = &alias_entries[index];
                if has_output {
                    write!(f, [hard_line_break()])?;
                }

                let ty = f.context().tree.get(entry.type_id);
                format_type_declaration(&entry.name, &[], None, entry.type_id, ty, f)?;
            }

            has_output = true;
        }

        // explicit top level items
        let mut item_ids = Vec::new();

        for (id, _) in tree.iter_nodes::<TypeAlias>() {
            let start = top_level_item_start(tree, id);
            item_ids.push((start, id.id, NodeType::TypeAlias));
        }

        for (id, _) in tree.iter_nodes::<Global>() {
            let start = top_level_item_start(tree, id);
            item_ids.push((start, id.id, NodeType::Global));
        }

        for (id, _) in tree.iter_nodes::<Function>() {
            let start = top_level_item_start(tree, id);
            item_ids.push((start, id.id, NodeType::Function));
        }

        item_ids.sort_by_key(|(start, node_id, _)| (*start, *node_id));

        for (index, (_, node_id, node_type)) in item_ids.iter().enumerate() {
            let next_boundary = item_ids.get(index + 1).map(|(start, _, _)| *start);

            match node_type {
                NodeType::TypeAlias => format_top_level_item(
                    tree,
                    LocalNodeId::<TypeAlias>::new(*node_id),
                    next_boundary,
                    has_output,
                    f,
                )?,
                NodeType::Global => format_top_level_item(
                    tree,
                    LocalNodeId::<Global>::new(*node_id),
                    next_boundary,
                    has_output,
                    f,
                )?,
                NodeType::Function => format_top_level_item(
                    tree,
                    LocalNodeId::<Function>::new(*node_id),
                    next_boundary,
                    has_output,
                    f,
                )?,
                _ => unreachable!(),
            }

            has_output = true;
        }

        Ok(())
    }
}

/// Format one explicit top level item.
fn format_top_level_item<'a, T>(
    tree: &NodeTree,
    id: LocalNodeId<T>,
    next_boundary: Option<u32>,
    has_output: bool,
    f: &mut MirFormatter<'a, '_>,
) -> FormatResult<()>
where
    NodeTree: NodeTreeImpl<T>,
    T: FormatMirNode<'a, T> + Node + Clone,
{
    // item spacing
    if has_output {
        write!(f, [empty_line()])?;
    }

    // leading comments
    if has_output {
        write_node_leading_comments_after_separator(tree, id, f)?;
    } else {
        write_node_leading_comments(tree, id, f)?;
    }

    // item and trailing comments
    write!(f, [id])?;

    if let Some(next_boundary) = next_boundary {
        if let Some(span) = tree.get_span(id) {
            write_inline_comment_after(tree, span.end, next_boundary, f)?;
        }
    } else {
        write_top_level_comments_after(tree, id, f)?;
    }

    Ok(())
}
