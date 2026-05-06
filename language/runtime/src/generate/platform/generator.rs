use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use destack_artifact::{
    DiskCacheStore, EmitFormat, EnvironmentStamp, Platform, ProfileFlags, ProfileKey, Runtime,
};
use destack_compiler::{Compiler, CompilerOptions};
use destack_linter::Linter;
use destack_session::Session;
use destack_source::{DiagnosticCollection, DiagnosticSeverity, FileSystem, PhysicalFileSystem};
use destack_workspace::{HostEnvironment, Profile, ProfileId, Ref, Repository, Revision};

use crate::context::GeneratorContext;
use crate::option::parse_generator_options;
use crate::platform::collect::{
    collect_platform_bindings, collect_platform_constants, collect_platform_types,
    module_platform_domain, normalize_binding_catalog, validate_binding_catalog,
};
use crate::platform::emit::{
    RenderSpec, generate_platform_capability_kind, render_platform_bindings_index,
    render_test_harness,
};
use crate::platform::model::{
    BindingCatalog, BindingEntry, BindingType, ConstantCatalog, ConstantEntry, ModuleAbiTypes,
    ModuleSpec, WorkspaceLayout,
};

/// Generator for runtime binding code from the compiler catalog.
pub(crate) struct RuntimeGenerator {
    /// Shared repository state for compiler operations.
    repository: Arc<Repository>,
    /// The active workspace revision.
    revision: Revision,
    /// Revision-scoped semantic helper context.
    context: Arc<GeneratorContext>,
    /// Compiler instance for analysis passes.
    compiler: Arc<Compiler>,
    /// Private generator session for artifact driving.
    session: Session,
    /// The current diagnostics from generator analysis.
    current_diagnostics: Mutex<DiagnosticCollection>,
    /// The generator filesystem layout.
    layout: WorkspaceLayout,
}

impl RuntimeGenerator {
    /// Collect the normalized binding catalog for one profile.
    fn collect_catalog(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
    ) -> BindingCatalog {
        let catalog = collect_platform_bindings(
            &self.compiler,
            &self.context,
            self.repository.strings.as_ref(),
            profile_id,
            platform_modules,
        );
        let catalog = normalize_binding_catalog(catalog);
        validate_binding_catalog(&catalog);

        catalog
    }

    /// Collect the platform constant catalog for one profile.
    fn collect_constants(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
    ) -> ConstantCatalog {
        collect_platform_constants(
            &self.compiler,
            &self.context,
            self.repository.strings.as_ref(),
            profile_id,
            platform_modules,
        )
    }

    /// Collect ABI types grouped by their owning module.
    fn collect_abi_types(
        &self,
        catalog: &BindingCatalog,
        constants: &ConstantCatalog,
        exported_types: &[BindingType],
    ) -> std::collections::BTreeMap<String, ModuleAbiTypes> {
        ModuleAbiTypes::collect_by_module(catalog, constants, exported_types)
    }

    /// Collect generated module metadata grouped by module.
    fn collect_modules(
        &self,
        catalog: &BindingCatalog,
    ) -> std::collections::BTreeMap<String, ModuleSpec> {
        ModuleSpec::collect(catalog, &self.layout)
    }

    /// Build the generator state from a workspace root.
    fn new(cwd: PathBuf) -> Self {
        // repository and imported root revision
        let file_system: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem::new());
        let repository = Arc::new(Repository::new(
            cwd.clone(),
            Arc::new(DiskCacheStore::new()),
            file_system,
            HostEnvironment::capture_process(),
        ));
        let reference = Ref::for_workspace_root(repository.workspace_root());
        let revision = repository
            .current(&reference)
            .unwrap_or_else(|error| panic!("failed to load runtime generator revision: {error}"));
        let context = Arc::new(GeneratorContext::new(repository.clone(), revision));

        // use one worker to avoid compiler analyze lock inversion in generator mode
        let mut compiler_options = CompilerOptions::default();
        compiler_options.workers = 1;

        // create a compiler instance for binding analysis
        let compiler = Arc::new(Compiler::new(repository.clone(), compiler_options));
        let session = Session::fork(
            cwd.clone(),
            cwd.clone(),
            repository.clone(),
            Ref::new("generator"),
            revision,
            compiler.clone(),
            Arc::new(Linter::new(repository.clone())),
            1,
            None,
        )
        .expect("runtime generator session should initialize");

