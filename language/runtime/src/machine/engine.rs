use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tspp_memory::MemoryMap;
use tspp_native::abi::{DynamicTable, VirtualTable};
use tspp_program as program;
use tspp_vm as vm;

use super::{Machine, native};
use crate::diagnostic::{RuntimeError, RuntimeResult};

const DEOPTIMIZE_ENTRY: usize = 0;

/// Process-local execution of one immutable Program.
#[derive(Clone)]
pub struct Engine {
    /// The immutable Program executed by this engine.
    program: Arc<program::Program>,
    /// Current execution targets keyed by dense function id.
    entries: Arc<EntryTable>,
    /// Limits used by worker-local bytecode machines.
    limits: vm::MachineLimits,
    /// Process-loaded native code when available.
    native: Option<Arc<native::Code>>,
}

impl Engine {
    /// Create one engine for an immutable Program.
    pub fn new(program: impl Into<Arc<program::Program>>, limits: vm::MachineLimits) -> Self {
        let program = program.into();
        let entries = Arc::new(EntryTable::new(&program, None));

        Self {
            program,
            entries,
            limits,
            native: None,
        }
    }

    /// Add loaded native code and prefer its available entries.
    pub fn native(mut self, native: native::Code) -> Self {
        let native = Arc::new(native);
        self.entries = Arc::new(EntryTable::new(&self.program, Some(&native)));
        self.native = Some(native);

        self
    }

    /// Return the immutable Program executed by this engine.
    pub fn program(&self) -> &Arc<program::Program> {
        &self.program
    }

    /// Return the current target for one Program function.
    pub(crate) fn target(&self, function: program::FunctionId) -> Option<Target> {
        self.entries.target(function)
    }

    /// Return typed native body addresses keyed by encoded callable word.
    pub(crate) fn functions(&self) -> *const usize {
        self.entries.native.functions.as_ptr()
    }

    /// Return virtual method entries keyed by Program virtual table id.
    pub(crate) fn virtuals(&self) -> *const *const VirtualTable {
        self.entries.native.virtuals.as_ptr().cast()
    }

    /// Return dynamic entries keyed by Program dynamic table id.
    pub(crate) fn dynamics(&self) -> *const *const DynamicTable {
        self.entries.native.dynamics.as_ptr().cast()
    }

    /// Return loaded native code when available.
    pub(crate) fn loaded_native(&self) -> Option<&native::Code> {
        self.native.as_deref()
    }

    /// Return the worker-local bytecode machine limits.
    pub(crate) const fn limits(&self) -> vm::MachineLimits {
        self.limits
    }

    /// Capture the process-local engine configuration.
    pub const fn image(&self) -> EngineImage {
        EngineImage {
            limits: self.limits,
            is_native_loaded: self.native.is_some(),
        }
    }

    /// Restore one process-local engine for a captured Program.
    pub fn restore(
        program: Arc<program::Program>,
        image: EngineImage,
        loader: Option<&dyn native::Loader>,
    ) -> RuntimeResult<Self> {
        let engine = Self::new(program.clone(), image.limits);
        if !image.is_native_loaded {
            return Ok(engine);
        }

        // reload process-local native code before selecting native entries
        let loader = loader.ok_or_else(|| {
            RuntimeError::Internal {
                message: "native engine restore requires a native loader".to_string(),
            }
            .boxed()
        })?;
        let code = loader.load(&program).map_err(Box::<RuntimeError>::from)?;

        Ok(engine.native(code))
    }

    /// Spawn one worker-local machine.
    pub fn spawn(&self, memory: Arc<MemoryMap>) -> RuntimeResult<Machine> {
        Machine::new(self.clone(), memory)
    }
}

impl fmt::Debug for Engine {
    /// Format the engine without exposing executable addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Engine")
            .field("program", &self.program)
            .field("entries", &self.entries)
            .field("limits", &self.limits)
            .field("loaded_native", &self.native)
            .finish()
    }
}

/// Captured process-local engine configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineImage {
    /// Limits used by worker-local bytecode machines.
    pub limits: vm::MachineLimits,
    /// Whether native code was loaded.
    pub is_native_loaded: bool,
}

/// Dense current execution targets keyed by Program function id.
#[derive(Debug)]
struct EntryTable {
    /// Current target for every Program function.
    targets: Box<[Option<Target>]>,
    /// Process-local tables read directly by generated native code.
    native: NativeTable,
}

