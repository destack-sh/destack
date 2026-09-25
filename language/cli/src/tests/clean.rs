use super::tests::{TestProgram, assert_success, execute};
use crate::command::clean::{CleanArgs, run};
use crate::common::ReportArgs;
use serde_json::json;
use tspp_source::FileSystem;

/// Cleans compiler output directories from destack.json.
#[test]
fn test_clean_removes_out_dir() {
    // setup
    let program = TestProgram::new("clean_out_dir");
    program.write_destack_config_with_base(json!({
        "compiler": {
            "outDir": "dist",
        },
    }));
    program.write_file("dist/output.txt", "ok");

    let args = CleanArgs {
        dir: Some(program.root.clone()),
        dist: true,
        cache: false,
        all: false,
        all_packages: false,
        dry_run: false,
        program: program.program_args(),
        report: ReportArgs::default(),
    };

    // run clean
    let code = execute(run(&args));

    // assert output directory removed
    assert_success(code);
    assert!(
        !program
            .fs
            .exists(&program.root.join("dist"))
            .unwrap_or(false)
    );
}
