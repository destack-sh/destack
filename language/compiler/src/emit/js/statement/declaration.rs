use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ModuleEmitter;

impl ModuleEmitter<'_> {
    /// Return the JavaScript form of one DIR export kind.
    fn export_kind(&self, export: dir::ExportKind) -> js::ExportKind {
        match export {
            dir::ExportKind::Named => js::ExportKind::Named,
            dir::ExportKind::Default => js::ExportKind::Default,
        }
    }

    /// Return the export attached to one declaration expression.
    pub(crate) fn declaration_export(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<js::ExportKind> {
        let dir::Expression::Declaration(declaration) = self.tree.get(expression) else {
            return None;
        };
        let declaration = self.tree.get(*declaration);

        declaration.export().map(|export| self.export_kind(export))
    }

    /// Return whether one variable declaration is exported.
    pub(crate) fn binding_export(
        &self,
        export: Option<dir::ExportKind>,
    ) -> Result<bool, EmitError> {
        match export {
            None => Ok(false),
            Some(dir::ExportKind::Named) => Ok(true),
            Some(dir::ExportKind::Default) => Err(self
                .internal_error("default export reached JavaScript variable emission".to_string())),
        }
    }

    /// Emit one runtime declaration from DIR into JavaScript.
    pub(crate) fn emit_declaration(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> Result<js::LocalNodeId<js::Declaration>, EmitError> {
        let declaration = self.tree.get(declaration_id);

        let declaration = match declaration {
            // emit a struct as a class
            dir::Declaration::Struct(declaration) => {
                let dir::Name::Identifier(name) = declaration.name else {
                    return Err(self.unhandled(
                        declaration_id.into_global_any(self.module),
                        Some("JavaScript class declarations require identifier names".to_string()),
                    ));
                };

                // emit runtime members
                let members = self.emit_members(&declaration.members)?;
                let name = self.emit_binding_identifier(name, declaration_id)?;
                let declaration = js::ClassDeclaration {
                    name: Some(name),
                    extends_expression: None,
                    members,
                };

                js::Declaration::Class(declaration)
            }
            // emit a class
            dir::Declaration::Class(declaration) => {
                // emit the optional class name
                let name = match declaration.name {
                    Some(dir::Name::Identifier(name)) => {
                        Some(self.emit_binding_identifier(name, declaration_id)?)
                    }
                    Some(dir::Name::String(_) | dir::Name::Index(_)) => {
                        return Err(self.unhandled(
                            declaration_id.into_global_any(self.module),
                            Some(
                                "JavaScript class declarations require identifier names"
                                    .to_string(),
                            ),
                        ));
                    }
                    None => None,
                };

                // emit inheritance and runtime members
                let extends_expression = declaration
                    .extends_type
                    .map(|extends_type| self.emit_type_callee(extends_type))
                    .transpose()?;
                let members = self.emit_members(&declaration.members)?;
                let declaration = js::ClassDeclaration {
                    name,
                    extends_expression,
                    members,
                };

                js::Declaration::Class(declaration)
            }
            // emit a function
            dir::Declaration::Function(declaration) => {
                let Some(body) = declaration.body else {
                    return Err(self.unhandled(
                        declaration_id.into_global_any(self.module),
                        Some("JavaScript functions require executable bodies".to_string()),
                    ));
                };

                // emit the function body and signature
                let signature = self.emit_function_signature(&declaration.signature)?;
                let body = self.emit_function_body(body)?;

                // emit the optional function name
                let name = match declaration.name {
                    Some(dir::Name::Identifier(name)) => {
                        Some(self.emit_binding_identifier(name, declaration_id)?)
                    }
                    Some(dir::Name::String(_) | dir::Name::Index(_)) => {
                        return Err(self.unhandled(
                            declaration_id.into_global_any(self.module),
                            Some(
                                "JavaScript function declarations require identifier names"
                                    .to_string(),
                            ),
                        ));
                    }
                    None => None,
                };
                let declaration = js::FunctionDeclaration {
                    name,
                    signature,
                    body,
                };

                js::Declaration::Function(declaration)
            }
            // reject declarations without defined runtime forms
            dir::Declaration::Enum(_) => {
                return Err(self.unhandled(
                    declaration_id.into_global_any(self.module),
                    Some("JavaScript enum representation is undefined".to_string()),
                ));
            }
            // reject non-ambient globals
            dir::Declaration::Global(_) => {
                return Err(self.unhandled(
                    declaration_id.into_global_any(self.module),
                    Some(
                        "non-ambient global declarations have no JavaScript runtime form"
                            .to_string(),
                    ),
                ));
            }
            // reject erased declarations
            dir::Declaration::Type(_) | dir::Declaration::Interface(_) => {
                return Err(self.internal_error(
                    "compile-time declaration reached JavaScript emission".to_string(),
                ));
            }
            // reject unsupported declaration containers
            dir::Declaration::Module(_) | dir::Declaration::Extension(_) => {
                return Err(self.unhandled(declaration_id.into_global_any(self.module), None));
            }
        };

        // insert the declaration
        let declaration_id = self.insert_from_source(declaration, declaration_id);

        Ok(declaration_id)
    }
}
