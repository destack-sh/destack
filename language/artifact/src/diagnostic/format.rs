use std::hash::Hash;

use destack_builtin::LanguageSymbol;
use destack_core::StringId;
use destack_dir::{self as dir, GlobalNodeIdAny, StaticKey};
use destack_mir as mir;
use destack_source::{FileType, ModuleId, PackageId, ProfileId, TargetId, Uri};

use crate::{DiagnosticError, ProviderContext};

/// Format one value inside a diagnostic message.
pub trait DiagnosticFormat {
    /// Format this value for a diagnostic message.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash;
}

impl DiagnosticFormat for String {
    /// Format a string directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.clone())
    }
}

impl DiagnosticFormat for &str {
    /// Format a string directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for bool {
    /// Format a boolean directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u8 {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u16 {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u32 {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for u64 {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for i32 {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for i64 {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for usize {
    /// Format an integer directly.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl<T: DiagnosticFormat> DiagnosticFormat for Option<T> {
    /// Format an optional value.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        match self {
            Some(value) => value.diagnostic_format(context),
            None => Ok("<none>".to_string()),
        }
    }
}

impl<T: DiagnosticFormat> DiagnosticFormat for Vec<T> {
    /// Format a vector of values.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        let mut formatted = Vec::with_capacity(self.len());

        for value in self {
            formatted.push(value.diagnostic_format(context)?);
        }

        Ok(format!("[{}]", formatted.join(", ")))
    }
}

impl DiagnosticFormat for std::path::PathBuf {
    /// Format a path for display.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.display().to_string())
    }
}

impl DiagnosticFormat for FileType {
    /// Format one file type for display.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
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
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}

impl DiagnosticFormat for StringId {
    /// Format one string id for display.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        context.format_string_id(*self)
    }
}

impl DiagnosticFormat for StaticKey {
    /// Format one static key for display.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        let formatted = match self {
            StaticKey::Name(name) | StaticKey::Number(name) => {
                return name.diagnostic_format(context);
            }
            StaticKey::Symbol(symbol) => match symbol {
                dir::SymbolKey::Unique(_) => "<unique symbol>".to_string(),
                dir::SymbolKey::WellKnown(symbol) => symbol.global_symbol_name().to_string(),
                dir::SymbolKey::Registry(name) => {
                    let name = name.diagnostic_format(context)?;
                    format!("Symbol.for({name})")
                }
            },
        };

        Ok(formatted)
    }
}

impl DiagnosticFormat for ModuleId {
    /// Format one module id for display.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        context.format_module_id(*self)
    }
}

impl DiagnosticFormat for PackageId {
    /// Format one package id for display.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        context.format_package_id(*self)
    }
}

impl DiagnosticFormat for TargetId {
    /// Format one target id for display.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        context.format_target_id(*self)
    }
}

impl DiagnosticFormat for ProfileId {
    /// Format one profile id for display.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(format!("#{}", self.0))
    }
}

impl DiagnosticFormat for GlobalNodeIdAny {
    /// Format one global node id for display.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.local_id.ty.name().to_string())
    }
}

impl DiagnosticFormat for dir::AnchoredGlobalNodeId {
    /// Format one anchored DIR node for display.
    fn diagnostic_format<R>(
        &self,
        context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        self.node_id.diagnostic_format(context)
    }
}

impl DiagnosticFormat for mir::AnchoredGlobalNodeId {
    /// Format one anchored MIR node for display.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.node_id.local_id.ty.name().to_string())
    }
}

impl DiagnosticFormat for dir::Visibility {
    /// Format one visibility value for display.
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
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
    fn diagnostic_format<R>(
        &self,
        _context: &dyn ProviderContext<Revision = R>,
    ) -> Result<String, DiagnosticError>
    where
        R: Copy + Eq + Hash,
    {
        Ok(self.to_string())
    }
}