        // resolve generator output paths from the runtime crate root
        let layout = WorkspaceLayout::from_runtime_crate();

        // return the assembled generator context
        Self {
            repository,
            revision,
            context,
            compiler,
            session,
            current_diagnostics: Mutex::new(DiagnosticCollection::new()),
            layout,
        }
    }

    /// Load platform modules from library packages.
    fn load_platform_modules(&self, profile_key: &ProfileKey) -> Vec<destack_source::ModuleId> {
        // load the platform library modules
        self.repository
            .load_builtin_library("platform", profile_key)
            .expect("platform library package is missing")
    }

    /// Analyze platform modules for one profile.
    fn analyze_platform_modules(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
    ) -> Result<(), String> {
        let artifact_keys = self.platform_analysis_roots(profile_id, platform_modules);
        self.session
            .provide_artifacts(&artifact_keys)
            .map_err(|error| error.to_string())?;

        // publish diagnostics from the current module artifact families
        let mut diagnostics = DiagnosticCollection::new();
        for module_id in platform_modules {
            diagnostics.merge_from(&self.repository.module_artifact_diagnostics(
                self.revision,
                *module_id,
                profile_id,
            ));
        }

        let mut current_diagnostics = self
            .current_diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *current_diagnostics = diagnostics;

        Ok(())
    }

    /// Format one platform analysis failure with the current library diagnostics.
    fn format_platform_analysis_failure(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
        requirement: &destack_compiler::RequirementSet,
    ) -> String {
        let mut diagnostics = DiagnosticCollection::new();
        let profile_key = self.profile_key();

        if let Some(selection) = self
            .repository
            .builtins()
            .cached_library_selection(&profile_key)
        {
            for module_id in selection.library_modules {
                diagnostics.merge_from(&self.repository.module_artifact_diagnostics(
                    self.revision,
                    module_id,
                    profile_id,
                ));
            }
        }

        for module_id in platform_modules {
            diagnostics.merge_from(&self.repository.module_artifact_diagnostics(
                self.revision,
                *module_id,
                profile_id,
            ));
        }

        if diagnostics.is_empty() {
            return format!("failed to analyze platform modules: {requirement:?}");
        }

        self.repository
            .print_diagnostics(self.revision, &diagnostics);

        format!(
            "failed to analyze platform modules: {} diagnostics emitted",
            diagnostics.len()
        )
    }

    /// Build the requested platform analysis roots.
    fn platform_analysis_roots(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
    ) -> Vec<destack_artifact::ArtifactKey> {
        let mut artifact_keys = vec![
            destack_artifact::ArtifactKey::language_environment(profile_id),
            destack_artifact::ArtifactKey::library_environment(profile_id),
        ];

        for module_id in platform_modules {
            artifact_keys.push(destack_artifact::ArtifactKey::dir_patched(
                *module_id, profile_id,
            ));
        }

        artifact_keys
    }

    /// Print diagnostics and stop on errors.
    /// TODO #Cleanup: use proper diagnostic reporting here?
    fn report_diagnostics(&self) -> Result<(), String> {
        let diagnostics = self
            .current_diagnostics
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();

        // exit early when no errors are present
        if !diagnostics.has_diagnostics_of_severity(DiagnosticSeverity::Error) {
            return Ok(());
        }

        self.repository
            .print_diagnostics(self.revision, &diagnostics);

        Err("binding generation failed due to diagnostics".to_string())
    }

    /// Filter platform modules by selected domain names.
    fn select_modules(
        &self,
        platform_modules: &[destack_source::ModuleId],
        domains: Option<&BTreeSet<String>>,
    ) -> Result<Vec<destack_source::ModuleId>, String> {
        // return all modules when no domain filter is active
        let Some(domains) = domains else {
            return Ok(platform_modules.to_vec());
        };

        // select modules by platform domain
        let mut selected = Vec::new();
        for module_id in platform_modules {
            let module = self.context.get(*module_id);
            let module = module.as_ref();

            // keep matching modules
            if Self::module_matches_requested_domains(self.repository.as_ref(), &module, domains) {
                selected.push(*module_id);
            }
        }

        // fail loudly when no module matched the requested domains
        if selected.is_empty() {
            let requested = domains.iter().cloned().collect::<Vec<_>>().join(", ");
            let available_modules = platform_modules
                .iter()
                .take(12)
                .map(|module_id| {
                    let module = self.context.get(*module_id);
                    let module = module.as_ref();
                    module.uri.as_ref().to_string()
                })
                .collect::<Vec<_>>()
                .join(", ");

            return Err(format!(
                "no platform modules matched requested domains: {requested}; available modules: {available_modules}"
            ));
        }

        Ok(selected)
    }

    /// Build the platform profile key for binding generation.
    fn profile_key(&self) -> ProfileKey {
        ProfileKey::new(
            EmitFormat::Native,
            Runtime::NativeManaged,
            self.host_platform(),
            None,
            None,
            None,
            vec!["core".to_string(), "platform".to_string()],
            None,
            Vec::new(),
            false,
            false,
            false,
            EnvironmentStamp::from_env_all(),
            ProfileFlags::default(),
        )
    }

    /// Resolve the host platform for binding generation.
    fn host_platform(&self) -> Platform {
        if cfg!(target_os = "windows") {
            return Platform::Windows;
        }
        if cfg!(target_os = "macos") {
            return Platform::MacOS;
        }
        if cfg!(target_os = "linux") {
            return Platform::Linux;
        }
        if cfg!(target_os = "fuchsia") {
            return Platform::Fuchsia;
        }
        if cfg!(target_os = "redox") {
            return Platform::Redox;
        }
        if cfg!(target_os = "hermit") {
            return Platform::Hermit;
        }
        if cfg!(target_os = "ios") {
            return Platform::IOS;
        }
        if cfg!(target_os = "android") {
            return Platform::Android;
        }
        if cfg!(target_os = "wasi") {
            return Platform::Wasi;
        }
        if cfg!(target_os = "emscripten") {
            return Platform::Emscripten;
        }

        Platform::Universal
    }

    /// Collect and render bindings for all platform modules.
    fn generate_bindings(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
        write_platform_index: bool,
        refresh_stubs: bool,
    ) {
        // collect module inputs
        let catalog = self.collect_catalog(profile_id, platform_modules);
        let constants = self.collect_constants(profile_id, platform_modules);
        let exported_types = collect_platform_types(
            &self.compiler,
            &self.context,
            self.repository.strings.as_ref(),
            profile_id,
            platform_modules,
        );
        let abi_types = self.collect_abi_types(&catalog, &constants, &exported_types);
        let modules = self.collect_modules(&catalog);

        // write generated files
        self.write_modules(&catalog, &constants, &abi_types, &modules, refresh_stubs);

        // top-level index
        if write_platform_index {
            self.write_platform_index(&catalog);
        }
    }
}

