//! MIR interpreter implementation.

use std::collections::HashMap;

use destack_mir as mir;
use destack_source::StringPool;

use crate::diagnostic::{Error, Result};
use crate::memory::{Heap, Value};

use super::Frame;

/// External function type.
pub type ExternalFn = Box<dyn Fn(&[Value]) -> Result<Value> + Send + Sync>;

/// MIR interpreter.
///
/// Executes MIR functions by walking the instruction tree. Maintains a call
/// stack and a heap for managed allocations.
#[allow(dead_code)]
pub struct Interpreter {
    /// The MIR tree being executed.
    tree: mir::NodeTree,
    /// String pool for names.
    strings: StringPool,
    /// The managed heap.
    heap: Heap,
    /// External function handlers.
    externals: HashMap<String, ExternalFn>,
    /// Maximum call stack depth.
    max_stack_depth: usize,
}

impl std::fmt::Debug for Interpreter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interpreter")
            .field("heap", &self.heap)
            .field("externals", &format!("<{} handlers>", self.externals.len()))
            .field("max_stack_depth", &self.max_stack_depth)
            .finish_non_exhaustive()
    }
}

impl Interpreter {
    /// Create a new interpreter.
    pub fn new(tree: mir::NodeTree, strings: StringPool) -> Self {
        Self {
            tree,
            strings,
            heap: Heap::new(),
            externals: HashMap::new(),
            max_stack_depth: 1024,
        }
    }

    /// Register an external function handler.
    pub fn register_external<F>(&mut self, name: &str, handler: F)
    where
        F: Fn(&[Value]) -> Result<Value> + Send + Sync + 'static,
    {
        self.externals.insert(name.to_string(), Box::new(handler));
    }

    /// Set the maximum call stack depth.
    pub fn set_max_stack_depth(&mut self, depth: usize) {
        self.max_stack_depth = depth;
    }

    /// Execute a function by name.
    pub fn call_by_name(&mut self, name: &str, args: &[Value]) -> Result<Value> {
        let func_id = self
            .tree
            .iter_nodes::<mir::Function>()
            .find(|(_, f)| self.strings.get(f.name) == name)
            .map(|(id, _)| id)
            .ok_or_else(|| Error::ExternalFunctionNotFound {
                name: name.to_string(),
            })?;

        self.call(func_id, args)
    }

