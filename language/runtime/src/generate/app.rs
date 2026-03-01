use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ResolveTask, TaskOutcome};
use destack_source::DiagnosticSeverity;
use destack_workspace::{
    EnvSnapshot, OutputFormat, Platform, ProfileFlags, ProfileId, ProfileKey, Program, Runtime,
    Session,
};

use crate::capability::generate_platform_capability_kind;
use crate::collect::collect_platform_bindings;
use crate::emit::{
    DomainAbiTypes, collect_domain_abi_types, render_abi_types, render_domain_bindings,
    render_domain_mod_stub, render_domain_test_harness_generated, render_domain_test_harness_stub,
    render_domain_tests_basic_stub, render_domain_tests_mod_stub, render_domain_tests_stub,
    render_host_router_stub, render_host_stub, render_native_stub, render_os_backend_mod_stub,
    render_platform_bindings_index, render_runtime_mod_stub, render_runtime_native_stub,
    render_runtime_vm_stub, render_simulation_mod_stub, render_simulation_native_stub,
    render_simulation_vm_stub, render_vm_stub, runtime_domain_abi_types_path,
    runtime_domain_bindings_path, runtime_domain_host_path, runtime_domain_mod_path,
    runtime_domain_native_path, runtime_domain_runtime_mod_path,
    runtime_domain_runtime_native_path, runtime_domain_runtime_vm_path,
    runtime_domain_simulation_mod_path, runtime_domain_simulation_native_path,
    runtime_domain_simulation_vm_path, runtime_domain_test_harness_generated_path,
    runtime_domain_test_harness_path, runtime_domain_tests_basic_path,
    runtime_domain_tests_dir_path, runtime_domain_tests_mod_path, runtime_domain_tests_path,
    runtime_domain_unix_mod_path, runtime_domain_unsupported_path, runtime_domain_vm_path,
    runtime_domain_windows_mod_path, runtime_platform_generated_path, write_domain_bindings,
};
use crate::model::{BindingEntry, CatalogBindingScope};
use crate::normalize::normalize_binding_catalog;
use crate::option::parse_generator_options;
use crate::refresh::{write_missing_stub_file, write_stub_file};
use crate::validate::validate_binding_catalog;

/// Compiler state used while generating bindings.
struct GeneratorContext {
    /// Shared session state for compiler operations.
    session: Arc<Session>,
    /// Program handle for compiled modules.
    program: Arc<Program>,
    /// Compiler instance for analysis passes.
    compiler: Arc<Compiler>,
}

impl GeneratorContext {
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

        // return the assembled generator context
        Self {
            session,
            program,
            compiler,
        }
    }
}

/// Run the platform binding generator.
pub(crate) fn run() {
    // parse runtime binding generator options
    let options = parse_generator_options();

    // build a compiler session for platform bindings
    let cwd = std::env::current_dir().expect("failed to resolve current directory");
    let context = GeneratorContext::new(cwd);

    // configure the native profile for platform modules
    let profile_key = platform_profile_key();
    let profile_id = context.program.profiles.get_or_create(profile_key.clone());

    // load the platform modules for analysis
    let platform_modules = load_platform_modules(&context.session, &profile_key);
    let selected_modules = filter_platform_modules(
        &context.program,
        &platform_modules,
        options.domains.as_ref(),
    );
    let analysis_modules = selected_modules.clone();

    // run analysis passes before extraction
    analyze_platform_modules(
        &context.compiler,
        &context.program,
        profile_id,
        &analysis_modules,
    );

    // validate diagnostics before rendering output
    report_diagnostics(&context.program);

    // collect bindings and render outputs
    generate_bindings(
        &context.program,
        context.session.strings.as_ref(),
        profile_id,
        &selected_modules,
        options.domains.is_none(),
        options.refresh_stubs,
    );

    // regenerate runtime capability kinds from intrinsic capability source of truth
    generate_platform_capability_kind();
}

