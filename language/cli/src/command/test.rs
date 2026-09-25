use std::fs;
use std::path::{Path, PathBuf};

use clap::Args;
use tspp_doc::Example;
use tspp_repository::Execution;
use tspp_session::Executor;
use tspp_workspace::{CommandRevision, TestInput};

use crate::common::{
    CommandOptionsBuilder, CommandResult, ProgramArgs, ReportArgs, command_error,
    ensure_no_watch_or_dev, finish_workspace_message_command, report_error,
    run_workspace_command_or_report,
};
use crate::console;
use crate::example::ExampleRunner;

/// The file extension of documents whose examples `--doc` checks.
const DOCUMENT_EXTENSION: &str = "md";

/// Arguments for the test command.
#[derive(Args, Debug, Clone)]
pub struct TestArgs {
    /// The program options.
    #[command(flatten)]
    pub program: ProgramArgs,

    /// Report output options.
    #[command(flatten)]
    pub report: ReportArgs,

    /// Check the TS++ examples of Markdown documents instead of running tests.
    #[arg(long)]
    pub doc: bool,

    /// The Markdown files or directories whose examples `--doc` checks.
    #[arg(value_name = "PATHS", requires = "doc")]
    pub paths: Vec<PathBuf>,
}

/// Run tests.
pub async fn run(args: &TestArgs) -> i32 {
    if args.doc {
        return run_examples(args).await;
    }
    if let Some(code) = ensure_no_watch_or_dev("test", &args.program, &args.report) {
        return code;
    }

    // build workspace command options
    let common = match CommandOptionsBuilder::new(&args.program) {
        Ok(common) => common.build(),
        Err(error) => return report_error("test", &args.report, &error.to_string()),
    };
    let request = TestInput {
        ..(CommandRevision::Current, common).into()
    };

    // execute the workspace command
    let result = match run_workspace_command_or_report(
        "test",
        &args.report,
        &args.program,
        async |workspace, progress| {
            let result = workspace
                .test(request, progress)
                .await
                .map_err(command_error)?;

            CommandResult::from_output(result)
        },
    )
    .await
    {
        Ok(result) => result,
        Err(code) => return code,
    };

    // emit command output based on the report format
    finish_workspace_message_command("test", &args.report, &result)
}

/// Check the examples of every requested document and report each failed expectation.
async fn run_examples(args: &TestArgs) -> i32 {
    let documents = match collect_documents(&args.paths) {
        Ok(documents) => documents,
        Err(error) => return report_error("test", &args.report, &error),
    };
    let executor = match Executor::new(Execution::Threaded, args.program.workers as usize) {
        Ok(executor) => executor,
        Err(error) => {
            return report_error(
                "test",
                &args.report,
                &format!("executor start failed: {error}"),
            );
        }
    };
    let runner = match ExampleRunner::new(executor) {
        Ok(runner) => runner,
        Err(error) => return report_error("test", &args.report, &error),
    };
    let mut passed = 0;
    let mut failed = 0;

    // check each example of each document in order
    for document in &documents {
        let examples = match fs::read_to_string(document)
            .map_err(|error| error.to_string())
            .and_then(|source| Example::parse_document(&source).map_err(|error| error.to_string()))
        {
            Ok(examples) => examples,
            Err(error) => {
                console::error(&format!("{}: {error}", document.display()));
                failed += 1;
                continue;
            }
        };
        for example in &examples {
            let label = format!("{} > {}", document.display(), example.name);
            match runner.check(example).await {
                Ok(failures) if failures.is_empty() => passed += 1,
                Ok(failures) => {
                    failed += 1;
                    console::error(&label);
                    for failure in failures {
                        console::print(&format!("    {failure}"));
                    }
                }
                Err(error) => {
                    failed += 1;
                    console::error(&format!("{label}: {error}"));
                }
            }
        }
    }

    // summarize the run
    let summary = format!("{passed} examples passed, {failed} failed");
    if failed == 0 {
        console::success(&summary);

        0
    } else {
        console::error(&summary);

        1
    }
}

/// Collect the Markdown documents under the requested paths, sorted by path.
fn collect_documents(paths: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut documents = Vec::new();
    let mut pending = paths.to_vec();

    // walk directories and keep Markdown files
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            let entries =
                fs::read_dir(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            for entry in entries {
                pending.push(
                    entry
                        .map_err(|error| format!("{}: {error}", path.display()))?
                        .path(),
                );
            }
        } else if is_document(&path) {
            documents.push(path);
        }
    }
    if documents.is_empty() {
        return Err("no Markdown documents found".to_string());
    }
    documents.sort();

    Ok(documents)
}

/// Return whether one path is a Markdown document.
fn is_document(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == DOCUMENT_EXTENSION)
}
