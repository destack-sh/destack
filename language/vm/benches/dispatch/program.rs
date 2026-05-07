use std::path::PathBuf;
use std::{env, fs};

/// Directory containing benchmark MIR programs.
const PROGRAM_DIRECTORY: &str = "benches/dispatch/program";

/// One MIR program measured by the dispatch benchmark.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Program {
    /// The MIR file stem and Criterion benchmark name.
    pub(crate) name: &'static str,
    /// The function called by the benchmark harness.
    pub(crate) entry: &'static str,
}

impl Program {
    /// Read the MIR program for this benchmark.
    pub(crate) fn read(self) -> String {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push(PROGRAM_DIRECTORY);
        path.push(format!("{}.mir", self.name));

        fs::read_to_string(&path).expect("benchmark MIR should be readable")
    }
}
