use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use destack_base::{ImmutableStringPool, StringId};
use destack_dir::{
    AccessorKind, Argument, Asynchrony, BindingAnchor, BindingKind, BindingModifier,
    BindingOperator, Declaration, Dumper, DumperOptions, DynamicKey, Export, ExportKind,
    Expression, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature, Generics, GlobalNodeIdAny, GlobalSymbolId, GlobalTypeId, IntrinsicType,
    LocalSymbolId, LocalTypeId, Mutability, NodeType, NodeVisitor, Parameter, Path, PrimitiveType,
    Property, ScalarLiteral, StaticArgument, StaticExpression, StaticKey, StaticProperty, Symbol,
    SymbolKey, SymbolKind, SymbolSpace, SymbolTable, SymbolType, Timing, Type, TypeBinaryOperator,
    TypeElement, TypeField, TypeIndexSignature, TypeLiteral, TypeMappedModifiers,
    TypeMappedParameter, TypeModifier, TypePredicateSubject, TypeTable, TypeUnaryOperator,
    VarianceBound, WhereClause,
};
use destack_source::ModuleId;
use destack_workspace::{
    ModuleContent, ModuleGraphKey, ModuleSignature, ModuleSignatureAugmentation,
    ModuleSignatureBinding, ModuleSignatureExport, ModuleSignatureKey, ProfileId,
};
use indexmap::IndexMap;
use rustc_hash::FxHasher;

use crate::{AnalyzeResult, Compiler};

