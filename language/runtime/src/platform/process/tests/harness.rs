use destack_vm as vm;

use super::{
    ProcessFdActionKind, ProcessFdActionSpec, ProcessHarnessContext, ProcessSpawnOptionsSpec,
    ProcessStdioKind, ProcessStdioSpec, ProcessWaitKind, ProcessWaitStatusRecord,
    native_path_from_utf8, native_path_to_utf8, native_string, vm_path_from_utf8, vm_path_to_utf8,
    vm_process_fd_action_slice, vm_process_stdio_slice, vm_string, vm_string_slice,
};
use crate::diagnostic::RuntimeResult;
use crate::platform::abi::{NativeSlice, NativeStringRef, NativeStringSlice};
use crate::platform::fs::native as fs_native;
use crate::platform::process::{
    GroupId, ProcessFdAction, ProcessFdActionVm, ProcessSpawnOptions, ProcessSpawnOptionsVm,
    ProcessStdio, ProcessStdioVm, ProcessWaitStatus, ProcessWaitStatusVm, Signal,
};
#[cfg(any(target_os = "linux", windows))]
use crate::platform::process::{ProcessCpuSet, ProcessCpuSetVm};
#[cfg(any(target_os = "linux", windows))]
use crate::platform::thread::ThreadCpu;
use crate::platform::{NativeArray, VmArray, VmSlice, fs, process as process_platform, resource};
use crate::tests::platform::{vm_test_byte_array, vm_test_byte_slice, vm_test_string};

#[path = "harness.generated.rs"]
mod generated;

#[allow(unused_imports)]
pub(crate) use generated::*;

impl<'call> ProcessHarnessContext<'call> {
    /// Return true when this harness call is running through VM bindings.
    pub(crate) fn is_vm(&self) -> bool {
        self.vm_context.is_some()
    }

