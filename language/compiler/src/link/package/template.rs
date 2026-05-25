/// One configured output file name template.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OutputFileNameTemplate<'a> {
    /// The configured template text.
    template: &'a str,
}

/// The values available while rendering one output file name template.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct OutputFileNameValues<'a> {
    /// The relative directory value for `[dir]`.
    pub(crate) directory: Option<&'a str>,
    /// The base name value for `[name]`.
    pub(crate) name: &'a str,
    /// The content hash value for `[hash]`.
    pub(crate) hash: Option<&'a str>,
    /// The output format value for `[format]`.
    pub(crate) format: Option<&'a str>,
    /// The extension value for `[ext]`.
    pub(crate) extension: &'a str,
}

impl<'a> OutputFileNameTemplate<'a> {
    /// Create one output file name template renderer.
    pub(crate) fn new(template: &'a str) -> Self {
        Self { template }
    }

    /// Render this template against one value set.
    pub(crate) fn render(self, values: OutputFileNameValues<'_>) -> String {
        let directory = values.directory.unwrap_or("");
        let directory_prefix = if directory.is_empty() {
            String::new()
        } else {
            format!("{directory}/")
        };
        let extension_with_dot = format!(".{}", values.extension);
        let rendered = self
            .template
            .replace("[dir]/", &directory_prefix)
            .replace("[dir]", directory)
            .replace("[name]", values.name)
            .replace("[hash]", values.hash.unwrap_or(""))
            .replace("[format]", values.format.unwrap_or(""))
            .replace("[extname]", &extension_with_dot)
            .replace("[ext]", values.extension);

        normalize_output_template_path(&rendered)
    }
}

/// Normalize one rendered output template path.
fn normalize_output_template_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");

    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::{OutputFileNameTemplate, OutputFileNameValues};

    /// Render one template with all supported tokens.
    #[test]
    fn test_renders_output_file_name_template_tokens() {
        let template = OutputFileNameTemplate::new("assets/[dir]/[name]-[hash][extname]");
        let rendered = template.render(OutputFileNameValues {
            directory: Some("images/icons"),
            name: "icon",
            hash: Some("1234abcd"),
            format: Some("esm"),
            extension: "svg",
        });

        assert_eq!(rendered, "assets/images/icons/icon-1234abcd.svg");
    }

    /// Omit the directory separator when `[dir]` is empty.
    #[test]
    fn test_omits_empty_directory_prefix() {
        let template = OutputFileNameTemplate::new("assets/[dir]/[name].[ext]");
        let rendered = template.render(OutputFileNameValues {
            directory: None,
            name: "site",
            hash: None,
            format: None,
            extension: "css",
        });

        assert_eq!(rendered, "assets/site.css");
    }

    /// Render one template with the output format token.
    #[test]
    fn test_renders_output_file_name_template_format_token() {
        let template = OutputFileNameTemplate::new("chunks/[name]-[hash]-[format].[ext]");
        let rendered = template.render(OutputFileNameValues {
            directory: None,
            name: "chunk",
            hash: Some("1234abcd"),
            format: Some("esm"),
            extension: "js",
        });

        assert_eq!(rendered, "chunks/chunk-1234abcd-esm.js");
    }
}
