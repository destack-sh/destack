//! JSON Schema generation for dsconfig.json.
//!
//! Run via: `just schema` or `cargo run -p destack_workspace --features schema`

use destack_workspace::DsConfigJson;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(DsConfigJson);
    let json = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{json}");
}
