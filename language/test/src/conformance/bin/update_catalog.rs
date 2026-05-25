use std::process::ExitCode;

use destack_test::conformance::update_catalog_report_target;

fn main() -> ExitCode {
    match update_catalog_report_target() {
        Ok(updated_files) => {
            for file in updated_files {
                println!("updated {}", file.display());
            }

            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
