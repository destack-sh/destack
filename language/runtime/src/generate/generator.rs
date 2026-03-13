use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{BuildKey, Compiler, CompilerOptions, Task, TaskOutcome};
use destack_source::DiagnosticSeverity;
use destack_workspace::{
    ArtifactKey, EnvSnapshot, OutputFormat, Platform, ProfileFlags, ProfileId, ProfileKey, Program,
    Runtime, Session,
};

use crate::analyze::{
    BindingCatalog, BindingEntry, BindingType, ConstantCatalog, ConstantEntry,
    collect_platform_bindings, collect_platform_constants, collect_platform_types,
    normalize_binding_catalog, validate_binding_catalog,
};
use crate::capability::generate_platform_capability_kind;
use crate::emit::{RenderSpec, render_platform_bindings_index};
use crate::model::{ModuleAbiTypes, ModuleSpec, WorkspaceLayout};
use crate::option::parse_generator_options;

/// Generator for runtime binding code from the compiler catalog.
pub(crate) struct RuntimeGenerator {
    /// Shared session state for compiler operations.
    session: Arc<Session>,
    /// Program handle for compiled modules.
    program: Arc<Program>,
    /// Compiler instance for analysis passes.
    compiler: Arc<Compiler>,
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
            &self.program,
            self.session.strings.as_ref(),
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
            &self.program,
            self.session.strings.as_ref(),
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
        // build the session and program from the workspace root
        let session = Arc::new(Session::new(cwd.clone()));
        let program = session.add_root(cwd);

        // use one worker to avoid compiler analyze lock inversion in generator mode
        let mut compiler_options = CompilerOptions::default();
        compiler_options.workers = 1;

        // create a compiler instance for binding analysis
        let compiler = Arc::new(Compiler::new(
            session.clone(),
            program.clone(),
            compiler_options,
        ));

        // resolve generator output paths from the runtime crate root
        let layout = WorkspaceLayout::from_runtime_crate();

