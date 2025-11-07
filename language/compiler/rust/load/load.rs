use crate::Compiler;

use dyst_source::{File, Uri};

/// Task to load a file into the compiler.
#[derive(Debug, Clone)]
pub enum LoadTask {
    /// Feed a preloaded file.
    LoadFile { file: File },
    /// Load a file from disk and feed it.
    LoadFileFromDisk { path: Uri },
}

impl<'a> Compiler<'a> {
    /// Process a load task.
    pub fn process_load(&mut self, task: LoadTask) {
        todo!("process_load({task:?})")
    }
}
