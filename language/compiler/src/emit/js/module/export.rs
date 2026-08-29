use destack_dir as dir;
use destack_js as js;
use destack_source::{NodeSpanRegion, NodeSpanType};

use crate::EmitError;
use crate::emit::js::ModuleEmitter;

impl ModuleEmitter<'_> {
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