impl RuntimeGenerator {
    /// Normalize generated file contents to one trailing newline.
    fn normalize_generated_output(&self, contents: &str) -> String {
        let contents = contents.trim_end_matches('\n');
        format!("{contents}\n")
    }

    /// Write one generated file to disk.
    fn write_file(&self, path: &Path, contents: &str) {
        // parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create output directory");
        }

        let contents = self.normalize_generated_output(contents);
        fs::write(path, contents).expect("failed to write generated file");
    }

    /// Synchronize one generated stub file when refresh rules allow it.
    fn sync_stub(&self, path: &Path, generated: &str, refresh_stubs: bool) {
        // create missing files directly
        if !path.exists() {
            self.write_file(path, generated);
            return;
        }

        // preserve handwritten edits unless stub refresh is requested
        if !refresh_stubs {
            return;
        }

        let Ok(existing) = fs::read_to_string(path) else {
            self.write_file(path, generated);
            return;
        };

        // only refresh known generator stubs
        if !existing.contains("// generated by generate-bindings: stub, do not edit") {
            return;
        }

        let generated = self.normalize_generated_output(generated);
        if existing != generated {
            self.write_file(path, &generated);
        }
    }

    /// Write one generated stub file only when the target path is missing.
    fn write_missing_stub(&self, path: &Path, generated: &str) {
        if !path.exists() {
            self.write_file(path, generated);
            return;
        }

        let Ok(existing) = fs::read_to_string(path) else {
            self.write_file(path, generated);
            return;
        };

        if !existing.contains("// generated by generate-bindings: stub, do not edit") {
            return;
        }

        let generated = self.normalize_generated_output(generated);
        if existing != generated {
            self.write_file(path, &generated);
        }
    }

    /// Write generated bindings and ABI files for all collected modules.
    fn write_modules(
        &self,
        catalog: &BindingCatalog,
        constants: &ConstantCatalog,
        abi_types: &std::collections::BTreeMap<String, ModuleAbiTypes>,
        modules: &std::collections::BTreeMap<String, ModuleSpec>,
        refresh_stubs: bool,
    ) {
        let binding_modules = catalog.keys().cloned().collect::<BTreeSet<_>>();
        let mut abi_modules = binding_modules.clone();
        abi_modules.extend(abi_types.keys().filter_map(|module_name| {
            if binding_modules.contains(module_name) {
                return None;
            }

            let layout = self.layout.module_layout(module_name);
            if layout.bindings_path.exists() {
                return None;
            }

            Some(module_name.clone())
        }));

        let empty_types = ModuleAbiTypes::default();

        for module_name in &abi_modules {
            // bindings plus abi
            if let Some(bindings) = catalog.get(module_name) {
                let module = modules
                    .get(module_name)
                    .cloned()
                    .unwrap_or_else(|| ModuleSpec::new(module_name, bindings, &self.layout));
                let types = abi_types.get(module_name).unwrap_or(&empty_types);
                let module_constants = constants.get(module_name);

                self.write_module_stubs(&module, bindings, refresh_stubs);
                self.write_module_bindings(&module, bindings);
                self.write_module_abi(&module, types, module_constants);
                continue;
            }

            // abi-only foreign modules
            let types = abi_types.get(module_name).unwrap_or(&empty_types);
            let layout = self.layout.module_layout(module_name);
            let rendered = types.render(module_name, constants.get(module_name));
            self.write_file(&layout.abi_types_path, &rendered);
        }
    }

    /// Write the generated top-level platform index.
    fn write_platform_index(&self, catalog: &BindingCatalog) {
        let binding_modules = catalog.keys().cloned().collect::<BTreeSet<_>>();
        let rendered = render_platform_bindings_index(&binding_modules);
        let path = self.layout.platform_generated_path();

        self.write_file(&path, &rendered);
    }

    /// Write the handwritten and generated stubs for one module.
    fn write_module_stubs(
        &self,
        module: &ModuleSpec,
        bindings: &BTreeMap<String, BindingEntry>,
        refresh_stubs: bool,
    ) {
        use crate::platform::emit::{
            render_host_router_stub, render_host_stub, render_module_mod_stub,
            render_native_host_router_stub, render_native_stub, render_os_backend_mod_stub,
            render_simulation_mod_stub, render_simulation_native_stub, render_simulation_vm_stub,
            render_vm_stub,
        };

        // module surface
        let module_stub =
            render_module_mod_stub(module.has_host_dispatch, module.has_simulation_dispatch);
        self.write_missing_stub(&module.layout.mod_path, &module_stub);

        let host_module_path = module.layout.dir.join("host/mod.rs");
        let native_stub = if module.has_host_dispatch && !host_module_path.exists() {
            render_native_host_router_stub()
        } else {
            render_native_stub(&module.name, bindings)
        };
        self.sync_stub(&module.layout.native_path, &native_stub, refresh_stubs);

        let vm_stub = render_vm_stub(&module.name, bindings);
        self.sync_stub(&module.layout.vm_path, &vm_stub, refresh_stubs);

        // host routing
        if module.has_host_dispatch {
            if !host_module_path.exists() {
                let host_stub = render_host_router_stub();
                self.write_missing_stub(&module.layout.host_path, &host_stub);
            }

            let unix_stub = render_os_backend_mod_stub();
            self.write_missing_stub(&module.layout.unix_mod_path, &unix_stub);

            let windows_stub = render_os_backend_mod_stub();
            self.write_missing_stub(&module.layout.windows_mod_path, &windows_stub);

            let unsupported_stub = render_host_stub(&module.name, bindings);
            self.sync_stub(
                &module.layout.unsupported_path,
                &unsupported_stub,
                refresh_stubs,
            );
        }

        // simulation routing
        if module.has_simulation_dispatch {
            let simulation_mod_stub = render_simulation_mod_stub();
            self.sync_stub(
                &module.layout.simulation_mod_path,
                &simulation_mod_stub,
                refresh_stubs,
            );

            let simulation_native_stub = render_simulation_native_stub(&module.name, bindings);
            self.sync_stub(
                &module.layout.simulation_native_path,
                &simulation_native_stub,
                refresh_stubs,
            );

            let simulation_vm_stub = render_simulation_vm_stub(&module.name, bindings);
            self.sync_stub(
                &module.layout.simulation_vm_path,
                &simulation_vm_stub,
                refresh_stubs,
            );
        } else {
            self.remove_simulation_stubs(module);
        }

        // stale layout
        self.remove_legacy_impl(module);
    }

    /// Write generated binding output for one module.
    fn write_module_bindings(
        &self,
        module: &ModuleSpec,
        bindings: &BTreeMap<String, BindingEntry>,
    ) {
        let spec = RenderSpec::new(&module.name, bindings);
        let generated = spec.render();

        self.write_file(&module.layout.bindings_path, &generated);

        // generated test harness
        self.write_module_test_harness(module, &spec);
    }

    /// Write generated ABI output for one module.
    fn write_module_abi(
        &self,
        module: &ModuleSpec,
        types: &ModuleAbiTypes,
        constants: Option<&BTreeMap<String, ConstantEntry>>,
    ) {
        let abi_types = types.render(&module.name, constants);

        self.write_file(&module.layout.abi_types_path, &abi_types);
    }

    /// Write one generated test harness when the module uses one.
    fn write_module_test_harness(&self, module: &ModuleSpec, spec: &RenderSpec<'_>) {
        let harness_stub_path = module.layout.dir.join("tests/harness.rs");
        let harness_generated_path = module.layout.dir.join("tests/harness.generated.rs");

        // skip modules that do not use the generated harness pattern
        if !harness_stub_path.exists() && !harness_generated_path.exists() {
            return;
        }

        let harness = render_test_harness(spec);
        self.write_file(&harness_generated_path, &harness);
    }

    /// Remove the simulation scaffold when the module does not use it.
    fn remove_simulation_stubs(&self, module: &ModuleSpec) {
        let Some(simulation_directory) = module.layout.simulation_mod_path.parent() else {
            panic!(
                "failed to resolve simulation directory for module {}",
                module.layout.dir.display()
            );
        };
        if !simulation_directory.exists() {
            return;
        }

        fs::remove_dir_all(simulation_directory).unwrap_or_else(|error| {
            panic!(
                "failed to remove stale simulation directory {}: {error}",
                simulation_directory.display()
            )
        });
    }

    /// Remove the legacy nested implementation scaffold.
    fn remove_legacy_impl(&self, module: &ModuleSpec) {
        let legacy_impl_dir = module.layout.dir.join("runtime");

        if !legacy_impl_dir.exists() {
            return;
        }

        fs::remove_dir_all(&legacy_impl_dir).unwrap_or_else(|error| {
            panic!(
                "failed to remove stale runtime implementation directory {}: {error}",
                legacy_impl_dir.display()
            )
        });
    }
}

