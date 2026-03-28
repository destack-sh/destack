/// Write one C doc comment block.
pub(super) fn push_c_doc_comment(output: &mut String, documentation: &str, indent: usize) {
    let prefix = " ".repeat(indent);
    output.push_str(&format!("{prefix}/// {}\n", documentation.trim()));
}

/// Write one Swift doc comment block.
pub(super) fn push_swift_doc_comment(output: &mut String, documentation: &str, indent: usize) {
    let prefix = " ".repeat(indent);
    output.push_str(&format!("{prefix}/// {}\n", documentation.trim()));
}
