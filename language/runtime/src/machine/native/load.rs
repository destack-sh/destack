use tspp_program::Program;

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

    /// Map and prepare one durable native code image.
    fn load_code(&self, program: &Program, code: &tspp_native::Code) -> Result<Code, Error>;
}