impl RuntimeGenerator {
    /// Run the platform binding generator from the current workspace.
    pub(crate) fn run() -> Result<(), String> {
        // parse runtime binding generator options
        let options = parse_generator_options()?;

        // build a compiler repository for platform bindings
        let cwd = std::env::current_dir()
            .map_err(|error| format!("failed to resolve current directory: {error}"))?;
        let generator = Self::new(cwd);

        // configure the native profile for platform modules
        let profile_key = generator.profile_key();
        let profile_id = Profile::id_for_key(&profile_key);

        // load the platform modules for analysis
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules =
            generator.select_modules(&platform_modules, options.domains.as_ref())?;

        // keep full analysis for whole-library generation
        // targeted domain runs should only analyze the selected surface so unrelated module drift
        // does not block regeneration of one module under audit
        let analysis_modules = if options.domains.is_some() {
            selected_modules.clone()
        } else {
            platform_modules.clone()
        };

        // run analysis passes before extraction
        generator.analyze_platform_modules(profile_id, &analysis_modules)?;

        // validate diagnostics before rendering output
        generator.report_diagnostics()?;

        // collect bindings and render outputs
        generator.generate_bindings(
            profile_id,
            &selected_modules,
            options.domains.is_none(),
            options.refresh_stubs,
        );

        // regenerate runtime capability kinds from intrinsic capability source of truth
        generate_platform_capability_kind();

        Ok(())
    }

