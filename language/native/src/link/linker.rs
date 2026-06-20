use destack_program::{Program, native as program_native};

use crate::{Code, Error};

/// Process-local native linker.
pub trait Linker {
    /// Load one durable program into process-local native code.
    fn load(&self, program: &Program) -> Result<Code, Error> {
        let Some(code) = program.native.as_ref() else {
            return Err(Error::NativeCodeMissing);
        };

        self.link_code(code)
    }

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

    /// Link durable native code into process-local native code.
    fn link_code(&self, code: &program_native::Code) -> Result<Code, Error> {
        match &code.image {
            program_native::Image::Resident => self.link_resident(code),
            program_native::Image::Library(library) => self.link_library(code, library),
            program_native::Image::Object(object) => self.link_object(code, object),
        }
    }
}
