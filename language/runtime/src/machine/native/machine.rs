use std::sync::Arc;

use destack_program as program;

use crate::diagnostic::RuntimeResult;
use crate::worker::Activation;

use super::{Code, MachineImage};

/// Worker-owned native machine.
#[derive(Debug, Clone)]
pub struct Machine {
    /// Immutable linked Program.
    program: Arc<program::Program>,
    /// Process-local native code.
    code: Arc<Code>,
}

impl Machine {
    /// Create one native machine.
    pub fn new(program: Arc<program::Program>, code: Arc<Code>) -> Self {
        Self { program, code }
    }

    /// Return the immutable Program.
    pub fn program(&self) -> &program::Program {
        &self.program
    }

    /// Run one native function.
    pub fn run<'runtime, 'memory, 'state>(
        &self,
        activation: &mut program::Activation<'runtime, 'memory, Activation<'state>>,
        function: program::FunctionId,
        environment: Option<&program::Value>,
        arguments: &[program::Value],
    ) -> RuntimeResult<program::Outcome<program::Value>> {
        self.code.run(
            &self.program,
            activation,
            program::EntryPoint::from(function),
            environment,
            arguments,
        )
    }

    /// Capture one native machine with no retained frames.
    pub const fn image(&self) -> MachineImage {
        MachineImage
    }

    /// Restore one native machine with no retained frames.
    pub const fn restore(&mut self, _image: &MachineImage) {}

    /// Clear retained native execution state.
    pub const fn clear(&mut self) {}

    /// Fork one native machine.
    pub fn fork(&self) -> Self {
        self.clone()
    }
}
