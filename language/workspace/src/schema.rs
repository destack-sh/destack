//! JSON Schema generation for destack.json.
//!
//! Run via: `just schema` or `cargo run -p destack_workspace --features schema`

use destack_workspace::DestackJson;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(DestackJson);
    let json = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{json}");
}
