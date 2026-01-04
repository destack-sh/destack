use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::{StackPointer, Value};

use super::Interpreter;

impl Interpreter {
    /// Execute a single instruction.
    pub(super) fn execute_instruction(
        &mut self,
        inst_id: mir::LocalNodeId<mir::Instruction>,
    ) -> RuntimeResult<()> {
        // we use a match on a reference, extracting scalar data before mutation
        match self.tree.get(inst_id) {
            // dest = constant value
            mir::Instruction::Const { destination, value } => {
                let (dest, val) = (*destination, Value::from(value));
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, val);
            }

            // dest = left op right
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let (dest, op, l, r) = (*destination, *operator, *left, *right);
                let (lhs, rhs) = {
                    let frame = self.current_frame()?;
                    (frame.get_value(l)?, frame.get_value(r)?)
                };
                let result = self.execute_binary(op, lhs, rhs)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, result);
            }

            // dest = op argument
            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let (dest, op, arg_id) = (*destination, *operator, *argument);
                let arg = self.current_frame()?.get_value(arg_id)?;
                let result = self.execute_unary(op, arg)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, result);
            }

            // dest = argument as to_type
            mir::Instruction::Cast {
                destination,
                operator,
                argument,
                to_type,
            } => {
                let (dest, op, arg_id, ty) = (*destination, *operator, *argument, *to_type);
                let arg = self.current_frame()?.get_value(arg_id)?;
                let result = self.execute_cast(op, arg, ty)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, result);
            }

            // dest = function(arguments...)
            mir::Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                let (dest, func) = (*destination, *function);
                let args: Vec<_> = arguments.to_vec();
                self.execute_call(dest, func, &args)?;
            }

            // dest = callee(arguments...) where callee is a function pointer
            mir::Instruction::CallIndirect {
                destination,
                callee,
                arguments,
            } => {
                let (dest, callee_id) = (*destination, *callee);
                let args: Vec<_> = arguments.to_vec();
                // get the function pointer value
                let callee_val = self.current_frame()?.get_value(callee_id)?;

                // find function
                let function = match callee_val {
                    Value::FunctionPointer(func_id) => func_id,
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "function_pointer".to_string(),
                            actual: format!("{callee_val:?}"),
                        }));
                    }
                };

                self.execute_call(dest, function, &args)?;
            }

            // dest = local variable
            mir::Instruction::LocalGet { destination, local } => {
                let (dest, local_id) = (*destination, *local);
                let value = self.current_frame()?.get_local(local_id)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, value);
            }

            // local variable = value
            mir::Instruction::LocalSet { local, value } => {
                let (local_id, val_id) = (*local, *value);
                let val = self.current_frame()?.get_value(val_id)?;
                let frame = self.current_frame_mut()?;
                frame.set_local(local_id, val);
            }

            // dest = &global (get pointer to mutable global)
            mir::Instruction::GlobalAddr {
                destination,
                global,
            } => {
                let (dest, global_id) = (*destination, *global);
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, Value::GlobalPointer(global_id));
            }

            // dest = global constant value (load immutable global directly)
            mir::Instruction::GlobalConst {
                destination,
                global,
            } => {
                let (dest, global_id) = (*destination, *global);
                let value =
                    self.globals.get(global_id).copied().ok_or_else(|| {
                        self.make_error(Error::UndefinedGlobal { global: global_id })
                    })?;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, value);
            }

            // dest = *pointer
            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                let (dest, ptr_id) = (*destination, *pointer);
                self.statistics.loads += 1;
                let ptr = self.current_frame()?.get_value(ptr_id)?;

                let value = match ptr {
                    Value::ManagedReference(handle) | Value::Aggregate(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.managed_heap.get(handle) {
                            cell.slots.first().copied().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.raw_heap.get(ptr) {
                            cell.slots.first().copied().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::StackPointer(sp) => self.load_stack_slot(sp, 0)?,
                    Value::GlobalPointer(global) => *self
                        .globals
                        .get(global)
                        .ok_or_else(|| self.make_error(Error::UndefinedGlobal { global }))?,
                    _ => {
                        return Err(self.make_error(Error::InvalidPointerType {
                            actual: format!("{ptr:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(dest, value);
            }

            // *pointer = value
            mir::Instruction::Store { pointer, value } => {
                let (ptr_id, val_id) = (*pointer, *value);
                self.statistics.stores += 1;
                let frame = self.current_frame()?;
                let (ptr, val) = (frame.get_value(ptr_id)?, frame.get_value(val_id)?);

                match ptr {
                    Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.managed_heap.get_mut(handle) {
                            if cell.slots.is_empty() {
                                cell.slots.push(val);
                            } else {
                                cell.slots[0] = val;
                            }
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.raw_heap.get_mut(ptr) {
                            if cell.slots.is_empty() {
                                cell.slots.push(val);
                            } else {
                                cell.slots[0] = val;
                            }
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::StackPointer(sp) => {
                        self.store_stack_slot(sp, 0, val)?;
                    }
                    Value::GlobalPointer(global) => {
                        // check mutability
                        let global_def = self.tree.get(global);
                        if !global_def.is_mutable() {
                            return Err(self.make_error(Error::ImmutableGlobalWrite { global }));
                        }
                        self.globals.set(global, val);
                    }
                    _ => {
                        return Err(self.make_error(Error::InvalidPointerType {
                            actual: format!("{ptr:?}"),
                        }));
                    }
                }
            }

            // dest = aggregate.field[index]
            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let (dest, agg_id, idx) = (*destination, *aggregate, *index);
                let agg = self.current_frame()?.get_value(agg_id)?;

                // extract field from aggregate or heap cell
                let value = match agg {
                    Value::Aggregate(handle) | Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.managed_heap.get(handle) {
                            cell.slots.get(idx as usize).copied().ok_or_else(|| {
                                self.make_error(Error::InvalidFieldAccess {
                                    index: idx,
                                    field_count: cell.slots.len(),
                                })
                            })?
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.raw_heap.get(ptr) {
                            cell.slots.get(idx as usize).copied().ok_or_else(|| {
                                self.make_error(Error::InvalidFieldAccess {
                                    index: idx,
                                    field_count: cell.slots.len(),
                                })
                            })?
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::StackPointer(sp) => self.load_stack_slot(sp, idx as usize)?,
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "aggregate".to_string(),
                            actual: format!("{agg:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(dest, value);
            }

            // dest = aggregate with field[index] = value
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                let (dest, agg_id, idx, val_id) = (*destination, *aggregate, *index, *value);
                let frame = self.current_frame()?;
                let (agg, val) = (frame.get_value(agg_id)?, frame.get_value(val_id)?);

                // create new aggregate or update referenced storage
                let result = match agg {
                    // set field on aggregate or managed reference (both use heap)
                    Value::Aggregate(handle) | Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        let mut invalid_field_count = None;
                        if let Some(cell) = self.managed_heap.get_mut(handle) {
                            if (idx as usize) < cell.slots.len() {
                                cell.slots[idx as usize] = val;
                            } else {
                                invalid_field_count = Some(cell.slots.len());
                            }
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                        if let Some(field_count) = invalid_field_count {
                            return Err(self.make_error(Error::InvalidFieldAccess {
                                index: idx,
                                field_count,
                            }));
                        }
                        // return the same aggregate/reference (in-place mutation)
                        agg
                    }
                    // set field indirectly on raw pointer
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        let mut invalid_field_count = None;
                        if let Some(cell) = self.raw_heap.get_mut(ptr) {
                            if (idx as usize) < cell.slots.len() {
                                cell.slots[idx as usize] = val;
                            } else {
                                invalid_field_count = Some(cell.slots.len());
                            }
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                        if let Some(field_count) = invalid_field_count {
                            return Err(self.make_error(Error::InvalidFieldAccess {
                                index: idx,
                                field_count,
                            }));
                        }
                        Value::RawPointer(ptr)
                    }
                    // set field indirectly on stack pointer
                    Value::StackPointer(sp) => {
                        self.store_stack_slot(sp, idx as usize, val)?;
                        Value::StackPointer(sp)
                    }
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "aggregate".to_string(),
                            actual: format!("{agg:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(dest, result);
            }

            // dest = array[index]
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let (dest, arr_id, idx_id) = (*destination, *array, *index);
                let frame = self.current_frame()?;
                let (arr, idx) = (frame.get_value(arr_id)?, frame.get_value(idx_id)?);
                let idx_val = idx.as_uint().unwrap_or(0);

                // extract element at dynamic index
                let value = match arr {
                    // extract element from aggregate or managed reference (both use heap)
                    Value::Aggregate(handle) | Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.managed_heap.get(handle) {
                            cell.slots.get(idx_val as usize).copied().ok_or_else(|| {
                                self.make_error(Error::InvalidArrayAccess {
                                    index: idx_val,
                                    length: cell.slots.len() as u64,
                                })
                            })?
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    // extract element indirectly from raw pointer
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.raw_heap.get(ptr) {
                            cell.slots.get(idx_val as usize).copied().ok_or_else(|| {
                                self.make_error(Error::InvalidArrayAccess {
                                    index: idx_val,
                                    length: cell.slots.len() as u64,
                                })
                            })?
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    // extract element directly from aggregate
                    Value::StackPointer(sp) => self.load_stack_slot(sp, idx_val as usize)?,
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "array".to_string(),
                            actual: format!("{arr:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(dest, value);
            }

            // dest = array with [index] = value
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                let (dest, arr_id, idx_id, val_id) = (*destination, *array, *index, *value);
                let frame = self.current_frame()?;
                let (arr, idx, val) = (
                    frame.get_value(arr_id)?,
                    frame.get_value(idx_id)?,
                    frame.get_value(val_id)?,
                );
                let idx_val = idx.as_uint().unwrap_or(0);

                // create new array or update referenced storage
                let result = match arr {
                    // set element on aggregate or managed reference (both use heap)
                    Value::Aggregate(handle) | Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        let mut invalid_length = None;
                        if let Some(cell) = self.managed_heap.get_mut(handle) {
                            if (idx_val as usize) < cell.slots.len() {
                                cell.slots[idx_val as usize] = val;
                            } else {
                                invalid_length = Some(cell.slots.len() as u64);
                            }
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                        if let Some(length) = invalid_length {
                            return Err(self.make_error(Error::InvalidArrayAccess {
                                index: idx_val,
                                length,
                            }));
                        }
                        // return the same array/reference (in-place mutation)
                        arr
                    }
                    // set element indirectly on raw pointer
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        let mut invalid_length = None;
                        if let Some(cell) = self.raw_heap.get_mut(ptr) {
                            if (idx_val as usize) < cell.slots.len() {
                                cell.slots[idx_val as usize] = val;
                            } else {
                                invalid_length = Some(cell.slots.len() as u64);
                            }
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                        if let Some(length) = invalid_length {
                            return Err(self.make_error(Error::InvalidArrayAccess {
                                index: idx_val,
                                length,
                            }));
                        }
                        Value::RawPointer(ptr)
                    }
                    // set element indirectly on stack pointer
                    Value::StackPointer(sp) => {
                        self.store_stack_slot(sp, idx_val as usize, val)?;
                        Value::StackPointer(sp)
                    }
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "array".to_string(),
                            actual: format!("{arr:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(dest, result);
            }

            // dest = new gc-managed heap cell
            mir::Instruction::ManagedAlloc {
                destination,
                layout: _,
            } => {
                let dest = *destination;
                if self.managed_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let handle = self.managed_heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, Value::ManagedReference(handle));
            }

            // dest = new gc-managed array with length slots
            mir::Instruction::ManagedAllocArray {
                destination,
                element: _,
                length,
            } => {
                let (dest, len_id) = (*destination, *length);
                if self.managed_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let len_val = self.current_frame()?.get_value(len_id)?;
                let length = len_val.as_uint().unwrap_or(0) as usize;

                let handle = self.managed_heap.allocate_with_slots(length);
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, Value::ManagedReference(handle));
            }

            // dest = new manually-managed heap cell (raw pointer)
            mir::Instruction::RawAlloc {
                destination,
                layout: _,
            } => {
                let dest = *destination;
                if self.raw_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let ptr = self.raw_heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, Value::RawPointer(ptr));
            }

            // deallocate raw pointer
            mir::Instruction::RawFree { pointer } => {
                let ptr_id = *pointer;
                let ptr = self.current_frame()?.get_value(ptr_id)?;

                if let Value::RawPointer(p) = ptr {
                    if !self.raw_heap.free(p) {
                        return Err(self.make_error(Error::InvalidHeapHandle));
                    }
                } else {
                    return Err(self.make_error(Error::TypeMismatch {
                        expected: "raw_pointer".to_string(),
                        actual: format!("{ptr:?}"),
                    }));
                }
            }

            // dest = stack-allocated cell (frame-local storage)
            mir::Instruction::StackAlloc {
                destination,
                layout: _,
            } => {
                let dest = *destination;
                let frame_depth = self.call_stack.len() - 1;
                let slot = self.current_frame_mut()?.allocate_stack_cell();
                let sp = StackPointer::new(frame_depth, slot);
                let frame = self.current_frame_mut()?;
                frame.set_value(dest, Value::StackPointer(sp));
            }

            // intrinsic call
            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
                ordering,
            } => {
                let (dest, intr, ord) = (*destination, *intrinsic, *ordering);
                let args: Vec<_> = arguments.to_vec();
                let result = self.execute_intrinsic(intr, &args, ord)?;
                if let Some(d) = dest {
                    let frame = self.current_frame_mut()?;
                    frame.set_value(d, result);
                }
            }
        }

        Ok(())
    }

    /// Load a value from a stack-allocated cell.
    fn load_stack_slot(&self, sp: StackPointer, slot_index: usize) -> RuntimeResult<Value> {
        let frame = self
            .call_stack
            .get(sp.frame_idx)
            .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;

        let cell = frame
            .get_stack_cell(sp.slot)
            .ok_or_else(|| self.make_error(Error::InvalidHeapHandle))?;

        cell.slots
            .get(slot_index)
            .copied()
            .ok_or_else(|| {
                self.make_error(Error::InvalidFieldAccess {
                    index: slot_index as u32,
                    field_count: cell.slots.len(),
                })
            })
            .or_else(|_| {
                // if no slots yet, return Void (lazy initialization)
                if cell.slots.is_empty() && slot_index == 0 {
                    Ok(Value::Void)
                } else {
                    Err(self.make_error(Error::InvalidFieldAccess {
                        index: slot_index as u32,
                        field_count: cell.slots.len(),
                    }))
                }
            })
    }

    /// Store a value to a stack-allocated cell.
    fn store_stack_slot(
        &mut self,
        sp: StackPointer,
        slot_index: usize,
        value: Value,
    ) -> RuntimeResult<()> {
        let frame = self
            .call_stack
            .get_mut(sp.frame_idx)
            .ok_or_else(|| RuntimeError::new(Error::InvalidHeapHandle))?;

        let cell = frame
            .get_stack_cell_mut(sp.slot)
            .ok_or_else(|| RuntimeError::new(Error::InvalidHeapHandle))?;

        // grow slots if needed (lazy initialization)
        while cell.slots.len() <= slot_index {
            cell.slots.push(Value::Void);
        }
        cell.slots[slot_index] = value;
        Ok(())
    }
}
