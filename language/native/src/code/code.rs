use destack_mir as mir;
use destack_program::{FunctionId, native as program_native};

use crate::{Entry, Error, NativeEntry, NativeResumeEntry, ResumeEntry};

/// Process-local native code table.
#[derive(Debug, Clone)]
pub struct Code {
    /// The mapped executable region backing this code.
    region: Option<MemoryRegion>,
    /// Native code map for safepoints and deoptimization.
    map: program_native::CodeMap,
    /// Native entries keyed by program function id.
    function: Vec<Option<Entry>>,
    /// Native resume entries keyed by frame state id.
    resume: Vec<Option<ResumeEntry>>,
}

impl Code {
    /// Create one native code table.
    pub fn new(
        region: Option<MemoryRegion>,
        map: program_native::CodeMap,
        function: Vec<Option<Entry>>,
        resume: Vec<Option<ResumeEntry>>,
    ) -> Self {
        Self {
            region,
            map,
            function,
            resume,
        }
    }

    /// Link one durable native code payload into process-local native code.
    pub fn link(
        region: Option<MemoryRegion>,
        native: &program_native::Code,
        mut function: impl FnMut(&str) -> Option<NativeEntry>,
        mut resume: impl FnMut(&str) -> Option<NativeResumeEntry>,
    ) -> Result<Self, Error> {
        let entry = &native.entry;
        let mut code = Self {
            region,
            map: native.map.clone(),
            function: vec![None; entry.function_entry().len()],
            resume: vec![None; entry.resume_entry().len()],
        };

        for entry in entry.function_entry().iter().flatten() {
            let Some(function) = function(&entry.symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: entry.symbol.clone(),
                });
            };

            code.set_entry(entry.function, function);
        }

        for entry in entry.resume_entry().iter().flatten() {
            let Some(resume) = resume(&entry.symbol) else {
                return Err(Error::NativeSymbolMissing {
                    symbol: entry.symbol.clone(),
                });
            };

            code.set_resume(entry.frame_state, resume);
        }

        Ok(code)
    }

    /// Borrow the mapped executable region backing this code.
    pub const fn region(&self) -> Option<MemoryRegion> {
        self.region
    }

    /// Borrow the native code map.
    pub const fn map(&self) -> &program_native::CodeMap {
        &self.map
    }

    /// Return one native entry for one function.
    pub fn entry(&self, function: FunctionId) -> Option<&Entry> {
        self.function.get(function.index()).and_then(Option::as_ref)
    }

    /// Return one native resume entry for one frame state.
    pub fn resume(&self, frame_state: mir::FrameStateId) -> Option<&ResumeEntry> {
        self.resume
            .get(frame_state.0 as usize)
            .and_then(Option::as_ref)
    }

    /// Insert one native function pointer.
    pub fn set_entry(&mut self, function: FunctionId, entry: NativeEntry) {
        let index = function.index();
        if index >= self.function.len() {
            self.function.resize_with(index + 1, || None);
        }

        self.function[index] = Some(Entry::new(function, entry));
    }

    /// Insert one native resume function pointer.
    pub fn set_resume(&mut self, frame_state: mir::FrameStateId, entry: NativeResumeEntry) {
        let index = frame_state.0 as usize;
        if index >= self.resume.len() {
            self.resume.resize_with(index + 1, || None);
        }

        self.resume[index] = Some(ResumeEntry::new(frame_state, entry));
    }

    /// Return native function entries in dense function id order.
    pub fn function_entry(&self) -> &[Option<Entry>] {
        &self.function
    }

    /// Return native resume entries in dense frame state id order.
    pub fn resume_entry(&self) -> &[Option<ResumeEntry>] {
        &self.resume
    }
}

/// Mapped native memory region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    /// The base address.
    base: usize,
    /// The mapped byte size.
    byte_size: usize,
}

impl MemoryRegion {
    /// Create one mapped native memory region.
    pub const fn new(base: usize, byte_size: usize) -> Self {
        Self { base, byte_size }
    }

    /// Return one address inside this mapped region.
    pub fn address_at(self, offset: u32) -> Option<usize> {
        let offset = offset as usize;
        if offset >= self.byte_size {
            return None;
        }

        self.base.checked_add(offset)
    }
}