/// Filter platform modules by selected domains.
fn filter_platform_modules(
    program: &Program,
    platform_modules: &[destack_source::ModuleId],
    domains: Option<&BTreeSet<String>>,
) -> Vec<destack_source::ModuleId> {
    // return all modules when no domain filter is active
    let Some(domains) = domains else {
        return platform_modules.to_vec();
    };

    // select modules by canonical builtin uri prefix
    let mut filtered = Vec::new();
    for module_id in platform_modules {
        let module = program.modules.get(*module_id);
        let module = module.read();
        let module_uri = module.uri.as_ref();

        // keep module when any requested domain matches its uri
        if module_matches_requested_domains(module_uri, domains) {
            filtered.push(*module_id);
        }
    }

    // fail loudly when no module matched the requested domains
    if filtered.is_empty() {
        let requested = domains.iter().cloned().collect::<Vec<_>>().join(", ");
        panic!("no platform modules matched requested domains: {requested}");
    }

    let requested = domains.iter().cloned().collect::<Vec<_>>().join(", ");
    eprintln!(
        "generate-bindings: selected {} module(s) for domains [{requested}]",
        filtered.len()
    );

    filtered
}

/// Return true when one module uri matches one requested domain selector set.
fn module_matches_requested_domains(module_uri: &str, domains: &BTreeSet<String>) -> bool {
    domains.iter().any(|domain| {
        let prefix = format!("builtin://lib/platform/{domain}/");
        module_uri.starts_with(&prefix)
    })
}

