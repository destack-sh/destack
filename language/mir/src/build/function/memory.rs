use destack_core::StringId;

use crate::build::FunctionBuilder;
use crate::{Block, Copy, Global, Instruction, Local, LocalNodeId, Mutability, Place, Type, Value};

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionBuilder<'a> {
    /// Create a local variable (stack slot).
    pub fn local(&mut self, ty: LocalNodeId<Type>, mutability: Mutability) -> LocalNodeId<Local> {
        let local = self.insert(Local::new(ty, mutability));
        self.locals.push(local);
        local
    }

    /// Load from a local variable.
    pub fn local_get(&mut self, local: LocalNodeId<Local>, copy: Copy) -> Value {
        let ty = self.tree.get(local).ty;

        self.load(Place::local(local), ty, copy)
    }

    /// Store to a local variable.
    pub fn local_set(&mut self, local: LocalNodeId<Local>, value: Value) {
        self.store(Place::local(local), value);
    }

    /// Set a local at the end of one block, ahead of its terminator.
    pub fn local_set_at(
        &mut self,
        block: LocalNodeId<Block>,
        local: LocalNodeId<Local>,
        value: Value,
    ) {
        let instruction = self.insert(Instruction::Store {
            place: Place::local(local),
            value,
        });
        self.tree.get_mut(block).instructions.push(instruction);
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

    /// Read a global value.
    pub fn load_global(&mut self, global: LocalNodeId<Global>, copy: Copy) -> Value {
        let ty = self.tree.get(global).ty;

        self.load(Place::global(global), ty, copy)
    }

    /// Write a global value.
    pub fn store_global(&mut self, global: LocalNodeId<Global>, value: Value) {
        self.store(Place::global(global), value);
    }

    /// Read a place.
    pub fn load(&mut self, place: Place, result_type: LocalNodeId<Type>, copy: Copy) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Load {
            copy,
            destination,
            place,
            result_type,
        });
        self.define_value(destination, result_type);
        destination
    }

    /// Take the address of a place.
    pub fn address(&mut self, place: Place, result_type: LocalNodeId<Type>) -> Value {
        let destination = self.allocate_value();
        self.insert_instruction(Instruction::Address {
            destination,
            place,
            result_type,
        });
        self.define_value(destination, result_type);

        destination
    }

    /// Write a place.
    pub fn store(&mut self, place: Place, value: Value) {
        self.insert_instruction(Instruction::Store { place, value });
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
