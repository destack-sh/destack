use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ResolveTask, TaskOutcome};
use destack_source::DiagnosticSeverity;
use destack_workspace::{
    EnvSnapshot, OutputFormat, Platform, ProfileFlags, ProfileId, ProfileKey, Program, Runtime,
    Session,
};

use crate::binding::{
    DomainAbiTypes, collect_domain_abi_types, render_abi_types, render_domain_bindings,
    render_domain_mod_stub, render_host_router_stub, render_host_stub, render_native_stub,
    render_os_backend_mod_stub, render_platform_bindings_index, render_runtime_mod_stub,
    render_runtime_native_stub, render_runtime_vm_stub, render_simulated_mod_stub,
    render_simulated_native_stub, render_simulated_vm_stub, render_vm_stub,
    runtime_domain_abi_types_path, runtime_domain_bindings_path, runtime_domain_host_path,
    runtime_domain_mod_path, runtime_domain_native_path, runtime_domain_runtime_mod_path,
    runtime_domain_runtime_native_path, runtime_domain_runtime_vm_path,
    runtime_domain_simulated_mod_path, runtime_domain_simulated_native_path,
    runtime_domain_simulated_vm_path, runtime_domain_unix_mod_path,
    runtime_domain_unsupported_path, runtime_domain_vm_path, runtime_domain_windows_mod_path,
    runtime_platform_generated_path, write_domain_bindings,
};
use crate::catalog::collect_platform_bindings;
use crate::model::BindingScope;
use crate::option::parse_generator_options;
use crate::refresh::{write_missing_stub_file, write_stub_file};

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

    // run analysis passes before extraction
    analyze_platform_modules(
        &context.compiler,
        &context.program,
        profile_id,
        &platform_modules,
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
        if domains.iter().any(|domain| {
            let prefix = format!("builtin://lib/platform/{domain}/");
            module_uri.starts_with(&prefix)
        }) {
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

    // analyze module declarations in dependency order
    let modules_to_analyze = order_modules_for_analysis(program, platform_modules);

    // analyze module declarations sequentially
    for module_id in modules_to_analyze {
        let module = program.modules.get(module_id);
        let module = module.read();
        eprintln!(
            "generate-bindings: analyzing module {module_id:?} ({})",
            module.uri
        );
        let outcome = compiler.run_task(AnalyzeTask::AnalyzeModuleDeclare {
            module: compiler.module_stamp(module_id),
            profile: profile_stamp,
        });
        let task_name = format!("AnalyzeModuleDeclare({module_id:?})");
        assert_task_complete(outcome, &task_name, program);
    }
}

/// Sort modules so direct imports are analyzed before dependents.
fn order_modules_for_analysis(
    program: &Program,
    platform_modules: &[destack_source::ModuleId],
) -> Vec<destack_source::ModuleId> {
    // track module ids by normalized file path
    let mut module_path_map = HashMap::new();
    for module_id in platform_modules {
        let module = program.modules.get(*module_id);
        let module = module.read();
        if let Some(path) = module.path.as_ref() {
            module_path_map.insert(normalize_path(path), *module_id);
        }
    }

    // build direct dependency edges for relative imports
    let mut dependencies: HashMap<destack_source::ModuleId, Vec<destack_source::ModuleId>> =
        HashMap::new();
    for module_id in platform_modules {
        let module = program.modules.get(*module_id);
        let module = module.read();

        let Some(module_path) = module.path.as_ref() else {
            dependencies.insert(*module_id, Vec::new());
            continue;
        };

        let source = std::fs::read_to_string(module_path).unwrap_or_default();
        let mut module_dependencies = Vec::new();
        let parent = module_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""));

        for specifier in extract_relative_specifiers(&source) {
            let resolved = resolve_relative_specifier(parent, &specifier);
            if let Some(dependency_id) = module_path_map.get(&resolved) {
                if *dependency_id != *module_id {
                    module_dependencies.push(*dependency_id);
                }
            }
        }

        dependencies.insert(*module_id, module_dependencies);
    }

    // topologically order the module graph: imports first
    let mut ordered = Vec::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for module_id in platform_modules {
        visit_module_for_order(
            *module_id,
            &dependencies,
            &mut visiting,
            &mut visited,
            &mut ordered,
        );
    }

    ordered
}

