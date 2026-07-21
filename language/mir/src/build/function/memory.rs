use crate::build::FunctionBuilder;
use crate::{
    Access, Global, Instruction, Lifetime, Local, LocalNodeId, Mutability, Nullability,
    ReferenceKind, Space, Type, Value,
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
        let local = self.tree.insert(Local::new(ty, mutability));
        self.locals.push(local);
        local
    }

    /// Create a reference type for inline instruction typing.
    pub fn reference_type(
        &mut self,
        kind: ReferenceKind,
        pointee: LocalNodeId<Type>,
        access: Access,
        space: Space,
        nullability: Nullability,
    ) -> LocalNodeId<Type> {
        self.tree.intern_type(Type::Reference {
            kind,
            lifetime: Lifetime::empty(),
            space,
            access,
            pointee,
            nullability,
        })
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalGet { destination, local });
        let local_ty = self.tree.get(local).ty;
        self.define_value(destination, local_ty);
        destination
    }

    /// Get the address of a local variable.
    pub fn local_addr(
        &mut self,
        local: LocalNodeId<Local>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::LocalAddr {
            destination,
            local,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.insert_instruction(Instruction::LocalSet { local, value });
    }

    /// Get the address of a mutable global variable.
    pub fn global_addr(
        &mut self,
        global: LocalNodeId<Global>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::GlobalAddr {
            destination,
            global,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Declare an external global from inside a function body.
    pub fn external_global(
        &mut self,
        name: &str,
        ty: LocalNodeId<Type>,
        mutability: Mutability,
    ) -> LocalNodeId<Global> {
        let name_id = self.strings.intern(name);

        self.tree.insert(Global::import(name_id, ty, mutability))
    }

    /// Load one global value through its address.
    pub fn load_global(&mut self, global: LocalNodeId<Global>) -> Value {
        let global_ty = self.tree.get(global).ty;
        let global_space = self.tree.get(global).space;
        let global_pointer = self.reference_type(
            ReferenceKind::Raw,
            global_ty,
            Access::Readonly,
            global_space,
            Nullability::None,
        );
        let pointer = self.global_addr(global, global_pointer);

        self.load(pointer, global_ty)
    }

    /// Store one global value through its address.
    pub fn store_global(&mut self, global: LocalNodeId<Global>, value: Value) {
        let global_ty = self.tree.get(global).ty;
        let global_space = self.tree.get(global).space;
        let global_pointer = self.reference_type(
            ReferenceKind::Raw,
            global_ty,
            Access::Mutable,
            global_space,
            Nullability::None,
        );
        let pointer = self.global_addr(global, global_pointer);

        self.store(pointer, value);
    }

    /// Load from a pointer.
    pub fn load(&mut self, pointer_value: Value, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
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
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewZeroed {
            destination,
            layout,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Allocate uninitialized heap storage.
    pub fn new_uninit(
        &mut self,
        layout: LocalNodeId<Type>,
        result_type: LocalNodeId<Type>,
    ) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::NewUninit {
            destination,
            layout,
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
    pub fn drop_value(&mut self, value: Value) {
        self.insert_instruction(Instruction::Drop { value });
    }

    /// Free unique heap storage after drop elaboration.
    pub fn free(&mut self, value: Value) {
        self.insert_instruction(Instruction::Free { value });
    }

    // instruction builders: assumptions

    /// Assume a condition is true (UB if false).
    pub fn assume(&mut self, condition: Value) {
        self.insert_instruction(Instruction::Assume { condition });
    }
}
