/// Render one nested dump record.
pub(super) fn dump_record<const N: usize>(name: &str, fields: [(&str, String); N]) -> String {
    let fields = fields
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ");

    if fields.is_empty() {
        format!("<{name}>")
    } else {
        format!("<{name} {fields}>")
    }
}

/// Render comma separated values as one dump list.
pub(super) fn dump_list(values: String) -> String {
    format!("[{values}]")
}
