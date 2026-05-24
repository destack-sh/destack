use destack_artifact::GlobalEnvironment;
use destack_dir as dir;

use crate::check::{ArgumentTerm, CheckModuleState};

impl CheckModuleState {
    /// Commit declaration relations and extension entries.
    pub(super) fn commit_relations_and_extensions(&mut self, environment: &GlobalEnvironment) {
        let declarations = self
            .input
            .parsed
            .tree
            .iter_nodes_of_type::<dir::Declaration>()
            .map(|(id, declaration)| (id, declaration.clone()))
            .collect::<Vec<_>>();

        // write declaration side tables
        for (id, declaration) in declarations {
            let Some(symbol) = self.declaration_symbol(id.into_any()) else {
                continue;
            };

            match declaration {
                dir::Declaration::Class(declaration) => {
                    self.commit_extends_relation(
                        environment,
                        symbol,
                        declaration.extends_expression,
                    );
                    self.commit_implements_relations(
                        environment,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Struct(declaration) => {
                    self.commit_implements_relations(
                        environment,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Enum(declaration) => {
                    self.commit_implements_relations(
                        environment,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Interface(declaration) => {
                    self.commit_interface_extends_relations(
                        environment,
                        symbol,
                        &declaration.extends,
                    );
                }
                dir::Declaration::Extension(declaration) => {
                    self.commit_extension(environment, symbol, &declaration);
                    self.commit_implements_relations(
                        environment,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                _ => {}
            }
        }
    }

    /// Commit one extends relation.
    fn commit_extends_relation(
        &mut self,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        let Some(expression) = expression else {
            return;
        };
        let variable = self.node_type_variable(expression.into_global_any(self.input.module));
        let Some(ty) = self.commit_variable_type(environment, variable) else {
            return;
        };

        self.output
            .relations
            .push_extends(symbol, dir::Relation::extends(ty));
    }

    /// Commit interface parent relations.
    fn commit_interface_extends_relations(
        &mut self,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        extends: &[dir::InterfaceHeritage],
    ) {
        for heritage in extends {
            let Some(parent) = self.interface_heritage_type(environment, heritage) else {
                continue;
            };

            self.output
                .relations
                .push_extends(symbol, dir::Relation::extends(parent));
        }
    }

    /// Commit one interface heritage item.
    fn interface_heritage_type(
        &mut self,
        environment: &GlobalEnvironment,
        heritage: &dir::InterfaceHeritage,
    ) -> Option<dir::LocalTypeId> {
        let symbol = self.interface_heritage_symbol(heritage.expression)?;
        let arguments = self.build_generic_argument_terms(&heritage.generic_arguments);
        let arguments = self.commit_argument_terms(environment, &arguments)?;
        let source = heritage
            .expression
            .into_global_any(self.input.module)
            .local_id;
        let ty = self.commit_named_type(environment, symbol, arguments, source)?;

        Some(self.intern_type(ty, source))
    }

    /// Commit implemented interface relations.
    fn commit_implements_relations(
        &mut self,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        implemented: &[dir::LocalNodeId<dir::TypeExpression>],
    ) {
        for implemented in implemented {
            let variable = self.type_expression_variable(*implemented);
            let Some(ty) = self.commit_variable_type(environment, variable) else {
                continue;
            };

            self.output
                .relations
                .push_implements(symbol, dir::Relation::implements(ty));
        }
    }

    /// Commit one extension entry.
    fn commit_extension(
        &mut self,
        environment: &GlobalEnvironment,
        symbol: dir::GlobalSymbolId,
        declaration: &dir::ExtensionDeclaration,
    ) {
        let target = self.type_expression_variable(declaration.target_type);
        let Some(target_type) = self.commit_variable_type(environment, target) else {
            return;
        };
        let target = self.get_type(target_type);
        let Some(target_symbol) = self.type_nominal_symbol(&target) else {
            return;
        };
        let form = if target_symbol.module_id == self.input.module {
            dir::ExtensionForm::Inherent
        } else if declaration.name.is_some() {
            dir::ExtensionForm::Named
        } else {
            dir::ExtensionForm::Local
        };
        let extension = dir::Extension::new(symbol, form, target_symbol, target_type);

        self.output.extensions.insert_extension(extension);
    }

    /// Return the nominal symbol named by one committed type.
    fn type_nominal_symbol(&self, ty: &dir::Type) -> Option<dir::GlobalSymbolId> {
        match ty {
            dir::Type::Form(form) => {
                let ty = self.get_type(form.value);

                self.type_nominal_symbol(&ty)
            }
            dir::Type::Named(named) => Some(named.symbol),
            _ => None,
        }
    }

    /// Build static argument terms for one committed heritage clause.
    fn build_generic_argument_terms(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> Vec<ArgumentTerm> {
        let mut terms = Vec::with_capacity(arguments.len());

        // build terms from explicit argument syntax
        for argument in arguments {
            let argument = self.input.view().get(*argument).clone();
            let term = match argument {
                dir::GenericArgument::Type { value } => {
                    ArgumentTerm::Type(self.type_expression_variable(value))
                }
                dir::GenericArgument::SpreadType { value } => {
                    ArgumentTerm::SpreadType(self.type_expression_variable(value))
                }
                dir::GenericArgument::Value { value } => {
                    ArgumentTerm::Static(self.static_expression_variable(value))
                }
                dir::GenericArgument::SpreadValue { value } => {
                    ArgumentTerm::SpreadStatic(self.static_expression_variable(value))
                }
                dir::GenericArgument::Error => continue,
            };

            terms.push(term);
        }

        terms
    }
}
