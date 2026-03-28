/// Push one formatted C++ documentation comment line.
pub(super) fn push_cpp_doc_comment(output: &mut String, documentation: &str, indent: usize) {
    let prefix = " ".repeat(indent);
    let documentation = documentation.trim();

    let documentation = if documentation.ends_with('.') {
        documentation.to_string()
    } else {
        format!("{documentation}.")
    };

    output.push_str(&format!("{prefix}/// {documentation}\n"));
}

/// Write one Kotlin doc comment block.
pub(super) fn push_kotlin_doc_comment(output: &mut String, documentation: &str, indent: usize) {
    let prefix = " ".repeat(indent);
    let documentation = documentation.trim();

    let documentation = if documentation.ends_with('.') {
        documentation.to_string()
    } else {
        format!("{documentation}.")
    };

    output.push_str(&format!("{prefix}/**\n"));
    output.push_str(&format!("{prefix} * {documentation}\n"));
    output.push_str(&format!("{prefix} */\n"));
}
