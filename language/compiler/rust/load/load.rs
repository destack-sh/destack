use crate::Compiler;

use dyst_source::{File, Uri};

/// Request to load a file into the compiler.
#[derive(Debug, Clone)]
pub enum LoadTask {
    /// Feed a preloaded file.
    LoadFile { file: File },
    /// Load a file from disk and feed it.
    LoadFileFromDisk { path: Uri },
}

impl<'a> Compiler<'a> {
    /// Process a load request.
    pub fn process_load(&mut self, request: LoadTask) {
        todo!("process_load({request:?})")
    }
}
