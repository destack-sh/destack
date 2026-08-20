mod assertion;
mod call;
mod case;
mod file;
mod markdown;
mod patch;
mod position;
mod range;
mod response;
mod revision;
mod run;
mod suite;
mod workspace;

use assertion::*;
use call::*;
use case::*;
use file::*;
use markdown::*;
use patch::*;
use position::*;
use range::*;
use response::*;
use revision::*;
use run::*;
use suite::*;
use workspace::*;

use libtest_mimic::Arguments;

/// Run the query fixtures through the standard Rust test interface.
fn main() {
    let mut arguments = Arguments::from_args();
    let suite = QuerySuite::load().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    if suite.is_blessing() {
        arguments.test_threads = Some(1);
    }
    let trials = suite.trials();

    libtest_mimic::run(&arguments, trials).exit();
}