        // return the assembled generator context
        Self {
            session,
            program,
            compiler,
            layout,
        }
    }

    /// Load platform modules from builtin libraries.
    fn load_platform_modules(&self, profile_key: &ProfileKey) -> Vec<destack_source::ModuleId> {
        // load the builtin platform library modules
        self.session
            .builtins
            .load_lib(
                "platform",
                self.session.files.clone(),
                self.session.modules.clone(),
                profile_key,
            )
            .expect("platform builtin lib is missing")
    }

    /// Analyze platform modules for one profile.
    fn analyze_platform_modules(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
    ) {
        // resolve builtin symbols for the target profile
        let builtins_outcome = self.compiler.run_task(Task::new(BuildKey::Artifact(
            ArtifactKey::LanguageEnvironment {
                profile: profile_id,
            },
        )));
        self.assert_task_complete(builtins_outcome, "ResolveBuiltins");

        // resolve builtin libraries for the target profile
        let resolve_outcome =
            self.compiler
                .run_task(Task::new(BuildKey::Artifact(ArtifactKey::LibEnvironment {
                    profile: profile_id,
                })));
        self.assert_task_complete(resolve_outcome, "ResolveLibs");

        // build the full platform surface sequentially
        for module_id in platform_modules {
            let outcome =
                self.compiler
                    .run_task(Task::new(BuildKey::Artifact(ArtifactKey::DirPatched {
                        module: *module_id,
                        profile: profile_id,
                    })));
            let task_name = format!("BuildDirPatched({module_id:?})");
            self.assert_task_complete(outcome, &task_name);
        }
    }

    /// Print diagnostics and stop on errors.
    /// TODO #Cleanup: use proper diagnostic reporting here?
    fn report_diagnostics(&self) {
        // exit early when no errors are present
        if !self
            .program
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            return;
        }

        panic!("binding generation failed due to diagnostics");
    }

    /// Filter platform modules by selected domain names.
    fn select_modules(
        &self,
        platform_modules: &[destack_source::ModuleId],
        domains: Option<&BTreeSet<String>>,
    ) -> Vec<destack_source::ModuleId> {
        // return all modules when no domain filter is active
        let Some(domains) = domains else {
            return platform_modules.to_vec();
        };

        // select modules by canonical builtin uri prefix
        let mut selected = Vec::new();
        for module_id in platform_modules {
            let module = self.program.modules.get(*module_id);
            let module = module.read();
            let module_uri = module.uri.as_ref();

            // keep matching modules
            if Self::module_matches_requested_domains(module_uri, domains) {
                selected.push(*module_id);
            }
        }

        // fail loudly when no module matched the requested domains
        if selected.is_empty() {
            let requested = domains.iter().cloned().collect::<Vec<_>>().join(", ");
            panic!("no platform modules matched requested domains: {requested}");
        }

        selected
    }

    /// Build the platform profile key for binding generation.
    fn profile_key(&self) -> ProfileKey {
        ProfileKey::new(
            OutputFormat::Native,
            Runtime::NativeHosted,
            self.host_platform(),
            None,
            None,
            None,
            vec!["native".to_string(), "platform".to_string()],
            false,
            false,
            false,
            EnvSnapshot::from_env_all(),
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
        if cfg!(target_os = "freebsd") {
            return Platform::FreeBsd;
        }
        if cfg!(target_os = "openbsd") {
            return Platform::OpenBsd;
        }
        if cfg!(target_os = "netbsd") {
            return Platform::NetBsd;
        }
        if cfg!(target_os = "dragonfly") {
            return Platform::DragonFly;
        }
        if cfg!(target_os = "solaris") {
            return Platform::Solaris;
        }
        if cfg!(target_os = "illumos") {
            return Platform::Illumos;
        }
        if cfg!(target_os = "haiku") {
            return Platform::Haiku;
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

    /// Assert that one compiler task completed successfully.
    fn assert_task_complete(&self, outcome: TaskOutcome, task_name: &str) {
        match outcome {
            TaskOutcome::Complete { product: _ } => {}
            TaskOutcome::Skipped { reason } => {
                panic!("binding generation task {task_name} was skipped: {reason:?}");
            }
            TaskOutcome::Error { error } => {
                panic!(
                    "binding generation task {task_name} failed: {} ({error:?})",
                    error.message(&self.program),
                );
            }
            TaskOutcome::Yield { requirement } => {
                panic!("binding generation task {task_name} yielded unexpectedly: {requirement:?}");
            }
        }
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
            &self.program,
            self.session.strings.as_ref(),
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
        use crate::emit::{
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
        let generated = RenderSpec::new(&module.name, bindings).render();

        self.write_file(&module.layout.bindings_path, &generated);
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
    pub(crate) fn run() {
        // parse runtime binding generator options
        let options = parse_generator_options();

        // build a compiler session for platform bindings
        let cwd = std::env::current_dir().expect("failed to resolve current directory");
        let generator = Self::new(cwd);

        // configure the native profile for platform modules
        let profile_key = generator.profile_key();
        let profile_id = generator
            .program
            .profiles
            .get_or_create(profile_key.clone());

        // load the platform modules for analysis
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules =
            generator.select_modules(&platform_modules, options.domains.as_ref());

        // keep full analysis for whole-library generation
        // targeted domain runs should only analyze the selected surface so unrelated module drift
        // does not block regeneration of one module under audit
        let analysis_modules = if options.domains.is_some() {
            selected_modules.clone()
        } else {
            platform_modules.clone()
        };

        // run analysis passes before extraction
        generator.analyze_platform_modules(profile_id, &analysis_modules);

        // validate diagnostics before rendering output
        generator.report_diagnostics();

        // collect bindings and render outputs
        generator.generate_bindings(
            profile_id,
            &selected_modules,
            options.domains.is_none(),
            options.refresh_stubs,
        );

        // regenerate runtime capability kinds from intrinsic capability source of truth
        generate_platform_capability_kind();
    }

    /// Return true when one module uri matches one requested domain selector set.
    fn module_matches_requested_domains(module_uri: &str, domains: &BTreeSet<String>) -> bool {
        domains.iter().any(|domain| {
            let prefix = format!("builtin://lib/platform/{domain}/");
            module_uri.starts_with(&prefix)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use super::{BindingType, RuntimeGenerator, collect_platform_types};

    /// Match platform module uris by requested domains.
    #[test]
    fn test_module_matches_requested_domains() {
        let domains = BTreeSet::from(["crypto".to_string(), "fs".to_string()]);

        assert!(RuntimeGenerator::module_matches_requested_domains(
            "builtin://lib/platform/crypto/key.ds",
            &domains
        ));
        assert!(RuntimeGenerator::module_matches_requested_domains(
            "builtin://lib/platform/fs/path.ds",
            &domains
        ));
        assert!(!RuntimeGenerator::module_matches_requested_domains(
            "builtin://lib/platform/net/socket.ds",
            &domains
        ));
    }

    /// Keep runtime binding catalog extraction live for one representative platform domain.
    #[test]
    fn test_collect_catalog_keeps_time_domain_bindings_non_empty() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let generator = RuntimeGenerator::new(workspace_root);
        let profile_key = generator.profile_key();
        let profile_id = generator
            .program
            .profiles
            .get_or_create(profile_key.clone());
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules = generator.select_modules(
            &platform_modules,
            Some(&BTreeSet::from(["time".to_string()])),
        );

        generator.analyze_platform_modules(profile_id, &selected_modules);

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
        let profile_id = generator
            .program
            .profiles
            .get_or_create(profile_key.clone());
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules = generator.select_modules(
            &platform_modules,
            Some(&BTreeSet::from(["fs".to_string(), "os".to_string()])),
        );

        generator.analyze_platform_modules(profile_id, &selected_modules);

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
        let profile_id = generator
            .program
            .profiles
            .get_or_create(profile_key.clone());
        let platform_modules = generator.load_platform_modules(&profile_key);
        let selected_modules = generator.select_modules(
            &platform_modules,
            Some(&BTreeSet::from(["crypto".to_string()])),
        );

        generator.analyze_platform_modules(profile_id, &selected_modules);

        let exported_types = collect_platform_types(
            &generator.compiler,
            &generator.program,
            generator.session.strings.as_ref(),
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