/// Build the platform profile key for binding generation.
fn platform_profile_key() -> ProfileKey {
    ProfileKey::new(
        OutputFormat::Native,
        Runtime::NativeHosted,
        host_platform(),
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
fn host_platform() -> Platform {
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

/// Load platform modules from builtin libraries.
fn load_platform_modules(
    session: &Session,
    profile_key: &ProfileKey,
) -> Vec<destack_source::ModuleId> {
    // load the builtin platform library modules
    session
        .builtins
        .load_lib(
            "platform",
            session.files.clone(),
            session.modules.clone(),
            profile_key,
        )
        .expect("platform builtin lib is missing")
}

/// Enqueue analysis tasks for platform modules.
fn analyze_platform_modules(
    compiler: &Compiler,
    program: &Program,
    profile_id: ProfileId,
    platform_modules: &[destack_source::ModuleId],
) {
    // build the profile stamp for analysis tasks
    let profile_stamp = compiler.profile_stamp(profile_id);

    // resolve builtin symbols for the target profile
    eprintln!("generate-bindings: resolving builtins");
    let builtins_outcome = compiler.run_task(ResolveTask::ResolveBuiltins {
        profile: profile_stamp,
    });
    assert_task_complete(builtins_outcome, "ResolveBuiltins", program);

    // resolve builtin libraries for the target profile
    eprintln!("generate-bindings: resolving platform libs");
    let resolve_outcome = compiler.run_task(ResolveTask::ResolveLibs {
        profile: profile_stamp,
    });
    assert_task_complete(resolve_outcome, "ResolveLibs", program);

    // analyze module declarations sequentially
    for module_id in platform_modules {
        let module = program.modules.get(*module_id);
        let module = module.read();
        eprintln!(
            "generate-bindings: analyzing module {module_id:?} ({})",
            module.uri
        );
        let outcome = compiler.run_task(AnalyzeTask::AnalyzeModuleDeclare {
            module: compiler.module_stamp(*module_id),
            profile: profile_stamp,
        });
        let task_name = format!("AnalyzeModuleDeclare({module_id:?})");
        assert_task_complete(outcome, &task_name, program);
    }
}

/// Assert that a compiler task completed successfully.
fn assert_task_complete(outcome: TaskOutcome, task_name: &str, program: &Program) {
    match outcome {
        TaskOutcome::Complete => {}
        TaskOutcome::Skipped { reason } => {
            panic!("binding generation task {task_name} was skipped: {reason:?}");
        }
        TaskOutcome::Error { error } => {
            panic!(
                "binding generation task {task_name} failed: {} ({error:?})",
                error.message(program),
            );
        }
        TaskOutcome::Yield { dependency } => {
            panic!("binding generation task {task_name} yielded unexpectedly: {dependency:?}");
        }
    }
}

/// Print diagnostics and stop on errors.
fn report_diagnostics(program: &Program) {
    // exit early when no errors are present
    if !program
        .diagnostics
        .has_diagnostics_of_severity(DiagnosticSeverity::Error)
    {
        return;
    }

    // emit diagnostics and fail fast
    let diagnostics = program.diagnostics.drain();
    for diagnostic in diagnostics {
        let file = program.files.get(diagnostic.file_id);
        eprintln!(
            "{}:{}:{}: {} {}",
            file.uri,
            diagnostic.primary_span.span.start,
            diagnostic.primary_span.span.end,
            diagnostic.code,
            diagnostic.message
        );
        eprintln!("{diagnostic:?}");
    }
    panic!("binding generation failed due to diagnostics");
}

/// Collect and render bindings for all platform domains.
fn generate_bindings(
    program: &Program,
    strings: &destack_base::StringPool,
    profile_id: ProfileId,
    platform_modules: &[destack_source::ModuleId],
    write_platform_index: bool,
    refresh_stubs: bool,
) {
    // collect bindings from the analyzed program
    let catalog = collect_platform_bindings(program, strings, profile_id, platform_modules);
    let catalog = normalize_binding_catalog(catalog);
    validate_binding_catalog(&catalog);
    let domain_types = collect_domain_abi_types(&catalog);

    // render bindings for each discovered domain
    let binding_domains = catalog.keys().cloned().collect::<BTreeSet<_>>();
    let mut abi_domains = binding_domains.clone();
    if write_platform_index {
        abi_domains.extend(domain_types.keys().cloned());
    }

    let empty_types = DomainAbiTypes::default();

    for domain in &binding_domains {
        ensure_domain_mod_has_tests(domain);
        let bindings = catalog
            .get(domain)
            .unwrap_or_else(|| panic!("missing bindings for domain {domain}"));
        ensure_domain_test_scaffold(domain, bindings);
    }

    for domain in &abi_domains {
        if let Some(bindings) = catalog.get(domain) {
            let has_host_dispatch = bindings
                .values()
                .any(|entry| entry.scope != CatalogBindingScope::Runtime);
            let has_runtime_dispatch = bindings
                .values()
                .any(|entry| entry.scope == CatalogBindingScope::Runtime);

            let mod_path = runtime_domain_mod_path(domain);
            let stub = render_domain_mod_stub(has_host_dispatch, has_runtime_dispatch);
            write_missing_stub_file(&mod_path, &stub);

            let generated = render_domain_bindings(domain, bindings);
            let path = runtime_domain_bindings_path(domain);
            write_domain_bindings(&path, &generated);

            let native_path = runtime_domain_native_path(domain);
            let stub = render_native_stub(domain, bindings);
            write_stub_file(&native_path, &stub, refresh_stubs);

            let vm_path = runtime_domain_vm_path(domain);
            let stub = render_vm_stub(domain, bindings);
            write_stub_file(&vm_path, &stub, refresh_stubs);

            if has_host_dispatch {
                let host_path = runtime_domain_host_path(domain);
                let stub = render_host_router_stub();
                write_missing_stub_file(&host_path, &stub);

                let unix_mod_path = runtime_domain_unix_mod_path(domain);
                let stub = render_os_backend_mod_stub();
                write_missing_stub_file(&unix_mod_path, &stub);

                let windows_mod_path = runtime_domain_windows_mod_path(domain);
                let stub = render_os_backend_mod_stub();
                write_missing_stub_file(&windows_mod_path, &stub);

                let unsupported_path = runtime_domain_unsupported_path(domain);
                let stub = render_host_stub(domain, bindings);
                write_stub_file(&unsupported_path, &stub, refresh_stubs);

                let simulation_mod_path = runtime_domain_simulation_mod_path(domain);
                let stub = render_simulation_mod_stub();
                write_stub_file(&simulation_mod_path, &stub, refresh_stubs);

                let simulation_native_path = runtime_domain_simulation_native_path(domain);
                let stub = render_simulation_native_stub(domain, bindings);
                write_stub_file(&simulation_native_path, &stub, refresh_stubs);

                let simulation_vm_path = runtime_domain_simulation_vm_path(domain);
                let stub = render_simulation_vm_stub(domain, bindings);
                write_stub_file(&simulation_vm_path, &stub, refresh_stubs);
            }

            if has_runtime_dispatch {
                let runtime_mod_path = runtime_domain_runtime_mod_path(domain);
                let stub = render_runtime_mod_stub();
                write_stub_file(&runtime_mod_path, &stub, refresh_stubs);

                let runtime_native_path = runtime_domain_runtime_native_path(domain);
                let stub = render_runtime_native_stub(domain, bindings);
                write_stub_file(&runtime_native_path, &stub, refresh_stubs);

                let runtime_vm_path = runtime_domain_runtime_vm_path(domain);
                let stub = render_runtime_vm_stub(domain, bindings);
                write_stub_file(&runtime_vm_path, &stub, refresh_stubs);
            }
        }

        let types = domain_types.get(domain).unwrap_or(&empty_types);
        let abi_types = render_abi_types(domain, types);
        let abi_types_path = runtime_domain_abi_types_path(domain);
        write_domain_bindings(&abi_types_path, &abi_types);
    }

    if write_platform_index {
        let platform_generated = render_platform_bindings_index(&binding_domains);
        let platform_path = runtime_platform_generated_path();
        write_domain_bindings(&platform_path, &platform_generated);
    }
}

/// Ensure one domain module declares its test module.
fn ensure_domain_mod_has_tests(domain: &str) {
    let mod_path = runtime_domain_mod_path(domain);
    if !mod_path.exists() {
        return;
    }

    let Ok(source) = fs::read_to_string(&mod_path) else {
        return;
    };
    if source.contains("mod tests;") {
        return;
    }

    let declaration = "#[cfg(test)]\nmod tests;\n";
    let rewritten = if let Some(position) = source.find("pub mod vm;") {
        let (before, after) = source.split_at(position);
        let mut output = String::new();
        output.push_str(before);
        if !before.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(declaration);
        output.push_str(after);
        output
    } else {
        let mut output = source;
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push_str(declaration);
        output
    };

    write_domain_bindings(&mod_path, &rewritten);
}

/// Ensure one domain has a canonical test scaffold.
fn ensure_domain_test_scaffold(domain: &str, bindings: &BTreeMap<String, BindingEntry>) {
    let tests_dir = runtime_domain_tests_dir_path(domain);
    if !tests_dir.exists() {
        fs::create_dir_all(&tests_dir).expect("failed to create tests directory");
    }

    let tests_mod_path = runtime_domain_tests_mod_path(domain);
    let tests_mod = render_domain_tests_mod_stub();
    write_missing_stub_file(&tests_mod_path, &tests_mod);

    let tests_path = runtime_domain_tests_path(domain);
    let tests = render_domain_tests_stub(domain);
    write_missing_stub_file(&tests_path, &tests);

    let harness_mod_path = runtime_domain_test_harness_path(domain);
    let harness_mod = render_domain_test_harness_stub();
    write_missing_stub_file(&harness_mod_path, &harness_mod);

    let harness_path = runtime_domain_test_harness_generated_path(domain);
    let harness = render_domain_test_harness_generated(domain, bindings);
    write_domain_bindings(&harness_path, &harness);

    let basic_path = runtime_domain_tests_basic_path(domain);
    let basic = render_domain_tests_basic_stub(domain);
    write_missing_stub_file(&basic_path, &basic);
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::module_matches_requested_domains;

    /// Match platform module uris by requested domains.
    #[test]
    fn test_module_matches_requested_domains() {
        let domains = BTreeSet::from(["crypto".to_string(), "fs".to_string()]);

        assert!(module_matches_requested_domains(
            "builtin://lib/platform/crypto/key.ds",
            &domains
        ));
        assert!(module_matches_requested_domains(
            "builtin://lib/platform/fs/path.ds",
            &domains
        ));
        assert!(!module_matches_requested_domains(
            "builtin://lib/platform/net/socket.ds",
            &domains
        ));
    }
}
