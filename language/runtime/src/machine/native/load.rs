use destack_native as native;
use destack_program::Program;

use super::{Code, Error};

/// Process-local native code loader.
pub trait Loader {
    /// Load one durable program into process-local native code.
    fn load(&self, program: &Program) -> Result<Code, Error> {
        let Some(code) = program.native() else {
            return Err(Error::NativeCodeMissing);
        };

        self.load_code(program, code)
    }

    /// Link code already resident in this process.
    fn load_resident(&self, code: &native::Code) -> Result<Code, Error>;

    /// Load one native library image.
    fn load_library(
        &self,
        program: &Program,
        code: &native::Code,
        library: native::Library,
    ) -> Result<Code, Error>;

    /// Load one packed native object archive.
    fn load_archive(
        &self,
        program: &Program,
        code: &native::Code,
        archive: native::Archive,
    ) -> Result<Code, Error>;

    /// Load durable native code into process-local native code.
    fn load_code(&self, program: &Program, code: &native::Code) -> Result<Code, Error> {
        match &code.image {
            native::Image::Resident => self.load_resident(code),
            native::Image::Library(library) => self.load_library(program, code, *library),
            native::Image::Archive(archive) => self.load_archive(program, code, *archive),
        }
    }
}
