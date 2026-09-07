use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use destack_artifact::{Data, DirBound, DirExpanded, DirMaterialized, DirParsed, Script};
use destack_js as js;
use destack_repository::Module;
use destack_serde::Value;
use destack_source::{Loader, ModuleId, ProvenanceId, ProvenanceJournal, ProvenanceTable};

use crate::link::TargetLocation;
use crate::{LinkError, LinkResult};

use super::super::JsLinker;
use super::{OutputGraph, OutputId, OutputLayout, Plan};

/// The generated local binding for one resource module.
const MODULE_DEFAULT_NAME: &str = "__module";

/// The global constructor used for binary values.
const UINT8_ARRAY_NAME: &str = "Uint8Array";

/// One generated JavaScript resource module.
struct ResourceEmitter {
    /// The module under construction.
    module: js::Module,
    /// The source provenance attributed to generated nodes.
    source: ProvenanceId,
    /// The generated module default symbol.
    module_default: js::SymbolId,
    /// The unresolved runtime globals.
    external_symbols: Vec<js::SymbolId>,
}

impl ResourceEmitter {
    /// Create one resource emitter.
    fn new(
        module: ModuleId,
        source: ProvenanceId,
        provenance: ProvenanceTable,
        journal: &mut ProvenanceJournal<'_>,
    ) -> Self {
        let mut output = js::Module::new(module, provenance);
        let name = output.strings.intern(MODULE_DEFAULT_NAME);
        let symbol = output.insert_symbol(
            name,
            js::SymbolNamespace::Value,
            js::ScopeId::ROOT,
            journal.derive(source),
        );

        Self {
            module: output,
            source,
            module_default: symbol,
            external_symbols: Vec::new(),
        }
    }

