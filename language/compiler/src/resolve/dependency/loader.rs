use destack_dir::{Argument, Expression, LocalNodeId, NodeTree, ScalarLiteral};
use destack_workspace::Loader;

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
        arguments: Option<&Vec<LocalNodeId<Argument>>>,
        tree: &NodeTree,
    ) -> LoaderAttribute {
        let Some(arguments) = arguments else {
            return LoaderAttribute::None;
        };
        let type_key = self.program.strings.intern("type");
        for arg_id in arguments {
            let Argument::Named { name, value, .. } = tree.get(*arg_id) else {
                continue;
            };
            if *name != type_key {
                continue;
            }

            // get string value from expression
            let value = match tree.get(*value) {
                Expression::ScalarLiteral {
                    value: ScalarLiteral::String(s),
                } => self.program.strings.get(*s).to_string(),
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
