use std::fmt;

use destack_native as native;
use destack_native::abi;
use destack_program as program;
use destack_program::{EntryPoint, FunctionId, Outcome, Program, Value};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::worker::Activation;

use super::{Call, Entry, Error, Library, Mapping};

/// Process-local native code table.
#[derive(Debug, Clone)]
pub struct Code {
    /// The process-local native image backing this code.
    image: Image,
    /// Native entries keyed by program function id.
    entries: Vec<Option<Entry>>,
}

impl Code {
    /// Create one native code table.
    pub fn new(image: Image, entries: Vec<Option<Entry>>) -> Self {
        Self { image, entries }
    }

    /// Link one durable native code payload into process-local native code.
    pub fn link(
        image: Image,
        program: &Program,
        native: &native::Code,
        mut function: impl FnMut(&str) -> Option<abi::Entry>,
    ) -> Result<Self, Error> {
        let sections = program.sections();
        let mut code = Self {
            image,
            entries: vec![None; native.entries(sections).len()],
        };

        for (index, entry) in native
            .entries(sections)
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.get().map(|entry| (index, entry)))
        {
            let Some(symbol) = program.string(entry.symbol) else {
                return Err(Error::ProgramStringMissing {
                    string: entry.symbol,
                });
            };
            let Some(function) = function(symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: symbol.to_owned(),
                });
            };

            code.set_entry(FunctionId(index as u32), function);
        }

        Ok(code)
    }

    /// Borrow the process-local native image backing this code.
    pub const fn image(&self) -> &Image {
        &self.image
    }

    /// Return one native entry for one function.
    pub fn entry(&self, function: FunctionId) -> Option<&Entry> {
        self.entries.get(function.index()).and_then(Option::as_ref)
    }

    /// Insert one native function pointer.
    pub fn set_entry(&mut self, function: FunctionId, entry: abi::Entry) {
        let index = function.index();
        if index >= self.entries.len() {
            self.entries.resize_with(index + 1, || None);
        }

        self.entries[index] = Some(Entry::new(function, entry));
    }

    /// Return native function entries in dense function id order.
    pub fn entries(&self) -> &[Option<Entry>] {
        &self.entries
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
        entry: EntryPoint,
        environment: Option<&Value>,
        args: &[Value],
    ) -> RuntimeResult<Outcome<Value>> {
        let Some(entry) = self.entry(entry.function()) else {
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
        let mut call = Call::new(program, activation);
        let mut activation = call.activation(&mut exit);

        // enter generated native code
        let code = entry.call(&mut activation, &arguments, &mut result);

        self.outcome_from_exit(program, result_type, code, result, exit, &mut call)
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
            abi::ExitKind::Awaited
            | abi::ExitKind::Yielded
            | abi::ExitKind::Deoptimized
            | abi::ExitKind::Stopped => Err(Error::StateUnavailable {
                kind,
                safepoint: exit.safepoint,
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
