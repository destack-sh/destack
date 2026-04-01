use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use destack_artifact::{
    Ast, Data, DirPatched, EmitFormat, Loader, ScriptArtifact, ScriptDeclaration, ScriptLanguage,
    ScriptSlot, ScriptSlotKind,
};
use destack_core::StringPool;
use destack_dir::{self as dir};
use destack_js::{self as js};
use destack_source::{File, FileContent, FileType, ModuleId};
use destack_workspace::{Module, Target};
use serde_json::Value as JsonValue;

use super::{JsBackend, lower_module};
use crate::{CodegenJsError, CodegenJsResult, CodegenJsWarning, ScriptModule, ScriptSymbolId};

/// The exported default binding name for one generated non-code script module.
const DEFAULT_EXPORT_NAME: &str = "default";

/// The global constructor name used for binary runtime values.
const UINT8_ARRAY_NAME: &str = "Uint8Array";

/// The global document binding used for stylesheet runtime injection.
const DOCUMENT_NAME: &str = "document";

/// Return the generated binding name for one non-code script module.
fn resource_binding_name(module_id: ModuleId) -> String {
    format!("__destack_resource_{:08x}", module_id.local_id)
}

/// One generator for script module artifacts.
#[derive(Debug)]
pub struct ScriptArtifactGenerator<'a> {
    /// The current module snapshot.
    module: Arc<Module>,
    /// The current module file.
    file: Arc<File>,
    /// The current module AST.
    ast: Arc<Ast>,
    /// The current patched DIR artifact.
    dir: Arc<DirPatched>,
    /// The current parsed data payload when one exists.
    data: Option<Arc<Data>>,
    /// The shared string pool.
    strings: Arc<StringPool>,
    /// The target configuration.
    target: &'a Target,
}

impl<'a> ScriptArtifactGenerator<'a> {
    /// Create one script artifact generator.
    pub fn new(
        module: Arc<Module>,
        file: Arc<File>,
        ast: Arc<Ast>,
        dir: Arc<DirPatched>,
        data: Option<Arc<Data>>,
        strings: Arc<StringPool>,
        target: &'a Target,
    ) -> Self {
        Self {
            module,
            file,
            ast,
            dir,
            data,
            strings,
            target,
        }
    }

    /// Generate one script artifact.
    pub fn generate(
        self,
    ) -> CodegenJsResult<(ScriptArtifact, Vec<CodegenJsWarning>, Vec<CodegenJsError>)> {
        // validate target
        if !self.target.uses_js_generate_pipeline() {
            return Err(CodegenJsError::UnsupportedTarget {
                format: format!("{:?}", self.target.emit),
                message: Some("expected JS, TS, or HTML".to_string()),
            });
        }

        // current module inputs
        let module = self.module.as_ref();
        let file = self.file.as_ref();
        let ast = self.ast.as_ref();
        let dir = self.dir.as_ref();
        let data = self.data.as_deref();

        // runtime wrapper modules for non-code loaders
        if !module.is_code() {
            let artifact = self.generate_non_code_script_artifact(module, dir, file, data)?;

            return Ok((artifact, Vec::new(), Vec::new()));
        }

        // emit one lowered JavaScript module tree
        let lower = lower_module(module, ast, self.strings.as_ref(), dir, self.target)?;
        let warnings = lower.warnings;
        let errors = lower.errors;
        let artifact = self.finish_script_artifact(lower.module, None, true)?;

        Ok((artifact, warnings, errors))
    }

    /// Finish one script artifact from one lowered module tree.
    fn finish_script_artifact(
        &self,
        module: ScriptModule,
        slot: Option<ScriptSlot>,
        has_top_level_side_effects: bool,
    ) -> CodegenJsResult<ScriptArtifact> {
        // declaration output
        let declaration = if self.target.declaration && matches!(self.target.emit, EmitFormat::Js) {
            Some(ScriptDeclaration::default())
        } else {
            None
        };

        // language selection
        let language = match self.target.emit {
            EmitFormat::Js | EmitFormat::Html => ScriptLanguage::JavaScript,
            EmitFormat::Ts => ScriptLanguage::TypeScript,
            _ => {
                return Err(CodegenJsError::UnsupportedTarget {
                    format: format!("{:?}", self.target.emit),
                    message: Some("expected JS, TS, or HTML".to_string()),
                });
            }
        };

        // linkage metadata
        let linkage = JsBackend::collect_script_linkage(&module);

        Ok(ScriptArtifact {
            language,
            module,
            slot,
            linkage,
            declaration,
            source_map: None,
            has_top_level_side_effects,
        })
    }

