use schemars::schema_for;
use tspp_cli::common::CommandReport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema_for!(CommandReport);
    let json = serde_json::to_string_pretty(&schema)?;
    println!("{json}");

    Ok(())
}