/// Visit one module in DFS order and append it after dependencies.
fn visit_module_for_order(
    module_id: destack_source::ModuleId,
    dependencies: &HashMap<destack_source::ModuleId, Vec<destack_source::ModuleId>>,
    visiting: &mut HashSet<destack_source::ModuleId>,
    visited: &mut HashSet<destack_source::ModuleId>,
    ordered: &mut Vec<destack_source::ModuleId>,
) {
    // skip modules that are already fully processed
    if visited.contains(&module_id) {
        return;
    }

    // stop on cycles: retain stable order for cycle members
    if !visiting.insert(module_id) {
        return;
    }

    // visit dependencies first
    if let Some(module_dependencies) = dependencies.get(&module_id) {
        for dependency_id in module_dependencies {
            visit_module_for_order(*dependency_id, dependencies, visiting, visited, ordered);
        }
    }

    // append this module and mark complete
    visiting.remove(&module_id);
    visited.insert(module_id);
    ordered.push(module_id);
}

/// Extract relative import specifiers from a module source string.
fn extract_relative_specifiers(source: &str) -> Vec<String> {
    let mut specifiers = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !(line.starts_with("import ") || line.starts_with("export ")) {
            continue;
        }

        if let Some(specifier) = extract_specifier_from_line(line) {
            if specifier.starts_with('.') {
                specifiers.push(specifier);
            }
        }
    }

    specifiers
}

/// Extract one module specifier from an import/export line.
fn extract_specifier_from_line(line: &str) -> Option<String> {
    // from "..."
    if let Some(index) = line.find("from \"") {
        let rest = &line[index + 6..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    // from '...'
    if let Some(index) = line.find("from '") {
        let rest = &line[index + 6..];
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }
    // import "..."
    if let Some(index) = line.find('"') {
        let rest = &line[index + 1..];
        let end = rest.find('"')?;
        return Some(rest[..end].to_string());
    }
    // import '...'
    if let Some(index) = line.find('\'') {
        let rest = &line[index + 1..];
        let end = rest.find('\'')?;
        return Some(rest[..end].to_string());
    }

    None
}

/// Resolve one relative specifier to a normalized file path.
fn resolve_relative_specifier(base: &std::path::Path, specifier: &str) -> PathBuf {
    let mut joined = base.join(specifier);
    if joined.extension().is_none() {
        joined.set_extension("ds");
    }
    normalize_path(&joined)
}

/// Normalize a path for stable dependency map lookups.
fn normalize_path(path: &std::path::Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
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
    let domain_types = collect_domain_abi_types(&catalog);

    // render bindings for each discovered domain
    let binding_domains = catalog.keys().cloned().collect::<BTreeSet<_>>();
    let mut abi_domains = binding_domains.clone();
    abi_domains.extend(domain_types.keys().cloned());

    let empty_types = DomainAbiTypes::default();

    for domain in &abi_domains {
        if let Some(bindings) = catalog.get(domain) {
            let has_world_dispatch = bindings
                .values()
                .any(|entry| entry.scope != BindingScope::Runtime);
            let has_runtime_dispatch = bindings
                .values()
                .any(|entry| entry.scope == BindingScope::Runtime);

            let mod_path = runtime_domain_mod_path(domain);
            let stub = render_domain_mod_stub(has_world_dispatch, has_runtime_dispatch);
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

            if has_world_dispatch {
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

                let simulated_mod_path = runtime_domain_simulated_mod_path(domain);
                let stub = render_simulated_mod_stub();
                write_stub_file(&simulated_mod_path, &stub, refresh_stubs);

                let simulated_native_path = runtime_domain_simulated_native_path(domain);
                let stub = render_simulated_native_stub(domain, bindings);
                write_stub_file(&simulated_native_path, &stub, refresh_stubs);

                let simulated_vm_path = runtime_domain_simulated_vm_path(domain);
                let stub = render_simulated_vm_stub(domain, bindings);
                write_stub_file(&simulated_vm_path, &stub, refresh_stubs);
            }

            if has_runtime_dispatch {
                let runtime_mod_path = runtime_domain_runtime_mod_path(domain);
                let stub = render_runtime_mod_stub();
                write_stub_file(&runtime_mod_path, &stub, refresh_stubs);

                let runtime_native_path = runtime_domain_runtime_native_path(domain);
                let stub = render_runtime_native_stub(domain);
                write_stub_file(&runtime_native_path, &stub, refresh_stubs);

                let runtime_vm_path = runtime_domain_runtime_vm_path(domain);
                let stub = render_runtime_vm_stub(domain);
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