    /// Generate one runtime wrapper module for one non-code loader.
    fn generate_non_code_script_artifact(
        &self,
        module: &Module,
        dir: &DirPatched,
        file: &File,
        data: Option<&Data>,
    ) -> CodegenJsResult<ScriptArtifact> {
        // runtime naming and mutable module builder
        let mut builder = NonCodeScriptModuleBuilder::new(module.id, dir.anchor_node);

        // exported runtime value
        let export_value =
            self.build_non_code_module_export_value(module, file, data, &mut builder)?;
        let binding_name = resource_binding_name(module.id);
        let module = if file.ty == FileType::Css {
            builder.finish_stylesheet_default_export(export_value.expression_id, &binding_name)
        } else {
            builder.finish_default_export(export_value.expression_id, &binding_name)
        };

        self.finish_script_artifact(module, export_value.slot, false)
    }

    /// Build the exported runtime value for one non-code module.
    fn build_non_code_module_export_value(
        &self,
        module: &Module,
        file: &File,
        data: Option<&Data>,
        builder: &mut NonCodeScriptModuleBuilder,
    ) -> CodegenJsResult<NonCodeExportValue> {
        // linked stylesheet modules
        if file.ty == FileType::Css {
            let expression_id = builder.insert_string_expression("");

            return Ok(NonCodeExportValue {
                expression_id,
                slot: Some(ScriptSlot {
                    expression_id,
                    kind: ScriptSlotKind::LinkedStylesheetUrl,
                }),
            });
        }

        // structured data modules
        if module.loader.is_data() {
            let Some(value) = data else {
                return Err(CodegenJsError::Internal {
                    message: format!("missing data payload for module '{}'", module.uri),
                });
            };
            let value = match value {
                Data::Json(value) => value,
            };

            let expression_id = builder.insert_json_expression(value)?;

            return Ok(NonCodeExportValue::plain(expression_id));
        }

        // base64 string modules
        if module.loader == Loader::Base64 {
            let FileContent::Binary { content } = &file.content else {
                return Err(CodegenJsError::Internal {
                    message: format!(
                        "base64 loader expected binary file content for '{}'",
                        module.uri
                    ),
                });
            };
            let encoded = BASE64_STANDARD.encode(content);

            return Ok(NonCodeExportValue::plain(
                builder.insert_string_expression(&encoded),
            ));
        }

        // text modules
        if module.loader.is_text() {
            let FileContent::Text { content } = &file.content else {
                return Err(CodegenJsError::Internal {
                    message: format!(
                        "text loader expected text file content for '{}'",
                        module.uri
                    ),
                });
            };

            return Ok(NonCodeExportValue::plain(
                builder.insert_string_expression(content),
            ));
        }

        // binary runtime values
        if module.loader == Loader::Binary {
            let FileContent::Binary { content } = &file.content else {
                return Err(CodegenJsError::Internal {
                    message: format!(
                        "binary loader expected binary file content for '{}'",
                        module.uri
                    ),
                });
            };

            return Ok(NonCodeExportValue::plain(
                builder.insert_binary_expression(content),
            ));
        }

        // file loader runtime values
        if module.loader.is_file() {
            let expression_id = builder.insert_string_expression("");

            return Ok(NonCodeExportValue {
                expression_id,
                slot: Some(ScriptSlot {
                    expression_id,
                    kind: ScriptSlotKind::LinkedAssetUrl,
                }),
            });
        }

        Err(CodegenJsError::Internal {
            message: format!(
                "unsupported non-code loader '{}' for module '{}'",
                module.loader.as_str(),
                module.uri
            ),
        })
    }
}

/// One generated export value for a non-code wrapper module.
#[derive(Debug, Clone)]
struct NonCodeExportValue {
    /// The generated runtime expression.
    expression_id: js::LocalNodeId<js::Expression>,
    /// The generated script slot when one exists.
    slot: Option<ScriptSlot>,
}

