use std::collections::BTreeSet;
use std::path::Path;
use std::process::{Command, ExitCode};

use clap::{Parser, ValueEnum};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use destack_test::core::{RunOptions, Runner, Suite, fixtures_dir};
use destack_test::mdtest::discover_md_files;
use destack_test::specification::{
    SpecificationFormatSuite, SpecificationSuite, format_specification_fixtures,
};

const SPEC_STACK_BYTES: &str = "268435456";
const SPEC_CHILD_ENV: &str = "DESTACK_SPEC_CHILD";

#[derive(Debug, Clone, Copy, ValueEnum)]
enum IsolationLevel {
    None,
    Group,
    File,
    Test,
}

#[derive(Parser, Debug, Clone)]
#[command(name = "specification", about = "Run Destack specification tests")]
struct SpecificationOptions {
    #[command(flatten)]
    test: RunOptions,

    /// Format source blocks in specification markdown fixtures.
    #[arg(long)]
    format: bool,

    /// Check source blocks in specification markdown fixtures.
    #[arg(long)]
    format_check: bool,

    /// Isolate tests in subprocesses by group, file, or test.
    #[arg(long, value_enum, default_value_t = IsolationLevel::None)]
    isolate: IsolationLevel,
}

fn main() -> ExitCode {
    init_tracing();

    // increase worker stack size to reduce aborts from deep type recursion
    unsafe {
        // safe: set once before worker threads spawn
        std::env::set_var("RUST_MIN_STACK", SPEC_STACK_BYTES);
    }

    let options = SpecificationOptions::parse();

    if options.format {
        return match format_specification_fixtures() {
            Ok(changed) => {
                println!("formatted {changed} specification markdown files");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        };
    }

    if options.format_check {
        let suite = match SpecificationFormatSuite::load() {
            Ok(suite) => suite,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        };
        return Runner::run_suite(suite, &options.test);
    }

    // run a single unified suite in normal mode
    let is_child = std::env::var_os(SPEC_CHILD_ENV).is_some();
    let normal_mode = matches!(options.isolate, IsolationLevel::None);
    if is_child || options.test.filter.is_some() || options.test.list || normal_mode {
        let suite = SpecificationSuite::load();
        return Runner::run_suite(suite, &options.test);
    }

    run_isolated(&options.test, options.isolate)
}

fn init_tracing() {
    // enable tracing when RUST_LOG is set
    if std::env::var_os("RUST_LOG").is_none() {
        return;
    }

    let _ = tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().with_test_writer())
        .try_init();
}

fn run_isolated(options: &RunOptions, isolate: IsolationLevel) -> ExitCode {
    let spec_dir = fixtures_dir().join("specification");
    let entries = match isolation_entries(&spec_dir, isolate) {
        Ok(entries) => entries,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if entries.is_empty() {
        println!("no tests to run");
        return ExitCode::SUCCESS;
    }

    let mut failures: Vec<(String, std::process::ExitStatus)> = Vec::new();
    for entry in entries {
        let (label, filter) = entry;
        println!();
        println!("== {label} ==");
        let status = run_child(options, &filter);
        if !status.success() {
            print_child_failure(&label, &status);
            failures.push((label, status));
        }
    }

    if failures.is_empty() {
        return ExitCode::SUCCESS;
    }

    println!();
    println!("isolation failures: {}", failures.len());
    for (label, status) in failures.iter().take(20) {
        print_child_failure(label, status);
    }
    if failures.len() > 20 {
        println!("...and {} more", failures.len() - 20);
    }

    ExitCode::FAILURE
}

fn isolation_entries(
    spec_dir: &Path,
    isolate: IsolationLevel,
) -> Result<Vec<(String, String)>, String> {
    match isolate {
        IsolationLevel::None => Ok(Vec::new()),
        IsolationLevel::Group => top_level_groups(spec_dir).map(|groups| {
            groups
                .into_iter()
                .map(|group| {
                    let filter = format!("{group}/");
                    (group, filter)
                })
                .collect()
        }),
        IsolationLevel::File => {
            let entries = discover_md_files(spec_dir)
                .map_err(|error| {
                    format!(
                        "failed to discover specification fixtures in {}: {error}",
                        spec_dir.display()
                    )
                })?
                .into_iter()
                .filter_map(|md_path| {
                    let relative = md_path.strip_prefix(spec_dir).ok()?;
                    let relative = relative.to_string_lossy().to_string();
                    Some(relative)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .map(|relative| (relative.clone(), relative))
                .collect();

            Ok(entries)
        }
        IsolationLevel::Test => {
            let suite = SpecificationSuite::load();
            let mut entries: BTreeSet<String> = BTreeSet::new();
            for case in suite.discover(&RunOptions::default()) {
                entries.insert(case.full_name());
            }
            Ok(entries
                .into_iter()
                .map(|name| (name.clone(), name))
                .collect())
        }
    }
}

fn top_level_groups(spec_dir: &Path) -> Result<BTreeSet<String>, String> {
    let mut groups = BTreeSet::new();
    let md_paths = discover_md_files(spec_dir).map_err(|error| {
        format!(
            "failed to discover specification fixtures in {}: {error}",
            spec_dir.display()
        )
    })?;
    for md_path in md_paths {
        let Ok(relative) = md_path.strip_prefix(spec_dir) else {
            continue;
        };
        let Some(component) = relative.components().next() else {
            continue;
        };
        let group = component.as_os_str().to_string_lossy().to_string();
        groups.insert(group);
    }

    Ok(groups)
}

fn run_child(options: &RunOptions, filter: &str) -> std::process::ExitStatus {
    let exe = std::env::current_exe().expect("failed to resolve current executable");
    let mut command = Command::new(exe);
    command.env(SPEC_CHILD_ENV, "1");
    command.arg(filter);
    command.arg("--jobs");
    command.arg(options.jobs.to_string());
    command.arg("--markdown-timeout-ms");
    command.arg(options.case_timeout_ms.to_string());
    if options.sequential {
        command.arg("--sequential");
    }
    if options.verbose {
        command.arg("--verbose");
    }
    if options.update_known_failures {
        command.arg("--update-known-failures");
    }
    command.status().expect("failed to run specification child")
}

#[cfg(unix)]
fn print_child_failure(group: &str, status: &std::process::ExitStatus) {
    use std::os::unix::process::ExitStatusExt;

    if let Some(signal) = status.signal() {
        println!("group {group} aborted with signal {signal}");
    } else if let Some(code) = status.code() {
        println!("group {group} exited with code {code}");
    } else {
        println!("group {group} failed");
    }
}

#[cfg(not(unix))]
fn print_child_failure(group: &str, status: &std::process::ExitStatus) {
    if let Some(code) = status.code() {
        println!("group {group} exited with code {code}");
    } else {
        println!("group {group} failed");
    }
}
