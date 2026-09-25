use std::path::Path;

use tspp_core::stable_hash_bytes;

/// Return the sanitized directory token for one asset module path.
pub(super) fn directory_token(
    root_dir: Option<&Path>,
    package_dir: &Path,
    source_path: &Path,
) -> String {
    let relative_path = source_path
        .strip_prefix(package_dir)
        .ok()
        .or_else(|| root_dir.and_then(|root_dir| source_path.strip_prefix(root_dir).ok()));
    let Some(relative_path) = relative_path else {
        return String::new();
    };
    let Some(parent) = relative_path.parent() else {
        return String::new();
    };

    let segments = parent
        .components()
        .filter_map(|component| {
            let segment = component.as_os_str().to_str()?;
            sanitize_output_name(segment)
        })
        .collect::<Vec<_>>();

    segments.join("/")
}

/// Return the sanitized `[name]` token for one asset module path.
pub(super) fn name_token(source_path: &Path) -> Option<String> {
    let stem = source_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())?;

    sanitize_output_name(stem)
}

/// Return one stable content hash for exact Asset bytes.
pub(super) fn content_hash(bytes: &[u8]) -> String {
    let hash = stable_hash_bytes(bytes);

    format!("{:08x}", hash as u32)
}

/// Percent encode one text payload for one data URL.
pub(super) fn percent_encode_for_data_url(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());

    for byte in text.bytes() {
        if matches!(
            byte,
            b'A'..=b'Z'
                | b'a'..=b'z'
                | b'0'..=b'9'
                | b'-'
                | b'_'
                | b'.'
                | b'~'
                | b'/'
                | b':'
        ) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }

    encoded
}

/// Sanitize one output name segment for emitted assets.
fn sanitize_output_name(name: &str) -> Option<String> {
    let mut sanitized = String::new();
    let mut previous_was_dash = false;

    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            sanitized.push(character);
            previous_was_dash = false;
            continue;
        }

        if !previous_was_dash {
            sanitized.push('-');
            previous_was_dash = true;
        }
    }

    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        return None;
    }

    Some(sanitized.to_string())
}