impl NonCodeExportValue {
    /// Create one plain export value with no later linker rewrite.
    fn plain(expression_id: js::LocalNodeId<js::Expression>) -> Self {
        Self {
            expression_id,
            slot: None,
        }
    }
}

/// One mutable builder for a generated non-code script module.
#[derive(Debug)]
struct NonCodeScriptModuleBuilder {
    /// The generated module id.
    module_id: ModuleId,
    /// The source anchor for inserted nodes.
    anchor: dir::LocalNodeIdAny,
    /// The generated JS syntax tree.
    tree: js::NodeTree,
    /// The generated string pool.
    strings: StringPool,
}

impl NonCodeScriptModuleBuilder {
    /// Create one non-code script module builder.
    fn new(module_id: ModuleId, anchor: dir::LocalNodeIdAny) -> Self {
        Self {
            module_id,
            anchor,
            tree: js::NodeTree::new(),
            strings: StringPool::new(),
        }
    }

    /// Finish one wrapper module that exports one default local binding.
    fn finish_default_export(
        mut self,
        value: js::LocalNodeId<js::Expression>,
        binding_name: &str,
    ) -> ScriptModule {
        // binding ids
        let binding_name_id = self.strings.intern(binding_name);
        let default_name_id = self.strings.intern(DEFAULT_EXPORT_NAME);

        // local binding
        let pattern = self.tree.insert_from_source_any(
            js::Pattern::Binding {
                mutability: None,
                name: binding_name_id,
            },
            self.module_id,
            self.anchor,
        );
        self.tree
            .set_symbol(pattern, ScriptSymbolId::ModuleDefault(self.module_id));
        let declarator = self.tree.insert_from_source_any(
            js::Declarator {
                pattern,
                ty: None,
                value: Some(value),
            },
            self.module_id,
            self.anchor,
        );
        let let_statement = self.tree.insert_from_source_any(
            js::Statement::Let {
                descriptor: js::DeclarationDescriptor {
                    kind: js::DeclarationKind::Definition,
                    abstraction: js::DeclarationAbstraction::Concrete,
                    anchor: js::BindingAnchor::Instance,
                    name: None,
                    export: None,
                },
                mutability: js::Mutability::Immutable,
                declarators: vec![declarator],
            },
            self.module_id,
            self.anchor,
        );

        // export default alias
        let export_item = self.tree.insert_from_source_any(
            js::DependencyItem {
                mode: js::DependencyMode::Item,
                kind: Some(js::DependencyKind::Value),
                name: Some(js::Name::Identifier(binding_name_id)),
                alias: Some(default_name_id),
                value: None,
            },
            self.module_id,
            self.anchor,
        );
        self.tree
            .set_symbol(export_item, ScriptSymbolId::ModuleDefault(self.module_id));
        let export_statement = self.tree.insert_from_source_any(
            js::Statement::Export {
                kind: js::DependencyKind::Value,
                target: None,
                target_module: None,
                items: vec![export_item],
                attributes: None,
            },
            self.module_id,
            self.anchor,
        );

        ScriptModule {
            tree: self.tree,
            roots: vec![let_statement.into_any(), export_statement.into_any()],
            strings: self.strings,
        }
    }

    /// Finish one stylesheet wrapper module with one injected runtime side effect.
    fn finish_stylesheet_default_export(
        mut self,
        value: js::LocalNodeId<js::Expression>,
        binding_name: &str,
    ) -> ScriptModule {
        let link_binding_name =
            format!("__destack_stylesheet_link_{:08x}", self.module_id.local_id);

        // default export binding
        let let_statement = self.insert_bound_value_statement(binding_name, value);

        // stylesheet runtime injection
        let if_statement =
            self.insert_stylesheet_injection_statement(binding_name, &link_binding_name);

        // default export alias
        let export_statement = self.insert_default_export_statement(binding_name);

        ScriptModule {
            tree: self.tree,
            roots: vec![
                let_statement.into_any(),
                if_statement.into_any(),
                export_statement.into_any(),
            ],
            strings: self.strings,
        }
    }