    /// Insert one generated JavaScript node.
    fn insert<T>(&mut self, node: T, journal: &mut ProvenanceJournal<'_>) -> js::LocalNodeId<T>
    where
        T: js::Node,
        js::Tree: js::TreeStore<T>,
    {
        self.module.tree.insert(node, journal.derive(self.source))
    }

    /// Build one occurrence of the generated module default.
    fn module_default(&mut self, journal: &mut ProvenanceJournal<'_>) -> js::Identifier {
        let name = self.module.symbols.get(self.module_default).name;

        js::Identifier {
            original_name: name,
            symbol: self.module_default,
            provenance: journal.derive(self.source),
        }
    }

    /// Insert one string literal expression.
    fn string(
        &mut self,
        value: &str,
        journal: &mut ProvenanceJournal<'_>,
    ) -> js::LocalNodeId<js::Expression> {
        let value = self.module.strings.intern(value);
        let literal = js::StringLiteral {
            value,
            provenance: journal.derive(self.source),
        };

        self.insert(
            js::Expression::Literal {
                value: js::Literal::String(literal),
            },
            journal,
        )
    }

    /// Insert one JSON value expression.
    fn json(
        &mut self,
        value: &Value,
        journal: &mut ProvenanceJournal<'_>,
    ) -> js::LocalNodeId<js::Expression> {
        match value {
            Value::Null => self.insert(
                js::Expression::Literal {
                    value: js::Literal::Null,
                },
                journal,
            ),
            Value::Bool(value) => self.insert(
                js::Expression::Literal {
                    value: js::Literal::Boolean(*value),
                },
                journal,
            ),
            Value::Signed(value) => self.number(*value as f64, journal),
            Value::Unsigned(value) => self.number(*value as f64, journal),
            Value::Float(value) => self.number(*value, journal),
            Value::String(value) => self.string(value, journal),
            Value::Array(values) => {
                let mut elements = Vec::with_capacity(values.len());

                // emit every array element
                for value in values {
                    let value = self.json(value, journal);
                    let element = self.insert(js::ArrayElement::Expression { value }, journal);
                    elements.push(element);
                }

                self.insert(js::Expression::ArrayLiteral { elements }, journal)
            }
            Value::Object(values) => {
                let mut properties = Vec::with_capacity(values.len());

                // emit every object property
                for (name, value) in values {
                    let value = self.json(value, journal);
                    let name = self.module.strings.intern(name);
                    let key = js::PropertyName::String(js::StringLiteral {
                        value: name,
                        provenance: journal.derive(self.source),
                    });
                    let property = self.insert(js::Property::Field { key, value }, journal);
                    properties.push(property);
                }

                self.insert(js::Expression::ObjectLiteral { properties }, journal)
            }
        }
    }

    /// Insert one numeric literal expression.
    fn number(
        &mut self,
        value: f64,
        journal: &mut ProvenanceJournal<'_>,
    ) -> js::LocalNodeId<js::Expression> {
        self.insert(
            js::Expression::Literal {
                value: js::Literal::Number(value),
            },
            journal,
        )
    }

    /// Insert one byte array expression.
    fn binary(
        &mut self,
        bytes: &[u8],
        journal: &mut ProvenanceJournal<'_>,
    ) -> js::LocalNodeId<js::Expression> {
        let mut elements = Vec::with_capacity(bytes.len());

        // emit each byte as one array element
        for byte in bytes {
            let value = self.number(f64::from(*byte), journal);
            let element = self.insert(js::ArrayElement::Expression { value }, journal);
            elements.push(element);
        }

        // build the typed array construction
        let array = self.insert(js::Expression::ArrayLiteral { elements }, journal);
        let name = self.module.strings.intern(UINT8_ARRAY_NAME);
        let symbol = self.module.insert_symbol(
            name,
            js::SymbolNamespace::Value,
            js::ScopeId::ROOT,
            journal.derive(self.source),
        );
        self.external_symbols.push(symbol);
        let constructor = self.insert(
            js::Expression::Identifier {
                identifier: js::Identifier {
                    original_name: name,
                    symbol,
                    provenance: journal.derive(self.source),
                },
            },
            journal,
        );
        let argument = self.insert(js::Argument::Positional { value: array }, journal);

        self.insert(
            js::Expression::New {
                left: constructor,
                arguments: vec![argument],
            },
            journal,
        )
    }

    /// Finish the resource module around one runtime value.
    fn finish(
        mut self,
        value: js::LocalNodeId<js::Expression>,
        is_exported: bool,
        journal: &mut ProvenanceJournal<'_>,
    ) -> Script {
        let pattern = self.module_default(journal);
        let pattern = self.insert(
            js::Pattern::Binding {
                identifier: pattern,
            },
            journal,
        );
        let declarator = self.insert(
            js::Declarator {
                pattern,
                value: Some(value),
            },
            journal,
        );
        let binding = self.insert(
            js::Statement::Let {
                is_exported: false,
                mutability: js::Mutability::Immutable,
                declarators: vec![declarator],
            },
            journal,
        );
        self.module.roots.push(binding);

        // expose the resource value from standalone outputs
        if is_exported {
            let identifier = self.module_default(journal);
            let value = self.insert(js::Expression::Identifier { identifier }, journal);
            let export = self.insert(js::Statement::ExportDefault { value }, journal);
            self.module.roots.push(export);
        }

        let mut script = Script::new(self.module, Vec::new(), Vec::new());
        script.set_default_module(self.module_default, script.module.id);
        for symbol in self.external_symbols {
            script.set_external_symbol(symbol);
        }

        script
    }
}

impl JsLinker<'_> {
    /// Build the final linked asset URL for one file-loader module.
    fn file_loader_reference(
        &self,
        output: OutputId,
        layout: &OutputLayout,
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
                    target: *self.target_id,
                    message: "assets.mode = 'reference' cannot bind imported file values"
                        .to_string(),
                });
            }
        };
        let source_location =
            layout
                .output_location(output)
                .ok_or_else(|| LinkError::Internal {
                    anchor: self.package_id.into(),
                    package: self.package_id,
                    message: format!("missing output placement for output id {}", output.0),
                })?;
        let target = TargetLocation::new(self.package_dir, self.target, self.target_name());

        Ok(target.runtime_reference(source_location, &output_location))
    }

    /// Build one JavaScript module for a non-code resource.
    pub(in crate::link::js) fn build_resource_js_output(
        &self,
        output: OutputId,
        module: ModuleId,
        graph: &OutputGraph,
        plan: &Plan,
    ) -> LinkResult<Script> {
        let source_module = self.module(module)?;
        let profile = self.profile_id()?;
        let bound = self
            .artifacts
            .read::<DirBound>((module, profile))
            .map_err(|error| self.missing_resource_artifact(module, "bound DIR", error))?;
        let parsed = self
            .artifacts
            .read::<DirParsed>(module)
            .map_err(|error| self.missing_resource_artifact(module, "parsed DIR", error))?;
        let expanded = self
            .artifacts
            .read::<DirExpanded>((module, profile))
            .map_err(|error| self.missing_resource_artifact(module, "expanded DIR", error))?;
        let materialized = self
            .artifacts
            .read::<DirMaterialized>((module, profile))
            .map_err(|error| self.missing_resource_artifact(module, "materialized DIR", error))?;
        let view = expanded.view(&parsed);
        let source = view.provenance_any(bound.module_node);
        let mut provenance = materialized.provenance.extend();
        let mut journal = provenance.record("link-javascript-resource");
        let mut emitter = ResourceEmitter::new(
            module,
            source,
            materialized.provenance.clone(),
            &mut journal,
        );
        let value = self.resource_value(
            output,
            source_module.as_ref(),
            plan,
            &mut emitter,
            &mut journal,
        )?;
        let output = graph.output(output).ok_or_else(|| LinkError::Internal {
            anchor: module.into(),
            package: self.package_id,
            message: format!("missing JS output graph node for output id {}", output.0),
        })?;
        let is_exported = output.facade_module() == module;
        let mut script = emitter.finish(value, is_exported, &mut journal);
        script.module.provenance = provenance.finish();

        Ok(script)
    }

    /// Build the runtime value represented by one resource module.
    fn resource_value(
        &self,
        output: OutputId,
        module: &Module,
        plan: &Plan,
        emitter: &mut ResourceEmitter,
        provenance: &mut ProvenanceJournal<'_>,
    ) -> LinkResult<js::LocalNodeId<js::Expression>> {
        if module.loader.is_data() {
            let value = self.data(module.id)?;
            let Data::Json(value) = value.as_ref();

            return Ok(emitter.json(value, provenance));
        }

        if module.loader == Loader::Base64 {
            let file = self.file(module.file_id)?;
            if file.is_text() {
                return Err(self.resource_type_error(module, "base64", "binary"));
            }
            let encoded = STANDARD.encode(file.bytes());

            return Ok(emitter.string(&encoded, provenance));
        }

        if module.loader.is_text() {
            let file = self.file(module.file_id)?;
            if !file.is_text() {
                return Err(self.resource_type_error(module, "text", "text"));
            }

            return Ok(emitter.string(file.text(), provenance));
        }

        if module.loader == Loader::Binary {
            let file = self.file(module.file_id)?;
            if file.is_text() {
                return Err(self.resource_type_error(module, "binary", "binary"));
            }

            return Ok(emitter.binary(file.bytes(), provenance));
        }

        if module.loader.is_file() {
            let reference = self.file_loader_reference(output, plan.output_layout(), module)?;

            return Ok(emitter.string(&reference, provenance));
        }

        Err(LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!(
                "unsupported asset loader '{}' for module '{}'",
                module.loader.as_str(),
                module.uri
            ),
        })
    }

    /// Build one missing resource artifact error.
    fn missing_resource_artifact(
        &self,
        module: ModuleId,
        artifact: &str,
        error: impl std::fmt::Debug,
    ) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!("missing {artifact} for asset module {module:?}: {error:?}"),
        }
    }

    /// Build one resource loader type error.
    fn resource_type_error(&self, module: &Module, loader: &str, expected: &str) -> LinkError {
        LinkError::Internal {
            anchor: self.package_id.into(),
            package: self.package_id,
            message: format!(
                "{loader} loader expected {expected} file content for '{}'",
                module.uri
            ),
        }
    }
}
