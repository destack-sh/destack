use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use destack_artifact::{Data, EmitFormat, ScriptDeclaration, ScriptLanguage, ScriptOutput};
use destack_codegen_js as js;
use destack_core::StringPool;
use destack_dir as dir;
use destack_source::{FileContent, Loader, ModuleId};
use destack_workspace::Module;
use serde_json::Value as JsonValue;

use crate::link::TargetLocation;
use crate::{LinkError, LinkResult};

use super::super::ScriptLinker;
use super::{OutputGraph, OutputId, OutputLayout, Plan};

/// The local export name for one default export item.
const DEFAULT_EXPORT_NAME: &str = "default";

/// The global constructor name used for binary runtime values.
const UINT8_ARRAY_NAME: &str = "Uint8Array";

/// Insert one local immutable binding for one runtime value.
fn insert_bound_value_statement(
    tree: &mut js::Tree,
    strings: &mut StringPool,
    module_id: ModuleId,
    anchor: dir::LocalNodeIdAny,
    binding_name: &str,
    value: js::LocalNodeId<js::Expression>,
) -> js::LocalNodeId<js::Statement> {
    let binding_name_id = strings.intern(binding_name);
    let pattern = tree.insert_from_source_any(
        js::Pattern::Binding {
            mutability: None,
            name: binding_name_id,
        },
        module_id,
        anchor,
    );
    tree.set_symbol(pattern, js::ScriptSymbolId::ModuleDefault(module_id));
    let declarator = tree.insert_from_source_any(
        js::Declarator {
            pattern,
            ty: None,
            value: Some(value),
        },
        module_id,
        anchor,
    );

    tree.insert_from_source_any(
        js::Statement::Let {
            export: None,
            is_ambient: false,
            mutability: js::Mutability::Immutable,
            declarators: vec![declarator],
        },
        module_id,
        anchor,
    )
}

/// Insert one default export alias for one local binding.
fn insert_default_export_statement(
    tree: &mut js::Tree,
    strings: &mut StringPool,
    module_id: ModuleId,
    anchor: dir::LocalNodeIdAny,
    binding_name: &str,
) -> js::LocalNodeId<js::Statement> {
    let binding_name_id = strings.intern(binding_name);
    let default_name_id = strings.intern(DEFAULT_EXPORT_NAME);
    let export_item = tree.insert_from_source_any(
        js::DependencyItem {
            binding: js::DependencyBinding::Named,
            form: Some(js::DependencyForm::Plain),
            name: Some(js::Name::Identifier(binding_name_id)),
            alias: Some(default_name_id),
            value: None,
        },
        module_id,
        anchor,
    );
    tree.set_symbol(export_item, js::ScriptSymbolId::ModuleDefault(module_id));

    tree.insert_from_source_any(
        js::Statement::Export {
            form: js::DependencyForm::Plain,
            target: None,
            target_module: None,
            items: vec![export_item],
            attributes: None,
        },
        module_id,
        anchor,
    )
}

/// Insert one string expression into the generated module tree.
fn insert_string_expression(
    tree: &mut js::Tree,
    strings: &mut StringPool,
    module_id: ModuleId,
    anchor: dir::LocalNodeIdAny,
    value: &str,
) -> js::LocalNodeId<js::Expression> {
    tree.insert_from_source_any(
        js::Expression::ScalarLiteral {
            value: js::ScalarLiteral::String(strings.intern(value)),
        },
        module_id,
        anchor,
    )
}