impl Compiler {
    /// Update and store the module signature for a profile.
    pub(crate) fn update_module_signature(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> AnalyzeResult<()> {
        // build the current signature
        let signature = self.build_module_signature(module_id, profile_id)?;

        // load the previous signature if any
        let key = ModuleSignatureKey::new(module_id, profile_id);
        let previous = self
            .program
            .index
            .module_signatures
            .get(&key)
            .map(|entry| entry.value().clone());

        // update stored signature
        self.program
            .index
            .module_signatures
            .insert(key, signature.clone());

        // invalidate dependents only when the signature hash changed
        if let Some(previous) = previous
            && previous.hash != signature.hash
        {
            self.invalidate_module_dependents(module_id, profile_id);
        }

        Ok(())
    }

    /// Invalidate dependents when a module signature changes.
    fn invalidate_module_dependents(&self, module_id: ModuleId, profile_id: ProfileId) {
        // resolve module graph for the profile
        let key = ModuleGraphKey::new(profile_id);
        let Some(graph) = self.program.index.module_graphs.get(&key) else {
            return;
        };

        // snapshot dependents before dropping the graph guard
        let dependents = graph.dependents_for(module_id);
        drop(graph);

        // drop profile data for dependents
        for dependent in dependents {
            self.invalidate_module_profile_data(dependent, profile_id);
        }
    }

    /// Invalidate profile data for a module.
    fn invalidate_module_profile_data(&self, module_id: ModuleId, profile_id: ProfileId) {
        // drop profile dirs and dependent artifacts
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        match &mut module.content {
            ModuleContent::Code(code) => {
                code.dirs.retain(|dir| dir.profile_id != Some(profile_id));
                code.comptimes
                    .retain(|entry| entry.profile_id != profile_id);
                code.mirs.clear();
            }
            ModuleContent::Data { dirs, .. } => {
                dirs.retain(|dir| dir.profile_id != Some(profile_id));
            }
            ModuleContent::Text { dirs, .. } => {
                dirs.retain(|dir| dir.profile_id != Some(profile_id));
            }
            ModuleContent::Binary { dirs, .. } => {
                dirs.retain(|dir| dir.profile_id != Some(profile_id));
            }
            ModuleContent::Unloaded => {}
        }
    }

    /// Build a module signature for the given profile.
    fn build_module_signature(
        &self,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> AnalyzeResult<ModuleSignature> {
        // set up signature hasher
        let strings = self.signature_strings();
        let mut signature_hasher = SignatureHasher::new(self, profile_id, strings);

        // load module and profile data
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let profile = self.program.profile(profile_id);
        let profile_version = profile.version;

        // return an empty signature when the profile dir is missing
        let Some(dir) = module.dir_maybe(profile_id) else {
            return Ok(ModuleSignature::new(
                module_id,
                profile_id,
                module.version,
                profile_version,
                0,
                Vec::new(),
                None,
                IndexMap::new(),
                Vec::new(),
                Vec::new(),
            ));
        };

        // collect export signatures
        let (exports, export_assignment_target, export_assignment_hash) =
            self.collect_export_signatures(module_id, dir, &mut signature_hasher);

        // collect global augmentation signatures
        let (global_augmentations, augmentation_signatures) =
            self.collect_global_augmentation_signatures(module_id, dir, &mut signature_hasher);

        // collect module binding signatures
        let module_bindings =
            self.collect_module_binding_signatures(module_id, dir, &mut signature_hasher);

        // build symbol signature map
        let mut symbol_signatures = IndexMap::new();
        for export in &exports {
            if let (Some(target), Some(signature_hash)) = (export.target, export.signature_hash) {
                symbol_signatures.entry(target).or_insert(signature_hash);
            }
        }
        if let (Some(target), Some(signature_hash)) =
            (export_assignment_target, export_assignment_hash)
        {
            symbol_signatures.entry(target).or_insert(signature_hash);
        }
        for (symbol_id, signature_hash) in augmentation_signatures {
            symbol_signatures.entry(symbol_id).or_insert(signature_hash);
        }

        // hash signature components
        let hash = signature_hasher.hash_module_signature(
            &exports,
            export_assignment_hash,
            &global_augmentations,
            &module_bindings,
        );

        Ok(ModuleSignature::new(
            module_id,
            profile_id,
            module.version,
            profile_version,
            hash,
            exports,
            export_assignment_hash,
            symbol_signatures,
            global_augmentations,
            module_bindings,
        ))
    }

    /// Collect module export signatures for a profile.
    fn collect_export_signatures(
        &self,
        module_id: ModuleId,
        dir: &destack_workspace::ModuleDir,
        signature_hasher: &mut SignatureHasher<'_>,
    ) -> (
        Vec<ModuleSignatureExport>,
        Option<GlobalSymbolId>,
        Option<u64>,
    ) {
        // lock local symbol and type tables
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // collect and sort exports for stable ordering
        let exports = dir.exported_symbols.read();
        let mut entries: Vec<Export> = exports.values().cloned().collect();
        entries.sort_by_key(|export| signature_hasher.export_sort_key(export));
        drop(exports);

        // map exports to signature data
        let mut signatures = Vec::with_capacity(entries.len());
        for export in entries {
            let target = export.target.resolved();
            let (symbol_kind, symbol_type, signature_hash) = if let Some(target) = target {
                signature_hasher.signature_for_symbol(target, module_id, &symbols, &types)
            } else {
                (None, None, None)
            };

            signatures.push(ModuleSignatureExport {
                key: export.key,
                space: export.space,
                kind: export.kind,
                target,
                symbol_kind,
                symbol_type,
                signature_hash,
            });
        }

        // resolve export assignment signature from the export assignment symbol
        let export_assignment_target = symbols
            .get_symbol(dir.export_assignment_symbol)
            .target_symbol;
        let export_assignment_hash =
            export_assignment_target.map(|symbol| signature_hasher.symbol_signature_hash(symbol));

        (signatures, export_assignment_target, export_assignment_hash)
    }

    /// Collect signatures for global augmentation symbols.
    fn collect_global_augmentation_signatures(
        &self,
        module_id: ModuleId,
        dir: &destack_workspace::ModuleDir,
        signature_hasher: &mut SignatureHasher<'_>,
    ) -> (
        Vec<ModuleSignatureAugmentation>,
        IndexMap<GlobalSymbolId, u64>,
    ) {
        // lock local symbol and type tables
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // gather augmentation symbols
        let mut augmentations = Vec::new();
        let mut augmentation_signatures = IndexMap::new();
        for symbol_id in (0..symbols.symbol_count()).map(LocalSymbolId::new) {
            let symbol = symbols.get_symbol(symbol_id);
            if !symbol.origin.is_global_augmentation() {
                continue;
            }

            // compute signature hash for the symbol
            let global_symbol_id = symbol_id.into_global(module_id);
            let signature_hash =
                signature_hasher.symbol_signature_hash_in_tables(global_symbol_id, symbol, &types);

            augmentations.push(ModuleSignatureAugmentation {
                key: symbol.key,
                space: symbol.space,
                symbol_kind: symbol.kind,
                symbol_type: symbol.ty,
                signature_hash: Some(signature_hash),
            });
            augmentation_signatures
                .entry(global_symbol_id)
                .or_insert(signature_hash);
        }

        // sort for stable ordering
        augmentations.sort_by(|left, right| {
            (
                signature_hasher.symbol_space_rank(left.space),
                signature_hasher.option_static_key_hash(left.key),
                left.symbol_kind,
                left.symbol_type,
            )
                .cmp(&(
                    signature_hasher.symbol_space_rank(right.space),
                    signature_hasher.option_static_key_hash(right.key),
                    right.symbol_kind,
                    right.symbol_type,
                ))
        });

        (augmentations, augmentation_signatures)
    }

    /// Collect signatures for module bindings.
    fn collect_module_binding_signatures(
        &self,
        module_id: ModuleId,
        dir: &destack_workspace::ModuleDir,
        signature_hasher: &mut SignatureHasher<'_>,
    ) -> Vec<ModuleSignatureBinding> {
        // lock module binding data
        let bindings = dir.module_bindings.read();
        let binding_exports = dir.module_binding_exports.read();
        let symbols = dir.symbols.read();
        let types = dir.types.read();

        // build signature data per binding
        let mut signatures = Vec::with_capacity(bindings.len());
        for binding in bindings.iter() {
            let exports = binding_exports
                .get(&binding.declaration.into_any())
                .map(|exports| {
                    // collect binding export entries in stable order
                    let mut entries: Vec<Export> = exports.exports.values().cloned().collect();
                    entries.sort_by_key(|export| signature_hasher.export_sort_key(export));

                    // build signature entries for exports
                    let mut signatures = Vec::with_capacity(entries.len());
                    for export in entries {
                        let target = export.target.resolved();
                        let (symbol_kind, symbol_type, signature_hash) =
                            if let Some(target) = target {
                                signature_hasher
                                    .signature_for_symbol(target, module_id, &symbols, &types)
                            } else {
                                (None, None, None)
                            };

                        signatures.push(ModuleSignatureExport {
                            key: export.key,
                            space: export.space,
                            kind: export.kind,
                            target,
                            symbol_kind,
                            symbol_type,
                            signature_hash,
                        });
                    }

                    signatures
                })
                .unwrap_or_default();

            let export_assignment_target = symbols
                .get_symbol(binding.export_assignment_symbol)
                .target_symbol;
            let export_assignment_hash = export_assignment_target
                .map(|symbol| signature_hasher.symbol_signature_hash(symbol));

            signatures.push(ModuleSignatureBinding {
                specifier: binding.specifier,
                exports,
                export_assignment_hash,
            });
        }

        // sort bindings by specifier for stable ordering
        signatures.sort_by(|left, right| left.specifier.cmp(&right.specifier));

        signatures
    }
}

/// Hasher for module signature data.
pub(super) struct SignatureHasher<'a> {
    /// The compiler context to resolve symbols and types.
    compiler: &'a Compiler,
    /// The profile id used for signature data.
    profile_id: ProfileId,
    /// Immutable strings for stable hashing.
    strings: Arc<ImmutableStringPool>,
    /// Cached symbol signature hashes.
    symbol_hashes: HashMap<GlobalSymbolId, u64>,
    /// Cached type signature hashes.
    type_hashes: HashMap<GlobalTypeId, u64>,
    /// Cached node signature hashes.
    node_hashes: HashMap<GlobalNodeIdAny, u64>,
    /// Symbols currently being hashed.
    symbol_in_progress: HashSet<GlobalSymbolId>,
    /// Types currently being hashed.
    type_in_progress: HashSet<GlobalTypeId>,
}

impl<'a> SignatureHasher<'a> {
    /// Create a new signature hasher.
    pub(super) fn new(
        compiler: &'a Compiler,
        profile_id: ProfileId,
        strings: Arc<ImmutableStringPool>,
    ) -> Self {
        // set up signature caches
        Self {
            compiler,
            profile_id,
            strings,
            symbol_hashes: HashMap::new(),
            type_hashes: HashMap::new(),
            node_hashes: HashMap::new(),
            symbol_in_progress: HashSet::new(),
            type_in_progress: HashSet::new(),
        }
    }

    /// Build a stable sort key for export entries.
    pub(super) fn export_sort_key(&self, export: &Export) -> (u8, u64, u8) {
        // build the sort key
        (
            self.symbol_space_rank(export.space),
            self.static_key_hash(export.key),
            self.export_kind_rank(export.kind),
        )
    }

