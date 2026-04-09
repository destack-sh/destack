use destack_workspace::DestackJson;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(DestackJson);
    let json = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{json}");
}
