use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{EmitFormat, EnvSnapshot, Platform, ProfileFlags, ProfileKey, Runtime};
use destack_compiler::{Compiler, CompilerOptions};
use destack_source::DiagnosticSeverity;
use destack_workspace::{ProfileId, Program, Session};

use crate::host::generate_host_artifacts;
use crate::option::parse_generator_options;
use crate::platform::collect::{
    collect_platform_bindings, collect_platform_constants, collect_platform_types,
    normalize_binding_catalog, validate_binding_catalog,
};
use crate::platform::emit::{PlatformFileWriter, generate_platform_capability_kind};
use crate::platform::model::{
    BindingCatalog, BindingType, ConstantCatalog, ModuleAbiTypes, ModuleSpec, WorkspaceLayout,
};

/// The builtin URI prefix for platform generator modules.
const PLATFORM_URI_PREFIX: &str = "builtin://platform/";

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
            .load_library(
                "platform",
                self.session.files.clone(),
                self.session.modules.clone(),
                profile_key,
            )
            .expect("platform builtin lib is missing")
    }

    /// Print diagnostics and stop on errors.
    /// TODO #Cleanup: use proper diagnostic reporting here?
    fn report_diagnostics(&self) -> Result<(), String> {
        // exit early when no errors are present
        if !self
            .program
            .diagnostics
            .has_diagnostics_of_severity(DiagnosticSeverity::Error)
        {
            return Ok(());
        }

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

        // select modules by canonical builtin uri prefix
        let mut selected = Vec::new();
        for module_id in platform_modules {
            let module = self.program.modules.get(*module_id);
            let module = module.as_ref();
            let module_uri = module.uri.as_ref();

            // keep matching modules
            if Self::module_matches_requested_domains(module_uri, domains) {
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
                    let module = self.program.modules.get(*module_id);
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

    /// Collect and render bindings for all platform modules.
    fn generate_bindings(
        &self,
        profile_id: ProfileId,
        platform_modules: &[destack_source::ModuleId],
        write_platform_index: bool,
        refresh_stubs: bool,
    ) -> Result<(), String> {
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

        // stop before writing when lazy analysis produced diagnostics
        self.report_diagnostics()?;

        let file_writer = PlatformFileWriter::new(&self.layout);

        // write generated files
        file_writer.write_modules(&catalog, &constants, &abi_types, &modules, refresh_stubs);

        // top-level index
        if write_platform_index {
            file_writer.write_platform_index(&catalog);
        }

        // host bridge structural outputs
        for file in generate_host_artifacts(&self.layout, &abi_types) {
            file_writer.write_file(&file.path, &file.contents);
        }

        Ok(())
    }
}

impl RuntimeGenerator {
    /// Run the platform binding generator from the current workspace.
    pub(crate) fn run() -> Result<(), String> {
        // parse runtime binding generator options
        let options = parse_generator_options()?;

        // build a compiler session for platform bindings
        let cwd = std::env::current_dir()
            .map_err(|error| format!("failed to resolve current directory: {error}"))?;
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
            generator.select_modules(&platform_modules, options.domains.as_ref())?;

        // collect bindings and render outputs
        generator.generate_bindings(
            profile_id,
            &selected_modules,
            options.domains.is_none(),
            options.refresh_stubs,
        )?;

        // regenerate runtime capability kinds from intrinsic capability source of truth
        generate_platform_capability_kind();

        Ok(())
    }

    /// Return true when one module uri matches one requested domain selector set.
    fn module_matches_requested_domains(module_uri: &str, domains: &BTreeSet<String>) -> bool {
        domains.iter().any(|domain| {
            let prefix = format!("{PLATFORM_URI_PREFIX}{domain}/");
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
            "builtin://platform/crypto/key.ds",
            &domains
        ));
        assert!(RuntimeGenerator::module_matches_requested_domains(
            "builtin://platform/fs/path.ds",
            &domains
        ));
        assert!(!RuntimeGenerator::module_matches_requested_domains(
            "builtin://platform/net/socket.ds",
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
        let selected_modules = generator
            .select_modules(
                &platform_modules,
                Some(&BTreeSet::from(["time".to_string()])),
            )
            .expect("time domain should resolve");

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
        let selected_modules = generator
            .select_modules(
                &platform_modules,
                Some(&BTreeSet::from(["fs".to_string(), "os".to_string()])),
            )
            .expect("fs and os domains should resolve");

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
        let selected_modules = generator
            .select_modules(
                &platform_modules,
                Some(&BTreeSet::from(["crypto".to_string()])),
            )
            .expect("crypto domain should resolve");

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
