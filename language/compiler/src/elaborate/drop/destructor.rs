use tspp_core::StringPool;
use tspp_mir as mir;
use tspp_source::ModuleId;

use super::DestructorBody;

/// Builder for generated MIR destructor functions.
pub(in crate::elaborate) struct DestructorBuilder<'a> {
    /// The module the destructors belong to.
    module: ModuleId,
    /// The MIR tree receiving generated functions.
    tree: &'a mut mir::Tree,
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
        module: ModuleId,
        tree: &'a mut mir::Tree,
        target: mir::TargetLayout,
        drops: &'a mut mir::DropTable,
        effects: &'a mut mir::EffectTable,
        strings: &'a StringPool,
    ) -> Self {
        Self {
            module,
            tree,
            target,
            drops,
            effects,
            strings,
        }
    }

    /// Build destructors required by managed allocation sites.
    pub(in crate::elaborate) fn build_allocations(&mut self) {
        let mut roots = Vec::new();
        self.collect_allocations(&mut roots);
        roots.sort_unstable();
        roots.dedup();

        for (ty, storage) in roots {
            self.build_destructor(ty, storage);
        }
    }

    /// Build frame destructors required by planned drops.
    pub(in crate::elaborate) fn build_frames(
        &mut self,
        roots: impl IntoIterator<Item = mir::TypeId>,
    ) {
        let mut roots = roots.into_iter().collect::<Vec<_>>();
        roots.sort_unstable();
        roots.dedup();

        for ty in roots {
            self.build_destructor(ty, mir::Storage::Frame);
        }
    }

    /// Collect managed heap allocation types with their exact storage.
    fn collect_allocations(&self, roots: &mut Vec<(mir::TypeId, mir::Storage)>) {
        // inspect every allocation in every defined function
        for (_, function) in self.tree.iter_nodes::<mir::Function>() {
            for block in function.blocks() {
                let block = self.tree.get(*block);

                // collect infallible allocation types
                for instruction in &block.instructions {
                    let allocation = match self.tree.get(*instruction) {
                        mir::Instruction::NewZeroed {
                            storage_type,
                            space,
                            ..
                        }
                        | mir::Instruction::NewUninit {
                            storage_type,
                            space,
                            ..
                        } => Some((*storage_type, *space)),
                        mir::Instruction::NewSliceZeroed { element, space, .. }
                        | mir::Instruction::NewSliceUninit { element, space, .. } => {
                            Some((*element, *space))
                        }
                        _ => None,
                    };
                    if let Some((ty, space)) = allocation {
                        roots.push((ty, mir::Storage::heap(space)));
                    }
                }

                // collect fallible allocation types
                let allocation = match self.tree.get(block.terminator) {
                    mir::Terminator::NewZeroedTry {
                        storage_type,
                        space,
                        ..
                    }
                    | mir::Terminator::NewUninitTry {
                        storage_type,
                        space,
                        ..
                    } => Some((*storage_type, *space)),
                    mir::Terminator::NewSliceZeroedTry { element, space, .. }
                    | mir::Terminator::NewSliceUninitTry { element, space, .. } => {
                        Some((*element, *space))
                    }
                    _ => None,
                };
                if let Some((ty, space)) = allocation {
                    roots.push((ty, mir::Storage::heap(space)));
                }
            }
        }
    }

    /// Build the destructor reached through one owning type.
    fn build_destructor(&mut self, ty: mir::TypeId, storage: mir::Storage) {
        // build a unique pointee's destructor for each heap it may be released from
        if let mir::Type::Reference {
            kind: mir::Reference::Unique,
            pointee,
            ..
        } = self.tree.get(ty)
        {
            let pointee = *pointee;
            for space in mir::Space::HEAPS {
                self.build_destructor(pointee, mir::Storage::heap(space));
            }

            return;
        }

        // reuse an existing destructor
        if self.drops.destructor(ty, storage).is_some() {
            return;
        }

        // skip copy and scalar shapes
        if !self.drops.requires_destructor(ty, storage, self.tree) {
            return;
        }

        // declare before recursion so cycles can refer to the symbol
        let function = self.declare_destructor(ty, storage);
        self.drops.set_destructor(ty, storage, function);

        // close child declarations before writing this body
        self.build_child_destructors(ty, storage);
        self.build_destructor_body(ty, storage, function);
    }

    /// Declare the destructor for one type.
    fn declare_destructor(
        &mut self,
        ty: mir::TypeId,
        storage: mir::Storage,
    ) -> mir::LocalNodeId<mir::Function> {
        let segment = storage.segment();
        let name = self.strings.intern(&format!("drop.{segment}"));
        let arguments = vec![mir::GenericArgument::Type(ty)];
        let symbol = mir::Symbol::named(self.module, name).instantiate(&arguments, self.tree);
        // borrow the dropped storage for the destructor's own binder
        let binder = self.strings.intern("'a");
        let lifetimes = vec![mir::LifetimeParameter::new(Some(binder))];
        let pointer_type = mir::Type::Reference {
            kind: mir::Reference::Borrowed,
            lifetime: mir::Lifetime::bound(0),
            access: mir::Access::Exclusive,
            pointee: ty,
        };
        let pointer = self.tree.intern_type(pointer_type);
        let parameters = vec![mir::FunctionParameter::new(mir::Value::new(0), pointer)];
        let void = self.tree.void_type();

        // register a bodyless function first so recursive drops can call it
        let function = mir::Function::declare(self.module, name, lifetimes, parameters, void)
            .with_arguments(arguments)
            .with_symbol(symbol);

        self.tree.insert(function)
    }

    /// Build nested aggregate destructors before this body.
    fn build_child_destructors(&mut self, ty: mir::TypeId, storage: mir::Storage) {
        match self.tree.get(ty).clone() {
            // type Pair { left: File; right: File; }
            mir::Type::Struct { fields, .. } => {
                // build every field destructor
                for field in fields {
                    let field_ty = self.tree.get(field).ty;
                    self.build_destructor(field_ty, storage);
                }
            }
            // type Pair = (File, File);
            mir::Type::Tuple { elements, .. } => {
                // build every element destructor
                for element in elements {
                    self.build_destructor(element, storage);
                }
            }
            // type Handle = newtype<File>;
            mir::Type::Newtype { value, .. } => {
                self.build_destructor(value, storage);
            }
            // type Buffer = [File; 4];
            mir::Type::FixedArray {
                element, length, ..
            } => {
                if self.tree.static_value(length).length() != Some(0) {
                    self.build_destructor(element, storage);
                }
            }
            // slice<File, unique, mutable>
            mir::Type::Slice {
                kind: mir::Reference::Unique,
                element,
                ..
            } => {
                for space in mir::Space::HEAPS {
                    self.build_destructor(element, mir::Storage::heap(space));
                }
            }
            // type Result = variant<uint8> { 0uint8 = File; 1uint8 = void; };
            mir::Type::Variant { cases, .. } => {
                // build every payload destructor
                for case in cases {
                    self.build_destructor(case.ty, storage);
                }
            }
            // build the children of the type an application stands for, like Box<int32>
            mir::Type::Application { .. } => {
                let applied = mir::Substitution::resolve(ty, self.tree);
                if applied != ty {
                    self.build_child_destructors(applied, storage);
                }
            }
            // scalar and indirection types have no inline children
            _ => {}
        }
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
            self.effects,
            self.target.pointer_bits(),
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