/// Insert one JSON expression into the generated module tree.
fn insert_json_expression(
    tree: &mut js::Tree,
    strings: &mut StringPool,
    module_id: ModuleId,
    anchor: dir::LocalNodeIdAny,
    value: &JsonValue,
) -> LinkResult<js::LocalNodeId<js::Expression>> {
    match value {
        JsonValue::Null => Ok(tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Null,
            },
            module_id,
            anchor,
        )),
        JsonValue::Bool(value) => Ok(tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Boolean(*value),
            },
            module_id,
            anchor,
        )),
        JsonValue::Number(value) => {
            let number = value.as_f64().ok_or_else(|| LinkError::Internal {
                anchor: (module_id.package_id).into(),
                package: module_id.package_id,
                message: format!("failed to lower JSON number '{value}'"),
            })?;

            Ok(tree.insert_from_source_any(
                js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Number(number),
                },
                module_id,
                anchor,
            ))
        }
        JsonValue::String(value) => Ok(insert_string_expression(
            tree, strings, module_id, anchor, value,
        )),
        JsonValue::Array(values) => {
            let mut elements = Vec::with_capacity(values.len());

            for value in values {
                let value = insert_json_expression(tree, strings, module_id, anchor, value)?;
                let element = tree.insert_from_source_any(
                    js::ArrayElement::Expression { value },
                    module_id,
                    anchor,
                );
                elements.push(element);
            }

            Ok(tree.insert_from_source_any(
                js::Expression::ArrayLiteral { elements },
                module_id,
                anchor,
            ))
        }
        JsonValue::Object(values) => {
            let mut properties = Vec::with_capacity(values.len());

            for (name, value) in values {
                let value = insert_json_expression(tree, strings, module_id, anchor, value)?;
                let key = js::Key::Name(js::Name::String(strings.intern(name)));
                let property = tree.insert_from_source_any(
                    js::Property::Field {
                        modifiers: None,
                        key,
                        value,
                        is_shorthand: false,
                    },
                    module_id,
                    anchor,
                );
                properties.push(property);
            }

            Ok(tree.insert_from_source_any(
                js::Expression::ObjectLiteral { properties },
                module_id,
                anchor,
            ))
        }
    }
}

/// Insert one binary runtime value into the generated module tree.
fn insert_binary_expression(
    tree: &mut js::Tree,
    strings: &mut StringPool,
    module_id: ModuleId,
    anchor: dir::LocalNodeIdAny,
    bytes: &[u8],
) -> js::LocalNodeId<js::Expression> {
    let mut elements = Vec::with_capacity(bytes.len());

    for byte in bytes {
        let value = tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::Number(f64::from(*byte)),
            },
            module_id,
            anchor,
        );
        let element =
            tree.insert_from_source_any(js::ArrayElement::Expression { value }, module_id, anchor);
        elements.push(element);
    }

    let array =
        tree.insert_from_source_any(js::Expression::ArrayLiteral { elements }, module_id, anchor);
    let constructor = tree.insert_from_source_any(
        js::Expression::Path {
            path: js::Path {
                segments: smallvec::smallvec![strings.intern(UINT8_ARRAY_NAME)],
            },
            generic_arguments: vec![],
        },
        module_id,
        anchor,
    );
    let argument =
        tree.insert_from_source_any(js::Argument::Positional { value: array }, module_id, anchor);

    tree.insert_from_source_any(
        js::Expression::New {
            left: constructor,
            generic_arguments: vec![],
            arguments: vec![argument],
        },
        module_id,
        anchor,
    )
}

impl<'a> ScriptLinker<'a> {
    /// Build the final linked asset URL for one file-loader module.
    fn file_loader_reference(
        &self,
        output_id: OutputId,
        output_layout: &OutputLayout,
        module: &Module,
    ) -> LinkResult<String> {
        let asset = self.plan_asset_reference(module.id)?;
        let output_location = match asset {
            super::super::AssetReference::Inline { url } => return Ok(url),
            super::super::AssetReference::Emitted { output_location } => output_location,
            super::super::AssetReference::Original => {
                return Err(LinkError::InvalidTarget {
                    anchor: module.id.into(),
                    package: self.package_id,
                    target: self.target_id.clone(),
                    message:
                        "assets.binding = \"reference\" is not supported for code file imports"
                            .to_string(),
                });
            }
        };

        // one file-loader reference is always relative to the owning output
        let current_output =
            output_layout
                .output_location(output_id)
                .ok_or_else(|| LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!("missing output placement for output id {}", output_id.0),
                })?;
        let target_layout = TargetLocation::new(self.package_dir, self.target, self.target_name());

