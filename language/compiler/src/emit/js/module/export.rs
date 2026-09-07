use destack_dir as dir;
use destack_js as js;
use destack_source::{NodeSpanRegion, NodeSpanType};

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Move one direct export onto an explicit export statement.
    pub(crate) fn split_export(
        &mut self,
        statement: js::LocalNodeId<js::Statement>,
        source: dir::LocalNodeId<dir::Expression>,
    ) -> Result<Option<js::LocalNodeId<js::Statement>>, EmitError> {
        let value = self.output.tree.get(statement).clone();
        let (identifiers, export) = match value {
            js::Statement::Declaration {
                export: Some(export),
                declaration,
            } => {
                let declaration_value = self.output.tree.get(declaration);
                let identifier = match declaration_value {
                    js::Declaration::Class(declaration) => declaration.name,
                    js::Declaration::Function(declaration) => declaration.name,
                };

                // retain anonymous default declarations in their only valid declaration form
                let Some(identifier) = identifier else {
                    if export == js::ExportKind::Default {
                        return Ok(None);
                    }

                    return Err(
                        self.internal_error("named JavaScript export has no binding".to_string())
                    );
                };
                let replacement = js::Statement::Declaration {
                    export: None,
                    declaration,
                };
                self.output
                    .tree
                    .rewrite(statement, replacement, &mut self.provenance);

                (vec![identifier], Some(export))
            }
            js::Statement::Let {
                is_exported: true,
                mutability,
                declarators,
            } => {
                let mut identifiers = Vec::new();
                for declarator in &declarators {
                    let pattern = self.output.tree.get(*declarator).pattern;
                    self.collect_export_bindings(pattern, &mut identifiers);
                }
                let replacement = js::Statement::Let {
                    is_exported: false,
                    mutability,
                    declarators,
                };
                self.output
                    .tree
                    .rewrite(statement, replacement, &mut self.provenance);

                (identifiers, None)
            }
            js::Statement::Var {
                is_exported: true,
                declarators,
            } => {
                let mut identifiers = Vec::new();
                for declarator in &declarators {
                    let pattern = self.output.tree.get(*declarator).pattern;
                    self.collect_export_bindings(pattern, &mut identifiers);
                }
                let replacement = js::Statement::Var {
                    is_exported: false,
                    declarators,
                };
                self.output
                    .tree
                    .rewrite(statement, replacement, &mut self.provenance);

                (identifiers, None)
            }
            _ => return Ok(None),
        };

        // omit an empty export created by an empty binding pattern
        if identifiers.is_empty() {
            return Ok(None);
        }

        // export each local binding under its fixed public name
        let default_name = self.output.strings.intern("default");
        let mut specifiers = Vec::with_capacity(identifiers.len());
        for identifier in identifiers {
            let [local_provenance, exported_provenance] =
                self.provenance.split(identifier.provenance);
            let local = js::Identifier {
                provenance: local_provenance,
                ..identifier
            };
            let text = match export {
                Some(js::ExportKind::Default) => default_name,
                Some(js::ExportKind::Named) | None => identifier.original_name,
            };
            let exported = js::ModuleExportName::Identifier(js::IdentifierName {
                text,
                provenance: exported_provenance,
            });
            let specifier = js::ExportSpecifier { local, exported };
            let specifier = self.insert_from_source(specifier, source);
            specifiers.push(specifier);
        }

        let export = self.insert_from_source(js::Statement::Export { specifiers }, source);

        Ok(Some(export))
    }

    /// Collect every binding declared by one exported pattern.
    fn collect_export_bindings(
        &self,
        pattern: js::LocalNodeId<js::Pattern>,
        identifiers: &mut Vec<js::Identifier>,
    ) {
        match self.output.tree.get(pattern) {
            js::Pattern::Binding { identifier } => identifiers.push(*identifier),
            js::Pattern::Array { fields, rest } => {
                for field in fields {
                    if let js::ArrayPatternField::Positional { pattern, .. } =
                        self.output.tree.get(*field)
                    {
                        self.collect_export_bindings(*pattern, identifiers);
                    }
                }

                if let Some(rest) = rest {
                    self.collect_export_bindings(*rest, identifiers);
                }
            }
            js::Pattern::Object { fields, rest } => {
                for field in fields {
                    match self.output.tree.get(*field) {
                        js::ObjectPatternField::Named { pattern, .. } => {
                            self.collect_export_bindings(*pattern, identifiers);
                        }
                        js::ObjectPatternField::Shorthand { identifier, .. } => {
                            identifiers.push(*identifier);
                        }
                    }
                }

                if let Some(rest) = rest {
                    identifiers.push(*rest);
                }
            }
        }
    }

    /// Emit one ECMAScript export declaration.
    pub(crate) fn emit_export(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        target: Option<dir::StringId>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
        attributes: Option<&dir::ImportAttributeClause>,
    ) -> Result<js::Statement, EmitError> {
        let attributes = attributes
            .map(|attributes| self.emit_import_attributes(expression, attributes))
            .transpose()?;

        // emit a local export
        let Some(target) = target else {
            return self.emit_local_export(expression, items, attributes);
        };

        // emit a re-export
        let source = self.string_literal(target, expression, NodeSpanType::Main);

        self.emit_re_export(source, items, attributes)
    }

    /// Emit one local ECMAScript export.
    fn emit_local_export(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
        attributes: Option<Vec<js::LocalNodeId<js::ImportAttribute>>>,
    ) -> Result<js::Statement, EmitError> {
        // reject attributes on local exports
        if attributes.is_some() {
            return Err(self.unhandled(
                expression.into_global_any(self.module),
                Some("local JavaScript exports cannot carry import attributes".to_string()),
            ));
        }

        // emit a default value export
        if let [item] = items {
            let item_value = self.tree.get(*item);
            if let dir::DependencyItem::Binding {
                binding: dir::DependencyBinding::Default,
                value: Some(value),
                ..
            } = item_value
            {
                let value = self.emit_expression(*value)?;

                return Ok(js::Statement::ExportDefault { value });
            }
        }

        // emit named local exports
        let specifiers = items
            .iter()
            .map(|item| self.emit_local_export_specifier(*item))
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(js::Statement::Export { specifiers })
    }

    /// Emit one local ECMAScript export specifier.
    fn emit_local_export_specifier(
        &mut self,
        item: dir::LocalNodeId<dir::DependencyItem>,
    ) -> Result<js::LocalNodeId<js::ExportSpecifier>, EmitError> {
        let dependency = self.tree.get(item);

        // require a local binding
        let dir::DependencyItem::Binding {
            binding,
            name,
            alias,
            value: None,
        } = dependency
        else {
            return Err(self.unhandled(
                item.into_global_any(self.module),
                Some("invalid local JavaScript export item".to_string()),
            ));
        };

        // require a named export
        let local_name = match binding {
            dir::DependencyBinding::Named => name.map(|name| name.string()),
            dir::DependencyBinding::Default | dir::DependencyBinding::Namespace => None,
        };
        let Some(local_name) = local_name else {
            return Err(self.unhandled(
                item.into_global_any(self.module),
                Some("local JavaScript exports require a local name".to_string()),
            ));
        };

        // emit the local and exported names
        let local = self.emit_identifier_reference(
            local_name,
            item.into_any(),
            NodeSpanType::Region(NodeSpanRegion::Type),
        )?;
        let exported_name = alias.unwrap_or(local_name);
        let exported = self.export_name(
            dir::Name::Identifier(exported_name),
            item,
            NodeSpanType::Main,
        );
        let specifier = js::ExportSpecifier { local, exported };

        Ok(self.insert_from_source(specifier, item))
    }

    /// Emit one ECMAScript re-export.
    fn emit_re_export(
        &mut self,
        source: js::StringLiteral,
        items: &[dir::LocalNodeId<dir::DependencyItem>],
        attributes: Option<Vec<js::LocalNodeId<js::ImportAttribute>>>,
    ) -> Result<js::Statement, EmitError> {
        // emit a namespace re-export
        if let [item] = items {
            let dependency = self.tree.get(*item);
            if let dir::DependencyItem::Binding {
                binding: dir::DependencyBinding::Namespace,
                name: None,
                alias,
                value: None,
            } = dependency
            {
                let exported = alias.map(|alias| {
                    self.export_name(dir::Name::Identifier(alias), *item, NodeSpanType::Main)
                });

                return Ok(js::Statement::ExportAll {
                    exported,
                    source,
                    attributes,
                });
            }
        }

        let default_name = self.output.strings.intern("default");

        // emit explicit re-exports
        let specifiers = items
            .iter()
            .map(|item| self.emit_re_export_specifier(*item, default_name))
            .collect::<Result<Vec<_>, EmitError>>()?;

        Ok(js::Statement::ReExport {
            source,
            specifiers,
            attributes,
        })
    }

    /// Emit one ECMAScript re-export specifier.
    fn emit_re_export_specifier(
        &mut self,
        item: dir::LocalNodeId<dir::DependencyItem>,
        default_name: dir::StringId,
    ) -> Result<js::LocalNodeId<js::ReExportSpecifier>, EmitError> {
        let dependency = self.tree.get(item);

        // require a re-export binding
        let dir::DependencyItem::Binding {
            binding,
            name,
            alias,
            value: None,
        } = dependency
        else {
            return Err(self.unhandled(
                item.into_global_any(self.module),
                Some("invalid JavaScript re-export item".to_string()),
            ));
        };

        // select the imported name
        let imported = match binding {
            dir::DependencyBinding::Default => dir::Name::Identifier(default_name),
            dir::DependencyBinding::Named => {
                let Some(name) = *name else {
                    return Err(self.unhandled(
                        item.into_global_any(self.module),
                        Some("named JavaScript re-exports require an imported name".to_string()),
                    ));
                };

                name
            }
            dir::DependencyBinding::Namespace => {
                return Err(self.unhandled(
                    item.into_global_any(self.module),
                    Some("namespace re-export must be the only export item".to_string()),
                ));
            }
        };

        // emit the imported and exported names
        let exported_name = alias.unwrap_or(imported.string());
        let imported = self.export_name(imported, item, NodeSpanType::Region(NodeSpanRegion::Type));
        let exported = self.export_name(
            dir::Name::Identifier(exported_name),
            item,
            NodeSpanType::Main,
        );
        let specifier = js::ReExportSpecifier { imported, exported };

        Ok(self.insert_from_source(specifier, item))
    }
}
