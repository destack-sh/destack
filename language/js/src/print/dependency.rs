use crate::{
    ExportSpecifier, ImportAttribute, ImportAttributeName, ImportClause, ImportSpecifier,
    LocalNodeId, Module, ModuleExportName, ReExportSpecifier, StringLiteral, Tree, TreeStore,
};

use super::printer::{PrintError, PrintNode, Printer};

impl PrintNode for ImportSpecifier {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        let local_name = self.local.emitted_name(&module.symbols);
        if self.imported.value() == local_name {
            return printer.write_attributed(self.imported.provenance(), None, |printer| {
                printer.identifier(self.local, module)
            });
        }

        printer.module_export_name(self.imported, module)?;
        printer.word("as")?;
        printer.identifier(self.local, module)
    }
}

impl PrintNode for ImportAttribute {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        match self.name {
            ImportAttributeName::Identifier(name) => printer.identifier_name(name, module)?,
            ImportAttributeName::String(name) => printer.string(name, module)?,
        }
        printer.token(":")?;
        printer.string(self.value, module)
    }
}

impl PrintNode for ExportSpecifier {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        let local_name = self.local.emitted_name(&module.symbols);
        if self.exported.value() == local_name {
            return printer.write_attributed(self.exported.provenance(), None, |printer| {
                printer.identifier(self.local, module)
            });
        }

        printer.identifier(self.local, module)?;
        printer.word("as")?;
        printer.module_export_name(self.exported, module)
    }
}

impl PrintNode for ReExportSpecifier {
    fn print(&self, module: &Module, printer: &mut Printer) -> Result<(), PrintError> {
        if self.imported.value() == self.exported.value() {
            return printer.write_attributed(self.exported.provenance(), None, |printer| {
                printer.module_export_name(self.imported, module)
            });
        }

        printer.module_export_name(self.imported, module)?;
        printer.word("as")?;
        printer.module_export_name(self.exported, module)
    }
}

impl Printer {
    /// Print one import clause and its source.
    pub(super) fn import(
        &mut self,
        source: StringLiteral,
        clause: Option<&ImportClause>,
        module: &Module,
    ) -> Result<(), PrintError> {
        match clause {
            None => self.string(source, module),
            Some(ImportClause::Default { local }) => {
                self.identifier(*local, module)?;
                self.word("from")?;
                self.string(source, module)
            }
            Some(ImportClause::Namespace { default, local }) => {
                if let Some(default) = default {
                    self.identifier(*default, module)?;
                    self.token(",")?;
                }
                self.token("*")?;
                self.word("as")?;
                self.identifier(*local, module)?;
                self.word("from")?;
                self.string(source, module)
            }
            Some(ImportClause::Named {
                default,
                specifiers,
            }) => {
                if let Some(default) = default {
                    self.identifier(*default, module)?;
                    self.token(",")?;
                }
                self.specifiers(specifiers, module)?;
                self.word("from")?;
                self.string(source, module)
            }
        }
    }

    /// Print one import-attribute clause.
    pub(super) fn import_attributes(
        &mut self,
        attributes: &[LocalNodeId<ImportAttribute>],
        module: &Module,
    ) -> Result<(), PrintError> {
        self.word("with")?;
        self.specifiers(attributes, module)
    }

    /// Print one local export clause.
    pub(super) fn export(
        &mut self,
        specifiers: &[LocalNodeId<ExportSpecifier>],
        module: &Module,
    ) -> Result<(), PrintError> {
        self.specifiers(specifiers, module)
    }

    /// Print one re-export clause and its source.
    pub(super) fn re_export(
        &mut self,
        source: StringLiteral,
        specifiers: &[LocalNodeId<ReExportSpecifier>],
        module: &Module,
    ) -> Result<(), PrintError> {
        self.specifiers(specifiers, module)?;
        self.word("from")?;
        self.string(source, module)
    }

    /// Print one module export name.
    pub(super) fn module_export_name(
        &mut self,
        name: ModuleExportName,
        module: &Module,
    ) -> Result<(), PrintError> {
        match name {
            ModuleExportName::Identifier(name) => self.identifier_name(name, module),
            ModuleExportName::String(name) => self.string(name, module),
        }
    }

    /// Print one braced sequence of module specifiers or attributes.
    fn specifiers<T>(
        &mut self,
        specifiers: &[LocalNodeId<T>],
        module: &Module,
    ) -> Result<(), PrintError>
    where
        T: PrintNode,
        Tree: TreeStore<T>,
    {
        self.token("{")?;
        for (index, specifier) in specifiers.iter().copied().enumerate() {
            if index > 0 {
                self.token(",")?;
            }
            self.node(specifier, module)?;
        }
        self.token("}")
    }
}
