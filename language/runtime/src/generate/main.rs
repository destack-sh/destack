mod context;
mod error;
mod host;
mod option;
mod platform;

use platform::RuntimeGenerator;

fn main() {
    if let Err(error) = RuntimeGenerator::run() {
        eprintln!("generate-bindings failed: {error}");
        std::process::exit(1);
    }
}
