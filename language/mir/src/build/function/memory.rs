use destack_core::StringId;

use crate::build::FunctionBuilder;
use crate::{
    Access, AddressKind, Block, Copy, Exclusivity, Global, Instruction, Lifetime, Local,
    LocalNodeId, Mutability, Reference, Storage, Type, Value,
};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Return the existing or inserted pointer-sized unsigned integer type.
    pub fn ensure_usize_type(&mut self) -> LocalNodeId<Type> {
        if let Some((ty, _)) = self
            .tree
            .iter_nodes::<Type>()
            .find(|(_, ty)| matches!(ty, Type::Usize))
        {
            return ty;
        }

        self.tree.intern_type(Type::Usize)
    }

    /// Create a local variable (stack slot).
    pub fn local(&mut self, ty: LocalNodeId<Type>, mutability: Mutability) -> LocalNodeId<Local> {
        let local = self.insert(Local::new(ty, mutability));
        self.locals.push(local);
        local
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>, copy: Copy) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet {
            copy,
            destination,
            local,
        });
        let local_ty = self.tree.get(local).ty;
        self.define_value(destination, local_ty);
        destination
    }

    /// Get the address of a local variable.
    pub fn local_addr(
        &mut self,
        local: LocalNodeId<Local>,
        result_type: LocalNodeId<Type>,
        kind: AddressKind,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalAddr {
            destination,
            local,
            result_type,
            kind,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.insert_instruction(Instruction::LocalSet { local, value });
    }

    /// Set a local at the end of one block, ahead of its terminator.
    pub fn local_set_at(
        &mut self,
        block: LocalNodeId<Block>,
        local: LocalNodeId<Local>,
        value: Value,
    ) {
        let instruction = self.insert(Instruction::LocalSet { local, value });
        self.tree.get_mut(block).instructions.push(instruction);
    }

    /// Get the address of a mutable global variable.
    pub fn global_addr(
        &mut self,
        global: LocalNodeId<Global>,
        result_type: LocalNodeId<Type>,
        kind: AddressKind,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalAddr {
            destination,
            global,
            result_type,
            kind,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Declare an external global from inside a function body.
    pub fn external_global(
        &mut self,
        name: StringId,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Global> {
        self.insert(Global::import(self.module, name, ty, mutability))
    }

    /// Load one global value through its address.
    pub fn load_global(&mut self, global: LocalNodeId<Global>, copy: Copy) -> Value {
        let global_ty = self.tree.get(global).ty;
        let global_space = self.tree.get(global).space;
        let global_pointer = self.tree.intern_type(Type::Reference {
            kind: Reference::Borrowed(Exclusivity::Aliasable),
            lifetime: Lifetime::empty(),
            storage: Storage::global(global_space),
            access: Access::Readonly,
            pointee: global_ty,
        });
        let pointer = self.global_addr(global, global_pointer, AddressKind::Projection);

        self.load(pointer, global_ty, copy)
    }

    /// Store one global value through its address.
    pub fn store_global(&mut self, global: LocalNodeId<Global>, value: Value) {
        let global_ty = self.tree.get(global).ty;
        let global_space = self.tree.get(global).space;
        let global_pointer = self.tree.intern_type(Type::Reference {
            kind: Reference::Borrowed(Exclusivity::Aliasable),
            lifetime: Lifetime::empty(),
            storage: Storage::global(global_space),
            access: Access::Mutable,
            pointee: global_ty,
        });
        let pointer = self.global_addr(global, global_pointer, AddressKind::Projection);

        self.store(pointer, value);
    }

    /// Load from a pointer.
    pub fn load(
        &mut self,
        pointer_value: Value,
        result_type: LocalNodeId<Type>,
        copy: Copy,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
            copy,
            destination,
            pointer: pointer_value,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a pointer.
    pub fn store(&mut self, pointer_value: Value, value: Value) {
        self.insert_instruction(Instruction::Store {
            pointer: pointer_value,
            value,
        });
    }

    /// Create a linear uninitialized allocation token type.
    pub fn type_uninit(&mut self, value: LocalNodeId<Type>) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Uninit { value })
    }

    /// Allocate zeroed heap storage.
    pub fn new_zeroed(
        &mut self,
        storage_type: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewZeroed {
            destination,
            storage_type,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate uninitialized heap storage.
    pub fn new_uninit(
        &mut self,
        storage_type: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewUninit {
            destination,
            storage_type,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Complete initialization of one allocation.
    pub fn new_complete(&mut self, value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewComplete {
            destination,
            value,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate zeroed repeated heap storage.
    pub fn new_slice_zeroed(
        &mut self,
        element: LocalNodeId<Type>,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewSliceZeroed {
            destination,
            element,
            length,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate uninitialized repeated heap storage.
    pub fn new_slice_uninit(
        &mut self,
        element: LocalNodeId<Type>,
        length: Value,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewSliceUninit {
            destination,
            element,
            length,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Drop one value.
    pub fn drop_value(&mut self, value: Value) -> LocalNodeId<Instruction> {
        self.insert_instruction(Instruction::Drop { value })
    }

    /// Release one unique representation's backing allocation to the heap after its contents dropped.
    pub fn release(&mut self, value: Value) {
        self.insert_instruction(Instruction::Release { value });
    }

    // instruction builders: assumptions

    /// Assume a condition is true (UB if false).
    pub fn assume(&mut self, condition: Value) {
        self.insert_instruction(Instruction::Assume { condition });
    }
}
