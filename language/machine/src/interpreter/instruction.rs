//! Instruction execution implementation.

use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeError, RuntimeResult};
use crate::memory::{StackPointer, Value};

use super::Interpreter;

impl Interpreter {
    /// Execute a single instruction.
    pub(super) fn execute_instruction(
        &mut self,
        inst_id: mir::LocalNodeId<mir::Instruction>,
        instruction: &mir::Instruction,
    ) -> RuntimeResult<()> {
        match instruction {
            mir::Instruction::Constant { destination, value } => {
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value.into());
            }

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

            mir::Instruction::Cast {
                destination,
                kind,
                argument,
                to_type: _,
            } => {
                let arg = {
                    let frame = self.current_frame()?;
                    frame.get_value(*argument)?
                };
                let result = self.execute_cast(*kind, arg)?;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            mir::Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                self.execute_call(*destination, *function, arguments)?;
            }
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

            mir::Instruction::LocalGet { destination, local } => {
                let value = {
                    let frame = self.current_frame()?;
                    frame.get_local(*local)?
                };
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }
            mir::Instruction::LocalSet { local, value } => {
                let val = {
                    let frame = self.current_frame()?;
                    frame.get_value(*value)?
                };
                let frame = self.current_frame_mut()?;
                frame.set_local(*local, val);
            }

            // NOTE #Incomplete: global get/set
            mir::Instruction::GlobalGet {
                destination,
                global,
            } => {
                let _ = (destination, global);
                return Err(self.make_error_at(
                    Error::UnsupportedInstruction {
                        name: "global.get".to_string(),
                    },
                    inst_id,
                ));
            }
            mir::Instruction::GlobalSet { global, value } => {
                let _ = (global, value);
                return Err(self.make_error_at(
                    Error::UnsupportedInstruction {
                        name: "global.set".to_string(),
                    },
                    inst_id,
                ));
            }

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
                        if let Some(cell) = self.managed_heap.get(handle) {
                            cell.slots.first().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::RawPointer(ptr) => {
                        if let Some(cell) = self.raw_heap.get(ptr) {
                            cell.slots.first().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    Value::StackPointer(sp) => self.load_stack_slot(sp, 0)?,
                    _ => {
                        return Err(self.make_error(Error::InvalidPointerType {
                            actual: format!("{:?}", ptr),
                        }))
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            mir::Instruction::Store { pointer, value } => {
                let (ptr, val) = {
                    let frame = self.current_frame()?;
                    let ptr = frame.get_value(*pointer)?;
                    let val = frame.get_value(*value)?;
                    (ptr, val)
                };

                match ptr {
                    Value::ManagedReference(handle) => {
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
                    _ => {
                        return Err(self.make_error(Error::InvalidPointerType {
                            actual: format!("{:?}", ptr),
                        }))
                    }
                }
            }

            mir::Instruction::FieldGet {
                destination,
                aggregate,
                index,
            } => {
                let aggregate = {
                    let frame = self.current_frame()?;
                    frame.get_value(*aggregate)?
                };

                // extract value from aggregate
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

                // insert value into aggregate
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

                // extract value from array
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

                // insert value into array
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

            mir::Instruction::ManagedAlloc {
                destination,
                layout: _,
            } => {
                if self.managed_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                // allocate on managed heap
                let handle = self.managed_heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::ManagedReference(handle));
            }
            mir::Instruction::ManagedAllocArray {
                destination,
                element: _,
                length,
            } => {
                if self.managed_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                // length
                let length = {
                    let frame = self.current_frame()?;
                    let len = frame.get_value(*length)?;
                    len.as_uint().unwrap_or(0) as usize
                };

                // allocate on managed heap
                let handle = self.managed_heap.allocate_with_slots(length);
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            mir::Instruction::RawAlloc {
                destination,
                layout: _,
            } => {
                if self.raw_heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                // allocate on raw heap
                let ptr = self.raw_heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::RawPointer(ptr));
            }
            mir::Instruction::RawFree { pointer } => {
                let ptr = {
                    let frame = self.current_frame()?;
                    frame.get_value(*pointer)?
                };

                // free on raw heap
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

            mir::Instruction::StackAlloc {
                destination,
                layout: _,
            } => {
                let frame_depth = self.call_stack.len() - 1;
                // allocate on stack
                let slot = {
                    let frame = self.current_frame_mut()?;
                    frame.allocate_stack_cell()
                };
                let sp = StackPointer::new(frame_depth, slot);
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::StackPointer(sp));
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
