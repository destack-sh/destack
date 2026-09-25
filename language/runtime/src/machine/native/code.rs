use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use tspp_memory::MemoryMap;
use tspp_native as native;
use tspp_native::abi;
use tspp_program as program;
use tspp_program::{EntryPoint, FunctionId, Program, Value};
use tspp_vm as vm;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::Activation;

use super::trap::{enter, handle_trap};
use super::{Call, Error, Function, Mapping, Stop, Transfer, Unwind};

/// The native stack bytes kept below the stack limit for runtime operations and unwinding.
const NATIVE_STACK_RESERVE_BYTES: usize = 1024 * 1024;

/// Process-local native code table.
#[derive(Debug, Clone)]
pub struct Code {
    /// The executable mapping owning this code.
    mapping: Mapping,
    /// Durable native frame maps used by runtime capture.
    frames: native::CodeMap,
    /// Process-local native functions keyed by Program function id.
    functions: Vec<Option<Function>>,
}

/// Outcome returned by one native execution.
#[derive(Debug)]
pub enum Outcome {
    /// One language-visible execution outcome.
    Program(program::Outcome<Value>),
    /// Canonical execution retained for immediate bytecode continuation.
    Deoptimized,
}

impl Code {
    /// Resolve one mapped Program code image into process-local function pointers.
    pub fn mapped(
        mapping: Mapping,
        program: &Program,
        native: &native::Code,
    ) -> Result<Self, Error> {
        // install the trap handler before any generated code can run
        tspp_signal::register(handle_trap).map_err(|error| Error::Signal {
            signal: error.signal,
            code: error.code,
        })?;

        if native.abi_version != abi::VERSION {
            return Err(Error::AbiVersion {
                expected: abi::VERSION,
                actual: native.abi_version,
            });
        }
        let sections = program.sections();
        let required_alignment = native.alignment.bytes() as usize;
        if !mapping.base().is_multiple_of(required_alignment) {
            return Err(Error::NativeImageMisaligned {
                required: native.alignment.bytes(),
            });
        }
        if mapping.byte_size() < native.bytes(sections).len() {
            return Err(Error::NativeImageRange);
        }
        let mut functions = Vec::with_capacity(native.functions(sections).len());

        // resolve every linked range directly against the executable mapping
        for (index, function) in native.functions(sections).iter().enumerate() {
            let Some(function) = function.get() else {
                functions.push(None);

                continue;
            };
            let body = Self::range(&mapping, function.body.bytes)?;
            let entry = Self::range(&mapping, function.entry.bytes)?;
            if entry.is_empty() {
                return Err(Error::NativeEntryEmpty {
                    function: FunctionId(index as u32),
                });
            }
            let entry_address = entry.start;
            // SAFETY: the native linker emits every entry with the fixed TS++ entry ABI.
            let entry = unsafe { std::mem::transmute::<usize, abi::Entry>(entry_address) };
            let function = Function::new(FunctionId(index as u32), entry)
                .body(body, function.body.bytes.offset);
            functions.push(Some(function));
        }

        Ok(Self {
            mapping,
            frames: native.map(),
            functions,
        })
    }

