use std::sync::Arc;

use destack_program::{Program as DurableProgram, native as program_native};

use crate::{Code, Error, Program};

/// Process-local native linker.
pub trait Linker {
    /// Link code already resident in this process.
    fn link_resident(&self, code: &program_native::Code) -> Result<Code, Error>;

    /// Load and link one native library image.
    fn link_library(
        &self,
        code: &program_native::Code,
        library: &program_native::Library,
    ) -> Result<Code, Error>;

    /// Load and link one native object image.
    fn link_object(
        &self,
        code: &program_native::Code,
        object: &program_native::Object,
    ) -> Result<Code, Error>;
}

/// Native program loader.
pub struct Loader<'a> {
    /// Process-local native linker.
    linker: &'a dyn Linker,
}

impl std::fmt::Debug for Loader<'_> {
    /// Format this loader without exposing linker internals.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Loader").finish_non_exhaustive()
    }
}

impl<'a> Loader<'a> {
    /// Create one native loader.
    pub const fn new(linker: &'a dyn Linker) -> Self {
        Self { linker }
    }

    /// Load one durable program into process-local native code.
    pub fn load(&self, program: Arc<DurableProgram>) -> Result<Program, Error> {
        let Some(code) = program.native.as_ref() else {
            return Err(Error::NativeCodeMissing);
        };
        let code = self.link_code(code)?;

        Ok(Program::new(program, code))
    }

    /// Link durable native code into process-local native code.
    fn link_code(&self, code: &program_native::Code) -> Result<Code, Error> {
        match &code.image {
            program_native::Image::Resident => self.linker.link_resident(code),
            program_native::Image::Library(library) => self.linker.link_library(code, library),
            program_native::Image::Object(object) => self.linker.link_object(code, object),
        }
    }
}