    /// Return the VM context if available.
    #[allow(clippy::mut_from_ref)]
    fn vm_context_mut(&self) -> Option<&mut vm::BindingContext<'_>> {
        self.vm_context
            .map(|context| unsafe { &mut *(context as *mut vm::BindingContext<'_>) })
    }

    /// Write one byte slice through one file handle using native fs bindings.
    pub(crate) fn fs_write_bytes(
        &self,
        handle: resource::FileHandle,
        bytes: &[u8],
    ) -> RuntimeResult<u64> {
        let buffer = self.call_context.store_slice(bytes.to_vec());
        let mut written = 0_u64;
        unsafe {
            fs_native::destack_fs_write(
                self.call_context,
                &mut written as *mut u64,
                handle,
                buffer,
            )?;
        }

        Ok(written)
    }

    /// Close one file handle through native fs bindings.
    pub(crate) fn fs_close_handle(&self, handle: resource::FileHandle) -> RuntimeResult<()> {
        unsafe { fs_native::destack_fs_close(self.call_context, handle) }
    }

    /// Build one backend-specific string value.
    pub(crate) fn string_value(
        &self,
        value: &str,
    ) -> HarnessValue<NativeStringRef, vm::StringHandle> {
        match self.vm_context_mut() {
            Some(context) => {
                let value = vm_test_string(context, value);
                self.harness_value_vm(value)
            }
            None => {
                let value = self.call_context.store_string(value);
                self.harness_value(value)
            }
        }
    }

    /// Build one backend-specific UTF-8 string slice value.
    pub(crate) fn string_slice_value(
        &self,
        values: &[String],
    ) -> RuntimeResult<HarnessValue<NativeStringSlice, VmSlice<vm::StringHandle>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let values = vm_string_slice(context, values)?;
                Ok(self.harness_value_vm(values))
            }
            None => {
                let values = self.call_context.store_string_slice(
                    values
                        .iter()
                        .map(|value| self.call_context.store_string(value))
                        .collect(),
                );
                Ok(self.harness_value(values))
            }
        }
    }

    /// Build one backend-specific byte-slice value.
    pub(crate) fn bytes_slice_value(
        &self,
        bytes: &[u8],
    ) -> RuntimeResult<HarnessValue<NativeSlice<u8>, VmSlice<u8>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = vm_test_byte_slice(context, bytes);
                Ok(self.harness_value_vm(bytes))
            }
            None => {
                let bytes = self.call_context.store_slice(bytes.to_vec());
                Ok(self.harness_value(bytes))
            }
        }
    }

    /// Build one backend-specific byte-array value.
    pub(crate) fn bytes_array_value(
        &self,
        bytes: &[u8],
    ) -> RuntimeResult<HarnessValue<NativeArray<u8>, VmArray<u8>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let bytes = vm_test_byte_array(context, bytes);
                Ok(self.harness_value_vm(bytes))
            }
            None => {
                let bytes = self.call_context.store_array(bytes.to_vec());
                Ok(self.harness_value(bytes))
            }
        }
    }

    /// Build one backend-specific path value from UTF-8 text.
    pub(crate) fn path_value(
        &self,
        path: &str,
    ) -> RuntimeResult<HarnessValue<fs::OsPath, fs::OsPathVm>> {
        match self.vm_context_mut() {
            Some(context) => {
                let path = vm_path_from_utf8(context, path)?;
                Ok(self.harness_value_vm(path))
            }
            None => {
                let path = native_path_from_utf8(self.call_context, path);
                Ok(self.harness_value(path))
            }
        }
    }

    /// Build one backend-specific signal-slice value.
    pub(crate) fn signal_slice_value(
        &self,
        signals: &[Signal],
    ) -> RuntimeResult<HarnessValue<NativeSlice<Signal>, VmSlice<Signal>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let signals = VmSlice::from_values(&mut context.write(), signals)?;
                Ok(self.harness_value_vm(signals))
            }
            None => {
                let signals = self.call_context.store_slice(signals.to_vec());
                Ok(self.harness_value(signals))
            }
        }
    }

    /// Build one backend-specific group-id slice value.
    pub(crate) fn group_slice_value(
        &self,
        groups: &[GroupId],
    ) -> RuntimeResult<HarnessValue<NativeSlice<GroupId>, VmSlice<GroupId>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let groups = VmSlice::from_values(&mut context.write(), groups)?;
                Ok(self.harness_value_vm(groups))
            }
            None => {
                let groups = self.call_context.store_slice(groups.to_vec());
                Ok(self.harness_value(groups))
            }
        }
    }

    /// Build one backend-specific process spawn-options value.
    pub(crate) fn spawn_options_value(
        &self,
        options: &ProcessSpawnOptionsSpec,
    ) -> RuntimeResult<HarnessValue<ProcessSpawnOptions, ProcessSpawnOptionsVm>> {
        match self.vm_context_mut() {
            Some(context) => {
                let options = ProcessSpawnOptionsVm {
                    cwd: vm_path_from_utf8(context, &options.cwd)?,
                    detached: options.detached,
                    reset_signals: options.reset_signals,
                    new_process_group: options.new_process_group,
                };
                Ok(self.harness_value_vm(options))
            }
            None => {
                let options = ProcessSpawnOptions {
                    cwd: native_path_from_utf8(self.call_context, &options.cwd),
                    detached: options.detached,
                    reset_signals: options.reset_signals,
                    new_process_group: options.new_process_group,
                };
                Ok(self.harness_value(options))
            }
        }
    }

    /// Build one backend-specific process stdio slice value.
    pub(crate) fn stdio_slice_value(
        &self,
        values: &[ProcessStdioSpec],
    ) -> RuntimeResult<HarnessValue<NativeSlice<ProcessStdio>, VmSlice<ProcessStdioVm>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let values = vm_process_stdio_slice(context, values)?;
                Ok(self.harness_value_vm(values))
            }
            None => {
                let native_values = values
                    .iter()
                    .map(|value| match value.kind {
                        ProcessStdioKind::Descriptor => ProcessStdio::ProcessStdioDescriptor(
                            process_platform::ProcessStdioDescriptor {
                                kind: self.call_context.store_string("descriptor"),
                                descriptor: value.descriptor,
                            },
                        ),
                        ProcessStdioKind::File => {
                            let resource_id = value
                                .resource_id
                                .expect("file stdio should provide one resource id");
                            ProcessStdio::ProcessStdioFile(process_platform::ProcessStdioFile {
                                kind: self.call_context.store_string("file"),
                                file: resource::FileHandle(resource_id),
                            })
                        }
                        ProcessStdioKind::Inherit => ProcessStdio::ProcessStdioInherit(
                            process_platform::ProcessStdioInherit {
                                kind: self.call_context.store_string("inherit"),
                            },
                        ),
                        ProcessStdioKind::Null => {
                            ProcessStdio::ProcessStdioNull(process_platform::ProcessStdioNull {
                                kind: self.call_context.store_string("null"),
                            })
                        }
                        ProcessStdioKind::Pipe => {
                            let resource_id = value
                                .resource_id
                                .expect("pipe stdio should provide one resource id");
                            ProcessStdio::ProcessStdioPipe(process_platform::ProcessStdioPipe {
                                kind: self.call_context.store_string("pipe"),
                                pipe: resource::PipeHandle(resource_id),
                            })
                        }
                    })
                    .collect::<Vec<_>>();
                let values = self.call_context.store_slice(native_values);
                Ok(self.harness_value(values))
            }
        }
    }

    /// Build one backend-specific process fd-action slice value.
    pub(crate) fn fd_action_slice_value(
        &self,
        values: &[ProcessFdActionSpec],
    ) -> RuntimeResult<HarnessValue<NativeSlice<ProcessFdAction>, VmSlice<ProcessFdActionVm>>> {
        match self.vm_context_mut() {
            Some(context) => {
                let values = vm_process_fd_action_slice(context, values)?;
                Ok(self.harness_value_vm(values))
            }
            None => {
                let native_values = values
                    .iter()
                    .map(|value| match value.op {
                        ProcessFdActionKind::Close => ProcessFdAction::ProcessFdActionClose(
                            process_platform::ProcessFdActionClose {
                                kind: self.call_context.store_string("close"),
                                descriptor: value.source,
                            },
                        ),
                        ProcessFdActionKind::Dup2 => ProcessFdAction::ProcessFdActionDup2(
                            process_platform::ProcessFdActionDup2 {
                                kind: self.call_context.store_string("dup2"),
                                source: value.source,
                                target: value.target,
                            },
                        ),
                        ProcessFdActionKind::Open => ProcessFdAction::ProcessFdActionOpen(
                            process_platform::ProcessFdActionOpen {
                                kind: self.call_context.store_string("open"),
                                target: value.target,
                                path: native_path_from_utf8(self.call_context, &value.path),
                                flags: value.flags,
                                mode: value.mode,
                            },
                        ),
                    })
                    .collect::<Vec<_>>();
                let values = self.call_context.store_slice(native_values);
                Ok(self.harness_value(values))
            }
        }
    }

    /// Build one backend-specific group-aware CPU-set value.
    #[cfg(any(target_os = "linux", windows))]
    pub(crate) fn cpu_entries_value(
        &self,
        cpus: &[ThreadCpu],
    ) -> RuntimeResult<HarnessValue<ProcessCpuSet, ProcessCpuSetVm>> {
        match self.vm_context_mut() {
            Some(context) => {
                let cpus = ProcessCpuSetVm {
                    cpus: VmArray::from_values(&mut context.write(), cpus)?,
                };
                Ok(self.harness_value_vm(cpus))
            }
            None => {
                let cpus = ProcessCpuSet {
                    cpus: self.call_context.store_array(cpus.to_vec()),
                };
                Ok(self.harness_value(cpus))
            }
        }
    }

    /// Build one backend-specific CPU-set value.
    #[cfg(target_os = "linux")]
    pub(crate) fn cpu_set_value(
        &self,
        cpus: &[u32],
    ) -> RuntimeResult<HarnessValue<ProcessCpuSet, ProcessCpuSetVm>> {
        let cpus = cpus
            .iter()
            .map(|cpu| {
                Ok(ThreadCpu {
                    group: 0,
                    cpu: u16::try_from(*cpu).map_err(|_| {
                        crate::diagnostic::RuntimeError::from(
                            crate::platform::PlatformError::invalid_argument_value(
                                "cpus",
                                format!("cpu index {cpu} exceeds thread affinity ABI range"),
                            ),
                        )
                        .boxed()
                    })?,
                })
            })
            .collect::<RuntimeResult<Vec<_>>>()?;

        self.cpu_entries_value(&cpus)
    }

    /// Build one backend-specific shared native or VM value.
    pub(crate) fn unified_value<T>(&self, value: T) -> HarnessValue<T, T> {
        if self.vm_context.is_some() {
            HarnessValue::Vm(value)
        } else {
            HarnessValue::Native(value)
        }
    }

    /// Decode one backend-specific string value into UTF-8 text.
    pub(crate) fn string_from_value(
        &self,
        value: HarnessValue<NativeStringRef, vm::StringHandle>,
    ) -> RuntimeResult<String> {
        match value {
            HarnessValue::Native(value) => native_string(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm string value");
                vm_string(context, value)
            }
        }
    }

    /// Decode one backend-specific string-slice value into UTF-8 text values.
    pub(crate) fn string_list_from_value(
        &self,
        values: HarnessValue<NativeStringSlice, VmSlice<vm::StringHandle>>,
    ) -> RuntimeResult<Vec<String>> {
        match values {
            HarnessValue::Native(values) => {
                let values = unsafe { values.as_slice()? };
                values.iter().map(|value| native_string(*value)).collect()
            }
            HarnessValue::Vm(values) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm string-slice value");
                let values = values.read_values(&context.read())?;
                values
                    .into_iter()
                    .map(|value| vm_string(context, value))
                    .collect()
            }
        }
    }

    /// Decode one backend-specific byte-array value into bytes.
    pub(crate) fn bytes_from_array_value(
        &self,
        values: HarnessValue<NativeArray<u8>, VmArray<u8>>,
    ) -> RuntimeResult<Vec<u8>> {
        match values {
            HarnessValue::Native(values) => {
                let values = unsafe { values.as_slice()? };
                Ok(values.to_vec())
            }
            HarnessValue::Vm(values) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm byte-array value");
                values.read_bytes(&context.read())
            }
        }
    }

    /// Decode one backend-specific path value into UTF-8 text.
    pub(crate) fn path_string_from_value(
        &self,
        value: HarnessValue<fs::OsPath, fs::OsPathVm>,
    ) -> RuntimeResult<String> {
        match value {
            HarnessValue::Native(value) => native_path_to_utf8(value),
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm path value");
                vm_path_to_utf8(context, value)
            }
        }
    }

    /// Decode one backend-specific group-id slice value.
    pub(crate) fn group_list_from_value(
        &self,
        value: HarnessValue<NativeSlice<GroupId>, VmSlice<GroupId>>,
    ) -> RuntimeResult<Vec<GroupId>> {
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                Ok(value.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm group-id slice value");
                value.read_values(&context.read())
            }
        }
    }

    /// Decode one backend-specific signal-array value.
    pub(crate) fn signal_list_from_value(
        &self,
        value: HarnessValue<NativeArray<Signal>, VmArray<Signal>>,
    ) -> RuntimeResult<Vec<Signal>> {
        match value {
            HarnessValue::Native(value) => {
                let value = unsafe { value.as_slice()? };
                Ok(value.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm signal-array value");
                value.read_values(&context.read())
            }
        }
    }

    /// Decode one backend-specific CPU-set value into CPU entries.
    #[cfg(any(target_os = "linux", windows))]
    pub(crate) fn cpu_entries_from_value(
        &self,
        value: HarnessValue<ProcessCpuSet, ProcessCpuSetVm>,
    ) -> RuntimeResult<Vec<ThreadCpu>> {
        match value {
            HarnessValue::Native(value) => {
                let cpus = unsafe { value.cpus.as_slice()? };
                Ok(cpus.to_vec())
            }
            HarnessValue::Vm(value) => {
                let context = self
                    .vm_context_mut()
                    .expect("vm context required for vm cpu-set value");
                value.cpus.read_values(&context.read())
            }
        }
    }

    /// Decode one backend-specific CPU-set value into CPU index values.
    #[cfg(target_os = "linux")]
    pub(crate) fn cpu_list_from_value(
        &self,
        value: HarnessValue<ProcessCpuSet, ProcessCpuSetVm>,
    ) -> RuntimeResult<Vec<u32>> {
        let cpus = self.cpu_entries_from_value(value)?;
        Ok(cpus.into_iter().map(|cpu| u32::from(cpu.cpu)).collect())
    }

    /// Decode one wait-status payload into one normalized record.
    pub(crate) fn wait_status_from_value(
        &self,
        value: HarnessValue<ProcessWaitStatus, ProcessWaitStatusVm>,
    ) -> ProcessWaitStatusRecord {
        match value {
            HarnessValue::Native(value) => wait_status_record_from_native(value),
            HarnessValue::Vm(value) => wait_status_record_from_vm(value),
        }
    }
}

