use destack_workspace::ArtifactKey;

use crate::{BuildKey, BuildProduct, BuildRequirementError, Compiler, ImportError, ImportResult};
use destack_source::ModuleId;
use destack_workspace::{ModuleAst, ModuleDir};

impl Compiler {
    /// Build the parsed syntax tree for one module.
    pub fn process_ast(&self, module: ModuleId) -> ImportResult<BuildProduct> {
        let module_version = self.module_version(module);
        self.ensure_module_version_matches::<ImportError>(module, module_version)?;
        self.import_module_parse(module, module_version)?;

        let module_handle = self.program.modules.get(module);
        let module = module_handle.read();
        let ast = module
            .ast_maybe()
            .expect("missing AST on parsed module")
            .to_data();

        Ok(BuildProduct::Ast(ast))
    }

    /// Build base DIR for one module.
    pub fn process_dir_base(&self, module: ModuleId) -> ImportResult<BuildProduct> {
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
            return Ok(BuildProduct::Dir(entry.payload));
        }

        // rebuild one transient AST workspace from committed artifact truth when needed
        if let Some(ast) = self.program.artifacts.ast(module) {
            let module_handle = self.program.modules.get(module);
            let mut module_guard = module_handle.write();
            if module_guard.ast_maybe().is_none() {
                module_guard.set_ast(ModuleAst::from_data(ast.as_ref().clone()));
            }
        }

        // build one transient base DIR from the current AST
        let dir = {
            let module_handle = self.program.modules.get(module);
            let mut module_guard = module_handle.write();
            self.ensure_module_version_matches_guard::<ImportError>(&module_guard, module_version)?;
            let file_id = module_guard.file_id;
            let ast = module_guard
                .ast_maybe_mut()
                .expect("missing AST on parsed module");
            let anchor_id = ast.ensure_anchor_expression(file_id);

            if module_guard.is_code() {
                ModuleDir::new_base(module, module_version, anchor_id.id)
            } else {
                ModuleDir::new_data_base(module, module_version, anchor_id.id)
            }
        };

        // run bind, desugar, and validate on the transient base DIR
        let (result, dir) = self.with_active_base_dir_frame(module, dir, || -> ImportResult<()> {
            self.import_module_bind(module, module_version)?;
            self.import_module_desugar(module, module_version)?;
            self.import_module_validate(module, module_version)?;
            Ok(())
        });
        result?;
        if self.is_code_module(module) {
            self.stats.record_bind();
        }

        // write base DIR to cache
        let dir = dir.to_data();
        if let Some(cache) = cache_handle.as_ref() {
            self.ensure_module_version_matches::<ImportError>(module, module_version)?;
            if let Err(error) = cache.write_dir_base(dir.clone()) {
                tracing::debug!(?module, ?error, "import.module.bind.cache.write");
            }
        }

        Ok(BuildProduct::Dir(dir))
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
