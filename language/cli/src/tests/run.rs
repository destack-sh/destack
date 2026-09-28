use crate::command::run::{EvalArgs, RunArgs, eval, run};
use crate::common::{ReportArgs, TargetArgs};

use super::tests::{TestProgram, assert_success, execute, input_args_from_path};

/// Run a program whose entry module calls a function another module defines.
#[test]
fn test_run_a_program_calling_across_modules() {
    // declare one package with a library module and an entry module
    let program = TestProgram::new("run_across_modules");
    let manifest = format!(
        r#"{{ "name": "across", "packageManager": "tspp@{}", "targets": {{ "default": {{ "output": "program" }} }}, "defaultTarget": "default" }}"#,
        env!("CARGO_PKG_VERSION")
    );
    program.write_text("package.json", &manifest);
    program.write_text(
        "src/library.tspp",
        "export function answer(): int32 {\n    42\n}\n",
    );
    let path = program.write_text(
        "src/main.tspp",
        "import { answer } from \"./library\";\n\nconst value: int32 = answer();\n",
    );

    // run the entry through build, link, and the bytecode machine
    let args = RunArgs {
        input: input_args_from_path(path),
        target: TargetArgs::default(),
        program: program.program_args(),
        report: ReportArgs::default(),
    };
    let code = execute(run(&args));

    assert_success(code);
}

/// Evaluate inline code as a program of its own.
#[test]
fn test_eval_inline_code() {
    // evaluate from a working directory holding no package
    let program = TestProgram::new("eval_inline");
    program.write_text("README.md", "Scratch directory.\n");
    let args = EvalArgs {
        code: vec!["const total: int32 = 1 + 2;".to_string()],
        target: TargetArgs::default(),
        program: program.program_args(),
        report: ReportArgs::default(),
    };
    let code = execute(eval(&args));

    assert_success(code);
}

/// Await an async result at the top level of the entry module.
#[test]
fn test_await_an_async_result_at_the_top_level() {
    // declare one package whose entry module awaits at its top level
    let program = TestProgram::new("run_top_level_await");
    let manifest = format!(
        r#"{{ "name": "awaiting", "packageManager": "tspp@{}", "targets": {{ "default": {{ "output": "program" }} }}, "defaultTarget": "default" }}"#,
        env!("CARGO_PKG_VERSION")
    );
    program.write_text("package.json", &manifest);
    let path = program.write_text(
        "src/main.tspp",
        r#"async function answer(): Promise<int32> {
    42
}

async function settle(): Promise<void> {}

await settle();
const value = await answer();
if (value != 42) {
    panic("the awaited answer changed");
}
"#,
    );

    // run the entry through its fiber, parking at each await
    let args = RunArgs {
        input: input_args_from_path(path),
        target: TargetArgs::default(),
        program: program.program_args(),
        report: ReportArgs::default(),
    };
    let code = execute(run(&args));

    assert_success(code);
}

/// Finish an imported module's top-level await before the importer runs.
#[test]
fn test_await_in_an_imported_module_before_its_importer() {
    // declare one package whose library awaits the value its entry reads
    let program = TestProgram::new("run_imported_top_level_await");
    let manifest = format!(
        r#"{{ "name": "importing", "packageManager": "tspp@{}", "targets": {{ "default": {{ "output": "program" }} }}, "defaultTarget": "default" }}"#,
        env!("CARGO_PKG_VERSION")
    );
    program.write_text("package.json", &manifest);
    program.write_text(
        "src/library.tspp",
        r#"async function answer(): Promise<int32> {
    42
}

export const value: int32 = await answer();
"#,
    );
    let path = program.write_text(
        "src/main.tspp",
        r#"import { value } from "./library";

if (value != 42) {
    panic("the imported answer was read before it settled");
}
"#,
    );

    // run both initializers in import order
    let args = RunArgs {
        input: input_args_from_path(path),
        target: TargetArgs::default(),
        program: program.program_args(),
        report: ReportArgs::default(),
    };
    let code = execute(run(&args));

    assert_success(code);
}

/// Cancel a parked task, running its cleanup from the park.
#[test]
fn test_cancel_a_parked_task_through_its_cleanup() {
    // declare one package whose entry cancels a task parked inside a finally
    let program = TestProgram::new("run_cancel_parked_task");
    let manifest = format!(
        r#"{{ "name": "cancelling", "packageManager": "tspp@{}", "targets": {{ "default": {{ "output": "program" }} }}, "defaultTarget": "default" }}"#,
        env!("CARGO_PKG_VERSION")
    );
    program.write_text("package.json", &manifest);
    let path = program.write_text(
        "src/main.tspp",
        r#"import { Task } from "tspp:async";

const cleanup = Promise.withResolvers<boolean>();
const blocked = Promise.withResolvers<boolean>();
const resolve = cleanup.resolve;
const never = blocked.promise;

async function wait(): Task<void> {
    try {
        await never;
    } finally {
        resolve(true);
    }
}

const task = wait();
task.cancel();

const isCleaned = await cleanup.promise;
if (!isCleaned) {
    panic("the cancelled task skipped its cleanup");
}
"#,
    );

    // run the entry, which waits on the cleanup the cancellation runs
    let args = RunArgs {
        input: input_args_from_path(path),
        target: TargetArgs::default(),
        program: program.program_args(),
        report: ReportArgs::default(),
    };
    let code = execute(run(&args));

    assert_success(code);
}
