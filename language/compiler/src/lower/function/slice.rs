use destack_mir as mir;

use crate::CompilerResult;
use crate::lower::FunctionLowerer;

/// Build a slice whose element count becomes known during execution.
pub(in crate::lower) struct SliceBuilder {
    /// The initialized element representation.
    element: mir::TypeId,
    /// The uninitialized element representation.
    uninit: mir::TypeId,
    /// The current allocation descriptor.
    storage: mir::LocalNodeId<mir::Local>,
    /// The number of initialized elements.
    length: mir::LocalNodeId<mir::Local>,
    /// The heap receiving the allocations.
    space: mir::Space,
}

impl SliceBuilder {
    /// Allocate and initialize the known prefix of a growing slice.
    pub(in crate::lower) fn new(
        element: mir::TypeId,
        values: Vec<mir::Value>,
        lower: &mut FunctionLowerer<'_, '_, '_>,
    ) -> CompilerResult<Self> {
        // declare the uninitialized allocation and its descriptor
        let uninit = lower.builder.type_uninit(element);
        let allocation = lower.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::Reference::Unique,
            lifetime: mir::Lifetime::empty(),
            element: uninit,
            access: mir::Access::Mutable,
        });
        let space = lower.allocation_space(allocation)?;
        let capacity = lower.builder.usize_const(values.len() as u128);
        let value = lower
            .builder
            .new_slice_uninit(uninit, capacity, allocation, space);
        let storage = lower.builder.local(allocation, mir::Mutability::Mutable);
        lower.builder.local_set(storage, value);

        // count initialized elements separately from allocated capacity
        let size = lower.builder.tree_mut().intern_type(mir::Type::Usize);
        let length = lower.builder.local(size, mir::Mutability::Mutable);
        lower.builder.local_set(length, capacity);

        let builder = Self {
            element,
            uninit,
            storage,
            length,
            space,
        };

        // initialize the prefix in its allocated positions
        for (index, element) in values.into_iter().enumerate() {
            let index = lower.builder.usize_const(index as u128);
            let place = mir::Place::local(storage)
                .with_projection(mir::Projection::Deref)
                .with_projection(mir::Projection::Index { index });
            lower.builder.store(place, element);
        }

        Ok(builder)
    }

    /// Read the allocation capacity without moving its owner.
    fn capacity(&self, lower: &mut FunctionLowerer<'_, '_, '_>) -> mir::Value {
        // borrow the allocation through its stored owner
        let ty = lower.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::Reference::Borrowed,
            lifetime: mir::Lifetime::frame(),
            element: self.uninit,
            access: mir::Access::Readonly,
        });
        let place = mir::Place::local(self.storage).with_projection(mir::Projection::Deref);
        let slice = lower.builder.address(place, ty);

        lower.builder.slice_length(slice)
    }

    /// Append one initialized value, growing storage when it is full.
    pub(in crate::lower) fn push(
        &self,
        value: mir::Value,
        lower: &mut FunctionLowerer<'_, '_, '_>,
    ) {
        // grow geometrically before writing past the allocation
        let capacity = self.capacity(lower);
        let length = lower.builder.local_get(self.length);
        let full = lower
            .builder
            .binary(mir::BinaryOperator::Equal, length, capacity);
        let grow = lower.builder.block();
        let append = lower.builder.block();
        lower.builder.branch(full, grow, append);
        lower.builder.switch_to_block(grow);
        let doubled = lower
            .builder
            .binary(mir::BinaryOperator::Add, capacity, capacity);
        let one = lower.builder.usize_const(1);
        let capacity = lower.builder.binary(mir::BinaryOperator::Add, doubled, one);
        self.resize(capacity, lower);
        lower.builder.jump(append);

        // initialize the next element and advance the count
        lower.builder.switch_to_block(append);
        let place = mir::Place::local(self.storage)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Index { index: length });
        lower.builder.store(place, value);
        let one = lower.builder.usize_const(1);
        let length = lower.builder.binary(mir::BinaryOperator::Add, length, one);
        lower.builder.local_set(self.length, length);
    }

    /// Complete an owned slice whose allocation contains exactly its initialized elements.
    pub(in crate::lower) fn finish(self, lower: &mut FunctionLowerer<'_, '_, '_>) -> mir::Value {
        // trim unused capacity before completing the owned slice
        let capacity = self.capacity(lower);
        let length = lower.builder.local_get(self.length);
        let exact = lower
            .builder
            .binary(mir::BinaryOperator::Equal, length, capacity);
        let trim = lower.builder.block();
        let complete = lower.builder.block();
        lower.builder.branch(exact, complete, trim);
        lower.builder.switch_to_block(trim);
        self.resize(length, lower);
        lower.builder.jump(complete);

        // expose initialized storage at its owned slice representation
        lower.builder.switch_to_block(complete);
        let storage = lower.builder.local_get(self.storage);
        let slice = lower.builder.tree_mut().intern_type(mir::Type::Slice {
            kind: mir::Reference::Unique,
            lifetime: mir::Lifetime::empty(),
            element: self.element,
            access: mir::Access::Mutable,
        });

        lower.builder.new_complete(storage, slice)
    }

    /// Move initialized elements into an allocation of the requested capacity.
    fn resize(&self, capacity: mir::Value, lower: &mut FunctionLowerer<'_, '_, '_>) {
        // allocate the replacement and start at its first element
        let previous = lower.builder.local_get(self.storage);
        let allocation = lower.builder.tree().get(self.storage).ty;
        let replacement =
            lower
                .builder
                .new_slice_uninit(self.uninit, capacity, allocation, self.space);
        let length = lower.builder.local_get(self.length);
        let size = lower.builder.tree_mut().intern_type(mir::Type::Usize);
        let index = lower.builder.local(size, mir::Mutability::Mutable);
        let zero = lower.builder.usize_const(0);
        lower.builder.local_set(index, zero);
        let header = lower.builder.block();
        let body = lower.builder.block();
        let exit = lower.builder.block();
        lower.builder.jump(header);

        // move each initialized element in order
        lower.builder.switch_to_block(header);
        let position = lower.builder.local_get(index);
        let remains = lower
            .builder
            .binary(mir::BinaryOperator::LessThan, position, length);
        lower.builder.branch(remains, body, exit);
        lower.builder.switch_to_block(body);
        let source = mir::Place::value(previous)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Index { index: position });
        let value = lower.load_place(source, self.element);
        let target = mir::Place::value(replacement)
            .with_projection(mir::Projection::Deref)
            .with_projection(mir::Projection::Index { index: position });
        lower.builder.store(target, value);
        let one = lower.builder.usize_const(1);
        let next = lower
            .builder
            .binary(mir::BinaryOperator::Add, position, one);
        lower.builder.local_set(index, next);
        lower.builder.jump(header);

        // release the old allocation after all initialized elements moved
        lower.builder.switch_to_block(exit);
        lower.builder.release(previous);
        lower.builder.local_set(self.storage, replacement);
    }
}
