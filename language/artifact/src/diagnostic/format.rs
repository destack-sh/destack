use tspp_dir as dir;
use tspp_dir::LanguageItem;
use tspp_source::{FileType, ModuleId, PackageId, ProductId, ProfileId, TargetId, Uri};

use crate::{Code, DiagnosticContext, DiagnosticDisplay, DiagnosticError};

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
            FileType::Tspp => "tspp",
            FileType::TsppDeclaration => "d.tspp",
            FileType::JavaScript => "JavaScript",
            FileType::Text => "txt",
            FileType::Toml => "TOML",
            FileType::Yaml => "YAML",
            FileType::Json => "JSON",
            FileType::Dotenv => "dotenv",
            FileType::Html => "html",
            FileType::Markdown => "markdown",
            FileType::Css => "css",
            FileType::Svg => "svg",
            FileType::Wasm => "wasm",
            FileType::SourceMap => "source map",
            FileType::Object => "object",
            FileType::Binary => "binary",
        };

        Ok(name.to_string())
    }
}

impl DiagnosticFormat for Code {
    /// Format one executable representation.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.canonical_tag().to_string())
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

impl DiagnosticFormat for ProductId {
    /// Format one product id for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
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

impl DiagnosticFormat for LanguageItem {
    /// Format one language symbol for display.
    fn format_diagnostic(
        &self,
        _formatter: &DiagnosticFormatter<'_>,
    ) -> Result<String, DiagnosticError> {
        Ok(self.to_string())
    }
}
