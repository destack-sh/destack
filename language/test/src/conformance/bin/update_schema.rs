use std::fs;
use std::path::{Path, PathBuf};

use destack_test::conformance::{
    STATUS_JSON_FILE_NAME, SUITE_JSON_FILE_NAME, StatusSet, SuiteMetadata,
};
use schemars::schema_for;

const CONFORMANCE_SCHEMA_DIRECTORY: &str = "language/test/fixtures/conformance/schema";

/// Write the checked in conformance schemas.
fn main() {
    // schema output directory
    let schema_directory = repo_root_dir().join(CONFORMANCE_SCHEMA_DIRECTORY);
    fs::create_dir_all(&schema_directory).expect("failed to create conformance schema directory");

    // suite metadata schema
    write_schema_file(
        &schema_directory.join(schema_file_name_for(SUITE_JSON_FILE_NAME)),
        &schema_for!(SuiteMetadata),
    );

    // status metadata schema
    write_schema_file(
        &schema_directory.join(schema_file_name_for(STATUS_JSON_FILE_NAME)),
        &schema_for!(StatusSet),
    );
}

/// Return the repository root directory.
fn repo_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("language/test should live two levels below the repository root")
        .to_path_buf()
}

/// Return the schema file name for one json file name.
fn schema_file_name_for(json_file_name: &str) -> String {
    json_file_name.replace(".json", ".schema.json")
}

/// Write one schema file with stable formatting.
fn write_schema_file(path: &Path, schema: &schemars::Reflect) {
    let json =
        serde_json::to_string_pretty(schema).expect("failed to serialize conformance schema");
    fs::write(path, format!("{json}\n")).expect("failed to write conformance schema");
}
