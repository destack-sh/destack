use crate::{Compiler, ImportResult};
use destack_source::{CacheKind, ModuleId};
use destack_workspace::ModuleDir;

impl Compiler {
    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    pub(crate) fn import_module_bind(&self, module_id: ModuleId) -> ImportResult<()> {
        self.require_import_module_parse(module_id)?;
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // resolve cache handle
        let cache_handle = self.cache_handle_for_module(module_id, None, None, CacheKind::Dir);

        // try to load base DIR from cache
        if let Some(cache) = cache_handle.as_ref()
            && let Ok(Some(entry)) = cache.read_dir()
        {
            let dir = ModuleDir::from_data(entry.payload);
            let module = self.program.modules.get(module_id);
            let mut module = module.write();
            let code = module.code_mut();
            code.dir_base = Some(dir);
            code.dirs.clear();
            tracing::trace!(?module_id, "import.module.bind.cache");
            return Ok(());
        }

        let module = self.program.modules.get(module_id);

        // initialize DIR
        {
            let mut module = module.write();
            let (module_id, module_version) = (module.id, module.version);
            let code = module.code_mut();
            code.dir_base = Some(ModuleDir::new_base(module_id, module_version));
            code.dirs.clear();
        }

        // bind module roots
        let roots = {
            let module = module.read();
            self.bind_module_roots(&module, module.ast())
        };
        {
            let mut module = module.write();
            module.dir_base_mut().roots.extend(roots);
        };

        // attach annotations
        {
            let module = module.read();
            let dir = module.dir_base();
            let ast = module.ast();
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut types = dir.types.write();
            let scope = (
                dir.namespace_scope,
                symbols.get_scope_mark(dir.namespace_scope),
            );
            self.attach_annotations(&module, ast, scope, &mut tree, &mut symbols, &mut types);
        }

        // mark global augmentations (for declaration merging)
        {
            let module = module.read();
            self.mark_global_augmentation_symbols(&module);
        }

        // write base DIR to cache
        if let Some(cache) = cache_handle.as_ref() {
            let module = module.read();
            let payload = module.dir_base().to_data();
            if let Err(error) = cache.write_dir(payload) {
                tracing::debug!(?module_id, ?error, "import.module.bind.cache.write");
            }
        }

        Ok(())
    }
}