    /// Return true when one module matches one requested domain selector set.
    fn module_matches_requested_domains(
        repository: &Repository,
        module: &destack_workspace::Module,
        domains: &BTreeSet<String>,
    ) -> bool {
        let Some(domain) = module_platform_domain(repository, module) else {
            return false;
        };

        domains.contains(&domain)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use crate::platform::collect::collect_platform_types;
    use crate::platform::model::BindingType;

    use super::RuntimeGenerator;

    /// Match platform modules by requested domains.
    #[test]
    fn test_module_matches_requested_domains() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let generator = RuntimeGenerator::new(workspace_root);
        let profile_key = generator.profile_key();
        let platform_modules = generator.load_platform_modules(&profile_key);
        let domains = BTreeSet::from(["crypto".to_string(), "fs".to_string()]);
        let mut matches = Vec::new();
        let mut misses = Vec::new();

        for module_id in platform_modules {
            let module = generator.context.get(module_id);
            let module = module.as_ref();

            if RuntimeGenerator::module_matches_requested_domains(
                generator.repository.as_ref(),
                module,
                &domains,
            ) {
                matches.push(module.uri.as_ref().to_string());
            } else {
                misses.push(module.uri.as_ref().to_string());
            }
        }

        assert!(matches.iter().any(|uri| uri.contains("/platform/crypto/")));
        assert!(matches.iter().any(|uri| uri.contains("/platform/fs/")));
        assert!(misses.iter().any(|uri| uri.contains("/platform/net/")));
    }

