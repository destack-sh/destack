use tspp_repository::manifest_schema;

fn main() {
    let schema = manifest_schema();
    let options = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{options}");
}
