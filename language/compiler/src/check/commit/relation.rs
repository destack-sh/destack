use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, Condition, GenericArgument};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit declaration relations and extension entries into checked tables.
    pub(super) fn commit_relation_and_extension_tables(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) {
        let declarations = self
            .module(module)
            .parsed
            .tree
            .iter_nodes_of_type::<dir::Declaration>()
            .map(|(id, declaration)| (id, declaration.clone()))
            .collect::<Vec<_>>();

        // write declaration side tables
        for (id, declaration) in declarations {
            let Some(symbol) = self.declaration_symbol(module, id.into_any()) else {
                continue;
            };

            match declaration {
                dir::Declaration::Class(declaration) => {
                    self.commit_extends_relation(
                        environment,
                        module,
                        output,
                        symbol,
                        declaration.extends_expression,
                    );
                    self.commit_implements_relations(
                        environment,
                        module,
                        output,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Struct(declaration) => {
                    self.commit_implements_relations(
                        environment,
                        module,
                        output,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Enum(declaration) => {
                    self.commit_implements_relations(
                        environment,
                        module,
                        output,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Interface(declaration) => {
                    self.commit_interface_extends_relations(
                        environment,
                        module,
                        output,
                        symbol,
                        &declaration.extends,
                    );
                }
                dir::Declaration::Extension(declaration) => {
                    self.commit_extension(environment, module, output, symbol, &declaration);
                    self.commit_implements_relations(
                        environment,
                        module,
                        output,
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
        module: ModuleId,
        output: &mut CheckModuleOutput,
        symbol: dir::GlobalSymbolId,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        let Some(expression) = expression else {
            return;
        };
        let variable = self.intern_node_type_variable(module, expression.into_global_any(module));
        let Some(ty) = self.commit_variable_type(module, output, environment, variable) else {
            return;
        };

        output
            .relations
            .push_extends(symbol, dir::Relation::extends(ty));
    }

    /// Commit interface parent relations.
    fn commit_interface_extends_relations(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        symbol: dir::GlobalSymbolId,
        extends: &[dir::InterfaceHeritage],
    ) {
        for heritage in extends {
            let Some(parent) = self.interface_heritage_type(environment, module, output, heritage)
            else {
                continue;
            };

            output
                .relations
                .push_extends(symbol, dir::Relation::extends(parent));
        }
    }

    /// Commit one interface heritage item.
    fn interface_heritage_type(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        heritage: &dir::InterfaceHeritage,
    ) -> Option<dir::LocalTypeId> {
        let symbol = self.require_interface_heritage_symbol(module, heritage.expression)?;
        let arguments =
            self.build_committed_generic_argument_terms(module, &heritage.generic_arguments);
        let source = heritage.expression.into_global_any(module).local_id;
        let arguments =
            self.commit_argument_terms(module, output, environment, &arguments, source)?;
        let ty = self.commit_named_type(module, output, environment, symbol, arguments, source)?;

        Some(self.commit_intern_type(module, output, ty, source))
    }

    /// Commit implemented interface relations.
    fn commit_implements_relations(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        symbol: dir::GlobalSymbolId,
        implemented: &[dir::LocalNodeId<dir::TypeExpression>],
    ) {
        for implemented in implemented {
            let variable = self.intern_local_node_type_variable(module, *implemented);
            let Some(ty) = self.commit_variable_type(module, output, environment, variable) else {
                continue;
            };

            output
                .relations
                .push_implements(symbol, dir::Relation::implements(ty));
        }
    }

    /// Commit one extension entry.
    fn commit_extension(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        symbol: dir::GlobalSymbolId,
        declaration: &dir::ExtensionDeclaration,
    ) {
        let target = self.intern_local_node_type_variable(module, declaration.target_type);
        let Some(target_type) = self.commit_variable_type(module, output, environment, target)
        else {
            return;
        };
        let target = self.commit_type_value(module, output, target_type);
        let Some(target_symbol) = self.type_nominal_symbol(module, output, &target) else {
            return;
        };
        let form = if target_symbol.module_id == module {
            dir::ExtensionForm::Inherent
        } else if declaration.name.is_some() {
            dir::ExtensionForm::Named
        } else {
            dir::ExtensionForm::Local
        };
        let extension = dir::Extension::new(symbol, form, target_symbol, target_type);

        output.extensions.insert_extension(extension);
    }

    /// Return the nominal symbol named by one committed type.
    fn type_nominal_symbol(
        &self,
        module: ModuleId,
        output: &CheckModuleOutput,
        ty: &dir::Type,
    ) -> Option<dir::GlobalSymbolId> {
        match ty {
            dir::Type::Form(form) => {
                let ty = self.commit_type_value(module, output, form.value);

                self.type_nominal_symbol(module, output, &ty)
            }
            dir::Type::Named(named) => Some(named.symbol),
            _ => None,
        }
    }

    /// Build static argument terms for one committed heritage clause.
    fn build_committed_generic_argument_terms(
        &mut self,
        module: ModuleId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> Vec<GenericArgument> {
        let mut terms = Vec::with_capacity(arguments.len());

        // build terms from explicit argument syntax
        for argument in arguments {
            let argument = self.module(module).view().get(*argument).clone();
            let term = match argument {
                dir::GenericArgument::Type { value } => GenericArgument::Type(
                    self.intern_local_node_type_variable(module, value).into(),
                ),
                dir::GenericArgument::SpreadType { value } => GenericArgument::SpreadType(
                    self.intern_local_node_type_variable(module, value).into(),
                ),
                dir::GenericArgument::AssociatedType { name, value } => {
                    GenericArgument::AssociatedType {
                        name,
                        value: self.intern_local_node_type_variable(module, value).into(),
                    }
                }
                dir::GenericArgument::Value { value } => GenericArgument::Static(
                    self.define_static_expression_variable(module, value, Condition::Always)
                        .into(),
                ),
                dir::GenericArgument::SpreadValue { value } => GenericArgument::SpreadStatic(
                    self.define_static_expression_variable(module, value, Condition::Always)
                        .into(),
                ),
                dir::GenericArgument::AssociatedConst { name, value } => {
                    GenericArgument::AssociatedConst {
                        name,
                        value: self
                            .define_static_expression_variable(module, value, Condition::Always)
                            .into(),
                    }
                }
                dir::GenericArgument::Error => continue,
            };

            terms.push(term);
        }

        terms
    }
}
