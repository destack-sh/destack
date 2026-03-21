mod analyze;
mod capability;
mod emit;
mod error;
mod generator;
mod model;
mod option;

use generator::RuntimeGenerator;

fn main() {
    if let Err(error) = RuntimeGenerator::run() {
        eprintln!("generate-bindings failed: {error}");
        std::process::exit(1);
    }
}
