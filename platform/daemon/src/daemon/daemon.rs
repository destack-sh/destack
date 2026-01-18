use std::path::Path;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions};
use destack_workspace::{FileUpdate, Session};

use crate::{DaemonError, DaemonUpdate};

/// Persistent daemon state for incremental compilation.
#[derive(Debug, Clone)]
pub struct Daemon {
    /// The active session for this daemon.
    pub session: Arc<Session>,
    /// Default compiler options for daemon work.
    pub compiler_options: CompilerOptions,
}

impl Daemon {
    /// Create a daemon for the given session.
    pub fn new(session: Arc<Session>) -> Self {
        Self {
            session,
            compiler_options: CompilerOptions::default(),
        }
    }

    /// Create a daemon with explicit compiler options.
    pub fn with_options(session: Arc<Session>, compiler_options: CompilerOptions) -> Self {
        Self {
            session,
            compiler_options,
        }
    }

    /// Apply a text update and re-analyze the owning module.
    pub fn update_file(&self, path: &Path, content: String) -> Result<DaemonUpdate, DaemonError> {
        // update the filesystem content
        self.session
            .fs
            .write_string(path, &content)
            .map_err(|error| DaemonError::FileWrite {
                path: path.to_path_buf(),
                error,
            })?;

        self.update_virtual_file(path, content)
    }

    /// Apply a text update without writing to the filesystem.
    pub fn update_virtual_file(
        &self,
        path: &Path,
        content: String,
    ) -> Result<DaemonUpdate, DaemonError> {
        // locate the program for the path
        let program = self.session.find_program_for_path(path);

        // create compiler with existing session
        let compiler = Compiler::new(
            self.session.clone(),
            program.clone(),
            self.compiler_options.clone(),
        );

        // resolve path to module or fall back to tracked files
        let module_id = match compiler.resolve_path_to_module(&path.to_path_buf()) {
            Ok(module_id) => Some(module_id),
            Err(error) => {
                if program.files.get_id_by_path(path).is_some() {
                    None
                } else {
                    return Err(DaemonError::Resolve {
                        path: path.to_path_buf(),
                        error: Box::new(error),
                    });
                }
            }
        };

        // resolve file id for invalidation
        let file_id = match module_id {
            Some(module_id) => program.modules.get(module_id).read().file_id,
            None => match program.files.get_id_by_path(path) {
                Some(file_id) => file_id,
                None => {
                    return Err(DaemonError::FileNotTracked {
                        path: path.to_path_buf(),
                    });
                }
            },
        };
        let invalidation = program
            .invalidate_file(file_id, FileUpdate::Text { content })
            .map_err(|error| DaemonError::Invalidation {
                path: path.to_path_buf(),
                error: Box::new(error),
            })?;

        // reset diagnostics for a clean publish pass
        let _ = program.diagnostics.drain();

        // analyze the module when available
        if let Some(module_id) = module_id {
            let profile = program.default_profile_id_for_module(module_id);
            let module = compiler.module_stamp(module_id);
            let profile = compiler.profile_stamp(profile);
            compiler.enqueue(AnalyzeTask::AnalyzeModuleValidate { module, profile });
            compiler.compile();
        }

        // collect diagnostics for the updated file
        let diagnostics = program
            .diagnostics
            .iter()
            .into_iter()
            .filter(|diagnostic| diagnostic.file_id == file_id)
            .collect();

        Ok(DaemonUpdate {
            module_id,
            file_id,
            invalidation,
            diagnostics,
        })
    }
}
