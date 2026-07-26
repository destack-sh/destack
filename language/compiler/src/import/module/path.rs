use std::path::{Path, PathBuf};

use destack_repository::Revision;
use destack_source::{CODE_FILE_TYPES, FileType, Loader, ModuleId};

use crate::{Compiler, CompilerResult};

/// Resolution of one module path against the repository module set.
pub(in crate::import) enum ModulePathResolution {
    /// The path escapes the logical workspace root.
    Unsupported,
    /// No candidate path names a module.
    Missing {
        /// The candidate paths inspected.
        candidates: usize,
    },
    /// Exactly one candidate path names a module.
    Resolved {
        /// The candidate paths inspected.
        candidates: usize,
        /// The exact path that selected the module.
        path: PathBuf,
        /// The selected module.
        module: ModuleId,
    },
    /// Multiple candidate paths name modules.
    Ambiguous {
        /// The candidate paths inspected.
        candidates: usize,
        /// The matching paths and modules.
        matches: Vec<(PathBuf, ModuleId)>,
    },
}

impl ModulePathResolution {
    /// Return the number of candidate paths inspected.
    pub(in crate::import) fn candidates(&self) -> usize {
        match self {
            Self::Unsupported => 0,
            Self::Missing { candidates }
            | Self::Resolved { candidates, .. }
            | Self::Ambiguous { candidates, .. } => *candidates,
        }
    }
}

impl Compiler {
    /// Resolve one logical path against every applicable source extension.
    pub(in crate::import) fn resolve_module_path(
        &self,
        revision: Revision,
        path: &Path,
        loader: Option<Loader>,
    ) -> CompilerResult<ModulePathResolution> {
        let Some(path) = self.normalize_workspace_path(path.to_path_buf()) else {
            return Ok(ModulePathResolution::Unsupported);
        };
        let paths = Self::module_candidate_paths(path, loader);
        let candidates = paths.len();
        let mut matches = Vec::new();

        // resolve every candidate through the repository module graph
        for path in paths {
            let module = self.module_id_for_path(revision, &path)?;
            if let Some(module) = module {
                matches.push((path, module));
            }
        }

        let resolution = match matches.len() {
            0 => ModulePathResolution::Missing { candidates },
            1 => {
                let (path, module) = matches.remove(0);

                ModulePathResolution::Resolved {
                    candidates,
                    path,
                    module,
                }
            }
            _ => ModulePathResolution::Ambiguous {
                candidates,
                matches,
            },
        };

        Ok(resolution)
    }

    /// Return candidate module paths in deterministic order.
    fn module_candidate_paths(path: PathBuf, loader: Option<Loader>) -> Vec<PathBuf> {
        // retain an explicit extension
        if path.extension().is_some() {
            vec![path]
        }
        // apply an explicit loader extension
        else if let Some(extension) = loader.and_then(Loader::extension) {
            vec![path.with_extension(extension)]
        }
        // try each source extension
        else {
            CODE_FILE_TYPES
                .iter()
                .filter_map(FileType::extension)
                .map(|extension| path.with_extension(extension))
                .collect()
        }
    }
}
