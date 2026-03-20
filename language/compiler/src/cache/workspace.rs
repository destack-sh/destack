use crate::compile::Compiler;
use destack_workspace::{
    CacheMode, WorkspaceIndexError, WorkspaceIndexHeader, WorkspaceIndexSnapshot, WorkspaceStore,
    hash_workspace_config,
};

use super::{CacheHasher, compiler_version};

impl Compiler {
    /// Resolve the persisted workspace index store when disk mode is enabled.
    fn workspace_store(&self) -> Option<WorkspaceStore<'_>> {
        if self.workspace_cache_mode() != CacheMode::Disk {
            return None;
        }

        let cache_root = self.session.workspace_cache_dir();
        Some(WorkspaceStore::new(
            self.session.cache_store.as_ref(),
            &cache_root,
        ))
    }

    /// Load the workspace index into the current session.
    pub(crate) fn load_workspace_index(&self) -> Result<(), WorkspaceIndexError> {
        let Some(workspace_store) = self.workspace_store() else {
            return Ok(());
        };
        let header = self.workspace_index_header()?;
        let snapshot = workspace_store.load(&header)?;
        let Some(snapshot) = snapshot else {
            return Ok(());
        };

        // apply the loaded index to the session
        self.session.apply_workspace_index(&snapshot);

        Ok(())
    }

    /// Flush the current program state into the workspace index.
    pub(crate) fn flush_workspace_index(&self) -> Result<(), WorkspaceIndexError> {
        let Some(workspace_store) = self.workspace_store() else {
            return Ok(());
        };
        let header = self.workspace_index_header()?;
        let snapshot = WorkspaceIndexSnapshot::from_program(&self.program, header)?;
        workspace_store.save(&snapshot)?;

        Ok(())
    }

    /// Build the current workspace index header.
    fn workspace_index_header(&self) -> Result<WorkspaceIndexHeader, WorkspaceIndexError> {
        let workspace = self.session.workspace_snapshot();
        let config_hash = hash_workspace_config(&workspace, self.session.fs.as_ref())?;

        // include the effective compiler options
        let mut compiler_hasher = CacheHasher::new();
        compiler_hasher.hash_compiler_options(&self.options);
        let compiler_options_hash = compiler_hasher.finish();

        // include the import resolution options separately
        let mut resolve_hasher = CacheHasher::new();
        resolve_hasher.hash_resolve_options(&self.options.import_resolve);
        let resolve_options_hash = resolve_hasher.finish();

        Ok(WorkspaceIndexHeader::new(
            compiler_version(),
            self.session.workspace_root(),
            config_hash,
            compiler_options_hash,
            resolve_options_hash,
            self.workspace_cache_validate(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use destack_source::{File, FileId, FileType, TemporaryPhysicalFileSystem, Uri};
    use destack_workspace::{
        CacheValidate, Destack, Session, Workspace, WorkspaceIndexHeader, WorkspaceStore,
        hash_workspace_config,
    };

    use crate::{CacheHasher, Compiler, CompilerOptions};

    use super::compiler_version;

    /// Load the workspace index through compiler construction.
    #[test]
    fn test_compiler_loads_workspace_index() {
        // set up a disk backed workspace with one source file
        let root = TemporaryPhysicalFileSystem::new_with_prefix("workspace_index_disk");
        let root_path = root.root().to_path_buf();
        root.write_bytes("package.json", br#"{ "name": "workspace-index-test" }"#)
            .unwrap();
        let config_path = root.path_for("destack.json");
        let config_content = r#"{ "cache": { "mode": "disk" } }"#;
        root.write_text("destack.json", config_content).unwrap();
        let module_path = root.path_for("main.ts");
        root.write_text("main.ts", "export const value: number = 1;")
            .unwrap();

        // parse the workspace config
        let config_file = File::from_text_as_jsonc(
            FileId::new(1),
            "destack.json".to_string(),
            Uri::from_path(&config_path),
            Some(config_path.clone()),
            FileType::Json,
            config_content.to_string(),
        )
        .unwrap_or_else(|error| panic!("failed to parse destack.json: {error}"));
        let config = Destack::parse(&Arc::new(config_file))
            .unwrap_or_else(|error| panic!("failed to build destack.json: {error}"));

        // register a module and flush the workspace index
        let workspace = Workspace::single_package(root_path.clone()).with_config(config.clone());
        let session = Arc::new(Session::workspace(root_path.clone(), Arc::new(workspace)));
        let program = session.add_root(root_path.clone());
        let compiler = Compiler::new(
            session.clone(),
            program.clone(),
            CompilerOptions {
                workers: 1,
                ..CompilerOptions::default()
            },
        );
        let expected_string = session.strings.intern("workspace-string");
        let module_id = compiler
            .resolve_path_to_module(&module_path)
            .unwrap_or_else(|error| panic!("failed to register module: {error:?}"));
        compiler
            .flush_workspace_index()
            .unwrap_or_else(|error| panic!("failed to flush workspace index: {error}"));

        // load the saved snapshot directly
        let cache_root = session.workspace_cache_dir();
        let index_store = WorkspaceStore::new(session.cache_store.as_ref(), &cache_root);
        let workspace = session.workspace_snapshot();
        let config_hash = hash_workspace_config(&workspace, session.fs.as_ref())
            .unwrap_or_else(|error| panic!("failed to hash config: {error}"));

        let mut compiler_hasher = CacheHasher::new();
        compiler_hasher.hash_compiler_options(&compiler.options);
        let compiler_options_hash = compiler_hasher.finish();

        let mut resolve_hasher = CacheHasher::new();
        resolve_hasher.hash_resolve_options(&compiler.options.import_resolve);
        let resolve_options_hash = resolve_hasher.finish();

        let header = WorkspaceIndexHeader::new(
            compiler_version(),
            root_path.clone(),
            config_hash,
            compiler_options_hash,
            resolve_options_hash,
            CacheValidate::Strict,
        );
        let snapshot = index_store
            .load(&header)
            .unwrap_or_else(|error| panic!("failed to read workspace index: {error}"))
            .unwrap_or_else(|| panic!("expected workspace index"));

        // check the saved snapshot contents
        assert!(
            snapshot.files.contains_key(&module_path),
            "expected snapshot to include module file"
        );
        assert!(
            snapshot.modules.contains_key(&module_id),
            "expected snapshot to include module id"
        );

        // rebuild a new compiler and ensure it loads the same snapshot
        let workspace = Workspace::single_package(root_path.clone()).with_config(config);
        let session = Arc::new(Session::workspace(root_path.clone(), Arc::new(workspace)));
        let program = session.add_root(root_path.clone());
        let _compiler = Compiler::new(session.clone(), program.clone(), CompilerOptions::default());

        assert!(
            session.has_workspace_index_file_entry(&module_path),
            "expected workspace index to load file entry"
        );
        assert!(
            session.has_workspace_index_module_entry(module_id),
            "expected workspace index to load module entry"
        );
        assert_eq!(
            session.strings.intern("workspace-string"),
            expected_string,
            "expected workspace index to restore the shared string universe"
        );
    }
}
