//! Instruction execution implementation.

use destack_mir as mir;

use crate::diagnostic::{Error, RuntimeResult};
use crate::memory::Value;

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

            mir::Instruction::GlobalGet { destination, global } => {
                let _ = (destination, global);
                return Err(self.make_error_at(
                    Error::UnsupportedInstruction { name: "global_get".to_string() },
                    inst_id,
                ));
            }

            mir::Instruction::GlobalSet { global, value } => {
                let _ = (global, value);
                return Err(self.make_error_at(
                    Error::UnsupportedInstruction { name: "global_set".to_string() },
                    inst_id,
                ));
            }

            mir::Instruction::Load { destination, pointer } => {
                let ptr = {
                    let frame = self.current_frame()?;
                    frame.get_value(*pointer)?
                };

                let value = match ptr {
                    Value::ManagedReference(handle) => {
                        if let Some(cell) = self.heap.get(handle) {
                            cell.slots.first().cloned().unwrap_or(Value::Void)
                        } else {
                            return Err(self.make_error(Error::InvalidHeapHandle));
                        }
                    }
                    _ => ptr,
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

                if let Value::ManagedReference(handle) = ptr {
                    if let Some(cell) = self.heap.get_mut(handle) {
                        if cell.slots.is_empty() {
                            cell.slots.push(val);
                        } else {
                            cell.slots[0] = val;
                        }
                    } else {
                        return Err(self.make_error(Error::InvalidHeapHandle));
                    }
                }
            }

            mir::Instruction::ExtractField {
                destination,
                aggregate,
                index,
            } => {
                let agg = {
                    let frame = self.current_frame()?;
                    frame.get_value(*aggregate)?
                };

                let value = match agg {
                    Value::Aggregate(fields) => {
                        fields.get(*index as usize).cloned().ok_or_else(|| {
                            self.make_error(Error::InvalidFieldAccess {
                                index: *index,
                                field_count: fields.len(),
                            })
                        })?
                    }
                    Value::ManagedReference(handle) => {
                        if let Some(cell) = self.heap.get(handle) {
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
                    _ => {
                        return Err(self.make_error(Error::TypeMismatch {
                            expected: "aggregate".to_string(),
                            actual: format!("{agg:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, value);
            }

            mir::Instruction::InsertField {
                destination,
                aggregate,
                index,
                value,
            } => {
                let (agg, val) = {
                    let frame = self.current_frame()?;
                    let agg = frame.get_value(*aggregate)?;
                    let val = frame.get_value(*value)?;
                    (agg, val)
                };

                let result = match agg {
                    Value::Aggregate(mut fields) => {
                        if (*index as usize) < fields.len() {
                            fields[*index as usize] = val;
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
                            actual: format!("{agg:?}"),
                        }));
                    }
                };

                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, result);
            }

            mir::Instruction::ExtractElement {
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

            mir::Instruction::InsertElement {
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

            mir::Instruction::ManagedAllocate { destination, layout: _ } => {
                if self.heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let handle = self.heap.allocate();
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            mir::Instruction::ManagedAllocateArray {
                destination,
                element: _,
                length,
            } => {
                if self.heap.cell_count() >= self.options.max_heap_cells {
                    return Err(self.make_error(Error::AllocationFailed));
                }

                let len_val = {
                    let frame = self.current_frame()?;
                    let len = frame.get_value(*length)?;
                    len.as_uint().unwrap_or(0) as usize
                };

                let handle = self.heap.allocate_with_slots(len_val);
                self.statistics.heap_allocations += 1;
                let frame = self.current_frame_mut()?;
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            mir::Instruction::Drop { value: _ } => {
                // drop is a no-op (gc handles cleanup)
            }

            // unsupported instructions
            mir::Instruction::RawAllocate { .. }
            | mir::Instruction::RawFree { .. }
            | mir::Instruction::StackAllocate { .. }
            | mir::Instruction::CallIndirect { .. } => {
                let name = format!("{instruction:?}");
                let name = name.split_whitespace().next().unwrap_or("unknown").to_string();
                return Err(self.make_error_at(
                    Error::UnsupportedInstruction { name },
                    inst_id,
                ));
            }
        }

        Ok(())
    }
}
