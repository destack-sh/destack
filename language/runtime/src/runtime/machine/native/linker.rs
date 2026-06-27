use destack_program::{Program, native};

use super::{Code, Error};

/// Process-local native linker.
pub trait Linker {
    /// Load one durable program into process-local native code.
    fn load(&self, program: &Program) -> Result<Code, Error> {
        let Some(code) = program.native_code() else {
            return Err(Error::NativeCodeMissing);
        };

        self.link_code(code)
    }

    /// Link code already resident in this process.
    fn link_resident(&self, code: &native::Code) -> Result<Code, Error>;

    /// Load and link one native library image.
    fn link_library(&self, code: &native::Code, library: &native::Library) -> Result<Code, Error>;

    /// Load and link one native object image.
    fn link_object(&self, code: &native::Code, object: &native::Object) -> Result<Code, Error>;

    /// Link durable native code into process-local native code.
    fn link_code(&self, code: &native::Code) -> Result<Code, Error> {
        match &code.image {
            native::Image::Resident => self.link_resident(code),
            native::Image::Library(library) => self.link_library(code, library),
            native::Image::Object(object) => self.link_object(code, object),
        }
    }
}