impl EntryTable {
    /// Build current targets from available execution forms.
    fn new(program: &program::Program, native: Option<&native::Code>) -> Self {
        let bytecode = *program.bytecode();
        let targets = program
            .functions()
            .entries(program.sections())
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let function = program::FunctionId(index as u32);

                // prefer loaded native code over interpreted bytecode
                if native.is_some_and(|native| native.function(function).is_some()) {
                    Some(Target::Native)
                } else if bytecode
                    .function(program.sections(), index)
                    .and_then(|function| function.code())
                    .is_some()
                {
                    Some(Target::Bytecode)
                } else {
                    None
                }
            })
            .collect();
        let native = NativeTable::new(program, native);

        Self { targets, native }
    }

    /// Return the current target for one Program function.
    fn target(&self, function: program::FunctionId) -> Option<Target> {
        self.targets.get(function.index()).copied().flatten()
    }
}

/// Stable process-local storage backing native dispatch pointers.
struct NativeTable {
    /// Typed native body addresses keyed by encoded callable word.
    functions: Box<[usize]>,
    /// Virtual entry addresses keyed by Program virtual table id.
    virtuals: Box<[usize]>,
    /// Dynamic entry addresses keyed by Program dynamic table id.
    dynamics: Box<[usize]>,
    /// Virtual method identities owning the virtual entry storage.
    virtual_tables: Box<[Box<[u32]>]>,
    /// Dynamic entry values owning the dynamic entry storage.
    dynamic_tables: Box<[Box<[u32]>]>,
}

impl NativeTable {
    /// Build dense process-local dispatch tables from one linked Program.
    fn new(program: &program::Program, native: Option<&native::Code>) -> Self {
        let function_count = program.functions().entries(program.sections()).len();
        let function_word_bias = program::FunctionId::WORD_BIAS as usize;
        let mut functions = Vec::with_capacity(function_count + function_word_bias);
        functions.resize(function_word_bias, DEOPTIMIZE_ENTRY);
        functions.extend((0..function_count).map(|index| {
            let function = native
                .and_then(|native| native.function(program::FunctionId(index as u32)))
                .and_then(native::Function::body_address);

            // leave unavailable bodies null so generated indirect calls deoptimize
            function.map_or(DEOPTIMIZE_ENTRY, |address| address)
        }));
        let functions = functions.into_boxed_slice();
        let virtual_tables = program
            .dispatch()
            .virtual_tables(program.sections())
            .iter()
            .map(|table| {
                program
                    .dispatch()
                    .virtual_methods(program.sections(), table)
                    .iter()
                    .map(|function| function.0)
                    .collect::<Box<_>>()
            })
            .collect::<Box<_>>();
        let dynamic_tables = program
            .dispatch()
            .dynamic_tables(program.sections())
            .iter()
            .map(|table| {
                let entries = program
                    .dispatch()
                    .dynamic_entries(program.sections(), table)
                    .iter()
                    .map(|entry| entry.value)
                    .collect::<Vec<_>>();
                let mut row = Vec::with_capacity(entries.len() + 1);
                row.push(table.concrete.0);
                row.extend(entries);

                row.into_boxed_slice()
            })
            .collect::<Box<_>>();

        // retain only process addresses in the hot dispatch columns
        let virtuals = virtual_tables
            .iter()
            .map(|row| row.as_ptr() as usize)
            .collect::<Box<_>>();
        let dynamics = dynamic_tables
            .iter()
            .map(|row| row.as_ptr() as usize)
            .collect::<Box<_>>();
        Self {
            functions,
            virtuals,
            dynamics,
            virtual_tables,
            dynamic_tables,
        }
    }
}

impl fmt::Debug for NativeTable {
    /// Format native dispatch storage without exposing process addresses.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let function_count = self.functions.len() - program::FunctionId::WORD_BIAS as usize;

        formatter
            .debug_struct("NativeTable")
            .field("function_count", &function_count)
            .field("virtual_table_count", &self.virtuals.len())
            .field(
                "virtual_method_count",
                &self
                    .virtual_tables
                    .iter()
                    .map(|row| row.len())
                    .sum::<usize>(),
            )
            .field("dynamic_table_count", &self.dynamics.len())
            .field(
                "dynamic_entry_count",
                &self
                    .dynamic_tables
                    .iter()
                    .map(|row| row.len())
                    .sum::<usize>(),
            )
            .finish()
    }
}

/// Current execution form for one Program function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// Interpret linked bytecode.
    Bytecode,
    /// Execute loaded native code.
    Native,
}
