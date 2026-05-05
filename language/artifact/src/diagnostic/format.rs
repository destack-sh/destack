use destack_dir as dir;
use destack_dir::LanguageSymbol;
use destack_source::{FileType, ModuleId, PackageId, ProfileId, TargetId, Uri};

use crate::{DiagnosticContext, DiagnosticDisplay, DiagnosticError};

/// Formatter for diagnostic message fields.
pub struct DiagnosticFormatter<'a> {
    /// The context that resolves revision-backed values.
    context: &'a dyn DiagnosticContext,
}

impl<'a> DiagnosticFormatter<'a> {
    /// Create a diagnostic formatter.
    pub fn new(context: &'a dyn DiagnosticContext) -> Self {
        Self { context }
    }

    /// Display one repository-backed value.
    pub fn display(&self, display: DiagnosticDisplay) -> Result<String, DiagnosticError> {
        self.context.display(display)
    }
}

impl std::fmt::Debug for DiagnosticFormatter<'_> {
    /// Format the formatter for debugging.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("DiagnosticFormatter").finish()
    }
}

/// Format one value inside a diagnostic message.
pub trait DiagnosticFormat {
    /// Format this value for a diagnostic message.
    fn format_diagnostic(
        &self,
        formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError>;
}

impl DiagnosticFormat for String {
    /// Format a string directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.clone())
    }
}

impl DiagnosticFormat for &str {
    /// Format a string directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for bool {
    /// Format a boolean directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u8 {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u16 {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u32 {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u64 {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for i32 {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for i64 {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for usize {
    /// Format an integer directly.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl<T: DiagnosticFormat> DiagnosticFormat for Vec<T> {
    /// Format a vector of values.
    fn format_diagnostic(
        &self,
        formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        let mut formatted = Vec::with_capacity(self.len());

        for value in self {
            formatted.push(value.format_diagnostic(formatter)?);
        }

        Ok(format!("[{}]", formatted.join(", ")))
    }
}

impl DiagnosticFormat for std::path::PathBuf {
    /// Format a path for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.display().to_string())
    }
}

impl DiagnosticFormat for FileType {
    /// Format one file type for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        let name = match self {
            FileType::Destack => "ds",
            FileType::DestackDeclaration => "d.ds",
            FileType::DestackText => "dst",
            FileType::DestackBinary => "dsb",
            FileType::JavaScript => "js",
            FileType::JavaScriptXml => "jsx",
            FileType::TypeScript => "ts",
            FileType::TypeScriptXml => "tsx",
            FileType::TypeScriptDeclaration => "d.ts",
            FileType::Text => "txt",
            FileType::Toml => "TOML",
            FileType::Yaml => "YAML",
            FileType::Json => "JSON",
            FileType::Env => "env",
            FileType::Html => "html",
            FileType::Markdown => "markdown",
            FileType::Css => "css",
            FileType::Svg => "svg",
            FileType::Wasm => "wasm",
            FileType::Node => "node",
            FileType::SourceMap => "source map",
            FileType::Object => "object",
            FileType::DestackMir => "mir",
            FileType::Image => "image",
            FileType::Font => "font",
            FileType::Audio => "audio",
            FileType::Video => "video",
            FileType::Model => "model",
            FileType::Neural => "neural model",
            FileType::Document => "document",
            FileType::Binary => "binary",
            FileType::Unknown => "unknown",
        };

        Ok(name.to_string())
    }
}

impl DiagnosticFormat for Uri {
    /// Format one URI for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for ModuleId {
    /// Format one module id for display.
    fn format_diagnostic(
        &self,
        formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        formatter.display(DiagnosticDisplay::Module(*self))
    }
}

impl DiagnosticFormat for PackageId {
    /// Format one package id for display.
    fn format_diagnostic(
        &self,
        formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        formatter.display(DiagnosticDisplay::Package(*self))
    }
}

impl DiagnosticFormat for TargetId {
    /// Format one target id for display.
    fn format_diagnostic(
        &self,
        formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        formatter.display(DiagnosticDisplay::Target(*self))
    }
}

impl DiagnosticFormat for ProfileId {
    /// Format one profile id for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(format!("#{}", self.0))
    }
}

impl DiagnosticFormat for dir::Visibility {
    /// Format one visibility value for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        let formatted = match self {
            dir::Visibility::Public => "public".to_string(),
            dir::Visibility::Protected => "protected".to_string(),
            dir::Visibility::Private => "private".to_string(),
        };

        Ok(formatted)
    }
}

impl DiagnosticFormat for LanguageSymbol {
    /// Format one language symbol for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}