impl<T> HarnessValue<T, T> {
    /// Consume one same-typed harness value and return the inner value.
    pub(crate) fn into_inner(self) -> T {
        match self {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }
}

impl<Native: std::fmt::Debug, Vm: std::fmt::Debug> std::fmt::Debug for HarnessValue<Native, Vm> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HarnessValue::Native(value) => value.fmt(formatter),
            HarnessValue::Vm(value) => value.fmt(formatter),
        }
    }
}

impl<T: PartialEq> PartialEq<T> for HarnessValue<T, T> {
    fn eq(&self, other: &T) -> bool {
        match self {
            HarnessValue::Native(value) => value.eq(other),
            HarnessValue::Vm(value) => value.eq(other),
        }
    }
}

/// Decode one native wait status into one normalized record.
fn wait_status_record_from_native(value: ProcessWaitStatus) -> ProcessWaitStatusRecord {
    match value {
        ProcessWaitStatus::ProcessWaitContinuedStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Continued,
            exit_code: None,
            signal: None,
            core_dumped: false,
        },
        ProcessWaitStatus::ProcessWaitExitedStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Exited,
            exit_code: Some(value.exit_code),
            signal: None,
            core_dumped: false,
        },
        ProcessWaitStatus::ProcessWaitRunningStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Running,
            exit_code: None,
            signal: None,
            core_dumped: false,
        },
        ProcessWaitStatus::ProcessWaitSignaledStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Signaled,
            exit_code: None,
            signal: Some(value.signal),
            core_dumped: value.core_dumped,
        },
        ProcessWaitStatus::ProcessWaitStoppedStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Stopped,
            exit_code: None,
            signal: Some(value.signal),
            core_dumped: false,
        },
    }
}