    /// Execute a function by id.
    pub fn call(
        &mut self,
        func_id: mir::LocalNodeId<mir::Function>,
        args: &[Value],
    ) -> Result<Value> {
        let function = self.tree.get(func_id);

        // Check for external function
        if function.is_external {
            let name = self.strings.get(function.name).to_string();
            let handler = self
                .externals
                .get(&name)
                .ok_or(Error::ExternalFunctionNotFound { name })?;
            return handler(args);
        }

        // Create initial frame
        let entry_block = function
            .entry
            .ok_or(Error::UndefinedFunction { function: func_id })?;
        let mut frame = Frame::new(func_id, entry_block);

        // Bind parameters
        for (i, param) in function.parameters.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Void);
            frame.set_value(param.value, value);
        }

        // Execute
        self.execute_frame(&mut frame)
    }

    /// Execute a single frame until it returns.
    fn execute_frame(&mut self, frame: &mut Frame) -> Result<Value> {
        loop {
            // Clone instruction ids and terminator to avoid borrow conflicts
            let block = self.tree.get(frame.current_block);
            let instruction_ids = block.instructions.clone();
            let terminator = block.terminator.clone();

            // Execute instructions
            for inst_id in instruction_ids {
                let instruction = self.tree.get(inst_id).clone();
                self.execute_instruction(frame, &instruction)?;
            }

            // Execute terminator
            match &terminator {
                mir::Terminator::Return { value } => {
                    return Ok(value
                        .map(|v| frame.get_value(v))
                        .transpose()?
                        .unwrap_or(Value::Void));
                }
                mir::Terminator::Jump { target, arguments } => {
                    let args: Vec<Value> = arguments
                        .iter()
                        .map(|v| frame.get_value(*v))
                        .collect::<Result<_>>()?;
                    self.jump_to_block(frame, *target, &args)?;
                }
                mir::Terminator::Branch {
                    condition,
                    then_target,
                    then_arguments,
                    else_target,
                    else_arguments,
                } => {
                    let cond = frame.get_value(*condition)?;
                    if cond.is_truthy() {
                        let args: Vec<Value> = then_arguments
                            .iter()
                            .map(|v| frame.get_value(*v))
                            .collect::<Result<_>>()?;
                        self.jump_to_block(frame, *then_target, &args)?;
                    } else {
                        let args: Vec<Value> = else_arguments
                            .iter()
                            .map(|v| frame.get_value(*v))
                            .collect::<Result<_>>()?;
                        self.jump_to_block(frame, *else_target, &args)?;
                    }
                }
                mir::Terminator::Switch {
                    value,
                    default,
                    default_arguments,
                    cases,
                } => {
                    let val = frame.get_value(*value)?;
                    let int_val = val.as_int().unwrap_or(0);

                    let (target, args) = cases
                        .iter()
                        .find(|c| c.value == int_val)
                        .map(|c| {
                            let args: Result<Vec<Value>> =
                                c.arguments.iter().map(|v| frame.get_value(*v)).collect();
                            (c.target, args)
                        })
                        .unwrap_or_else(|| {
                            let args: Result<Vec<Value>> = default_arguments
                                .iter()
                                .map(|v| frame.get_value(*v))
                                .collect();
                            (*default, args)
                        });

                    self.jump_to_block(frame, target, &args?)?;
                }
                mir::Terminator::Unreachable => {
                    return Err(Error::Unreachable);
                }
            }
        }
    }

    /// Jump to a block with arguments.
    fn jump_to_block(
        &self,
        frame: &mut Frame,
        target: mir::LocalNodeId<mir::Block>,
        args: &[Value],
    ) -> Result<()> {
        let block = self.tree.get(target);

        // Bind block parameters
        for (i, param) in block.parameters.iter().enumerate() {
            let value = args.get(i).cloned().unwrap_or(Value::Void);
            frame.set_value(param.value, value);
        }

        frame.current_block = target;
        Ok(())
    }

    /// Execute a single instruction.
    fn execute_instruction(
        &mut self,
        frame: &mut Frame,
        instruction: &mir::Instruction,
    ) -> Result<()> {
        match instruction {
            mir::Instruction::Constant { destination, value } => {
                frame.set_value(*destination, value.into());
            }

            mir::Instruction::Binary {
                destination,
                operator,
                left,
                right,
            } => {
                let lhs = frame.get_value(*left)?;
                let rhs = frame.get_value(*right)?;
                let result = self.execute_binary(*operator, lhs, rhs)?;
                frame.set_value(*destination, result);
            }

            mir::Instruction::Unary {
                destination,
                operator,
                argument,
            } => {
                let arg = frame.get_value(*argument)?;
                let result = self.execute_unary(*operator, arg)?;
                frame.set_value(*destination, result);
            }

            mir::Instruction::Cast {
                destination,
                kind,
                argument,
                to_type: _,
            } => {
                let arg = frame.get_value(*argument)?;
                let result = self.execute_cast(*kind, arg)?;
                frame.set_value(*destination, result);
            }

            mir::Instruction::Call {
                destination,
                function,
                arguments,
            } => {
                let args: Vec<Value> = arguments
                    .iter()
                    .map(|v| frame.get_value(*v))
                    .collect::<Result<_>>()?;
                let result = self.call(*function, &args)?;
                if let Some(dest) = destination {
                    frame.set_value(*dest, result);
                }
            }

            // Memory operations
            mir::Instruction::Load {
                destination,
                pointer,
            } => {
                let ptr = frame.get_value(*pointer)?;
                // TODO: Implement actual memory loads
                frame.set_value(*destination, ptr);
            }

            mir::Instruction::Store {
                pointer: _,
                value: _,
            } => {
                // TODO: Implement actual memory stores
            }

            // Local variables
            mir::Instruction::LocalGet { destination, local } => {
                let value = frame.get_local(*local)?;
                frame.set_value(*destination, value);
            }

            mir::Instruction::LocalSet { local, value } => {
                let val = frame.get_value(*value)?;
                frame.set_local(*local, val);
            }

            // Allocation instructions
            mir::Instruction::ManagedAllocate {
                destination,
                layout: _,
            } => {
                let handle = self.heap.allocate();
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            mir::Instruction::ManagedAllocateArray {
                destination,
                element: _,
                length: _,
            } => {
                let handle = self.heap.allocate();
                frame.set_value(*destination, Value::ManagedReference(handle));
            }

            mir::Instruction::RawAllocate {
                destination,
                layout: _,
            } => {
                frame.set_value(*destination, Value::RawPointer(0));
            }

            mir::Instruction::RawFree { pointer: _ } => {
                // TODO: Implement raw memory free
            }

            mir::Instruction::StackAllocate {
                destination,
                layout: _,
            } => {
                frame.set_value(*destination, Value::RawPointer(0));
            }

            mir::Instruction::Drop { value: _ } => {
                // TODO: Implement drop semantics
            }

            _ => {
                // Other instructions not yet implemented
            }
        }

        Ok(())
    }

    /// Execute a binary operation.
    fn execute_binary(&self, op: mir::BinaryOperator, lhs: Value, rhs: Value) -> Result<Value> {
        use mir::BinaryOperator::*;

        match (op, &lhs, &rhs) {
            // Integer arithmetic
            (Add, Value::Int { value: a, width }, Value::Int { value: b, .. }) => Ok(Value::Int {
                value: a.wrapping_add(*b),
                width: *width,
            }),
            (Subtract, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                Ok(Value::Int {
                    value: a.wrapping_sub(*b),
                    width: *width,
                })
            }
            (Multiply, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                Ok(Value::Int {
                    value: a.wrapping_mul(*b),
                    width: *width,
                })
            }
            (SignedDivide, Value::Int { value: a, width }, Value::Int { value: b, .. }) => {
                if *b == 0 {
                    return Err(Error::DivisionByZero);
                }
                Ok(Value::Int {
                    value: a.wrapping_div(*b),
                    width: *width,
                })
            }

            // Integer comparison
            (Equal, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Ok(Value::Boolean(a == b))
            }
            (NotEqual, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Ok(Value::Boolean(a != b))
            }
            (SignedLessThan, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Ok(Value::Boolean(a < b))
            }
            (SignedLessEqual, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Ok(Value::Boolean(a <= b))
            }
            (SignedGreaterThan, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Ok(Value::Boolean(a > b))
            }
            (SignedGreaterEqual, Value::Int { value: a, .. }, Value::Int { value: b, .. }) => {
                Ok(Value::Boolean(a >= b))
            }

            // Boolean operations
            (And, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(*a && *b)),
            (Or, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(*a || *b)),
            (Xor, Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(*a ^ *b)),

            // Float arithmetic
            (FloatAdd, Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a + b)),
            (FloatSubtract, Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a - b)),
            (FloatMultiply, Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a * b)),
            (FloatDivide, Value::Float64(a), Value::Float64(b)) => Ok(Value::Float64(a / b)),

            _ => Ok(Value::Void),
        }
    }

    /// Execute a unary operation.
    fn execute_unary(&self, op: mir::UnaryOperator, arg: Value) -> Result<Value> {
        use mir::UnaryOperator::*;

        match (op, &arg) {
            (Negate, Value::Int { value, width }) => Ok(Value::Int {
                value: value.wrapping_neg(),
                width: *width,
            }),
            (FloatNegate, Value::Float64(f)) => Ok(Value::Float64(-f)),
            (Not, Value::Boolean(b)) => Ok(Value::Boolean(!b)),
            _ => Ok(Value::Void),
        }
    }

    /// Execute a cast operation.
    fn execute_cast(&self, kind: mir::CastKind, arg: Value) -> Result<Value> {
        use mir::CastKind::*;

        match kind {
            Bitcast => Ok(arg),
            Truncate => Ok(arg),
            ZeroExtend | SignExtend => Ok(arg),
            FloatToSignedInt => {
                if let Value::Float64(f) = arg {
                    Ok(Value::Int {
                        value: f as i64,
                        width: 64,
                    })
                } else {
                    Ok(arg)
                }
            }
            SignedIntToFloat => {
                if let Value::Int { value, .. } = arg {
                    Ok(Value::Float64(value as f64))
                } else {
                    Ok(arg)
                }
            }
            _ => Ok(arg),
        }
    }

    /// Get a reference to the heap.
    pub fn heap(&self) -> &Heap {
        &self.heap
    }

    /// Get a mutable reference to the heap.
    pub fn heap_mut(&mut self) -> &mut Heap {
        &mut self.heap
    }
}
