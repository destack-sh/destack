use destack_dir as dir;
use destack_js as js;
use destack_source::{NodeSpanList, NodeSpanRegion, NodeSpanType};

use crate::EmitError;
use crate::emit::js::ModuleEmitter;

impl ModuleEmitter<'_> {
    /// Emit one ECMAScript import declaration.
    pub(crate) fn emit_import(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: dir::StringId,
        items: Option<&[dir::LocalNodeId<dir::DependencyItem>]>,
        attributes: Option<&dir::ImportAttributeClause>,
    ) -> Result<js::Statement, EmitError> {
        let source = self.string_literal(target, expression, NodeSpanType::Main);
        let attributes = attributes
            .map(|attributes| self.emit_import_attributes(expression, attributes))
            .transpose()?;

        // emit an import without bindings
        let Some(items) = items else {
            return Ok(js::Statement::Import {
                source,
                clause: None,
                attributes,
            });
        };

        let clause = self.emit_import_clause(items)?;

        Ok(js::Statement::Import {
            source,
            clause: Some(clause),
            attributes,
        })
    }

    /// Emit one ECMAScript import clause.
    fn emit_import_clause(
        &mut self,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
    ) -> Result<js::ImportClause, EmitError> {
        let mut default = None;
        let mut namespace = None;
        let mut specifiers = Vec::with_capacity(items.len());

        // collect the clause bindings
        for item in items.iter().copied() {
            let binding = self.tree.get(item);

            // require a complete import binding
            let dir::DependencyItem::Binding {
                binding,
                name,
                alias,
                value,
            } = binding
            else {
                return Err(self.unhandled(
                    item.into_global_any(self.module),
                    Some("dependency error slots cannot reach JavaScript emission".to_string()),
                ));
            };

            // reject dependency values
            if value.is_some() {
                return Err(self.unhandled(
                    item.into_global_any(self.module),
                    Some("import bindings cannot carry values".to_string()),
                ));
            }

            match binding {
                // collect the default binding
                dir::DependencyBinding::Default => {
                    let Some(local) = alias else {
                        return Err(self.unhandled(
                            item.into_global_any(self.module),
                            Some("JavaScript default imports require a local name".to_string()),
                        ));
                    };

                    // require one default binding
                    if default.is_some() {
                        return Err(self.unhandled(
                            item.into_global_any(self.module),
                            Some("JavaScript imports permit one default binding".to_string()),
                        ));
                    }

                    // store the default binding
                    default = Some(self.emit_binding_identifier(*local, item)?);
                }
                // collect the namespace binding
                dir::DependencyBinding::Namespace => {
                    let Some(local) = alias else {
                        return Err(self.unhandled(
                            item.into_global_any(self.module),
                            Some("JavaScript namespace imports require a local name".to_string()),
                        ));
                    };

                    // keep namespace and named bindings exclusive
                    if namespace.is_some() || !specifiers.is_empty() {
                        return Err(self.unhandled(
                            item.into_global_any(self.module),
                            Some(
                                "JavaScript namespace and named imports cannot share a clause"
                                    .to_string(),
                            ),
                        ));
                    }

                    // store the namespace binding
                    namespace = Some(self.emit_binding_identifier(*local, item)?);
                }
                // collect one named binding
                dir::DependencyBinding::Named => {
                    let Some(imported) = *name else {
                        return Err(self.unhandled(
                            item.into_global_any(self.module),
                            Some("JavaScript named imports require an imported name".to_string()),
                        ));
                    };

                    // keep namespace and named bindings exclusive
                    if namespace.is_some() {
                        return Err(self.unhandled(
                            item.into_global_any(self.module),
                            Some(
                                "JavaScript namespace and named imports cannot share a clause"
                                    .to_string(),
                            ),
                        ));
                    }

                    let local_name = alias.unwrap_or(imported.string());
                    let local = self.emit_binding_identifier(local_name, item)?;
                    let imported = self.export_name(
                        imported,
                        item,
                        NodeSpanType::Region(NodeSpanRegion::Type),
                    );
                    let specifier = js::ImportSpecifier { imported, local };
                    let specifier = self.insert_from_source(specifier, item);

                    specifiers.push(specifier);
                }
            }
        }

        // select the exact clause form
        let clause = match (default, namespace, specifiers.is_empty()) {
            (Some(default), None, true) => js::ImportClause::Default { local: default },
            (default, Some(local), true) => js::ImportClause::Namespace { default, local },
            (default, None, false) => js::ImportClause::Named {
                default,
                specifiers,
            },
            (None, None, true) => js::ImportClause::Named {
                default: None,
                specifiers,
            },
            _ => {
                return Err(self.internal_error(
                    "JavaScript import clause has incompatible bindings".to_string(),
                ));
            }
        };

        Ok(clause)
    }

    /// Emit one ECMAScript import attribute clause.
    pub(super) fn emit_import_attributes(
        &mut self,
        source: dir::LocalNodeId<dir::Expression>,
        attributes: &dir::ImportAttributeClause,
    ) -> Result<Vec<js::LocalNodeId<js::ImportAttribute>>, EmitError> {
        let mut emitted = Vec::with_capacity(attributes.attributes.len());

        // emit the attribute entries
        for (index, attribute) in attributes.attributes.iter().enumerate() {
            let index = u16::try_from(index).map_err(|_| {
                self.internal_error("JavaScript import has too many attributes".to_string())
            })?;
            let span_type = NodeSpanType::ListItem(NodeSpanList::Entry, index);

            // emit the attribute name
            let name = match attribute.key {
                dir::Name::Identifier(text) => js::ImportAttributeName::Identifier(
                    self.identifier_name(text, source, span_type),
                ),
                dir::Name::String(value) => {
                    js::ImportAttributeName::String(self.string_literal(value, source, span_type))
                }
                dir::Name::Index(_) => {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript import attribute names cannot be indexes".to_string()),
                    ));
                }
            };

            // require a string value
            let dir::ImportAttributeValue::Literal(dir::Literal::String(value)) = attribute.value
            else {
                return Err(self.unhandled(
                    source.into_global_any(self.module),
                    Some("JavaScript import attribute values must be strings".to_string()),
                ));
            };

            // emit the complete attribute
            let value = self.string_literal(value, source, span_type);
            let attribute = js::ImportAttribute { name, value };

            emitted.push(self.insert_from_source(attribute, source));
        }

        Ok(emitted)
    }
}