        Ok(target_layout.runtime_reference(current_output, &output_location))
    }

    /// Build one resource script output for one non-code module.
    pub(in crate::link::script) fn build_resource_script_output(
        &self,
        output_id: OutputId,
        module_id: ModuleId,
        output_graph: &OutputGraph,
        plan: &Plan,
    ) -> LinkResult<ScriptOutput> {
        let source_module = self.module(module_id)?;
        let source_module = source_module.as_ref();
        let profile_id = self.profile_id_for_module(module_id)?;
        let dir = self
            .compiler
            .artifact_reader(self.context)
            .dir_bound(module_id, profile_id)
            .map_err(|error| LinkError::Internal {
                anchor: (self.package_id).into(),
                package: self.package_id,
                message: format!(
                    "missing bound DIR for resource module {:?} target '{}': {error:?}",
                    module_id,
                    self.target_name()
                ),
            })?;
        let mut tree = js::Tree::new();
        let mut strings = StringPool::new();
        let value = self.resource_value(
            output_id,
            source_module,
            plan,
            dir.module_node,
            &mut tree,
            &mut strings,
        )?;
        let should_export_default = if let Some(output) = output_graph.output(output_id) {
            output.facade_module() == Some(source_module.id)
        } else {
            false
        };

        let let_statement = insert_bound_value_statement(
            &mut tree,
            &mut strings,
            module_id,
            dir.module_node,
            js::MODULE_DEFAULT_NAME,
            value,
        );
        let mut roots = vec![let_statement.into_any()];

        // standalone resource outputs still need one module default export
        if should_export_default {
            let export_statement = insert_default_export_statement(
                &mut tree,
                &mut strings,
                module_id,
                dir.module_node,
                js::MODULE_DEFAULT_NAME,
            );
            roots.push(export_statement.into_any());
        }

        let script_module = js::Module {
            tree,
            roots,
            strings,
        };
        let language = match self.target.emit {
            EmitFormat::Js => ScriptLanguage::JavaScript,
            EmitFormat::Ts => ScriptLanguage::TypeScript,
            other => {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!("unsupported linked script language: {other:?}"),
                });
            }
        };
        let declaration = if self.target.declaration && matches!(self.target.emit, EmitFormat::Js) {
            Some(ScriptDeclaration::default())
        } else {
            None
        };

        Ok(ScriptOutput {
            language,
            module: script_module,
            declaration,
            source_map: None,
            has_top_level_side_effects: false,
        })
    }

    /// Link one final resource value for one non-code module.
    fn resource_value(
        &self,
        output_id: OutputId,
        module: &Module,
        plan: &Plan,
        anchor: dir::LocalNodeIdAny,
        tree: &mut js::Tree,
        strings: &mut StringPool,
    ) -> LinkResult<js::LocalNodeId<js::Expression>> {
        if module.loader.is_data() {
            let value = self.data(module.id)?;
            let Data::Json(value) = value.as_ref();

            return Ok(insert_json_expression(
                tree, strings, module.id, anchor, value,
            )?);
        }

        if module.loader == Loader::Base64 {
            let file = self.file(module.file_id)?;
            let FileContent::Binary { content } = file.content.payload() else {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "base64 loader expected binary file content for '{}'",
                        module.uri
                    ),
                });
            };
            let encoded = BASE64_STANDARD.encode(content);

            return Ok(insert_string_expression(
                tree, strings, module.id, anchor, &encoded,
            ));
        }

        if module.loader.is_text() {
            let file = self.file(module.file_id)?;
            let FileContent::Text { content } = file.content.payload() else {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "text loader expected text file content for '{}'",
                        module.uri
                    ),
                });
            };

            return Ok(insert_string_expression(
                tree, strings, module.id, anchor, content,
            ));
        }

        if module.loader == Loader::Binary {
            let file = self.file(module.file_id)?;
            let FileContent::Binary { content } = file.content.payload() else {
                return Err(LinkError::Internal {
                    anchor: (self.package_id).into(),
                    package: self.package_id,
                    message: format!(
                        "binary loader expected binary file content for '{}'",
                        module.uri
                    ),
                });
            };

            return Ok(insert_binary_expression(
                tree,
                strings,
                module.id,
                anchor,
                content.as_slice(),
            ));
        }

        if module.loader.is_file() {
            let linked_url = self.file_loader_reference(output_id, plan.output_layout(), module)?;

            return Ok(insert_string_expression(
                tree,
                strings,
                module.id,
                anchor,
                &linked_url,
            ));
        }

        Err(LinkError::Internal {
            anchor: (self.package_id).into(),
            package: self.package_id,
            message: format!(
                "unsupported resource loader '{}' for module '{}'",
                module.loader.as_str(),
                module.uri
            ),
        })
    }
}
