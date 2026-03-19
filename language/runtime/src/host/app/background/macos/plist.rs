use std::path::Path;

/// Render one launchd plist for one wrapper script.
pub(super) fn render_launchd_plist(
    label: &str,
    script_path: &Path,
    interval_seconds: u64,
) -> String {
    let label = plist_text(label);
    let script_path = plist_text(&script_path.to_string_lossy());

    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{label}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
    <string>{script_path}</string>\n\
  </array>\n\
  <key>RunAtLoad</key>\n\
  <false/>\n\
  <key>StartInterval</key>\n\
  <integer>{interval_seconds}</integer>\n\
</dict>\n\
</plist>\n"
    )
}

/// Escape one launchd plist text payload.
fn plist_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(character),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::render_launchd_plist;

    /// Render one launchd plist with escaped text values.
    #[test]
    fn test_render_launchd_plist_escapes_text_payloads() {
        let plist = render_launchd_plist("destack<&>", Path::new("/tmp/a&b<script>"), 300);

        assert!(plist.contains("<string>destack&lt;&amp;&gt;</string>"));
        assert!(plist.contains("<string>/tmp/a&amp;b&lt;script&gt;</string>"));
        assert!(plist.contains("<integer>300</integer>"));
    }
}
