use destack_workspace::DestackOptions;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(DestackOptions);
    let options = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{options}");
}
