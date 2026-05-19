use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use destack_test::core::{RunOptions, Runner};
use destack_test::stress::{FormatterStressSuite, ParserStressSuite};

#[derive(Parser, Debug, Clone)]
#[command(name = "stress", about = "run destack stress tests")]
struct StressOptions {
    /// run one generated parser fixture as a worker
    #[arg(long, hide = true)]
    run_parser_case: Option<PathBuf>,

    /// run one generated formatter fixture as a worker
    #[arg(long, hide = true)]
    run_formatter_case: Option<PathBuf>,

    /// materialize generated fixtures without running them
    #[arg(long)]
    generate: bool,

    /// run only parser stress tests
    #[arg(long)]
    parser: bool,

    /// run only formatter stress tests
    #[arg(long)]
    formatter: bool,

    #[command(flatten)]
    test: RunOptions,
}

fn main() -> ExitCode {
    let options = StressOptions::parse();

    if let Some(path) = options.run_parser_case {
        return exit_code(ParserStressSuite::run_worker(path));
    }

    if let Some(path) = options.run_formatter_case {
        return exit_code(FormatterStressSuite::run_worker(path));
    }

    let any_specific = options.parser || options.formatter;
    let run_parser = options.parser || !any_specific;
    let run_formatter = options.formatter || !any_specific;

    let mut any_failed = false;

    if options.generate {
        if run_parser {
            match ParserStressSuite::generate() {
                Ok(count) => eprintln!("generated {count} parser stress fixtures"),
                Err(message) => {
                    eprintln!("{message}");
                    any_failed = true;
                }
            }
        }

        if run_formatter {
            match FormatterStressSuite::generate() {
                Ok(count) => eprintln!("generated {count} formatter stress fixtures"),
                Err(message) => {
                    eprintln!("{message}");
                    any_failed = true;
                }
            }
        }

        return if any_failed {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        };
    }

    if run_parser {
        let result = Runner::run_suite(ParserStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if run_formatter {
        let result = Runner::run_suite(FormatterStressSuite, &options.test);
        if result != ExitCode::SUCCESS {
            any_failed = true;
        }
    }

    if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn exit_code(result: destack_test::core::CaseResult) -> ExitCode {
    match result {
        destack_test::core::CaseResult::Passed => ExitCode::SUCCESS,
        destack_test::core::CaseResult::Failed { message } => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
        destack_test::core::CaseResult::Skipped { reason } => {
            eprintln!("skipped: {reason}");
            ExitCode::SUCCESS
        }
        destack_test::core::CaseResult::Suite { failed, .. } if failed == 0 => ExitCode::SUCCESS,
        destack_test::core::CaseResult::Suite { .. } => ExitCode::FAILURE,
    }
}
