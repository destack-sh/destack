use std::fmt;
use std::sync::Arc;

use destack_memory::MemoryMap;
use destack_program as program;
use destack_vm as vm;
use serde::{Deserialize, Serialize};

use super::{Machine, native};
use crate::diagnostic::{RuntimeError, RuntimeResult};

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
    entries: Box<[Option<Target>]>,
}

impl EntryTable {
    /// Build current targets from available execution forms.
    fn new(program: &program::Program, native: Option<&native::Code>) -> Self {
        let bytecode = program.bytecode().copied();
        let entries = program
            .functions()
            .entries(program.sections())
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let function = program::FunctionId(index as u32);

                // prefer loaded native code over interpreted bytecode
                if native.is_some_and(|native| native.entry(function).is_some()) {
                    Some(Target::Native)
                } else if bytecode
                    .and_then(|bytecode| bytecode.function(program.sections(), index))
                    .and_then(|function| function.code())
                    .is_some()
                {
                    Some(Target::Bytecode)
                } else {
                    None
                }
            })
            .collect();

        Self { entries }
    }

    /// Return the current target for one Program function.
    fn target(&self, function: program::FunctionId) -> Option<Target> {
        self.entries.get(function.index()).copied().flatten()
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
