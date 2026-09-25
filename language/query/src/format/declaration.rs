use tspp_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;

impl Formatter<'_, '_, '_> {
    /// Format one binding type.
    pub(crate) fn binding_type(
        &self,
        declarator: &dir::Declarator,
        type_id: dir::GlobalTypeId,
    ) -> QueryResult<String> {
        let Some(value_id) = declarator.value else {
            return self.global_type(type_id);
        };
        let dir::Expression::Declaration(declaration_id) = self.module.view()?.get(value_id) else {
            return self.global_type(type_id);
        };
        let dir::Declaration::Function(function) = self.module.view()?.get(*declaration_id) else {
            return self.global_type(type_id);
        };
        if function.signature.form != dir::FunctionForm::Lambda {
            return self.global_type(type_id);
        }

        // retain authored names on an inferred callable type
        let parameter_names = function
            .signature
            .parameters
            .iter()
            .map(|parameter_id| {
                self.module
                    .parameter_name(self.module.view()?.get(*parameter_id))
            })
            .collect::<QueryResult<Vec<_>>>()?;

        self.callable_type(type_id, Some(&parameter_names))
    }

    /// Format one field type with its declaration modifiers.
    pub(crate) fn field_type(
        &self,
        type_text: String,
        is_static: bool,
        is_readonly: bool,
    ) -> String {
        let static_prefix = if is_static { "static " } else { "" };
        let readonly_prefix = if is_readonly { "readonly " } else { "" };

        format!("{static_prefix}{readonly_prefix}{type_text}")
    }

    /// Format one declaration signature.
    pub(super) fn declaration_signature(
        &self,
        declaration: &dir::Declaration,
    ) -> QueryResult<String> {
        let Some(name) = self.module.declaration_display_name(declaration) else {
            return Err(QueryError::missing("declaration signature name"));
        };
        let prefix = declaration_prefix(declaration);

        let text = match declaration {
            dir::Declaration::Function(declaration) => {
                return self.function_declaration(&name, &declaration.signature, &prefix);
            }
            dir::Declaration::Global(_) => format!("{prefix}global"),
            dir::Declaration::Module(_) => format!("{prefix}module"),
            dir::Declaration::Struct(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;

                format!("{prefix}struct {name}{generics}")
            }
            dir::Declaration::Class(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;

                format!("{prefix}class {name}{generics}")
            }
            dir::Declaration::Interface(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;
                let keyword = if declaration.is_nominal {
                    "newtype interface"
                } else {
                    "interface"
                };

                format!("{prefix}{keyword} {name}{generics}")
            }
            dir::Declaration::Enum(_) => format!("{prefix}enum {name}"),
            dir::Declaration::Type(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;
                let value_id = declaration.value.into_global_any(self.module.module_id());
                let value = self.node_type(value_id)?;
                let keyword = if declaration.is_nominal {
                    "newtype"
                } else {
                    "type"
                };

                format!("{prefix}{keyword} {name}{generics} = {value}")
            }
            dir::Declaration::Extension(_) => format!("{prefix}extension {name}"),
        };

        Ok(text)
    }
}

/// Format one declaration prefix.
fn declaration_prefix(declaration: &dir::Declaration) -> String {
    let export_prefix = match declaration.export() {
        Some(dir::ExportKind::Named) => "export ",
        Some(dir::ExportKind::Default) => "export default ",
        None => "",
    };
    let declare_prefix = if declaration.is_ambient() {
        "declare "
    } else {
        ""
    };
    let abstract_prefix = if declaration.is_abstract() {
        "abstract "
    } else {
        ""
    };

    format!("{export_prefix}{declare_prefix}{abstract_prefix}")
}
