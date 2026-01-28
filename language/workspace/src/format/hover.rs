/// Format hover information as markdown for display.
pub fn format_hover_markdown(
    signature: &str,
    type_text: Option<&str>,
    documentation: Option<&str>,
    location: Option<&str>,
) -> String {
    // start with the signature block
    let mut result = String::new();
    result.push_str("**Signature**\n\n");
    result.push_str("```destack\n");
    result.push_str(signature);
    result.push_str("\n```");

    // add type information when available
    if let Some(type_text) = type_text {
        result.push_str("\n\n**Type**\n\n");
        result.push_str("```destack\n");
        result.push_str(type_text);
        result.push_str("\n```");
    }

    // add documentation when available
    if let Some(documentation) = documentation {
        result.push_str("\n\n**Documentation**\n\n");
        result.push_str(documentation);
    }

    // add source location when available
    if let Some(location) = location {
        result.push_str("\n\n**Location**\n\n`");
        result.push_str(location);
        result.push('`');
    }

    // return the formatted markdown
    result
}
