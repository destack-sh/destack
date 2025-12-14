use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_compiler::{CompileOptions, Compiler, EmitTask};
use destack_source::{File, FileRegistry, FileSystem, FileType, PhysicalFileSystem, Uri};
use destack_workspace::{DsConfig, LanguageOptions, Program, Target};

use crate::harness::{
    RunContext, Runner, Suite, TestCase, TestOptions, TestResult, check_diagnostics,
    discover_test_directories, fixtures_dir,
};

use super::assert::compare_directory;
use super::discover::{SOURCE_EXTENSIONS, discover_source_files};

#[derive(Debug, Clone, Copy, Default)]
pub struct CodegenSuite;

impl Suite for CodegenSuite {
    fn name(&self) -> &'static str {
        "codegen"
    }

    fn discover(&self, _options: &TestOptions) -> Vec<TestCase> {
        let codegen_directory = fixtures_dir().join("codegen");
        discover_test_directories(&codegen_directory, "destack_test::codegen")
            .expect("failed to discover tests")
    }

    fn run(&self, case: &TestCase, _context: &RunContext<'_>) -> TestResult {
        run_codegen_case(case)
    }
}

/// Run all codegen tests.
pub fn run_codegen_tests(options: &TestOptions) -> std::process::ExitCode {
    Runner::run_suite(&CodegenSuite, options)
}

/// Run a single codegen test.
fn run_codegen_case(test: &TestCase) -> TestResult {
    // parse dsconfig.json to get targets
    let dsconfig_path = test.path.join("dsconfig.json");
    let dsconfig = match load_dsconfig(&dsconfig_path) {
        Ok(config) => config,
        Err(e) => {
            return TestResult::Failed {
                message: format!("failed to load dsconfig.json: {e}"),
            };
        }
    };

    // extract targets
    let targets: Vec<(String, Target)> = dsconfig
        .options
        .targets
        .iter()
        .map(|(name, opts)| (name.clone(), opts.to_target(name)))
        .collect();
    if targets.is_empty() {
        return TestResult::Failed {
            message: "no targets defined in dsconfig.json".to_string(),
        };
    }

    // set up program with physical filesystem
    let files = Arc::new(FileRegistry::new());
    let fs: Arc<dyn FileSystem> = Arc::new(PhysicalFileSystem);
    let program = Arc::new(Program::new(
        LanguageOptions::default(),
        test.path.clone(),
        fs,
        files,
    ));

    // set up compiler
    let compiler = Compiler::new(
        program.clone(),
        CompileOptions {
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
        let module = module.read();
        module.package_id
    };

    // set up package
    {
        let package = program.packages.get(package_id);
        let mut package = package.write();

        // set dsconfig so root_dir is available for output path resolution
        package.dsconfig = Some(dsconfig.clone());

        // add targets, rewriting out_dir to dist-actual/
        for (name, mut target) in targets.clone() {
            // rewrite dist/<target> to dist-actual/<target>
            let out_dir_str = target.out_dir.to_string_lossy();
            if out_dir_str.starts_with("dist/") {
                let new_out_dir = out_dir_str.replacen("dist/", "dist-actual/", 1);
                target.out_dir = PathBuf::from(new_out_dir);
            }
            package.targets.insert(name, target);
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

    // emit
    for (target_name, _) in &targets {
        compiler.enqueue(EmitTask::EmitPackage {
            package: package_id,
            target: target_name.clone(),
        });
    }
    compiler.compile();

    // check for errors
    let result = check_diagnostics(test, &program.files, &program.diagnostics);
    if result.is_failed() {
        return result;
    }
    drop(compiler);

    // compare dist-actual/ against dist/
    let dist_expected = test.path.join("dist");
    compare_directory(&dist_expected, &dist_actual)
}

/// Load dsconfig.json from a path.
fn load_dsconfig(path: &Path) -> Result<DsConfig, String> {
    // read the file
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;

    // create a File object for DsConfig::parse (using JSONC to support comments)
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

    DsConfig::parse(&file).map_err(|e| e.to_string())
}
