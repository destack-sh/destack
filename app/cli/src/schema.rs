use destack_cli::common::CommandReport;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(CommandReport);
    let json = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{json}");
}
