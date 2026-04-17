use destack_artifact::Loader;
use destack_dir::{ImportAttribute, ImportAttributeValue};

use crate::Compiler;

/// Result of reading one loader override from import attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoaderAttribute {
    /// No `type` attribute was present.
    None,
    /// Parsed one valid loader override.
    Loader(Loader),
    /// Found a `type` attribute, but the value is not a supported loader.
    InvalidType { value: String },
}

impl Compiler {
    /// Extract loader override from import attributes.
    ///
    /// Supports the `type` attribute to override the default loader:
    /// ```text
    /// import data from "./file" with { type: "json" }
    /// import raw from "./file" with { type: "text" }
    /// ```
    pub(crate) fn loader_from_import_attributes(
        &self,
        attributes: Option<&[ImportAttribute]>,
    ) -> LoaderAttribute {
        let Some(attributes) = attributes else {
            return LoaderAttribute::None;
        };
        let type_key = self.repository.strings.intern("type");
        for attribute in attributes {
            if attribute.key.string() != type_key {
                continue;
            }

            // get string value from the static attribute payload
            let value = match &attribute.value {
                ImportAttributeValue::ScalarLiteral(destack_dir::ScalarLiteral::String(value)) => {
                    self.repository.strings.get(*value).to_string()
                }
                _ => {
                    return LoaderAttribute::InvalidType {
                        value: "<non-string>".to_string(),
                    };
                }
            };

            return match Loader::from_type_attribute(value.as_str()) {
                Some(loader) => LoaderAttribute::Loader(loader),
                None => LoaderAttribute::InvalidType { value },
            };
        }

        LoaderAttribute::None
    }
}