    /// Return one process-local native function.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions
            .get(function.index())
            .and_then(Option::as_ref)
    }

    /// Resolve one linked code range inside an executable mapping.
    fn range(mapping: &Mapping, range: native::CodeRange) -> Result<std::ops::Range<usize>, Error> {
        let start = range.offset as usize;
        let end = start
            .checked_add(range.byte_len as usize)
            .ok_or(Error::NativeImageRange)?;
        if end > mapping.byte_size() {
            return Err(Error::NativeImageRange);
        }

        Ok(mapping.base() + start..mapping.base() + end)
    }

    /// Return process-local native functions in dense Program id order.
    pub fn functions(&self) -> &[Option<Function>] {
        &self.functions
    }

    /// Resolve one runtime entry name into one entrypoint.
    pub fn entry_by_name(&self, program: &Program, name: &str) -> Result<EntryPoint, Error> {
        let Some(function) = program.function_id_by_name(name) else {
            return Err(Error::EntryNotFound {
                name: name.to_string(),
            });
        };

        Ok(EntryPoint::from(function))
    }

    /// Run one native entrypoint.
    pub fn run<'program, 'runtime, 'memory, 'state>(
        &self,
        program: &'program Program,
        activation: &'program mut program::Activation<'runtime, 'memory, Activation<'state>>,
        functions: *const usize,
        virtuals: *const *const abi::VirtualTable,
        dynamics: *const *const abi::DynamicTable,
        memory: &MemoryMap,
        native_stack: &vm::NativeStack,
        entry: EntryPoint,
        environment: Option<&Value>,
        args: &[Value],
        profile: Option<&'program mut program::Profile>,
        captured: &mut Option<program::ActivationImage>,
    ) -> RuntimeResult<Outcome> {
        let Some(entry) = self.function(entry.function()) else {
            return Err(Error::EntryNotFound {
                name: format!("entry {}", entry.index()),
            }
            .into());
        };

        // flatten exact parameter words for the generated entry ABI
        let function = entry.function;
        let definition = program
            .function(function)
            .ok_or_else(|| Error::EntryNotFound {
                name: format!("entry {}", function.index()),
            })
            .map_err(Box::<RuntimeError>::from)?;
        let parameters = program
            .function_parameters(function)
            .ok_or_else(|| Error::EntryNotFound {
                name: format!("entry {}", function.index()),
            })
            .map_err(Box::<RuntimeError>::from)?;
        let expected = parameters.len() + usize::from(definition.environment().is_some());
        let actual = args.len() + usize::from(environment.is_some());
        if actual != expected {
            return Err(Error::ArgumentCount { expected, actual }.into());
        }
        let types = definition
            .environment()
            .into_iter()
            .chain(parameters.iter().copied());
        let values = environment.into_iter().chain(args);
        let mut arguments = Vec::new();
        for (ty, value) in types.zip(values) {
            arguments.extend_from_slice(program.value_words(ty, value)?);
        }

        // allocate exact result storage and one runtime-owned native call
        let result_type = program
            .function_result(function)
            .ok_or_else(|| Error::EntryNotFound {
                name: format!("entry {}", function.index()),
            })
            .map_err(Box::<RuntimeError>::from)?;
        let result_byte_len = program
            .type_byte_len(result_type)
            .ok_or(Error::TypeMissing { ty: result_type })
            .map_err(Box::<RuntimeError>::from)?;
        let mut result =
            vec![program::Word::ZERO; result_byte_len.div_ceil(program::Word::BYTE_LEN)];
        let mut exit = abi::Exit::new();
        let mut call = Call::new(
            program,
            self.frames,
            self.mapping.range(),
            &self.functions,
            memory,
            native_stack.world_range(),
            activation,
            profile,
        );

        // limit generated frames to the stack above the reserve
        if native_stack.byte_len() <= NATIVE_STACK_RESERVE_BYTES {
            return Err(RuntimeError::Internal {
                message: "a native stack no larger than its reserve".to_string(),
            }
            .boxed());
        }
        let stack_limit = native_stack.bottom() as usize + NATIVE_STACK_RESERVE_BYTES;
        let mut activation = call.activation(functions, virtuals, dynamics, stack_limit, &mut exit);
        let activation_pointer = &raw mut activation;

        // run on the fiber's native stack, containing platform unwinds before the stack switches back
        let execution = enter(activation_pointer, || {
            // SAFETY: the native stack is a mapped world range the fiber holds for the whole call
            unsafe {
                psm::on_stack(native_stack.bottom(), native_stack.byte_len(), || {
                    catch_unwind(AssertUnwindSafe(|| {
                        // SAFETY: the activation lives on this frame for the whole call
                        entry.call(activation_pointer, &arguments, &mut result)
                    }))
                })
            }
        });
        call.set_context(activation.context);

        // reject foreign Rust panics crossing generated code
        let kind = match execution {
            Ok(()) => abi::ExitKind::Completed,
            Err(unwind) if unwind.is::<Unwind>() => match call.take_transfer() {
                Some(Transfer::Error(error)) => return Err(error),
                Some(Transfer::Panic(payload)) => {
                    return Err(Error::Panicked { payload }.into());
                }
                Some(Transfer::Trap(trap)) => return Err(Error::Trapped { trap }.into()),
                Some(Transfer::Retain) => exit.kind,
                None => {
                    return Err(RuntimeError::Internal {
                        message: "native unwind has no retained transfer".to_string(),
                    }
                    .boxed());
                }
            },
            Err(unwind) => resume_unwind(unwind),
        };

        self.outcome_from_exit(
            program,
            result_type,
            kind,
            result,
            exit,
            &mut call,
            captured,
        )
    }

    /// Return one native outcome from native exit code and payloads.
    fn outcome_from_exit(
        &self,
        program: &Program,
        result_type: program::TypeId,
        kind: abi::ExitKind,
        result: Vec<program::Word>,
        exit: abi::Exit,
        call: &mut Call<'_, '_, '_, '_>,
        captured: &mut Option<program::ActivationImage>,
    ) -> RuntimeResult<Outcome> {
        match kind {
            abi::ExitKind::Completed => {
                let value = program.value(result_type, result)?;

                Ok(Outcome::Program(program::Outcome::Completed { value }))
            }
            abi::ExitKind::Cancelled => Ok(Outcome::Program(program::Outcome::Cancelled)),
            abi::ExitKind::Stopped => {
                let frame = self
                    .frames
                    .frame(program.sections(), exit.frame_map)
                    .ok_or_else(|| RuntimeError::Internal {
                        message: "native stop frame map is missing".to_string(),
                    })?;
                let state = program
                    .frame_state(program::FrameStateId(frame.state))
                    .ok_or_else(|| RuntimeError::Internal {
                        message: "native stop frame state is missing".to_string(),
                    })?;
                let point =
                    state
                        .point
                        .operation_point()
                        .ok_or_else(|| RuntimeError::Internal {
                            message: "native stop does not select an executable point".to_string(),
                        })?;
                let activation = call.take_activation()?;
                *captured = Some(activation);

                let reason = match call.take_stop().ok_or_else(|| RuntimeError::Internal {
                    message: "native stopped exit has no stop source".to_string(),
                })? {
                    Stop::Poll => program::StopReason::Pause { point },
                    Stop::Instruction(operation) => {
                        let point = program::ProgramPoint::new(point.function, operation);

                        program::StopReason::Instruction { point }
                    }
                };

                Ok(Outcome::Program(program::Outcome::Stopped { reason }))
            }
            abi::ExitKind::Deoptimized => {
                *captured = Some(call.take_activation()?);

                Ok(Outcome::Deoptimized)
            }
            abi::ExitKind::Awaited | abi::ExitKind::Yielded => Err(Error::StateUnavailable {
                kind,
                frame_map: exit.frame_map,
            }
            .into()),
            abi::ExitKind::Panicked => Err(Error::Panicked { payload: None }.into()),
        }
    }
}
