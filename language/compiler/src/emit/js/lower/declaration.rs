use crate::EmitError;
use tspp_dir as dir;
use tspp_js as js;

use crate::emit::js::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Lower an export kind from DIR into JavaScript.
    pub(crate) fn lower_export_kind(&self, export: dir::ExportKind) -> js::ExportKind {
        match export {
            dir::ExportKind::Named => js::ExportKind::Named,
            dir::ExportKind::Default => js::ExportKind::Default,
        }
    }

    /// Lower one variable export marker.
    pub(crate) fn lower_binding_export(
        &self,
        export: Option<dir::ExportKind>,
    ) -> Result<bool, EmitError> {
        match export {
            None => Ok(false),
            Some(dir::ExportKind::Named) => Ok(true),
            Some(dir::ExportKind::Default) => Err(self
                .internal_error("default export reached JavaScript variable lowering".to_string())),
        }
    }

    /// Lower one runtime declaration from DIR into JavaScript.
    pub(crate) fn lower_declaration(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> Result<js::LocalNodeId<js::Declaration>, EmitError> {
        let source_declaration_id = declaration_id;
        let declaration = self.dir_tree.get(declaration_id);

        let declaration = match declaration {
            dir::Declaration::Struct(declaration) => {
                let members = declaration
                    .members
                    .iter()
                    .map(|member_id| self.lower_member(*member_id))
                    .collect::<Result<Vec<_>, EmitError>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                let declaration = js::ClassDeclaration {
                    name: Some(self.lower_name(declaration.name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    extends_expression: None,
                    members,
                };

                js::Declaration::Class(declaration)
            }
            dir::Declaration::Class(declaration) => {
                let extends_expression = declaration
                    .extends_type
                    .map(|extends_type| self.lower_type_callee(extends_type))
                    .transpose()?;
                let members = declaration
                    .members
                    .iter()
                    .map(|member_id| self.lower_member(*member_id))
                    .collect::<Result<Vec<_>, EmitError>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                let declaration = js::ClassDeclaration {
                    name: declaration.name.map(|name| self.lower_name(name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    extends_expression,
                    members,
                };

                js::Declaration::Class(declaration)
            }
            dir::Declaration::Function(declaration) => {
                let Some(body) = declaration.body else {
                    return Err(self.unhandled(
                        declaration_id.into_global_any(self.module.id),
                        Some("JavaScript functions require executable bodies".to_string()),
                    ));
                };
                let signature = self.lower_function_signature(&declaration.signature)?;
                let body = self.lower_expression_as_block(body)?;
                let declaration = js::FunctionDeclaration {
                    name: declaration.name.map(|name| self.lower_name(name)),
                    export: declaration
                        .export
                        .map(|export| self.lower_export_kind(export)),
                    signature,
                    body,
                };

                js::Declaration::Function(declaration)
            }
            dir::Declaration::Enum(_) => {
                return Err(self.unhandled(
                    declaration_id.into_global_any(self.module.id),
                    Some("JavaScript enum representation is undefined".to_string()),
                ));
            }
            dir::Declaration::Global(_) => {
                return Err(self.unhandled(
                    declaration_id.into_global_any(self.module.id),
                    Some(
                        "non-ambient global declarations have no JavaScript runtime form"
                            .to_string(),
                    ),
                ));
            }
            dir::Declaration::Type(_) | dir::Declaration::Interface(_) => {
                return Err(self.internal_error(
                    "compile-time declaration reached JavaScript lowering".to_string(),
                ));
            }
            dir::Declaration::Module(_) | dir::Declaration::Extension(_) => {
                return Err(self.unhandled(declaration_id.into_global_any(self.module.id), None));
            }
        };

        let declaration_id =
            self.tree
                .insert_from_source(declaration, self.module.id, declaration_id);
        self.copy_source_node_symbol(declaration_id, source_declaration_id);

        Ok(declaration_id)
    }
}
