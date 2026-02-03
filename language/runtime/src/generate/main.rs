use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use destack_compiler::{AnalyzeTask, Compiler, CompilerOptions, ResolveTask};
use destack_source::DiagnosticSeverity;
use destack_workspace::{
    EnvSnapshot, OutputFormat, Platform, ProfileFlags, ProfileId, ProfileKey, Program, Runtime,
    Session,
};

mod binding;
mod collect;
mod format;
mod model;
mod replay;

use binding::{
    DomainAbiTypes, collect_domain_abi_types, render_abi_types, render_domain_bindings,
    render_native_stub, render_platform_bindings_index, render_vm_stub,
    runtime_domain_abi_types_path, runtime_domain_bindings_path, runtime_domain_native_path,
    runtime_domain_vm_path, runtime_platform_generated_path, write_domain_bindings,
};
use collect::collect_platform_bindings;

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

        // create a single worker compiler for deterministic generation
        let compiler = Arc::new(Compiler::new(
            session.clone(),
            program.clone(),
            CompilerOptions {
                workers: 1,
                ..CompilerOptions::default()
            },
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
fn main() {
    // build a compiler session for platform bindings
    let cwd = std::env::current_dir().expect("failed to resolve current directory");
    let context = GeneratorContext::new(cwd);

    // configure the native profile for platform modules
    let profile_key = platform_profile_key();
    let profile_id = context.program.profiles.get_or_create(profile_key.clone());

    // load the platform modules for analysis
    let platform_modules = load_platform_modules(&context.session, &profile_key);

    // run analysis passes before extraction
    analyze_platform_modules(&context.compiler, profile_id, &platform_modules);

    // validate diagnostics before rendering output
    report_diagnostics(&context.program);

    // collect bindings and render outputs
    generate_bindings(
        &context.program,
        context.session.strings.as_ref(),
        profile_id,
        &platform_modules,
    );
}

/// Build the platform profile key for binding generation.
fn platform_profile_key() -> ProfileKey {
    ProfileKey::new(
        OutputFormat::Native,
        Runtime::NativeHosted,
        Platform::Universal,
        vec!["native".to_string(), "platform".to_string()],
        false,
        false,
        EnvSnapshot::from_env_all(),
        ProfileFlags::default(),
    )
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
    profile_id: ProfileId,
    platform_modules: &[destack_source::ModuleId],
) {
    // build the profile stamp for analysis tasks
    let profile_stamp = compiler.profile_stamp(profile_id);

    // enqueue resolver and declaration analysis tasks
    compiler.enqueue(ResolveTask::ResolveLibs {
        profile: profile_stamp,
    });
    for module_id in platform_modules {
        compiler.enqueue(AnalyzeTask::AnalyzeModuleDeclare {
            module: compiler.module_stamp(*module_id),
            profile: profile_stamp,
        });
    }

    // run the compiler pipeline
    compiler.compile();
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
            let generated = render_domain_bindings(domain, bindings);
            let path = runtime_domain_bindings_path(domain);
            write_domain_bindings(&path, &generated);

            let native_path = runtime_domain_native_path(domain);
            if should_write_stub(&native_path) {
                let stub = render_native_stub(domain, bindings);
                write_domain_bindings(&native_path, &stub);
            }

            let vm_path = runtime_domain_vm_path(domain);
            if should_write_stub(&vm_path) {
                let stub = render_vm_stub(domain, bindings);
                write_domain_bindings(&vm_path, &stub);
            }
        }

        let types = domain_types.get(domain).unwrap_or(&empty_types);
        let abi_types = render_abi_types(domain, types);
        let abi_types_path = runtime_domain_abi_types_path(domain);
        write_domain_bindings(&abi_types_path, &abi_types);
    }

    let platform_generated = render_platform_bindings_index(&binding_domains);
    let platform_path = runtime_platform_generated_path();
    write_domain_bindings(&platform_path, &platform_generated);
}

/// Return true if a generated stub should be written to the given path.
fn should_write_stub(path: &std::path::Path) -> bool {
    !path.exists()
}