    /// Keep runtime binding catalog extraction live for one representative platform domain.
    #[test]
    fn test_collect_catalog_keeps_time_domain_bindings_non_empty() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let generator = RuntimeGenerator::new(workspace_root);
        let profile_key = generator.profile_key();
        let profile_id = Profile::id_for_key(&profile_key);
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules = generator
            .select_modules(
                &platform_modules,
                Some(&BTreeSet::from(["time".to_string()])),
            )
            .expect("time domain should resolve");

        generator
            .analyze_platform_modules(profile_id, &selected_modules)
            .expect("platform analysis should complete");

        let catalog = generator.collect_catalog(profile_id, &selected_modules);
        assert!(
            !catalog.is_empty(),
            "expected time-domain binding catalog to stay non-empty"
        );
        assert!(catalog.contains_key("time"));
    }

    /// Keep string arrays in binding signatures as arrays, not slices.
    #[test]
    fn test_collect_catalog_keeps_string_arrays_distinct_from_string_slices() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let generator = RuntimeGenerator::new(workspace_root);
        let profile_key = generator.profile_key();
        let profile_id = Profile::id_for_key(&profile_key);
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules = generator
            .select_modules(
                &platform_modules,
                Some(&BTreeSet::from(["fs".to_string(), "os".to_string()])),
            )
            .expect("fs and os domains should resolve");

        generator
            .analyze_platform_modules(profile_id, &selected_modules)
            .expect("platform analysis should complete");

        let catalog = generator.collect_catalog(profile_id, &selected_modules);
        let fs_binding = &catalog["fs"]["destack.fs.xattr.listxattr"];
        let os_binding = &catalog["os"]["destack.os.media.delete"];

        assert_eq!(
            fs_binding.return_binding,
            BindingType::Array(Box::new(BindingType::String))
        );
        assert_eq!(
            os_binding.parameters[0].binding_type,
            BindingType::Array(Box::new(BindingType::String))
        );
    }

    /// Keep exported string array fields as arrays in generated ABI types.
    #[test]
    fn test_collect_platform_types_keeps_string_array_fields_as_arrays() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let generator = RuntimeGenerator::new(workspace_root);
        let profile_key = generator.profile_key();
        let profile_id = Profile::id_for_key(&profile_key);
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules = generator
            .select_modules(
                &platform_modules,
                Some(&BTreeSet::from(["crypto".to_string()])),
            )
            .expect("crypto domain should resolve");

        generator
            .analyze_platform_modules(profile_id, &selected_modules)
            .expect("platform analysis should complete");

        let exported_types = collect_platform_types(
            &generator.compiler,
            &generator.context,
            generator.repository.strings.as_ref(),
            profile_id,
            &selected_modules,
        );
        let descriptor = exported_types
            .iter()
            .find_map(|binding_type| match binding_type {
                BindingType::Struct { name, fields, .. }
                    if name == "CryptoCertificateDescriptor" =>
                {
                    Some(fields)
                }
                _ => None,
            })
            .expect("missing CryptoCertificateDescriptor export");
        let subject_alternative_names = descriptor
            .iter()
            .find(|field| field.name == "subjectAlternativeNames")
            .expect("missing subjectAlternativeNames field");

        assert_eq!(
            subject_alternative_names.binding_type,
            BindingType::Array(Box::new(BindingType::String))
        );
    }
}
