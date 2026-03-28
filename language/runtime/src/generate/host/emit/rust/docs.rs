/// Write one Rust doc comment line.
pub(super) fn push_rust_doc_comment(output: &mut String, documentation: &str) {
    output.push_str(&format!("/// {}\n", documentation.trim()));
}

/// Write one indented Rust doc comment line.
pub(super) fn push_indented_rust_doc_comment(
    output: &mut String,
    indent: &str,
    documentation: &str,
) {
    output.push_str(&format!("{indent}/// {}\n", documentation.trim()));
}
