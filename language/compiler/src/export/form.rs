use tspp_dir as dir;
use tspp_dir::ExportForm;

use crate::export::state::ExportState;

impl ExportState<'_> {
    /// Classify one exported symbol's declared form.
    pub(in crate::export) fn symbol_form(&self, symbol: dir::LocalSymbolId) -> ExportForm {
        // exports without local declarations classify conservatively
        let Some(declaration) = self.bindings.get_symbol(symbol).declaration else {
            return ExportForm::Inferred;
        };
        let node = declaration.local_id;

        // declarations classify by their written annotations
        if let Ok(id) = node.try_into_typed::<dir::Declaration>() {
            return self.declaration_form(self.view.get(id));
        }

        // bindings classify by annotation, then literal initializers
        if let Some(declarator) = self.enclosing_declarator(node) {
            if declarator.ty.is_some() {
                return ExportForm::Declared;
            }
            let is_literal = declarator
                .value
                .is_some_and(|value| matches!(self.view.get(value), dir::Expression::Literal(_)));

            return match is_literal {
                true => ExportForm::Literal,
                false => ExportForm::Inferred,
            };
        }

        ExportForm::Inferred
    }

    /// Return the declarator containing one binding declaration node.
    fn enclosing_declarator(&self, mut node: dir::LocalNodeIdAny) -> Option<&dir::Declarator> {
        loop {
            if let Ok(declarator) = node.try_into_typed::<dir::Declarator>() {
                return Some(self.view.get(declarator));
            }

            node = self.view.get_parent_any(node)?;
        }
    }

    /// Classify one declaration's written form.
    fn declaration_form(&self, declaration: &dir::Declaration) -> ExportForm {
        match declaration {
            // type declarations write their complete types by construction
            dir::Declaration::Type(_)
            | dir::Declaration::Enum(_)
            | dir::Declaration::Interface(_) => ExportForm::Declared,

            // nominal members may leave their types to inference
            dir::Declaration::Struct(declaration) => self.members_form(&declaration.members),
            dir::Declaration::Class(declaration) => self.members_form(&declaration.members),
            dir::Declaration::Extension(declaration) => self.members_form(&declaration.members),

            // callables require inference for unwritten results and parameters
            dir::Declaration::Function(declaration) => self.signature_form(&declaration.signature),

            // wrappers never declare a type themselves
            dir::Declaration::Global(_) | dir::Declaration::Module(_) => ExportForm::Inferred,
        }
    }

    /// Classify one nominal's members as a joint form.
    fn members_form(&self, members: &[dir::LocalNodeId<dir::Member>]) -> ExportForm {
        for member in members {
            let declared = match self.view.get(*member) {
                // unannotated fields and constants leave their types to inference
                dir::Member::Field {
                    declared_type,
                    default,
                    ..
                }
                | dir::Member::AssociatedConst {
                    declared_type,
                    value: default,
                    ..
                } => {
                    declared_type.is_some()
                        || default.is_some_and(|value| {
                            matches!(self.view.get(value), dir::Expression::Literal(_))
                        })
                }
                dir::Member::Method { signature, .. } => {
                    self.signature_form(signature) == ExportForm::Declared
                }
                dir::Member::AssociatedType { .. }
                | dir::Member::StaticBlock { .. }
                | dir::Member::ConstBlock { .. }
                | dir::Member::Error => true,
            };
            if !declared {
                return ExportForm::Inferred;
            }
        }

        ExportForm::Declared
    }

    /// Classify one callable signature's written form.
    fn signature_form(&self, signature: &dir::FunctionSignature) -> ExportForm {
        // unwritten results require inference from the body
        if signature.return_type.is_none() {
            return ExportForm::Inferred;
        }

        // unannotated parameters require inference from their uses
        for parameter in &signature.parameters {
            let declared = match self.view.get(*parameter) {
                dir::Parameter::Named { declared_type, .. }
                | dir::Parameter::Pattern { declared_type, .. }
                | dir::Parameter::VariadicNamed { declared_type, .. }
                | dir::Parameter::VariadicPattern { declared_type, .. } => declared_type.is_some(),
                dir::Parameter::Error => true,
            };
            if !declared {
                return ExportForm::Inferred;
            }
        }

        ExportForm::Declared
    }
}
