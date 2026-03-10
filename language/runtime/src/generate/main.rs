mod analyze;
mod capability;
mod emit;
mod error;
mod generator;
mod model;
mod option;

use generator::RuntimeGenerator;

fn main() {
    RuntimeGenerator::run();
}
