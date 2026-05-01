use std::hash::Hash;

use destack_builtin::LanguageSymbol;
use destack_core::StringId;
use destack_dir::{self as dir, GlobalNodeIdAny, StaticKey};
use destack_mir as mir;
use destack_source::{FileType, ModuleId, PackageId, ProfileId, TargetId, Uri};

use crate::{DiagnosticAnchor, DiagnosticContext, DiagnosticError};

/// Formatter for diagnostic message fields.
pub struct DiagnosticFormatter<'a, R>
where
    R: Copy + Eq + Hash,
{
    /// The context that resolves revision-backed values.
    context: &'a dyn DiagnosticContext<Revision = R>,
    /// The primary diagnostic anchor used for anchor-relative values.
    anchor: &'a DiagnosticAnchor,
}

impl<'a, R> DiagnosticFormatter<'a, R>
where
    R: Copy + Eq + Hash,
{
    /// Create a diagnostic formatter.
    pub fn new(
        context: &'a dyn DiagnosticContext<Revision = R>,
        anchor: &'a DiagnosticAnchor,
    ) -> Self {
        Self { context, anchor }
    }

    /// Format one source string id relative to the primary anchor.
    pub fn format_string_id(&self, string: StringId) -> Result<String, DiagnosticError> {
        self.context.format_string_id(self.anchor, string)
    }

    /// Format one module id.
    pub fn format_module_id(&self, module: ModuleId) -> Result<String, DiagnosticError> {
        self.context.format_module_id(module)
    }

    /// Format one package id.
    pub fn format_package_id(&self, package: PackageId) -> Result<String, DiagnosticError> {
        self.context.format_package_id(package)
    }

    /// Format one target id.
    pub fn format_target_id(&self, target: TargetId) -> Result<String, DiagnosticError> {
        self.context.format_target_id(target)
    }
}

impl<R> std::fmt::Debug for DiagnosticFormatter<'_, R>
where
    R: Copy + Eq + Hash,
{
    /// Format the formatter for debugging.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DiagnosticFormatter")
            .field("anchor", self.anchor)
            .finish_non_exhaustive()
    }
}

/// Format one value inside a diagnostic message.
pub trait DiagnosticFormat {
    /// Format this value for a diagnostic message.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash;
}

impl DiagnosticFormat for String {
    /// Format a string directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.clone())
    }
}

impl DiagnosticFormat for &str {
    /// Format a string directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for bool {
    /// Format a boolean directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u8 {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u16 {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u32 {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u64 {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for i32 {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for i64 {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for usize {
    /// Format an integer directly.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl<T: DiagnosticFormat> DiagnosticFormat for Vec<T> {
    /// Format a vector of values.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        let mut formatted = Vec::with_capacity(self.len());

        for value in self {
            formatted.push(value.format_diagnostic(formatter)?);
        }

        Ok(format!("[{}]", formatted.join(", ")))
    }
}

impl DiagnosticFormat for std::path::PathBuf {
    /// Format a path for display.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.display().to_string())
    }
}

impl DiagnosticFormat for FileType {
    /// Format one file type for display.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
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
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for StringId {
    /// Format one string id for display.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        formatter.format_string_id(*self)
    }
}

impl DiagnosticFormat for StaticKey {
    /// Format one static key for display.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        let formatted = match self {
            StaticKey::Name(name) | StaticKey::Number(name) => {
                return name.format_diagnostic(formatter);
            }
            StaticKey::Symbol(symbol) => match symbol {
                dir::SymbolKey::Unique(_) => "<unique symbol>".to_string(),
                dir::SymbolKey::WellKnown(symbol) => symbol.global_symbol_name().to_string(),
                dir::SymbolKey::Registry(name) => {
                    let name = name.format_diagnostic(formatter)?;
                    format!("Symbol.for({name})")
                }
            },
        };

        Ok(formatted)
    }
}

impl DiagnosticFormat for ModuleId {
    /// Format one module id for display.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        formatter.format_module_id(*self)
    }
}

impl DiagnosticFormat for PackageId {
    /// Format one package id for display.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        formatter.format_package_id(*self)
    }
}

impl DiagnosticFormat for TargetId {
    /// Format one target id for display.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        formatter.format_target_id(*self)
    }
}

impl DiagnosticFormat for ProfileId {
    /// Format one profile id for display.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(format!("#{}", self.0))
    }
}

impl DiagnosticFormat for GlobalNodeIdAny {
    /// Format one global node id for display.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.local_id.ty.name().to_string())
    }
}

impl DiagnosticFormat for dir::AnchoredGlobalNodeId {
    /// Format one anchored DIR node for display.
    fn format_diagnostic<R>(
        &self,
        formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        self.node_id.format_diagnostic(formatter)
    }
}

impl DiagnosticFormat for mir::AnchoredGlobalNodeId {
    /// Format one anchored MIR node for display.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.node_id.local_id.ty.name().to_string())
    }
}

impl DiagnosticFormat for dir::Visibility {
    /// Format one visibility value for display.
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
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
    fn format_diagnostic<R>(
        &self,
        _formatter: &DiagnosticFormatter<'_, R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}