    /// Map symbol space to a stable ordering rank.
    pub(super) fn symbol_space_rank(&self, space: SymbolSpace) -> u8 {
        symbol_space_rank(space)
    }

    /// Map export kind to a stable ordering rank.
    fn export_kind_rank(&self, kind: ExportKind) -> u8 {
        export_kind_rank(kind)
    }

    /// Hash a static key into a stable fingerprint.
    fn static_key_hash(&self, key: StaticKey) -> u64 {
        self.hash_static_key(&key)
    }

    /// Hash an optional static key into a stable fingerprint.
    pub(super) fn option_static_key_hash(&self, key: Option<StaticKey>) -> Option<u64> {
        key.as_ref().map(|key| self.hash_static_key(key))
    }

    /// Hash module signature components into a stable signature hash.
    pub(super) fn hash_module_signature(
        &self,
        exports: &[ModuleSignatureExport],
        export_assignment_hash: Option<u64>,
        global_augmentations: &[ModuleSignatureAugmentation],
        module_bindings: &[ModuleSignatureBinding],
    ) -> u64 {
        let mut hasher = FxHasher::default();

        // hash exports
        exports.len().hash(&mut hasher);
        for export in exports {
            self.hash_export_signature(&mut hasher, export);
        }

        // hash export assignment
        export_assignment_hash.hash(&mut hasher);

        // hash global augmentations
        global_augmentations.len().hash(&mut hasher);
        for augmentation in global_augmentations {
            augmentation
                .key
                .as_ref()
                .map(|key| self.hash_static_key(key))
                .hash(&mut hasher);
            augmentation.space.hash(&mut hasher);
            augmentation.symbol_kind.hash(&mut hasher);
            augmentation.symbol_type.hash(&mut hasher);
            augmentation.signature_hash.hash(&mut hasher);
        }

        // hash module bindings
        module_bindings.len().hash(&mut hasher);
        for binding in module_bindings {
            self.hash_string_id(binding.specifier).hash(&mut hasher);
            binding.export_assignment_hash.hash(&mut hasher);
            binding.exports.len().hash(&mut hasher);
            for export in &binding.exports {
                self.hash_export_signature(&mut hasher, export);
            }
        }

        hasher.finish()
    }

    /// Hash a single export signature into the hasher.
    fn hash_export_signature(&self, hasher: &mut FxHasher, export: &ModuleSignatureExport) {
        self.hash_static_key(&export.key).hash(hasher);
        export.space.hash(hasher);
        export_kind_rank(export.kind).hash(hasher);
        export.target.hash(hasher);
        export.symbol_kind.hash(hasher);
        export.symbol_type.hash(hasher);
        export.signature_hash.hash(hasher);
    }

    /// Compute signature fields for a target symbol.
    pub(super) fn signature_for_symbol(
        &mut self,
        symbol_id: GlobalSymbolId,
        module_id: ModuleId,
        symbols: &SymbolTable,
        types: &TypeTable,
    ) -> (Option<SymbolKind>, Option<SymbolType>, Option<u64>) {
        // route to local tables when possible
        if symbol_id.module_id == module_id {
            let symbol = symbols.get_symbol(symbol_id.into_local());
            let signature_hash = self.symbol_signature_hash_in_tables(symbol_id, symbol, types);
            return (Some(symbol.kind), Some(symbol.ty), Some(signature_hash));
        }

        // fall back to remote lookup
        let (symbol_kind, symbol_type) = self.symbol_info_for_symbol(symbol_id);
        let signature_hash = self.symbol_signature_hash(symbol_id);
        (symbol_kind, symbol_type, Some(signature_hash))
    }

    /// Resolve symbol kind and type for a target symbol.
    fn symbol_info_for_symbol(
        &self,
        symbol_id: GlobalSymbolId,
    ) -> (Option<SymbolKind>, Option<SymbolType>) {
        // load the owning module and its symbols
        let module = self.compiler.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(self.profile_id) else {
            return (None, None);
        };
        let symbols = dir.symbols.read();
        let symbol = symbols.get_symbol(symbol_id.into_local());
        (Some(symbol.kind), Some(symbol.ty))
    }

