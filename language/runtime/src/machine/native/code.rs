use std::fmt;
use std::sync::Arc;

use destack_memory::MemoryMap;
use destack_native as native;
use destack_native::abi;
use destack_program as program;
use destack_program::{EntryPoint, FunctionId, Outcome, Program, Value};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::Activation;

use super::{Call, Error, Function, Library, Mapping, ModuleTable, Stop};

/// Process-local native code table.
#[derive(Debug, Clone)]
pub struct Code {
    /// The process-local native image backing this code.
    image: Image,
    /// Process-local modules referenced by generated code.
    modules: Arc<ModuleTable>,
    /// Durable native frame maps used by runtime capture.
    frames: native::CodeMap,
    /// Process-local native functions keyed by Program function id.
    functions: Vec<Option<Function>>,
}

impl Code {
    /// Create one native code table.
    pub fn new(
        image: Image,
        modules: Arc<ModuleTable>,
        frames: native::CodeMap,
        functions: Vec<Option<Function>>,
    ) -> Self {
        Self {
            image,
            modules,
            frames,
            functions,
        }
    }

    /// Link one durable native code payload into process-local native code.
    pub fn link(
        image: Image,
        modules: Arc<ModuleTable>,
        program: &Program,
        native: &native::Code,
        mut body: impl FnMut(&str) -> Option<usize>,
        mut entry: impl FnMut(&str) -> Option<abi::Entry>,
    ) -> Result<Self, Error> {
        let sections = program.sections();
        let mut code = Self {
            image,
            modules,
            frames: native.map,
            functions: vec![None; native.definitions(sections).len()],
        };

        for (index, definition) in
            native
                .definitions(sections)
                .iter()
                .enumerate()
                .filter_map(|(index, definition)| {
                    definition.get().map(|definition| (index, definition))
                })
        {
            let Some(body_symbol) = program.string(definition.body) else {
                return Err(Error::ProgramStringMissing {
                    string: definition.body,
                });
            };
            let Some(entry_symbol) = program.string(definition.entry) else {
                return Err(Error::ProgramStringMissing {
                    string: definition.entry,
                });
            };
            let Some(body) = body(body_symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: body_symbol.to_owned(),
                });
            };
            let Some(entry) = entry(entry_symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: entry_symbol.to_owned(),
                });
            };
            if native.module(sections, definition.module).is_none() {
                return Err(Error::NativeModuleMissing {
                    module: definition.module,
                });
            }

            let body_end = body + definition.body_byte_len as usize;
            let body = body..body_end;
            let function = Function::new(FunctionId(index as u32), entry).body(body);
            code.set_function(function);
        }

        Ok(code)
    }

    /// Borrow the process-local native image backing this code.
    pub const fn image(&self) -> &Image {
        &self.image
    }

    /// Borrow process-local Program identity mappings.
    pub fn modules(&self) -> &ModuleTable {
        &self.modules
    }

    /// Return one process-local native function.
    pub fn function(&self, function: FunctionId) -> Option<&Function> {
        self.functions
            .get(function.index())
            .and_then(Option::as_ref)
    }

    /// Insert one process-local native function.
    pub fn set_function(&mut self, function: Function) {
        let index = function.function.index();
        if index >= self.functions.len() {
            self.functions.resize_with(index + 1, || None);
        }

        self.functions[index] = Some(function);
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
        memory: &MemoryMap,
        entry: EntryPoint,
        environment: Option<&Value>,
        args: &[Value],
        captured: &mut Option<program::ActivationImage>,
    ) -> RuntimeResult<Outcome<Value>> {
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
        let mut call = Call::new(program, self.frames, &self.functions, memory, activation);
        let mut activation = call.activation(&mut exit);

        // enter generated native code
        let code = entry.call(&mut activation, &arguments, &mut result);

        self.outcome_from_exit(
            program,
            result_type,
            code,
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
        code: u32,
        result: Vec<program::Word>,
        exit: abi::Exit,
        call: &mut Call<'_, '_, '_, '_>,
        captured: &mut Option<program::ActivationImage>,
    ) -> RuntimeResult<Outcome<Value>> {
        if let Some(error) = call.take_error() {
            return Err(error);
        }

        let kind = abi::ExitKind::try_from(code)
            .map_err(Error::InvalidExit)
            .map_err(Box::<RuntimeError>::from)?;

        match kind {
            abi::ExitKind::Completed => {
                let value = program.value(result_type, result)?;

                Ok(Outcome::Completed { value })
            }
            abi::ExitKind::Cancelled => Ok(Outcome::Cancelled),
            abi::ExitKind::Trapped => {
                let trap = abi::Trap::try_from(exit.trap)
                    .map_err(Error::InvalidTrap)
                    .map_err(Box::<RuntimeError>::from)?;

                Err(Error::Trapped { trap }.into())
            }
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
                    Stop::Instruction => program::StopReason::Instruction { point },
                };

                Ok(Outcome::Stopped { reason })
            }
            abi::ExitKind::Awaited | abi::ExitKind::Yielded => Err(Error::StateUnavailable {
                kind,
                frame_map: exit.frame_map,
            }
            .into()),
            abi::ExitKind::Panicked => {
                let payload = call.take_panic();

                Err(Error::Panicked { payload }.into())
            }
        }
    }
}

/// Process-local native image backing callable code pointers.
#[derive(Debug, Clone)]
pub enum Image {
    /// Native symbols are already resident in this process.
    Resident,
    /// Native symbols are owned by one loaded library handle.
    Library(Library),
    /// Native symbols are owned by one executable memory mapping.
    Object(Mapping),
}

impl Image {
    /// Create one resident code image.
    pub const fn resident() -> Self {
        Self::Resident
    }

    /// Create one loaded library code image.
    pub fn library(owner: impl fmt::Debug + Send + Sync + 'static) -> Self {
        Self::Library(Library::new(owner))
    }

    /// Create one mapped object code image.
    pub fn object(
        base: usize,
        byte_len: usize,
        owner: impl fmt::Debug + Send + Sync + 'static,
    ) -> Self {
        Self::Object(Mapping::new(base, byte_len, owner))
    }
}
