use destack_workspace::ArtifactKey;

use crate::{BuildKey, BuildRequirementError, Compiler, ImportError, ImportResult};
use destack_source::ModuleId;
use destack_workspace::ImportDir;

impl Compiler {
    /// Build the parsed syntax tree for one module.
    pub fn process_ast(&self, module: ModuleId) -> ImportResult<()> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.import_module_parse(module, module_version)?;
        self.program
            .artifacts
            .ast(module)
            .expect("missing AST on parsed module");

        Ok(())
    }

    /// Build base DIR for one module.
    pub fn process_dir_base(&self, module: ModuleId) -> ImportResult<()> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.require_ast(module)?;

        // resolve cache handle
        let cache_handle =
            self.cache_handle_for_module(module, None, None, destack_source::CacheKind::DirBase);

        // return cached base DIR directly when available
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir_base()
        {
            self.ensure_module_version_matches::<ImportError>(module, module_version)?;
            tracing::trace!(?module, "import.module.bind.cache");
            self.program.artifacts.set_dir_base(module, entry.payload);
            return Ok(());
        }

        // read the committed AST artifact directly
        let ast = self.program.artifacts.ast(module);

        // build one transient base DIR from the current AST
        let (dir, ast) = {
            let module_handle = self.program.modules.get(module);
            let module_guard = module_handle.as_ref();
            self.ensure_module_version_matches_guard::<ImportError>(&module_guard, module_version)?;
            let ast = ast.expect("missing AST on parsed module");
            let anchor_id = ast
                .anchor_expression
                .expect("missing anchor expression on parsed module");

            let dir = if module_guard.is_code() {
                ImportDir::new_base(module, module_version, anchor_id.id)
            } else {
                ImportDir::new_data_base(module, module_version, anchor_id.id)
            };

            (dir, ast)
        };

        // run bind, desugar, and validate on the transient base DIR
        let mut dir = dir;
        self.import_module_bind(module, module_version, &ast, &mut dir)?;
        self.import_module_desugar(module, module_version, &mut dir)?;
        self.import_module_validate(module, module_version, &dir)?;
        if self.is_code_module(module) {
            self.stats.record_bind();
        }

        // write base DIR to cache
        let dir = dir.into_dir();
        if let Some(cache) = cache_handle.as_ref() {
            self.ensure_module_version_matches::<ImportError>(module, module_version)?;
            if let Err(error) = cache.write_dir_base(dir.clone()) {
                tracing::debug!(?module, ?error, "import.module.bind.cache.write");
            }
        }

        self.program.artifacts.set_dir_base(module, dir);

        Ok(())
    }

    /// Ensure a module AST exists.
    pub fn require_ast(&self, module: ModuleId) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::Ast { module }))
    }

    /// Ensure a module base DIR exists.
    pub fn require_dir_base(&self, module: ModuleId) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Artifact(ArtifactKey::DirBase { module }))
    }
}