    /// Compute a signature hash for a symbol using the owning module tables.
    pub(super) fn symbol_signature_hash(&mut self, symbol_id: GlobalSymbolId) -> u64 {
        // reuse cached hashes when available
        if let Some(hash) = self.symbol_hashes.get(&symbol_id) {
            return *hash;
        }

        // prefer precomputed module signature hashes
        if let Some(hash) = self.lookup_module_symbol_signature(symbol_id) {
            self.symbol_hashes.insert(symbol_id, hash);
            return hash;
        }

        // short circuit on recursion
        if !self.symbol_in_progress.insert(symbol_id) {
            return self.fallback_symbol_hash(symbol_id);
        }

        // load the owning module and its type tables
        let module = self.compiler.program.modules.get(symbol_id.module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(self.profile_id) else {
            let hash = self.fallback_symbol_hash(symbol_id);
            self.symbol_in_progress.remove(&symbol_id);
            self.symbol_hashes.insert(symbol_id, hash);
            return hash;
        };
        let symbols = dir.symbols.read();
        let types = dir.types.read();
        let symbol = symbols.get_symbol(symbol_id.into_local());

        // compute and cache the hash
        let hash = self.symbol_signature_hash_in_tables(symbol_id, symbol, &types);
        self.symbol_in_progress.remove(&symbol_id);
        self.symbol_hashes.insert(symbol_id, hash);

        hash
    }

    /// Look up a symbol signature hash from module signatures.
    fn lookup_module_symbol_signature(&self, symbol_id: GlobalSymbolId) -> Option<u64> {
        let key = ModuleSignatureKey::new(symbol_id.module_id, self.profile_id);
        self.compiler
            .program
            .index
            .module_signatures
            .get(&key)
            .and_then(|entry| entry.value().symbol_signatures.get(&symbol_id).copied())
    }

    /// Compute a signature hash for a symbol using local tables.
    pub(super) fn symbol_signature_hash_in_tables(
        &mut self,
        symbol_id: GlobalSymbolId,
        symbol: &Symbol,
        types: &TypeTable,
    ) -> u64 {
        // resolve the signature type id for the symbol
        let signature_type_id = self.signature_type_id_for_symbol(symbol_id, symbol, types);

        // hash symbol identity and signature type
        let mut hasher = FxHasher::default();
        symbol_id.hash(&mut hasher);
        signature_type_id
            .map(|type_id| self.hash_type_id_in_tables(symbol_id.module_id, type_id, types))
            .unwrap_or_else(|| self.fallback_symbol_hash(symbol_id))
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Resolve the signature type id for a symbol.
    fn signature_type_id_for_symbol(
        &self,
        symbol_id: GlobalSymbolId,
        symbol: &Symbol,
        types: &TypeTable,
    ) -> Option<LocalTypeId> {
        // prefer signature types from the primary declaration
        if let Some(declaration_id) = symbol.primary_declaration {
            if let Some(signature_type) = types.get_signature_type_for_node(declaration_id) {
                return Some(signature_type);
            }
            if let Some(ty) = types.get_declared_or_inferred_type_id(declaration_id) {
                return Some(ty);
            }
        }

        // fall back to instance or value types for symbols
        match symbol.space {
            SymbolSpace::Type | SymbolSpace::TypeValue => types.get_instance_type_id(symbol_id),
            SymbolSpace::Value => types.get_value_type_id(symbol_id),
            SymbolSpace::Label => None,
        }
    }

    /// Hash a type id using the provided type table.
    fn hash_type_id_in_tables(
        &mut self,
        module_id: ModuleId,
        type_id: LocalTypeId,
        types: &TypeTable,
    ) -> u64 {
        // reuse cached hashes when available
        let global_type_id = GlobalTypeId::new(module_id, type_id);
        if let Some(hash) = self.type_hashes.get(&global_type_id) {
            return *hash;
        }

        // short circuit on recursion
        if !self.type_in_progress.insert(global_type_id) {
            return self.fallback_type_hash(global_type_id);
        }

        // compute and cache the hash
        let ty = types.get_type(type_id);
        let hash = self.hash_type(module_id, ty, types);
        self.type_in_progress.remove(&global_type_id);
        self.type_hashes.insert(global_type_id, hash);

        hash
    }

    /// Hash a type into a stable fingerprint.
    fn hash_type(&mut self, module_id: ModuleId, ty: &Type, types: &TypeTable) -> u64 {
        // seed the hasher with the type variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(ty).hash(&mut hasher);

        // hash type details
        match ty {
            Type::TypeLiteral { value } => {
                self.hash_type_literal(value).hash(&mut hasher);
            }
            Type::InferVar { id } => {
                id.hash(&mut hasher);
            }
            Type::Value { value } => {
                self.hash_type_id_in_tables(module_id, *value, types)
                    .hash(&mut hasher);
            }
            Type::This => {}
            Type::Reference {
                symbol,
                static_arguments,
            } => {
                self.symbol_signature_hash(*symbol).hash(&mut hasher);
                self.hash_static_arguments(module_id, types, static_arguments)
                    .hash(&mut hasher);
            }
            Type::Unevaluated(node_id) => {
                self.hash_node(node_id.into_global_any(module_id))
                    .hash(&mut hasher);
            }
            Type::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.hash_type_id_in_tables(module_id, *left, types)
                    .hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *then_type, types)
                    .hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *else_type, types)
                    .hash(&mut hasher);
            }
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                self.hash_type_mapped_parameter(module_id, types, parameter)
                    .hash(&mut hasher);
                self.hash_type_mapped_modifiers(modifiers).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *value, types)
                    .hash(&mut hasher);
            }
            Type::Index { left, index } => {
                self.hash_type_id_in_tables(module_id, *left, types)
                    .hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *index, types)
                    .hash(&mut hasher);
            }
            Type::TemplateLiteral { strings, spans } => {
                strings.len().hash(&mut hasher);
                for string in strings {
                    self.hash_string_id(*string).hash(&mut hasher);
                }
                spans.len().hash(&mut hasher);
                for span in spans {
                    self.hash_type_id_in_tables(module_id, *span, types)
                        .hash(&mut hasher);
                }
            }
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                self.hash_string_id(*target).hash(&mut hasher);
                qualifier
                    .as_ref()
                    .map(|path| self.hash_path(path))
                    .hash(&mut hasher);
                self.hash_static_arguments(module_id, types, static_arguments)
                    .hash(&mut hasher);
            }
            Type::Infer { name, constraint } => {
                self.hash_string_id(*name).hash(&mut hasher);
                constraint
                    .map(|ty| self.hash_type_id_in_tables(module_id, ty, types))
                    .hash(&mut hasher);
            }
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                asserts.hash(&mut hasher);
                self.hash_type_predicate_subject(subject).hash(&mut hasher);
                target
                    .map(|ty| self.hash_type_id_in_tables(module_id, ty, types))
                    .hash(&mut hasher);
            }
            Type::Unary { operator, right } => {
                self.hash_type_unary_operator(operator).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
            }
            Type::Mutable { mutability, right } => {
                self.hash_mutability(mutability).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
            }
            Type::ValueOf {
                mutability,
                variance,
                right,
            } => {
                self.hash_optional_mutability(mutability).hash(&mut hasher);
                self.hash_optional_variance(variance).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
            }
            Type::ReferenceOf {
                mutability,
                variance,
                right,
            } => {
                self.hash_optional_mutability(mutability).hash(&mut hasher);
                self.hash_optional_variance(variance).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
            }
            Type::PointerOf { mutability, right } => {
                self.hash_optional_mutability(mutability).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                self.hash_type_id_in_tables(module_id, *left, types)
                    .hash(&mut hasher);
                self.hash_type_binary_operator(operator).hash(&mut hasher);
                self.hash_type_id_in_tables(module_id, *right, types)
                    .hash(&mut hasher);
            }
            Type::ArraySized { element, count } => {
                self.hash_type_id_in_tables(module_id, *element, types)
                    .hash(&mut hasher);
                self.hash_node(count.into_global_any(module_id))
                    .hash(&mut hasher);
            }
            Type::Array { element } => {
                element
                    .map(|ty| self.hash_type_id_in_tables(module_id, ty, types))
                    .hash(&mut hasher);
            }
            Type::Tuple { elements } => {
                elements.len().hash(&mut hasher);
                for element in elements {
                    self.hash_type_element(module_id, types, element)
                        .hash(&mut hasher);
                }
            }
            Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                let mut field_hashes: Vec<(u64, u64)> = fields
                    .iter()
                    .map(|field| self.hash_type_field(module_id, types, field))
                    .collect();
                field_hashes.sort();
                field_hashes.len().hash(&mut hasher);
                for (_, field_hash) in field_hashes {
                    field_hash.hash(&mut hasher);
                }

                call_signatures.len().hash(&mut hasher);
                for signature in call_signatures {
                    self.hash_type_id_in_tables(module_id, *signature, types)
                        .hash(&mut hasher);
                }

                construct_signatures.len().hash(&mut hasher);
                for signature in construct_signatures {
                    self.hash_type_id_in_tables(module_id, *signature, types)
                        .hash(&mut hasher);
                }

                let mut index_hashes: Vec<(u64, u64)> = index_signatures
                    .iter()
                    .map(|signature| self.hash_type_index_signature(module_id, types, signature))
                    .collect();
                index_hashes.sort();
                index_hashes.len().hash(&mut hasher);
                for (_, index_hash) in index_hashes {
                    index_hash.hash(&mut hasher);
                }
            }
            Type::Function {
                asynchrony,
                cardinality,
                static_parameters,
                this_parameter,
                dynamic_parameters,
                return_type,
            } => {
                self.hash_asynchrony(asynchrony).hash(&mut hasher);
                self.hash_function_cardinality(cardinality)
                    .hash(&mut hasher);

                static_parameters.len().hash(&mut hasher);
                for parameter in static_parameters {
                    self.hash_type_id_in_tables(module_id, *parameter, types)
                        .hash(&mut hasher);
                }

                this_parameter
                    .map(|ty| self.hash_type_id_in_tables(module_id, ty, types))
                    .hash(&mut hasher);
                dynamic_parameters.len().hash(&mut hasher);
                for parameter in dynamic_parameters {
                    self.hash_type_id_in_tables(module_id, *parameter, types)
                        .hash(&mut hasher);
                }
                return_type
                    .map(|ty| self.hash_type_id_in_tables(module_id, ty, types))
                    .hash(&mut hasher);
            }
            Type::Union { elements } => {
                let mut element_hashes: Vec<u64> = elements
                    .iter()
                    .map(|element| self.hash_type_id_in_tables(module_id, *element, types))
                    .collect();
                element_hashes.sort_unstable();
                element_hashes.len().hash(&mut hasher);
                for element_hash in element_hashes {
                    element_hash.hash(&mut hasher);
                }
            }
            Type::Intersection { elements } => {
                let mut element_hashes: Vec<u64> = elements
                    .iter()
                    .map(|element| self.hash_type_id_in_tables(module_id, *element, types))
                    .collect();
                element_hashes.sort_unstable();
                element_hashes.len().hash(&mut hasher);
                for element_hash in element_hashes {
                    element_hash.hash(&mut hasher);
                }
            }
            Type::Error => {}
        }

        hasher.finish()
    }

    /// Hash a type literal into a stable fingerprint.
    fn hash_type_literal(&mut self, literal: &TypeLiteral) -> u64 {
        // seed the hasher with the literal variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(literal).hash(&mut hasher);

        // hash literal contents
        match literal {
            TypeLiteral::Primitive(primitive) => {
                self.hash_primitive_type(primitive).hash(&mut hasher);
            }
            TypeLiteral::Intrinsic(intrinsic) => {
                self.hash_intrinsic_type(*intrinsic).hash(&mut hasher);
            }
            TypeLiteral::ScalarLiteral(value) => {
                self.hash_scalar_literal(value).hash(&mut hasher);
            }
            TypeLiteral::Never
            | TypeLiteral::Any
            | TypeLiteral::Infer
            | TypeLiteral::Undefined
            | TypeLiteral::Unknown
            | TypeLiteral::Object
            | TypeLiteral::Void
            | TypeLiteral::Null => {}
        }

        hasher.finish()
    }

    /// Hash a scalar literal into a stable fingerprint.
    fn hash_scalar_literal(&mut self, literal: &ScalarLiteral) -> u64 {
        // seed the hasher with the literal variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(literal).hash(&mut hasher);

        // hash literal contents
        match literal {
            ScalarLiteral::Boolean(value) => {
                value.hash(&mut hasher);
            }
            ScalarLiteral::Integer(value) | ScalarLiteral::Bigint(value) => {
                value.hash(&mut hasher);
            }
            ScalarLiteral::Float(value) => {
                value.to_bits().hash(&mut hasher);
            }
            ScalarLiteral::Character(value) => {
                value.hash(&mut hasher);
            }
            ScalarLiteral::String(value) => {
                self.hash_string_id(*value).hash(&mut hasher);
            }
            ScalarLiteral::RegexString { content, flags } => {
                self.hash_string_id(*content).hash(&mut hasher);
                flags
                    .map(|flags| self.hash_string_id(flags))
                    .hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Hash a primitive type into a stable fingerprint.
    fn hash_primitive_type(&mut self, primitive: &PrimitiveType) -> u64 {
        // seed the hasher with the primitive variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(primitive).hash(&mut hasher);

        // hash primitive details
        match primitive {
            PrimitiveType::Int(int_type) => {
                self.hash_int_type(int_type).hash(&mut hasher);
            }
            PrimitiveType::Float(float_type) => {
                self.hash_float_type(float_type).hash(&mut hasher);
            }
            PrimitiveType::Boolean
            | PrimitiveType::Character
            | PrimitiveType::String
            | PrimitiveType::Bigint
            | PrimitiveType::Number
            | PrimitiveType::Symbol
            | PrimitiveType::UniqueSymbol => {}
        }

        hasher.finish()
    }

    /// Hash an intrinsic type into a stable fingerprint.
    fn hash_intrinsic_type(&mut self, intrinsic: IntrinsicType) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(&intrinsic).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash an integer type into a stable fingerprint.
    fn hash_int_type(&mut self, int_type: &destack_dir::IntType) -> u64 {
        // seed the hasher with the integer variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(int_type).hash(&mut hasher);

        // hash integer details
        if let destack_dir::IntType::Arbitrary { width, is_signed } = int_type {
            width.hash(&mut hasher);
            is_signed.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Hash a float type into a stable fingerprint.
    fn hash_float_type(&mut self, float_type: &destack_dir::FloatType) -> u64 {
        // seed the hasher with the float variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(float_type).hash(&mut hasher);

        // hash float details
        if let destack_dir::FloatType::Arbitrary { width } = float_type {
            width.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Hash type mapped modifiers into a stable fingerprint.
    fn hash_type_mapped_modifiers(&mut self, modifiers: &TypeMappedModifiers) -> u64 {
        let mut hasher = FxHasher::default();
        self.hash_type_modifier(&modifiers.readonly)
            .hash(&mut hasher);
        self.hash_type_modifier(&modifiers.optional)
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a type mapped parameter into a stable fingerprint.
    fn hash_type_mapped_parameter(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        parameter: &TypeMappedParameter,
    ) -> u64 {
        // seed the hasher with the parameter name
        let mut hasher = FxHasher::default();
        self.hash_string_id(parameter.name).hash(&mut hasher);

        // hash parameter details
        self.hash_type_id_in_tables(module_id, parameter.constraint, types)
            .hash(&mut hasher);
        parameter
            .key_remap
            .map(|ty| self.hash_type_id_in_tables(module_id, ty, types))
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a type modifier into a stable fingerprint.
    fn hash_type_modifier(&mut self, modifier: &TypeModifier) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(modifier).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a type predicate subject into a stable fingerprint.
    fn hash_type_predicate_subject(&mut self, subject: &TypePredicateSubject) -> u64 {
        // seed the hasher with the subject variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(subject).hash(&mut hasher);

        // hash subject details
        match subject {
            TypePredicateSubject::Unresolved(name) => {
                self.hash_string_id(*name).hash(&mut hasher);
            }
            TypePredicateSubject::Symbol(symbol_id) => {
                symbol_id.hash(&mut hasher);
            }
            TypePredicateSubject::This => {}
        }

        hasher.finish()
    }

    /// Hash a type unary operator into a stable fingerprint.
    fn hash_type_unary_operator(&mut self, operator: &TypeUnaryOperator) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(operator).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a type binary operator into a stable fingerprint.
    fn hash_type_binary_operator(&mut self, operator: &TypeBinaryOperator) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(operator).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a mutability flag into a stable fingerprint.
    fn hash_mutability(&mut self, mutability: &Mutability) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(mutability).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash an optional mutability flag into a stable fingerprint.
    fn hash_optional_mutability(&mut self, mutability: &Option<Mutability>) -> u64 {
        let mut hasher = FxHasher::default();
        mutability
            .map(|value| self.hash_mutability(&value))
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Hash an optional variance bound into a stable fingerprint.
    fn hash_optional_variance(&mut self, variance: &Option<VarianceBound>) -> u64 {
        let mut hasher = FxHasher::default();
        variance
            .as_ref()
            .map(|value| self.hash_variance_bound(value))
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a variance bound into a stable fingerprint.
    fn hash_variance_bound(&mut self, variance: &VarianceBound) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(variance).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash an asynchrony flag into a stable fingerprint.
    fn hash_asynchrony(&mut self, asynchrony: &Asynchrony) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(asynchrony).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a function cardinality into a stable fingerprint.
    fn hash_function_cardinality(&mut self, cardinality: &FunctionCardinality) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(cardinality).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a type field into a stable fingerprint.
    fn hash_type_field(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        field: &TypeField,
    ) -> (u64, u64) {
        // seed the hasher with the field key
        let mut hasher = FxHasher::default();
        let key_hash = self.static_key_hash(field.key);
        key_hash.hash(&mut hasher);

        // hash field details
        self.hash_type_id_in_tables(module_id, field.ty, types)
            .hash(&mut hasher);
        field.is_optional.hash(&mut hasher);
        field.is_readonly.hash(&mut hasher);

        (key_hash, hasher.finish())
    }

    /// Hash a type element into a stable fingerprint.
    fn hash_type_element(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        element: &TypeElement,
    ) -> u64 {
        // seed the hasher with the element label
        let mut hasher = FxHasher::default();
        element
            .label
            .map(|label| self.hash_string_id(label))
            .hash(&mut hasher);

        // hash element details
        self.hash_type_id_in_tables(module_id, element.ty, types)
            .hash(&mut hasher);
        element.is_optional.hash(&mut hasher);
        element.is_readonly.hash(&mut hasher);
        element.is_rest.hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a type index signature into a stable fingerprint.
    fn hash_type_index_signature(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        signature: &TypeIndexSignature,
    ) -> (u64, u64) {
        // seed the hasher with the index name
        let mut hasher = FxHasher::default();
        self.hash_string_id(signature.name).hash(&mut hasher);

        // hash index details
        let key_hash = self.hash_type_id_in_tables(module_id, signature.key_type, types);
        key_hash.hash(&mut hasher);
        self.hash_type_id_in_tables(module_id, signature.value_type, types)
            .hash(&mut hasher);
        signature.is_readonly.hash(&mut hasher);

        (key_hash, hasher.finish())
    }

    /// Hash static arguments into a stable fingerprint.
    fn hash_static_arguments(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        static_arguments: &Option<Vec<StaticArgument>>,
    ) -> u64 {
        // seed the hasher with the optional flag
        let mut hasher = FxHasher::default();
        static_arguments.is_some().hash(&mut hasher);

        // hash static arguments when present
        if let Some(arguments) = static_arguments {
            arguments.len().hash(&mut hasher);
            for argument in arguments {
                self.hash_static_argument(module_id, types, argument)
                    .hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Hash a static argument into a stable fingerprint.
    fn hash_static_argument(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        argument: &StaticArgument,
    ) -> u64 {
        // seed the hasher with the argument variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(argument).hash(&mut hasher);

        // hash argument details
        match argument {
            StaticArgument::Unevaluated { node } => {
                self.hash_node(node.into_global_any(module_id))
                    .hash(&mut hasher);
            }
            StaticArgument::Evaluated { name, value } => {
                name.map(|name| self.hash_string_id(name)).hash(&mut hasher);
                self.hash_static_expression(module_id, types, value)
                    .hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Hash a static expression into a stable fingerprint.
    fn hash_static_expression(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        expression: &StaticExpression,
    ) -> u64 {
        // seed the hasher with the expression variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(expression).hash(&mut hasher);

        // hash expression details
        match expression {
            StaticExpression::Unevaluated { node } => {
                self.hash_node(node.into_global_any(module_id))
                    .hash(&mut hasher);
            }
            StaticExpression::ScalarLiteral { value } => {
                self.hash_scalar_literal(value).hash(&mut hasher);
            }
            StaticExpression::TypeLiteral { value } => {
                self.hash_type_literal(value).hash(&mut hasher);
            }
            StaticExpression::Declaration {
                declaration,
                static_arguments,
            } => {
                self.hash_node(declaration.into_global_any(module_id))
                    .hash(&mut hasher);
                self.hash_static_arguments(module_id, types, static_arguments)
                    .hash(&mut hasher);
            }
            StaticExpression::Type { ty } => {
                self.hash_type_id_in_tables(module_id, *ty, types)
                    .hash(&mut hasher);
            }
            StaticExpression::RangeExpression {
                start,
                end,
                is_inclusive,
            } => {
                self.hash_static_expression(module_id, types, start)
                    .hash(&mut hasher);
                self.hash_static_expression(module_id, types, end)
                    .hash(&mut hasher);
                is_inclusive.hash(&mut hasher);
            }
            StaticExpression::ArrayExpression { elements } => {
                elements.len().hash(&mut hasher);
                for element in elements {
                    self.hash_static_expression(module_id, types, element)
                        .hash(&mut hasher);
                }
            }
            StaticExpression::TupleExpression { elements } => {
                elements.len().hash(&mut hasher);
                for element in elements {
                    self.hash_static_expression(module_id, types, element)
                        .hash(&mut hasher);
                }
            }
            StaticExpression::ObjectExpression { properties } => {
                properties.len().hash(&mut hasher);
                for property in properties {
                    self.hash_static_property(module_id, types, property)
                        .hash(&mut hasher);
                }
            }
        }

        hasher.finish()
    }

    /// Hash a static property into a stable fingerprint.
    fn hash_static_property(
        &mut self,
        module_id: ModuleId,
        types: &TypeTable,
        property: &StaticProperty,
    ) -> u64 {
        // seed the hasher with the property variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(property).hash(&mut hasher);

        // hash property details
        match property {
            StaticProperty::Unevaluated { node } => {
                self.hash_node(node.into_global_any(module_id))
                    .hash(&mut hasher);
            }
            StaticProperty::Field {
                modifiers,
                key,
                value,
                default,
                symbol,
            } => {
                modifiers
                    .as_ref()
                    .map(|value| self.hash_binding_modifier(value))
                    .hash(&mut hasher);
                key.as_ref()
                    .map(|value| self.hash_dynamic_key(module_id, value))
                    .hash(&mut hasher);
                self.hash_static_expression(module_id, types, value)
                    .hash(&mut hasher);
                default
                    .as_ref()
                    .map(|value| self.hash_static_expression(module_id, types, value))
                    .hash(&mut hasher);
                symbol.hash(&mut hasher);
            }
            StaticProperty::Method {
                modifiers,
                key,
                signature,
                body,
                symbol,
            } => {
                modifiers
                    .as_ref()
                    .map(|value| self.hash_binding_modifier(value))
                    .hash(&mut hasher);
                key.as_ref()
                    .map(|value| self.hash_dynamic_key(module_id, value))
                    .hash(&mut hasher);
                self.hash_function_signature(module_id, signature)
                    .hash(&mut hasher);
                self.hash_static_expression(module_id, types, body)
                    .hash(&mut hasher);
                symbol.hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Hash a dynamic key into a stable fingerprint.
    fn hash_dynamic_key(&mut self, module_id: ModuleId, key: &DynamicKey) -> u64 {
        // seed the hasher with the key variant
        let mut hasher = FxHasher::default();
        std::mem::discriminant(key).hash(&mut hasher);

        // hash key details
        match key {
            DynamicKey::Name(name) => {
                self.hash_string_id(*name).hash(&mut hasher);
            }
            DynamicKey::Number(name) => {
                self.hash_string_id(*name).hash(&mut hasher);
            }
            DynamicKey::Expression(expression) => {
                self.hash_node(expression.into_global_any(module_id))
                    .hash(&mut hasher);
            }
            DynamicKey::NamedExpression { name, key } => {
                self.hash_string_id(*name).hash(&mut hasher);
                self.hash_node(key.into_global_any(module_id))
                    .hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Hash binding modifiers into a stable fingerprint.
    fn hash_binding_modifier(&mut self, modifier: &BindingModifier) -> u64 {
        // seed the hasher with the modifier structure
        let mut hasher = FxHasher::default();

        // hash modifier details
        modifier
            .kind
            .map(|value| self.hash_binding_kind(value))
            .hash(&mut hasher);
        modifier
            .anchor
            .map(|value| self.hash_binding_anchor(value))
            .hash(&mut hasher);
        modifier
            .mutability
            .map(|value| self.hash_mutability(&value))
            .hash(&mut hasher);
        modifier
            .visibility
            .map(|value| self.hash_visibility(&value))
            .hash(&mut hasher);
        modifier
            .operator
            .map(|value| self.hash_binding_operator(value))
            .hash(&mut hasher);
        modifier
            .accessor
            .map(|value| self.hash_accessor_kind(value))
            .hash(&mut hasher);
        modifier
            .timing
            .map(|value| self.hash_binding_timing(value))
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a binding kind into a stable fingerprint.
    fn hash_binding_kind(&mut self, kind: BindingKind) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(&kind).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a binding anchor into a stable fingerprint.
    fn hash_binding_anchor(&mut self, anchor: BindingAnchor) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(&anchor).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a binding operator into a stable fingerprint.
    fn hash_binding_operator(&mut self, operator: BindingOperator) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(&operator).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a binding accessor kind into a stable fingerprint.
    fn hash_accessor_kind(&mut self, accessor: AccessorKind) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(&accessor).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a binding timing into a stable fingerprint.
    fn hash_binding_timing(&mut self, timing: Timing) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(&timing).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a visibility into a stable fingerprint.
    fn hash_visibility(&mut self, visibility: &destack_dir::Visibility) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(visibility).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a function signature into a stable fingerprint.
    fn hash_function_signature(
        &mut self,
        module_id: ModuleId,
        signature: &FunctionSignature,
    ) -> u64 {
        // seed the hasher with the signature structure
        let mut hasher = FxHasher::default();

        // hash signature details
        self.hash_function_abstraction(&signature.abstraction)
            .hash(&mut hasher);
        self.hash_asynchrony(&signature.asynchrony)
            .hash(&mut hasher);
        self.hash_function_cardinality(&signature.cardinality)
            .hash(&mut hasher);
        signature
            .mode
            .map(|mode| self.hash_function_mode(&mode))
            .hash(&mut hasher);
        self.hash_function_kind(&signature.kind).hash(&mut hasher);
        signature
            .generics
            .as_ref()
            .map(|generics| self.hash_generics(module_id, generics))
            .hash(&mut hasher);
        signature
            .this_parameter
            .map(|parameter| self.hash_node(parameter.into_global_any(module_id)))
            .hash(&mut hasher);
        signature.dynamic_parameters.len().hash(&mut hasher);
        for parameter in &signature.dynamic_parameters {
            self.hash_node(parameter.into_global_any(module_id))
                .hash(&mut hasher);
        }
        signature
            .return_type
            .map(|value| self.hash_node(value.into_global_any(module_id)))
            .hash(&mut hasher);

        hasher.finish()
    }

    /// Hash function abstraction into a stable fingerprint.
    fn hash_function_abstraction(&mut self, abstraction: &FunctionAbstraction) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(abstraction).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash function mode into a stable fingerprint.
    fn hash_function_mode(&mut self, mode: &FunctionMode) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(mode).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash function kind into a stable fingerprint.
    fn hash_function_kind(&mut self, kind: &FunctionKind) -> u64 {
        let mut hasher = FxHasher::default();
        std::mem::discriminant(kind).hash(&mut hasher);

        hasher.finish()
    }

    /// Hash generics into a stable fingerprint.
    fn hash_generics(&mut self, module_id: ModuleId, generics: &Generics) -> u64 {
        // seed the hasher with the optional flags
        let mut hasher = FxHasher::default();
        generics.static_parameters.is_some().hash(&mut hasher);
        generics.where_clauses.is_some().hash(&mut hasher);

        // hash generic details
        if let Some(parameters) = &generics.static_parameters {
            parameters.len().hash(&mut hasher);
            for parameter in parameters {
                self.hash_node(parameter.into_global_any(module_id))
                    .hash(&mut hasher);
            }
        }
        if let Some(clauses) = &generics.where_clauses {
            clauses.len().hash(&mut hasher);
            for clause in clauses {
                self.hash_node(clause.into_global_any(module_id))
                    .hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// Hash a path into a stable fingerprint.
    fn hash_path(&mut self, path: &Path) -> u64 {
        // seed the hasher with the segment count
        let mut hasher = FxHasher::default();
        path.segments.len().hash(&mut hasher);

        // hash path details
        for segment in &path.segments {
            self.hash_string_id(*segment).hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Hash a node tree subtree into a stable fingerprint.
    fn hash_node(&mut self, node_id: GlobalNodeIdAny) -> u64 {
        // reuse cached hashes when available
        if let Some(hash) = self.node_hashes.get(&node_id) {
            return *hash;
        }

        // load the owning module and its node tree
        let module = self.compiler.program.modules.get(node_id.module_id);
        let module = module.read();
        let Some(dir) = module.dir_maybe(self.profile_id) else {
            let hash = self.fallback_node_hash(node_id);
            self.node_hashes.insert(node_id, hash);
            return hash;
        };
        let tree = dir.tree.read();

        // dump the node subtree with stable formatting
        let mut dumper = Dumper::new(
            &self.strings,
            &tree,
            DumperOptions {
                use_colors: false,
                include_node_ids: false,
            },
        );
        match node_id.local_id.ty {
            NodeType::Expression => {
                let local_id = node_id.into_local_typed::<Expression>();
                let expression = tree.get(local_id);
                dumper.visit_expression(&tree, local_id, expression);
            }
            NodeType::Argument => {
                let local_id = node_id.into_local_typed::<Argument>();
                let argument = tree.get(local_id);
                dumper.visit_argument(&tree, local_id, argument);
            }
            NodeType::Declaration => {
                let local_id = node_id.into_local_typed::<Declaration>();
                let declaration = tree.get(local_id);
                dumper.visit_declaration(&tree, local_id, declaration);
            }
            NodeType::Property => {
                let local_id = node_id.into_local_typed::<Property>();
                let property = tree.get(local_id);
                dumper.visit_property(&tree, local_id, property);
            }
            NodeType::Parameter => {
                let local_id = node_id.into_local_typed::<Parameter>();
                let parameter = tree.get(local_id);
                dumper.visit_parameter(&tree, local_id, parameter);
            }
            NodeType::WhereClause => {
                let local_id = node_id.into_local_typed::<WhereClause>();
                let where_clause = tree.get(local_id);
                dumper.visit_where_clause(&tree, local_id, where_clause);
            }
            _ => {
                let hash = self.fallback_node_hash(node_id);
                self.node_hashes.insert(node_id, hash);
                return hash;
            }
        }

        let output = dumper.finish();
        let mut hasher = FxHasher::default();
        output.hash(&mut hasher);
        let hash = hasher.finish();
        self.node_hashes.insert(node_id, hash);
        hash
    }

    /// Hash a fallback node id into a stable fingerprint.
    fn fallback_node_hash(&mut self, node_id: GlobalNodeIdAny) -> u64 {
        let mut hasher = FxHasher::default();
        node_id.hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a string id by its content.
    fn hash_string_id(&self, id: StringId) -> u64 {
        let mut hasher = FxHasher::default();
        self.strings.get(id).hash(&mut hasher);
        hasher.finish()
    }

    /// Hash a static key into a stable fingerprint.
    fn hash_static_key(&self, key: &StaticKey) -> u64 {
        let mut hasher = FxHasher::default();
        match key {
            StaticKey::Name(name) | StaticKey::Number(name) => {
                self.hash_string_id(*name).hash(&mut hasher);
            }
            StaticKey::Symbol(symbol) => {
                self.hash_symbol_key(symbol).hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Hash a symbol key into a stable fingerprint.
    fn hash_symbol_key(&self, key: &SymbolKey) -> u64 {
        let mut hasher = FxHasher::default();
        match key {
            SymbolKey::Unique(symbol) => {
                symbol.hash(&mut hasher);
            }
            SymbolKey::WellKnown(symbol) => {
                symbol.global_symbol_name().hash(&mut hasher);
            }
            SymbolKey::Registry(name) => {
                self.hash_string_id(*name).hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Hash a fallback symbol id into a stable fingerprint.
    fn fallback_symbol_hash(&mut self, symbol_id: GlobalSymbolId) -> u64 {
        let mut hasher = FxHasher::default();
        symbol_id.hash(&mut hasher);

        hasher.finish()
    }

    /// Hash a fallback type id into a stable fingerprint.
    fn fallback_type_hash(&mut self, type_id: GlobalTypeId) -> u64 {
        let mut hasher = FxHasher::default();
        type_id.hash(&mut hasher);

        hasher.finish()
    }
}

/// Map symbol space to a stable ordering rank.
fn symbol_space_rank(space: SymbolSpace) -> u8 {
    match space {
        SymbolSpace::Type => 0,
        SymbolSpace::Value => 1,
        SymbolSpace::TypeValue => 2,
        SymbolSpace::Label => 3,
    }
}

/// Map export kind to a stable ordering rank.
fn export_kind_rank(kind: ExportKind) -> u8 {
    match kind {
        ExportKind::Local => 0,
        ExportKind::ReExport => 1,
    }
}
