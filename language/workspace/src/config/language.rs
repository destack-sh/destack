use std::path::Path;

/// Module format for generated JavaScript output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JsModuleFormat {
    /// CommonJS modules.
    CommonJs,
    /// ES2015 modules.
    Es2015,
    /// ES2020 modules.
    Es2020,
    /// ES2022 modules.
    Es2022,
    /// ESNext modules.
    #[default]
    EsNext,
    /// Node16 modules.
    Node16,
    /// NodeNext modules.
    NodeNext,
    /// Preserve original module syntax.
    Preserve,
    /// No module system.
    None,
}

impl JsModuleFormat {
    /// Parse a JavaScript module format from config text.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "commonjs" => Some(Self::CommonJs),
            "es2015" | "es6" => Some(Self::Es2015),
            "es2020" => Some(Self::Es2020),
            "es2022" => Some(Self::Es2022),
            "esnext" => Some(Self::EsNext),
            "node16" => Some(Self::Node16),
            "nodenext" => Some(Self::NodeNext),
            "preserve" => Some(Self::Preserve),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    /// Return whether this module format emits ESM syntax.
    pub fn is_esm(self) -> bool {
        matches!(
            self,
            Self::Es2015
                | Self::Es2020
                | Self::Es2022
                | Self::EsNext
                | Self::Node16
                | Self::NodeNext
                | Self::Preserve
        )
    }
}

/// ECMAScript target for generated JavaScript output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EsTarget {
    /// ES5.
    Es5,
    /// ES2015.
    Es2015,
    /// ES2016.
    Es2016,
    /// ES2017.
    Es2017,
    /// ES2018.
    Es2018,
    /// ES2019.
    Es2019,
    /// ES2020.
    Es2020,
    /// ES2021.
    Es2021,
    /// ES2022.
    Es2022,
    /// ES2023.
    Es2023,
    /// ES2024.
    Es2024,
    /// ESNext.
    #[default]
    EsNext,
}

impl EsTarget {
    /// Parse an ECMAScript target from config text.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "es5" => Some(Self::Es5),
            "es2015" | "es6" => Some(Self::Es2015),
            "es2016" => Some(Self::Es2016),
            "es2017" => Some(Self::Es2017),
            "es2018" => Some(Self::Es2018),
            "es2019" => Some(Self::Es2019),
            "es2020" => Some(Self::Es2020),
            "es2021" => Some(Self::Es2021),
            "es2022" => Some(Self::Es2022),
            "es2023" => Some(Self::Es2023),
            "es2024" => Some(Self::Es2024),
            "esnext" => Some(Self::EsNext),
            _ => None,
        }
    }
}

/// Source form used by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SourceType {
    /// Script source.
    Script,
    /// Module source.
    #[default]
    Module,
}

impl SourceType {
    /// Return whether this source is parsed as a module.
    pub fn is_module(self) -> bool {
        matches!(self, Self::Module)
    }

    /// Return whether this source is parsed as a script.
    pub fn is_script(self) -> bool {
        matches!(self, Self::Script)
    }

    /// Detect the source type from one file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?;
        match extension {
            "ds" | "ts" | "tsx" | "mts" => Some(Self::Module),
            "cjs" | "cts" => Some(Self::Script),
            _ => None,
        }
    }

    /// Detect source type from file shape.
    pub fn detect(path: &Path, has_import_export: bool) -> Self {
        if let Some(source_type) = Self::from_extension(path) {
            return source_type;
        }

        if has_import_export {
            Self::Module
        } else {
            Self::Script
        }
    }
}
