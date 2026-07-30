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

        self.load_code(code)
    }

    /// Link code already resident in this process.
    fn load_resident(&self, code: &native::Code) -> Result<Code, Error>;

    /// Load one native library image.
    fn load_library(&self, code: &native::Code, library: &native::Library) -> Result<Code, Error>;

    /// Load one native object image.
    fn load_object(&self, code: &native::Code, object: &native::Object) -> Result<Code, Error>;

    /// Load durable native code into process-local native code.
    fn load_code(&self, code: &native::Code) -> Result<Code, Error> {
        match &code.image {
            native::Image::Resident => self.load_resident(code),
            native::Image::Library(library) => self.load_library(code, library),
            native::Image::Object(object) => self.load_object(code, object),
        }
    }
}
