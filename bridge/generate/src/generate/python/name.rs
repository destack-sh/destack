pub(super) fn python_parameter_name(name: &str) -> String {
    match name {
        "False" | "None" | "True" | "and" | "as" | "assert" | "async" | "await" | "break"
        | "class" | "continue" | "def" | "del" | "elif" | "else" | "except" | "finally" | "for"
        | "from" | "global" | "if" | "import" | "in" | "is" | "lambda" | "nonlocal" | "not"
        | "or" | "pass" | "raise" | "return" | "try" | "while" | "with" | "yield" => {
            format!("{name}_")
        }
        _ => name.to_string(),
    }
}

/// Return one valid generated Python path segment.
pub(super) fn python_segment_name(name: &str) -> String {
    python_parameter_name(name)
}

/// Return one valid generated Python field name.
pub(super) fn python_field_name(name: &str) -> String {
    let is_identifier_start = name
        .chars()
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic());

    if is_identifier_start {
        python_parameter_name(name)
    } else {
        format!("field_{name}")
    }
}

/// Return one valid generated Python payload field name.
pub(super) fn python_payload_field_name(name: &str) -> String {
    if name == "kind" {
        "kind_value".to_string()
    } else {
        python_field_name(name)
    }
}
