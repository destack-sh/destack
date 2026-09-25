use tspp_dir as dir;

use crate::{DocError, DocResult};

use super::{FormattedSignature, Printer};

impl Printer<'_, '_, '_> {
    /// Format one declaration signature.
    pub(super) fn declaration_signature(
        &self,
        declaration: &dir::Declaration,
    ) -> DocResult<FormattedSignature> {
        let Some(name) = self.module.declaration_display_name(declaration) else {
            return Err(DocError::missing("declaration signature name"));
        };
        let prefix = declaration_prefix(declaration);

        let text = match declaration {
            dir::Declaration::Function(declaration) => {
                return self.function_declaration(&name, &declaration.signature, &prefix);
            }
            dir::Declaration::Global(_) => FormattedSignature::plain(format!("{prefix}global")),
            dir::Declaration::Module(_) => FormattedSignature::plain(format!("{prefix}module")),
            dir::Declaration::Struct(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;

                FormattedSignature::named(format!("{prefix}struct "), &name, generics)
            }
            dir::Declaration::Class(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;

                FormattedSignature::named(format!("{prefix}class "), &name, generics)
            }
            dir::Declaration::Interface(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;
                let keyword = if declaration.is_nominal {
                    "newtype interface"
                } else {
                    "interface"
                };

                FormattedSignature::named(format!("{prefix}{keyword} "), &name, generics)
            }
            dir::Declaration::Enum(_) => {
                FormattedSignature::named(format!("{prefix}enum "), &name, String::new())
            }
            dir::Declaration::Type(declaration) => {
                let generics = self.generics(&declaration.generic_parameters)?;
                let value = self.type_expression(declaration.value)?;
                let keyword = if declaration.is_nominal {
                    "newtype"
                } else {
                    "type"
                };

                FormattedSignature::named(
                    format!("{prefix}{keyword} "),
                    &name,
                    format!("{generics} = {value}"),
                )
            }
            dir::Declaration::Extension(_) => {
                FormattedSignature::named(format!("{prefix}extension "), &name, String::new())
            }
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
