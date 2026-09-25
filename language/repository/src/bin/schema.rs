use tspp_repository::destack_schema;

fn main() {
    let schema = destack_schema();
    let options = serde_json::to_string_pretty(&schema).expect("failed to serialize schema");
    println!("{options}");
}
