use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, GenericArgument};

impl CheckState<'_> {
    /// Commit declaration relations and extension entries into checked tables.
    pub(super) fn commit_relation_and_extension_tables(
        &mut self,
        module: ModuleId,
        environment: &GlobalEnvironment,
    ) {
        let declarations = self
            .input(module)
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
                        symbol,
                        declaration.extends_expression,
                    );
                    self.commit_implements_relations(
                        environment,
                        module,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Struct(declaration) => {
                    self.commit_implements_relations(
                        environment,
                        module,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Enum(declaration) => {
                    self.commit_implements_relations(
                        environment,
                        module,
                        symbol,
                        &declaration.implements_types,
                    );
                }
                dir::Declaration::Interface(declaration) => {
                    self.commit_interface_extends_relations(
                        environment,
                        module,
                        symbol,
                        &declaration.extends,
                    );
                }
                dir::Declaration::Extension(declaration) => {
                    self.commit_extension(environment, module, symbol, &declaration);
                    self.commit_implements_relations(
                        environment,
                        module,
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
        symbol: dir::GlobalSymbolId,
        expression: Option<dir::LocalNodeId<dir::Expression>>,
    ) {
        let Some(expression) = expression else {
            return;
        };
        let variable = self.intern_node_type_variable(module, expression.into_global_any(module));
        let Some(ty) = self.commit_variable_type(environment, variable) else {
            return;
        };

        self.output_mut(module)
            .relations
            .push_extends(symbol, dir::Relation::extends(ty));
    }

    /// Commit interface parent relations.
    fn commit_interface_extends_relations(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        extends: &[dir::InterfaceHeritage],
    ) {
        for heritage in extends {
            let Some(parent) = self.interface_heritage_type(environment, module, heritage) else {
                continue;
            };

            self.output_mut(module)
                .relations
                .push_extends(symbol, dir::Relation::extends(parent));
        }
    }

    /// Commit one interface heritage item.
    fn interface_heritage_type(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        heritage: &dir::InterfaceHeritage,
    ) -> Option<dir::LocalTypeId> {
        let symbol = self.require_interface_heritage_symbol(module, heritage.expression)?;
        let arguments =
            self.build_committed_generic_argument_terms(module, &heritage.generic_arguments);
        let source = heritage.expression.into_global_any(module).local_id;
        let arguments = self.commit_argument_terms(module, environment, &arguments, source)?;
        let ty = self.commit_named_type(module, environment, symbol, arguments, source)?;

        Some(self.intern_type(module, ty, source))
    }

    /// Commit implemented interface relations.
    fn commit_implements_relations(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        implemented: &[dir::LocalNodeId<dir::TypeExpression>],
    ) {
        for implemented in implemented {
            let variable = self.intern_local_type_variable(module, *implemented);
            let Some(ty) = self.commit_variable_type(environment, variable) else {
                continue;
            };

            self.output_mut(module)
                .relations
                .push_implements(symbol, dir::Relation::implements(ty));
        }
    }

    /// Commit one extension entry.
    fn commit_extension(
        &mut self,
        environment: &GlobalEnvironment,
        module: ModuleId,
        symbol: dir::GlobalSymbolId,
        declaration: &dir::ExtensionDeclaration,
    ) {
        let target = self.intern_local_type_variable(module, declaration.target_type);
        let Some(target_type) = self.commit_variable_type(environment, target) else {
            return;
        };
        let target = self.local_type(module, target_type);
        let Some(target_symbol) = self.type_nominal_symbol(module, &target) else {
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

        self.output_mut(module)
            .extensions
            .insert_extension(extension);
    }

    /// Return the nominal symbol named by one committed type.
    fn type_nominal_symbol(&self, module: ModuleId, ty: &dir::Type) -> Option<dir::GlobalSymbolId> {
        match ty {
            dir::Type::Form(form) => {
                let ty = self.local_type(module, form.value);

                self.type_nominal_symbol(module, &ty)
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
            let argument = self.input(module).view().get(*argument).clone();
            let term = match argument {
                dir::GenericArgument::Type { value } => {
                    GenericArgument::Type(self.intern_local_type_variable(module, value).into())
                }
                dir::GenericArgument::SpreadType { value } => GenericArgument::SpreadType(
                    self.intern_local_type_variable(module, value).into(),
                ),
                dir::GenericArgument::Value { value } => GenericArgument::Static(
                    self.define_static_expression_variable(module, value).into(),
                ),
                dir::GenericArgument::SpreadValue { value } => GenericArgument::SpreadStatic(
                    self.define_static_expression_variable(module, value).into(),
                ),
                dir::GenericArgument::Error => continue,
            };

            terms.push(term);
        }

        terms
    }
}