    /// Insert one local immutable binding for one runtime value.
    fn insert_bound_value_statement(
        &mut self,
        binding_name: &str,
        value: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Statement> {
        let binding_name_id = self.strings.intern(binding_name);

        // local binding
        let pattern = self.tree.insert_from_source_any(
            js::Pattern::Binding {
                mutability: None,
                name: binding_name_id,
            },
            self.module_id,
            self.anchor,
        );
        self.tree
            .set_symbol(pattern, ScriptSymbolId::ModuleDefault(self.module_id));
        let declarator = self.tree.insert_from_source_any(
            js::Declarator {
                pattern,
                ty: None,
                value: Some(value),
            },
            self.module_id,
            self.anchor,
        );

        self.tree.insert_from_source_any(
            js::Statement::Let {
                descriptor: js::DeclarationDescriptor {
                    kind: js::DeclarationKind::Definition,
                    abstraction: js::DeclarationAbstraction::Concrete,
                    anchor: js::BindingAnchor::Instance,
                    name: None,
                    export: None,
                },
                mutability: js::Mutability::Immutable,
                declarators: vec![declarator],
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one default export alias for one local binding.
    fn insert_default_export_statement(
        &mut self,
        binding_name: &str,
    ) -> js::LocalNodeId<js::Statement> {
        let binding_name_id = self.strings.intern(binding_name);
        let default_name_id = self.strings.intern(DEFAULT_EXPORT_NAME);

        let export_item = self.tree.insert_from_source_any(
            js::DependencyItem {
                mode: js::DependencyMode::Item,
                kind: Some(js::DependencyKind::Value),
                name: Some(js::Name::Identifier(binding_name_id)),
                alias: Some(default_name_id),
                value: None,
            },
            self.module_id,
            self.anchor,
        );
        self.tree
            .set_symbol(export_item, ScriptSymbolId::ModuleDefault(self.module_id));

        self.tree.insert_from_source_any(
            js::Statement::Export {
                kind: js::DependencyKind::Value,
                target: None,
                target_module: None,
                items: vec![export_item],
                attributes: None,
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one runtime stylesheet injection guard.
    fn insert_stylesheet_injection_statement(
        &mut self,
        stylesheet_binding_name: &str,
        link_binding_name: &str,
    ) -> js::LocalNodeId<js::Statement> {
        let document_name = self.strings.intern(DOCUMENT_NAME);
        let undefined_name = self.strings.intern("undefined");
        let link_name = self.strings.intern("link");
        let rel_name = self.strings.intern("rel");
        let href_name = self.strings.intern("href");
        let create_element_name = self.strings.intern("createElement");
        let head_name = self.strings.intern("head");
        let append_child_name = self.strings.intern("appendChild");
        let stylesheet_name = self.strings.intern("stylesheet");
        let stylesheet_binding_name = self.strings.intern(stylesheet_binding_name);
        let link_binding_name = self.strings.intern(link_binding_name);

        // typeof document !== "undefined"
        let document_expression = self.insert_path_expression(&[document_name]);
        let document_type = self.tree.insert_from_source_any(
            js::Expression::Unary {
                operator: js::UnaryOperator::Typeof,
                right: document_expression,
            },
            self.module_id,
            self.anchor,
        );
        let undefined_expression = self.tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(undefined_name),
            },
            self.module_id,
            self.anchor,
        );
        let condition = self.tree.insert_from_source_any(
            js::Expression::Binary {
                left: document_type,
                operator: js::BinaryOperator::NotEqualStrict,
                right: undefined_expression,
            },
            self.module_id,
            self.anchor,
        );

        // const __destack_stylesheet_link_x = document.createElement("link")
        let document_expression = self.insert_path_expression(&[document_name]);
        let create_element_target =
            self.insert_member_expression(document_expression, create_element_name);
        let link_literal = self.tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(link_name),
            },
            self.module_id,
            self.anchor,
        );
        let link_argument = self.tree.insert_from_source_any(
            js::Argument::Positional {
                value: link_literal,
            },
            self.module_id,
            self.anchor,
        );
        let create_element_call = self.tree.insert_from_source_any(
            js::Expression::Call {
                position: js::PostfixPosition::Direct,
                left: create_element_target,
                static_arguments: None,
                dynamic_arguments: vec![link_argument],
            },
            self.module_id,
            self.anchor,
        );
        let link_statement =
            self.insert_bound_value_statement_by_id(link_binding_name, create_element_call);

        // __destack_stylesheet_link_x.rel = "stylesheet"
        let link_expression = self.insert_path_expression(&[link_binding_name]);
        let rel_target = self.insert_member_expression(link_expression, rel_name);
        let rel_value = self.tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(stylesheet_name),
            },
            self.module_id,
            self.anchor,
        );
        let rel_assign = self.tree.insert_from_source_any(
            js::Expression::Assign {
                left: rel_target,
                right: rel_value,
            },
            self.module_id,
            self.anchor,
        );
        let rel_statement = self.tree.insert_from_source_any(
            js::Statement::Expression {
                expression: rel_assign,
            },
            self.module_id,
            self.anchor,
        );

        // __destack_stylesheet_link_x.href = __destack_resource_x
        let link_expression = self.insert_path_expression(&[link_binding_name]);
        let href_target = self.insert_member_expression(link_expression, href_name);
        let href_value = self.insert_path_expression(&[stylesheet_binding_name]);
        let href_assign = self.tree.insert_from_source_any(
            js::Expression::Assign {
                left: href_target,
                right: href_value,
            },
            self.module_id,
            self.anchor,
        );
        let href_statement = self.tree.insert_from_source_any(
            js::Statement::Expression {
                expression: href_assign,
            },
            self.module_id,
            self.anchor,
        );

        // document.head.appendChild(__destack_stylesheet_link_x)
        let document_expression = self.insert_path_expression(&[document_name]);
        let head_expression = self.insert_member_expression(document_expression, head_name);
        let append_child_target = self.insert_member_expression(head_expression, append_child_name);
        let link_expression = self.insert_path_expression(&[link_binding_name]);
        let append_child_argument = self.tree.insert_from_source_any(
            js::Argument::Positional {
                value: link_expression,
            },
            self.module_id,
            self.anchor,
        );
        let append_child_call = self.tree.insert_from_source_any(
            js::Expression::Call {
                position: js::PostfixPosition::Direct,
                left: append_child_target,
                static_arguments: None,
                dynamic_arguments: vec![append_child_argument],
            },
            self.module_id,
            self.anchor,
        );
        let append_child_statement = self.tree.insert_from_source_any(
            js::Statement::Expression {
                expression: append_child_call,
            },
            self.module_id,
            self.anchor,
        );

        let then_block = self.tree.insert_from_source_any(
            js::Block {
                statements: vec![
                    link_statement,
                    rel_statement,
                    href_statement,
                    append_child_statement,
                ],
            },
            self.module_id,
            self.anchor,
        );

        self.tree.insert_from_source_any(
            js::Statement::If {
                condition,
                then_block,
                else_block: None,
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one local immutable binding for one runtime value by string id.
    fn insert_bound_value_statement_by_id(
        &mut self,
        binding_name: js::StringId,
        value: js::LocalNodeId<js::Expression>,
    ) -> js::LocalNodeId<js::Statement> {
        let pattern = self.tree.insert_from_source_any(
            js::Pattern::Binding {
                mutability: None,
                name: binding_name,
            },
            self.module_id,
            self.anchor,
        );
        let declarator = self.tree.insert_from_source_any(
            js::Declarator {
                pattern,
                ty: None,
                value: Some(value),
            },
            self.module_id,
            self.anchor,
        );

        self.tree.insert_from_source_any(
            js::Statement::Let {
                descriptor: js::DeclarationDescriptor {
                    kind: js::DeclarationKind::Definition,
                    abstraction: js::DeclarationAbstraction::Concrete,
                    anchor: js::BindingAnchor::Instance,
                    name: None,
                    export: None,
                },
                mutability: js::Mutability::Immutable,
                declarators: vec![declarator],
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one path expression from one segment list.
    fn insert_path_expression(
        &mut self,
        segments: &[js::StringId],
    ) -> js::LocalNodeId<js::Expression> {
        self.tree.insert_from_source_any(
            js::Expression::Path {
                path: js::Path {
                    segments: segments.iter().copied().collect(),
                },
                static_arguments: None,
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one member expression.
    fn insert_member_expression(
        &mut self,
        left: js::LocalNodeId<js::Expression>,
        name: js::StringId,
    ) -> js::LocalNodeId<js::Expression> {
        self.tree.insert_from_source_any(
            js::Expression::Member {
                left,
                name,
                static_arguments: None,
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one JSON expression into the generated module tree.
    fn insert_json_expression(
        &mut self,
        value: &JsonValue,
    ) -> CodegenJsResult<js::LocalNodeId<js::Expression>> {
        match value {
            JsonValue::Null => Ok(self.tree.insert_from_source_any(
                js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Null,
                },
                self.module_id,
                self.anchor,
            )),
            JsonValue::Bool(value) => Ok(self.tree.insert_from_source_any(
                js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Boolean(*value),
                },
                self.module_id,
                self.anchor,
            )),
            JsonValue::Number(value) => {
                let number = value.as_f64().ok_or_else(|| CodegenJsError::Internal {
                    message: format!("failed to lower JSON number '{value}'"),
                })?;

                Ok(self.tree.insert_from_source_any(
                    js::Expression::ScalarLiteral {
                        value: js::ScalarLiteral::Number(number),
                    },
                    self.module_id,
                    self.anchor,
                ))
            }
            JsonValue::String(value) => Ok(self.insert_string_expression(value)),
            JsonValue::Array(values) => {
                let mut elements = Vec::with_capacity(values.len());

                // array elements
                for value in values {
                    let value = self.insert_json_expression(value)?;
                    let element = self.tree.insert_from_source_any(
                        js::ArrayElement::Expression { value },
                        self.module_id,
                        self.anchor,
                    );
                    elements.push(element);
                }

                Ok(self.tree.insert_from_source_any(
                    js::Expression::ArrayLiteral { elements },
                    self.module_id,
                    self.anchor,
                ))
            }
            JsonValue::Object(values) => {
                let mut properties = Vec::with_capacity(values.len());

                // object fields
                for (name, value) in values {
                    let value = self.insert_json_expression(value)?;
                    let key = js::Key::Name(js::Name::String(self.strings.intern(name)));
                    let property = self.tree.insert_from_source_any(
                        js::Property::Field {
                            modifiers: None,
                            key: Some(key),
                            value: Some(value),
                            default: None,
                        },
                        self.module_id,
                        self.anchor,
                    );

                    properties.push(property);
                }

                Ok(self.tree.insert_from_source_any(
                    js::Expression::ObjectLiteral { properties },
                    self.module_id,
                    self.anchor,
                ))
            }
        }
    }

    /// Insert one string expression into the generated module tree.
    fn insert_string_expression(&mut self, value: &str) -> js::LocalNodeId<js::Expression> {
        self.tree.insert_from_source_any(
            js::Expression::ScalarLiteral {
                value: js::ScalarLiteral::String(self.strings.intern(value)),
            },
            self.module_id,
            self.anchor,
        )
    }

    /// Insert one binary runtime value into the generated module tree.
    fn insert_binary_expression(&mut self, bytes: &[u8]) -> js::LocalNodeId<js::Expression> {
        let mut elements = Vec::with_capacity(bytes.len());

        // byte literals
        for byte in bytes {
            let value = self.tree.insert_from_source_any(
                js::Expression::ScalarLiteral {
                    value: js::ScalarLiteral::Number(f64::from(*byte)),
                },
                self.module_id,
                self.anchor,
            );

            let element = self.tree.insert_from_source_any(
                js::ArrayElement::Expression { value },
                self.module_id,
                self.anchor,
            );
            elements.push(element);
        }

        // Uint8Array constructor
        let array = self.tree.insert_from_source_any(
            js::Expression::ArrayLiteral { elements },
            self.module_id,
            self.anchor,
        );
        let constructor = self.tree.insert_from_source_any(
            js::Expression::Path {
                path: js::Path {
                    segments: smallvec::smallvec![self.strings.intern(UINT8_ARRAY_NAME)],
                },
                static_arguments: None,
            },
            self.module_id,
            self.anchor,
        );
        let argument = self.tree.insert_from_source_any(
            js::Argument::Positional { value: array },
            self.module_id,
            self.anchor,
        );

        self.tree.insert_from_source_any(
            js::Expression::New {
                left: constructor,
                static_arguments: None,
                dynamic_arguments: vec![argument],
            },
            self.module_id,
            self.anchor,
        )
    }
}
