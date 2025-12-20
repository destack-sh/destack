//! Instruction execution implementation.

use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::{StackPointer, Value};

use super::Interpreter;

impl Interpreter {
    /// Execute a single instruction.
    pub(super) fn execute_instruction(
        &mut self,
        _inst_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> RuntimeResult<()> {
        match instruction {
            // dest = constant value
            mir::Instruction::Const { destination, value } => {
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value.into());
            }

            // dest = left op right
            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let (lhs, rhs) = {
                    let frame = self.current_frame()?;
                    let lhs = frame.get_value(*left)?;
                    let rhs = frame.get_value(*right)?;
                    (lhs, rhs)
                };
                let result = self.execute_binary(*operator, lhs, rhs)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            // dest = op argument
            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let arg = {
                    let frame = self.current_frame()?;
                    frame.get_value(*argument)?
                };
                let result = self.execute_unary(*operator, arg)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            // dest = argument as to_type
            mir::Instruction::Cast {
                destination,
                kind,
                argument,
                to_type,
            } => {
                let arg = {
                    let frame = self.current_frame()?;
                    frame.get_value(*argument)?
                };
                let result = self.execute_cast(*kind, arg, *to_type)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            // dest = function(arguments...)
            mir::Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                self.execute_call(*destination, *function, arguments)?;
            }

            // dest = callee(arguments...) where callee is a function pointer
            mir::Instruction::CallIndirect {
                destination,
                callee,
                arguments,
            } => {
                // get the function pointer value
                let callee_val = {
                    let frame = self.current_frame()?;
                    frame.get_value(*callee)?
                };

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

                // reuse the direct call implementation
                self.execute_call(*destination, function, arguments)?;
            }

            // dest = local variable
            mir::Instruction::LocalGet { destination, local } => {
                let value = {
                    let frame = self.current_frame()?;
                    frame.get_local(*local)?
                };
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            // local variable = value
            mir::Instruction::LocalSet { local, value } => {
                let val = {
                    let frame = self.current_frame()?;
                    frame.get_value(*value)?
                };
                let frame = self.current_frame_mut()?;
                frame.set_local(*local, val);
            }

            // dest = &global (get pointer to mutable global)
            mir::Instruction::GlobalAddr {
                destination,
                global,
            } => {
                // return a global pointer that can be used with load/store
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::GlobalPointer(*global));
            }

            // dest = global constant value (load immutable global directly)
            mir::Instruction::GlobalConst {
                destination,
                global,
            } => {
                let value =
                    self.globals.get(*global).cloned().ok_or_else(|| {
                        self.make_error(Error::UndefinedGlobal { global: *global })
                    })?;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            // dest = *pointer
            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                let ptr = {
                    let frame = self.current_frame()?;
                    frame.get_value(*pointer)?
                };

                let value = match ptr {
                    Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.managed_heap.get(handle) {
                            cell.slots.first().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::RawPointer(ptr) => {
                        if ptr.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.raw_heap.get(ptr) {
                            cell.slots.first().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::StackPointer(sp) => self.load_stack_slot(sp, 0)?,
                    Value::GlobalPointer(global) => self
                        .globals
                        .get(global)
                        .cloned()
                        .ok_or_else(|| self.make_error(Error::UndefinedGlobal { global }))?,
                    _ => {
                        return Err(self.make_error(Error::InvalidPointerType {
                            actual: format!("{ptr:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            // *pointer = value
            mir::Instruction::Store { pointer, value } => {
                let (ptr, val) = {
                    let frame = self.current_frame()?;
                    let ptr = frame.get_value(*pointer)?;
                    let val = frame.get_value(*value)?;
                    (ptr, val)
                };

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
                let aggregate = {
                    let frame = self.current_frame()?;
                    frame.get_value(*aggregate)?
                };

                // extract field from aggregate or heap cell
                let value = match aggregate {
                    Value::Aggregate(fields) => {
                        fields.get(*index as usize).cloned().ok_or_else(|| {
                            self.make_error(Error::InvalidFieldAccess {
                                index: *index,
                                field_count: fields.len(),
                            })
                        })?
                    }
                    Value::ManagedReference(handle) => {
                        if handle.is_null() {
                            return Err(self.make_error(Error::NullPointerDereference));
                        }
                        if let Some(cell) = self.managed_heap.get(handle) {
                            cell.slots.get(*index as usize).cloned().ok_or_else(|| {
                                self.make_error(Error::InvalidFieldAccess {
                                    index: *index,
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
                            cell.slots.get(*index as usize).cloned().ok_or_else(|| {
                                self.make_error(Error::InvalidFieldAccess {
                                    index: *index,
                                    field_count: cell.slots.len(),
                                })
                            })?
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::StackPointer(sp) => self.load_stack_slot(sp, *index as usize)?,
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "aggregate".to_string(),
                            actual: format!("{aggregate:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            // dest = aggregate with field[index] = value
            mir::Instruction::FieldSet {
                destination,
                aggregate,
                index,
                value,
            } => {
                let (aggregate, value) = {
                    let frame = self.current_frame()?;
                    let agg = frame.get_value(*aggregate)?;
                    let val = frame.get_value(*value)?;
                    (agg, val)
                };

                // create new aggregate with one field replaced
                let result = match aggregate {
                    Value::Aggregate(mut fields) => {
                        if (*index as usize) < fields.len() {
                            fields[*index as usize] = value;
                        } else {
                            return Err(self.make_error(Error::InvalidFieldAccess {
                                index: *index,
                                field_count: fields.len(),
                            }));
                        }
                        Value::Aggregate(fields)
                    }
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "aggregate".to_string(),
                            actual: format!("{aggregate:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            // dest = array[index]
            mir::Instruction::ElementGet {
                destination,
                array,
                index,
            } => {
                let (arr, idx) = {
                    let frame = self.current_frame()?;
                    let arr = frame.get_value(*array)?;
                    let idx = frame.get_value(*index)?;
                    (arr, idx)
                };
                let idx_val = idx.as_uint().unwrap_or(0);

                // extract element at dynamic index
                let value = match arr {
                    Value::Aggregate(elements) => {
                        elements.get(idx_val as usize).cloned().ok_or_else(|| {
                            self.make_error(Error::InvalidArrayAccess {
                                index: idx_val,
                                length: elements.len() as u64,
                            })
                        })?
                    }
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "array".to_string(),
                            actual: format!("{arr:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            // dest = array with [index] = value
            mir::Instruction::ElementSet {
                destination,
                array,
                index,
                value,
            } => {
                let (array, idx, val) = {
                    let frame = self.current_frame()?;
                    let array = frame.get_value(*array)?;
                    let idx = frame.get_value(*index)?;
                    let value = frame.get_value(*value)?;
                    (array, idx, value)
                };
                let idx_val = idx.as_uint().unwrap_or(0);

                // create new array with one element replaced
                let result = match array {
                    Value::Aggregate(mut elements) => {
                        if (idx_val as usize) < elements.len() {
                            elements[idx_val as usize] = val;
                        } else {
                            return Err(self.make_error(Error::InvalidArrayAccess {
                                index: idx_val,
                                length: elements.len() as u64,
                            }));
                        }
                        Value::Aggregate(elements)
                    }
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "array".to_string(),
                            actual: format!("{array:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            // dest = new gc-managed heap cell
            mir::Instruction::ManagedAlloc {
                destination,
                layout: _,
            } => {
                if self.managed_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let handle = self.managed_heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            // dest = new gc-managed array with length slots
            mir::Instruction::ManagedAllocArray {
                destination,
                element: _,
                length,
            } => {
                if self.managed_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let length = {
                    let frame = self.current_frame()?;
                    let len = frame.get_value(*length)?;
                    len.as_uint().unwrap_or(0) as usize
                };

                let handle = self.managed_heap.allocate_with_slots(length);
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            // dest = new manually-managed heap cell (raw pointer)
            mir::Instruction::RawAlloc {
                destination,
                layout: _,
            } => {
                if self.raw_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let ptr = self.raw_heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::RawPointer(ptr));
            }

            // deallocate raw pointer
            mir::Instruction::RawFree { pointer } => {
                let ptr = {
                    let frame = self.current_frame()?;
                    frame.get_value(*pointer)?
                };

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
                let frame_depth = self.call_stack.len() - 1;
                let slot = {
                    let frame = self.current_frame_mut()?;
                    frame.allocate_stack_cell()
                };
                let sp = StackPointer::new(frame_depth, slot);
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::StackPointer(sp));
            }

            // intrinsic call
            mir::Instruction::Intrinsic {
                destination,
                intrinsic,
                arguments,
                ordering,
            } => {
                let result = self.execute_intrinsic(*intrinsic, arguments, *ordering)?;
                if let Some(dest) = destination {
                    let frame = self.current_frame_mut()?;
                    frame.set_value(*dest, result);
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
            .cloned()
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
