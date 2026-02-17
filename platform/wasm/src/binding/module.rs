use destack_workspace as workspace;
use serde::{Deserialize, Serialize};

/// The type of module content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModuleType {
    /// Code module content.
    Code,
    /// Data module content.
    Data,
    /// Text module content.
    Text,
    /// Binary module content.
    Binary,
}

impl From<ModuleType> for workspace::ModuleType {
    fn from(module_type: ModuleType) -> Self {
        match module_type {
            ModuleType::Code => workspace::ModuleType::Code,
            ModuleType::Data => workspace::ModuleType::Data,
            ModuleType::Text => workspace::ModuleType::Text,
            ModuleType::Binary => workspace::ModuleType::Binary,
        }
    }
}

impl From<workspace::ModuleType> for ModuleType {
    fn from(module_type: workspace::ModuleType) -> Self {
        match module_type {
            workspace::ModuleType::Code => ModuleType::Code,
            workspace::ModuleType::Data => ModuleType::Data,
            workspace::ModuleType::Text => ModuleType::Text,
            workspace::ModuleType::Binary => ModuleType::Binary,
        }
    }
}

/// The module output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ModuleFormat {
    /// CommonJS modules.
    CommonJs,
    /// AMD modules.
    Amd,
    /// UMD modules.
    Umd,
    /// SystemJS modules.
    System,
    /// ES2015 modules.
    Es2015,
    /// ES2020 modules.
    Es2020,
    /// ES2022 modules.
    Es2022,
    /// ESNext modules.
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

impl From<ModuleFormat> for workspace::ModuleTarget {
    fn from(module_format: ModuleFormat) -> Self {
        match module_format {
            ModuleFormat::CommonJs => workspace::ModuleTarget::CommonJs,
            ModuleFormat::Amd => workspace::ModuleTarget::Amd,
            ModuleFormat::Umd => workspace::ModuleTarget::Umd,
            ModuleFormat::System => workspace::ModuleTarget::System,
            ModuleFormat::Es2015 => workspace::ModuleTarget::Es2015,
            ModuleFormat::Es2020 => workspace::ModuleTarget::Es2020,
            ModuleFormat::Es2022 => workspace::ModuleTarget::Es2022,
            ModuleFormat::EsNext => workspace::ModuleTarget::EsNext,
            ModuleFormat::Node16 => workspace::ModuleTarget::Node16,
            ModuleFormat::NodeNext => workspace::ModuleTarget::NodeNext,
            ModuleFormat::Preserve => workspace::ModuleTarget::Preserve,
            ModuleFormat::None => workspace::ModuleTarget::None,
        }
    }
}

impl From<workspace::ModuleTarget> for ModuleFormat {
    fn from(module_format: workspace::ModuleTarget) -> Self {
        match module_format {
            workspace::ModuleTarget::CommonJs => ModuleFormat::CommonJs,
            workspace::ModuleTarget::Amd => ModuleFormat::Amd,
            workspace::ModuleTarget::Umd => ModuleFormat::Umd,
            workspace::ModuleTarget::System => ModuleFormat::System,
            workspace::ModuleTarget::Es2015 => ModuleFormat::Es2015,
            workspace::ModuleTarget::Es2020 => ModuleFormat::Es2020,
            workspace::ModuleTarget::Es2022 => ModuleFormat::Es2022,
            workspace::ModuleTarget::EsNext => ModuleFormat::EsNext,
            workspace::ModuleTarget::Node16 => ModuleFormat::Node16,
            workspace::ModuleTarget::NodeNext => ModuleFormat::NodeNext,
            workspace::ModuleTarget::Preserve => ModuleFormat::Preserve,
            workspace::ModuleTarget::None => ModuleFormat::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keep module type mapping stable between wasm and workspace types.
    #[test]
    fn test_module_type_roundtrip() {
        let values = [
            ModuleType::Code,
            ModuleType::Data,
            ModuleType::Text,
            ModuleType::Binary,
        ];

        for value in values {
            let workspace_value: workspace::ModuleType = value.into();
            let mapped_value: ModuleType = workspace_value.into();
            assert_eq!(value, mapped_value);
        }
    }

    /// Keep module format mapping stable between wasm and workspace targets.
    #[test]
    fn test_module_format_roundtrip() {
        let values = [
            ModuleFormat::CommonJs,
            ModuleFormat::Amd,
            ModuleFormat::Umd,
            ModuleFormat::System,
            ModuleFormat::Es2015,
            ModuleFormat::Es2020,
            ModuleFormat::Es2022,
            ModuleFormat::EsNext,
            ModuleFormat::Node16,
            ModuleFormat::NodeNext,
            ModuleFormat::Preserve,
            ModuleFormat::None,
        ];

        for value in values {
            let workspace_value: workspace::ModuleTarget = value.into();
            let mapped_value: ModuleFormat = workspace_value.into();
            assert_eq!(value, mapped_value);
        }
    }
}