/// Decode one VM wait status into one normalized record.
fn wait_status_record_from_vm(value: ProcessWaitStatusVm) -> ProcessWaitStatusRecord {
    match value {
        ProcessWaitStatusVm::ProcessWaitContinuedStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Continued,
            exit_code: None,
            signal: None,
            core_dumped: false,
        },
        ProcessWaitStatusVm::ProcessWaitExitedStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Exited,
            exit_code: Some(value.exit_code),
            signal: None,
            core_dumped: false,
        },
        ProcessWaitStatusVm::ProcessWaitRunningStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Running,
            exit_code: None,
            signal: None,
            core_dumped: false,
        },
        ProcessWaitStatusVm::ProcessWaitSignaledStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Signaled,
            exit_code: None,
            signal: Some(value.signal),
            core_dumped: value.core_dumped,
        },
        ProcessWaitStatusVm::ProcessWaitStoppedStatus(value) => ProcessWaitStatusRecord {
            pid: value.pid,
            kind: ProcessWaitKind::Stopped,
            exit_code: None,
            signal: Some(value.signal),
            core_dumped: false,
        },
    }
}

impl<T> std::ops::Deref for HarnessValue<T, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            HarnessValue::Native(value) => value,
            HarnessValue::Vm(value) => value,
        }
    }
}
