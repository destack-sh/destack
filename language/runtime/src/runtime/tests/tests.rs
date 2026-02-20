use std::sync::Arc;

use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::PlatformContext;
use crate::runtime::engine::{
    Engine, EngineContinuation, EngineOutcome, NativeContinuation, RuntimeOutput, RuntimeValue,
};
use crate::runtime::{Runtime, RuntimeState};

/// Test engine that yields once, then completes.
#[derive(Debug, Default)]
pub(super) struct TestEngine {
    /// Number of resume calls executed.
    pub(super) resume_calls: usize,
}

impl Engine for TestEngine {
    /// Entry payload for this engine.
    type Entry = ();
    /// Runtime output type for this engine.
    type Output = RuntimeOutput;
    /// Runtime value type for this engine.
    type Value = RuntimeValue;

    /// Run one entrypoint without yielding.
    fn run(
        &mut self,
        _entry: &Self::Entry,
        _args: &[Self::Value],
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Value>> {
        Ok(EngineOutcome::Completed {
            output: void_output(),
        })
    }

    /// Resume one continuation and yield once before completion.
    fn resume(
        &mut self,
        _continuation: EngineContinuation,
        _value: Self::Value,
    ) -> RuntimeResult<EngineOutcome<Self::Output, Self::Value>> {
        // return one yielded continuation on the first resume
        if self.resume_calls == 0 {
            self.resume_calls += 1;
            return Ok(EngineOutcome::Yielded {
                continuation: EngineContinuation::Native(NativeContinuation::new(2)),
                value: RuntimeValue::VOID,
            });
        }

        // complete all later resumes
        self.resume_calls += 1;
        Ok(EngineOutcome::Completed {
            output: void_output(),
        })
    }
}

/// Build one runtime for runtime tests.
pub(super) fn test_runtime() -> Runtime {
    let state = Arc::new(RuntimeState::new(PlatformContext::new(Vec::new())));
    Runtime::new(state)
}

/// Build one void runtime output.
fn void_output() -> RuntimeOutput {
    RuntimeOutput {
        value: RuntimeValue::VOID,
        statistics: vm::telemetry::Statistics::default(),
        heap_cells: 0,
        raw_heap_cells: 0,
    }
}
