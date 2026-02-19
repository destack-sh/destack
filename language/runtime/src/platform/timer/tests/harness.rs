use super::*;
use crate::platform::timer::TimerOptionsVm;

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> TimerHarnessContext<'call> {
    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::ExternalCallContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::ExternalCallContext<'_>) })
    }

    /// Build one backend-specific timer options payload.
    pub(crate) fn timer_options_value(
        &mut self,
        options: TimerOptions,
    ) -> HarnessValue<TimerOptions, TimerOptionsVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(TimerOptionsVm {
                clock: options.clock,
                flags: options.flags,
            }),
            None => self.harness_value(options),
        }
    }

    /// Build one backend-specific timerfd spec payload.
    pub(crate) fn timer_fd_spec_value(
        &mut self,
        spec: TimerFdSpec,
    ) -> HarnessValue<TimerFdSpec, TimerFdSpecVm> {
        match self.vm_context_mut() {
            Some(_) => self.harness_value_vm(TimerFdSpecVm {
                initial_ns: spec.initial_ns,
                interval_ns: spec.interval_ns,
            }),
            None => self.harness_value(spec),
        }
    }
}
