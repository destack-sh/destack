use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{Compiler, CompilerOptions};
use destack_source::{File, FileSystem, FileType, PhysicalFileSystem, Uri};
use destack_workspace::{ArtifactKey, Destack, Session, Target, TargetId};

use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, check_diagnostics,
    discover_test_directories, fixtures_dir,
};

use super::assert::compare_directory;
use super::discover::{SOURCE_EXTENSIONS, discover_source_files};

#[derive(Debug, Clone, Copy, Default)]
pub struct EmitSuite;

impl Suite for EmitSuite {
    fn name(&self) -> &'static str {
        "emit"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let emit_directory = fixtures_dir().join("emit");
        discover_test_directories(&emit_directory, "destack_test::emit")
            .expect("failed to discover tests")
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_emit_case(case)
    }
}

/// Run all emit tests.
pub fn run_emit_tests(options: &TestOptions) -> std::process::ExitCode {
    Runner::run_suite(&EmitSuite, options)
}

/// Run a single emit test.
fn run_emit_case(test: &TestCase) -> TestResult {
    // parse destack.json to get targets
    let destack_config_path = test.path.join("destack.json");
    let config = match load_destack_config(&destack_config_path) {
        Ok(config) => config,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to load destack.json: {e}"),
            };
        }
    };

    // extract targets
    let targets: Vec<(String, Target)> = config
        .options
        .targets
        .iter()
        .map(|(name, opts)| (name.clone(), opts.to_target(name)))
        .collect();
    if targets.is_empty() {
        return TestResult::Failed {
            message: "no targets defined in destack.json".to_string(),
        };
    }

    // set up session and program with physical filesystem
    let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let session = Arc::new(Session::new(test.path.clone()).with_fs(fs));
    let program = session.add_root(test.path.clone());

    // set up compiler
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );

    // discover source files
    let source_dir = test.path.join("src");
    let source_files = match discover_source_files(&source_dir, SOURCE_EXTENSIONS) {
        Ok(files) => files,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to discover source files: {e}"),
            };
        }
    };
    if source_files.is_empty() {
        return TestResult::Failed {
            message: "no source files found in source directory".to_string(),
        };
    }

    // resolve modules
    let mut module_ids = Vec::new();
    for source_path in &source_files {
        let module_id = match compiler.resolve_path_to_module(source_path) {
            Ok(id) => id,
            Err(e) => {
                return TestResult::Failed {
                    message: format!("failed to resolve module {}: {e:?}", source_path.display()),
                };
            }
        };
        module_ids.push(module_id);
    }
    let package_id = {
        let module = program.modules.get(module_ids[0]);
        let module = module.as_ref();
        module.package_id
    };

    // set up package
    {
        let package = program.packages.get(package_id);
        let mut package = package.write();

        // set the config so root_dir is available for output path resolution
        package.config = Some(config.clone());

        // add targets, rewriting out_dir to dist-actual/
        for (name, mut target) in targets.clone() {
            // rewrite dist/<target> to dist-actual/<target>
            let out_dir_str = target.out_dir.to_string_lossy();
            if out_dir_str.starts_with("dist/") {
                let new_out_dir = out_dir_str.replacen("dist/", "dist-actual/", 1);
                target.out_dir = PathBuf::from(new_out_dir);
            }
            let target_id = TargetId::new(package_id, &name);
            package.targets.insert(target_id, target);
        }
    }

    // clean
    let dist_actual = test.path.join("dist-actual");
    if dist_actual.exists()
        && let Err(e) = fs::remove_dir_all(&dist_actual)
    {
        return TestResult::Failed {
            message: format!("failed to clean dist-actual/: {e}"),
        };
    }

    // link
    for (target_name, _) in &targets {
        let target_id = TargetId::new(package_id, target_name);
        compiler.enqueue(ArtifactKey::package_output(package_id, target_id));
    }
    compiler.compile();

    // check for errors
    let result = check_diagnostics(test, &program.files, &program.diagnostics);
    if result.is_failed() {
        return result;
    }
    drop(compiler);

    // emit
    let compiler = Compiler::new(
        session.clone(),
        program.clone(),
        CompilerOptions {
            workers: 1,
            ..Default::default()
        },
    );
    for (target_name, _) in &targets {
        let target_id = TargetId::new(package_id, target_name);
        if let Err(error) = compiler.emit_package(package_id, &target_id) {
            return TestResult::Failed {
                message: format!("failed to emit package output for {target_name}: {error:?}"),
            };
        }
    }
    drop(compiler);

    // compare dist-actual/ against dist/
    let dist_expected = test.path.join("dist");
    compare_directory(&dist_expected, &dist_actual)
}

/// Load destack.json from a path.
fn load_destack_config(path: &Path) -> Result<Destack, String> {
    // read the file
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    // create a File object for Destack::parse (using JSONC to support comments)
    let uri = Uri::from_path(path);
    let file_id = destack_source::FileId::new(0); // temporary id
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file = Arc::new(
        File::from_text_as_jsonc(
            file_id,
            name,
            uri,
            Some(path.to_path_buf()),
            FileType::Json,
            content,
        )
        .map_err(|e| e.to_string())?,
    );

    Destack::parse(&file).map_err(|e| e.to_string())
}
