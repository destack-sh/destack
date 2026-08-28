use destack_core::{FxIndexMap, StringPool};
use destack_mir as mir;
use destack_source::{ProvenanceBuilder, ProvenanceId};

use super::{DestructorBody, DropPlan};

const ELABORATE_DESTRUCTORS: &str = "elaborate-destructors";

/// Builder for generated MIR destructor functions.
pub(in crate::elaborate) struct DestructorBuilder<'a> {
    /// The MIR tree receiving generated functions.
    tree: &'a mut mir::Tree,
    /// The provenance transformations produced by elaboration.
    provenance: &'a mut ProvenanceBuilder,
    /// Target ABI layout.
    target: mir::TargetLayout,
    /// Canonical MIR drop table.
    drops: &'a mut mir::DropTable,
    /// Function and call effect table.
    effects: &'a mut mir::EffectTable,
    /// Strings needed by generated MIR names.
    strings: &'a StringPool,
}

impl<'a> DestructorBuilder<'a> {
    /// Create one destructor builder.
    pub(in crate::elaborate) fn new(
        tree: &'a mut mir::Tree,
        provenance: &'a mut ProvenanceBuilder,
        target: mir::TargetLayout,
        drops: &'a mut mir::DropTable,
        effects: &'a mut mir::EffectTable,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            tree,
            provenance,
            target,
            drops,
            effects,
            strings,
        }
    }

    /// Build every destructor required by allocations and planned frame drops.
    pub(in crate::elaborate) fn build(&mut self, plans: &[DropPlan]) {
        let mut worklist = Vec::new();
        self.collect_allocations(&mut worklist);
        self.collect_drops(plans, &mut worklist);

        // propagate each trigger through its reachable destructor types
        let mut requirements = FxIndexMap::default();
        while let Some((ty, storage, source)) = worklist.pop() {
            let (ty, storage) = self.normalize(ty, storage);
            let source = self
                .tree
                .type_declaration(ty)
                .map(|declaration| self.tree.provenance(declaration))
                .unwrap_or(source);
            if !Self::add_requirement(&mut requirements, (ty, storage), source) {
                continue;
            }
            if self.drops.destructor(ty, storage).is_some()
                || !self.drops.requires_destructor(ty, storage, self.tree)
            {
                continue;
            }

            for (child, storage) in self.children(ty, storage) {
                worklist.push((child, storage, source));
            }
        }

        // declare all generated symbols before building mutually recursive bodies
        let mut destructors = Vec::new();
        for ((ty, storage), sources) in requirements {
            if self.drops.destructor(ty, storage).is_some()
                || !self.drops.requires_destructor(ty, storage, self.tree)
            {
                continue;
            }

            let function = self.declare_destructor(ty, storage, &sources);
            self.drops.set_destructor(ty, storage, function);
            destructors.push((ty, storage, function));
        }

        // build bodies after every referenced destructor has a symbol
        for (ty, storage, function) in destructors {
            self.build_destructor_body(ty, storage, function);
        }
    }

    /// Collect managed allocation types and their allocating provenance.
    fn collect_allocations(&self, worklist: &mut Vec<(mir::TypeId, mir::Storage, ProvenanceId)>) {
        // inspect every allocation in every defined function
        for (_, function) in self.tree.iter_nodes::<mir::Function>() {
            for block in function.blocks() {
                let block = self.tree.get(*block);

                // collect infallible allocation types
                for instruction_id in &block.instructions {
                    let allocation = match self.tree.get(*instruction_id) {
                        mir::Instruction::NewZeroed {
                            storage_type,
                            result_type,
                            ..
                        }
                        | mir::Instruction::NewUninit {
                            storage_type,
                            result_type,
                            ..
                        } => Some((*storage_type, *result_type)),
                        mir::Instruction::NewSliceZeroed {
                            element,
                            result_type,
                            ..
                        }
                        | mir::Instruction::NewSliceUninit {
                            element,
                            result_type,
                            ..
                        } => Some((*element, *result_type)),
                        _ => None,
                    };
                    let Some((ty, result)) = allocation else {
                        continue;
                    };
                    if let Some(storage) = self.tree.managed_storage(result) {
                        let provenance = self.tree.provenance(*instruction_id);
                        worklist.push((ty, storage, provenance));
                    }
                }

                // collect fallible allocation types from their success result
                let terminator = self.tree.get(block.terminator);
                let allocation = match terminator {
                    mir::Terminator::NewZeroedTry {
                        storage_type,
                        success,
                        ..
                    }
                    | mir::Terminator::NewUninitTry {
                        storage_type,
                        success,
                        ..
                    } => Some((*storage_type, success)),
                    mir::Terminator::NewSliceZeroedTry {
                        element, success, ..
                    }
                    | mir::Terminator::NewSliceUninitTry {
                        element, success, ..
                    } => Some((*element, success)),
                    _ => None,
                };
                let Some((ty, target)) = allocation else {
                    continue;
                };
                let result = self
                    .tree
                    .get(target.block)
                    .parameters
                    .first()
                    .unwrap_or_else(|| unreachable!("fallible allocation success has no result"));
                if let Some(storage) = self.tree.managed_storage(result.ty) {
                    let provenance = self.tree.provenance(block.terminator);
                    worklist.push((ty, storage, provenance));
                }
            }
        }
    }

    /// Collect planned frame drops and their source provenance.
    fn collect_drops(
        &self,
        plans: &[DropPlan],
        worklist: &mut Vec<(mir::TypeId, mir::Storage, ProvenanceId)>,
    ) {
        for plan in plans {
            // collect drops inside blocks
            for (block_id, drops) in &plan.block_drops {
                let block = self.tree.get(*block_id);
                for drop in drops {
                    let provenance = if let Some(instruction) = block.instructions.get(drop.index) {
                        self.tree.provenance(*instruction)
                    } else {
                        self.tree.provenance(block.terminator)
                    };
                    let ty = plan.paths.get(drop.path).ty;
                    worklist.push((ty, mir::Storage::Frame, provenance));
                }
            }

            // collect drops on control flow edges
            for drop in &plan.edge_drops {
                let terminator = self.tree.get(drop.edge.source).terminator;
                let provenance = self.tree.provenance(terminator);
                for path in &drop.paths {
                    let ty = plan.paths.get(*path).ty;
                    worklist.push((ty, mir::Storage::Frame, provenance));
                }
            }
        }
    }

    /// Normalize an owning type to the stored value destroyed by its destructor.
    fn normalize(&self, ty: mir::TypeId, storage: mir::Storage) -> (mir::TypeId, mir::Storage) {
        match self.tree.ty(ty) {
            mir::Type::Reference {
                kind: mir::ReferenceKind::Unique,
                pointee,
                storage,
                ..
            } => (*pointee, *storage),
            _ => (ty, storage),
        }
    }

    /// Declare the destructor for one type.
    fn declare_destructor(
        &mut self,
        ty: mir::TypeId,
        storage: mir::Storage,
        sources: &[ProvenanceId],
    ) -> mir::LocalNodeId<mir::Function> {
        let name = self.strings.intern(&format!("drop.{}", storage.segment()));
        let argument = self.tree.intern_static(mir::Static::Type(ty));
        let arguments = vec![argument];
        let symbol = mir::Symbol::named(name).instantiate(&arguments, self.tree);
        let pointer_type = mir::Type::Reference {
            kind: mir::ReferenceKind::Borrowed,
            lifetime: mir::Lifetime::empty(),
            storage,
            access: mir::Access::Exclusive,
            pointee: ty,
            nullability: mir::Nullability::None,
        };
        let pointer = self.tree.intern_type(pointer_type);
        let void = self.tree.void_type();

        // record the generated function and parameter from every requiring source
        let mut provenance = self.provenance.record(ELABORATE_DESTRUCTORS);
        let [function_provenance, parameter_provenance] = provenance.generate_many(sources);
        let parameters = vec![mir::FunctionParameter::new(
            mir::Value::new(0),
            pointer,
            parameter_provenance,
        )];

        // register a bodyless function first so recursive drops can call it
        let function = mir::Function::declare(name, Vec::new(), parameters, void)
            .with_arguments(arguments)
            .with_symbol(symbol);

        self.tree.insert(function, function_provenance)
    }

    /// Return types and storage destroyed inside one destructor body.
    fn children(&self, ty: mir::TypeId, storage: mir::Storage) -> Vec<(mir::TypeId, mir::Storage)> {
        match self.tree.ty(ty) {
            // type Pair { left: File; right: File; }
            mir::Type::Struct { fields, .. } => {
                // include every field type
                fields.iter().map(|field| (field.ty, storage)).collect()
            }
            // type Pair = (File, File);
            mir::Type::Tuple { elements, .. } => {
                // include every element type
                elements.iter().map(|element| (*element, storage)).collect()
            }
            // type Handle = newtype<File>;
            mir::Type::Newtype { inner, .. } => {
                vec![(*inner, storage)]
            }
            // type Buffer = [File; 4];
            mir::Type::FixedArray {
                element, length, ..
            } => (*length > 0)
                .then_some((*element, storage))
                .into_iter()
                .collect(),
            // slice<File, unique, mutable>
            mir::Type::Slice {
                kind: mir::ReferenceKind::Unique,
                element,
                storage: slice_storage,
                ..
            } => {
                vec![(*element, *slice_storage)]
            }
            // type Result = variant<uint8> { 0uint8 = File; 1uint8 = void; };
            mir::Type::Variant { cases, .. } => {
                // include every payload type
                cases.iter().map(|case| (case.ty, storage)).collect()
            }
            // scalar and indirection types have no inline children
            _ => Vec::new(),
        }
    }

    /// Add one distinct trigger to a destructor requirement.
    fn add_requirement(
        requirements: &mut FxIndexMap<(mir::TypeId, mir::Storage), Vec<ProvenanceId>>,
        key: (mir::TypeId, mir::Storage),
        provenance: ProvenanceId,
    ) -> bool {
        let sources = requirements.entry(key).or_default();
        if sources.contains(&provenance) {
            return false;
        }

        sources.push(provenance);

        true
    }

    /// Build the destructor body.
    fn build_destructor_body(
        &mut self,
        ty: mir::TypeId,
        storage: mir::Storage,
        function: mir::LocalNodeId<mir::Function>,
    ) {
        // turn the declaration into a real body
        let mut builder = mir::FunctionBuilder::from_declared(
            self.tree,
            self.provenance,
            self.effects,
            self.target.pointer_bits(),
            ELABORATE_DESTRUCTORS,
            function,
        )
        .unwrap_or_else(|error| unreachable!("{error}"));

        let entry = builder.block();
        builder.switch_to_block(entry);
        let pointer = builder.function_parameter(0);

        // destroy one stored value without releasing the containing storage
        DestructorBody::new(self.drops, &mut builder).drop_root(ty, pointer, storage);
        builder.return_(None);
        builder
            .finish()
            .unwrap_or_else(|error| unreachable!("{error}"));
    }
}
