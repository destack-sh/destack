use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Argument, BindingKind, BindingModifier, BindingOperator, Expression, Module, Mutability,
    NodeId, Parameter, Path, Visibility,
};
use dyst_source::smallvec;

impl<'a> Compiler<'a> {
    /// Lower a binding modifiers into a DIR binding modifiers.
    pub fn lower_binding_modifier(
        &mut self,
        _module: &Module,
        modifiers: ast::BindingModifier,
    ) -> BindingModifier {
        let kind = modifiers.kind.map(|kind| match kind {
            ast::BindingKind::Must => BindingKind::Must,
            ast::BindingKind::Maybe => BindingKind::Maybe,
        });
        let mutability = modifiers.mutability.map(|mutability| match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        });
        let visibility = modifiers.visibility.map(|visibility| match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Protected => Visibility::Protected,
            ast::Visibility::Private => Visibility::Private,
        });
        let operator = modifiers.operator.map(|operator| match operator {
            ast::BindingOperator::AsConst => BindingOperator::AsConst,
        });
        BindingModifier {
            kind,
            mutability,
            visibility,
            operator,
        }
    }

    /// Lower a parameter into a DIR parameter.
    pub fn lower_parameter(
        &mut self,
        module: &Module,
        parameter_id: ast::NodeId<ast::Parameter>,
    ) -> NodeId<Parameter> {
        let parameter = module.get(parameter_id);
        match parameter {
            ast::Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let name = self.session.strings.intern_from(&module.strings, *name);
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let default = default.map(|default| self.lower_expression(module, default));
                self.session.tree.insert_from_ast(
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default,
                    },
                    module.id,
                    parameter_id,
                )
            }
            ast::Parameter::Pattern {
                modifiers,
                pattern,
                ty,
                default,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let pattern = self.lower_pattern(module, *pattern);
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                let default = default.map(|default| self.lower_expression(module, default));
                self.session.tree.insert_from_ast(
                    Parameter::Pattern {
                        modifiers,
                        pattern,
                        ty,
                        default,
                    },
                    module.id,
                    parameter_id,
                )
            }
            ast::Parameter::Variadic {
                modifiers,
                name,
                ty,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let name = self.session.strings.intern_from(&module.strings, *name);
                let ty = ty.map(|ty| self.lower_expression_to_type(module, ty));
                self.session.tree.insert_from_ast(
                    Parameter::Variadic {
                        modifiers,
                        name,
                        ty,
                    },
                    module.id,
                    parameter_id,
                )
            }
        }
    }

    /// Lower an argument into a DIR argument.
    pub fn lower_argument(
        &mut self,
        module: &Module,
        argument_id: ast::NodeId<ast::Argument>,
    ) -> NodeId<Argument> {
        let argument = module.get(argument_id);
        match argument {
            ast::Argument::Named {
                modifiers,
                name,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let name = self
                    .session
                    .strings
                    .intern_from(&module.strings, name.string());
                let value = self.lower_expression(module, *value);
                self.session.tree.insert_from_ast(
                    Argument::UnevaluatedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    module.id,
                    argument_id,
                )
            }
            ast::Argument::Shorthand { modifiers, name } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let name = self.session.strings.intern_from(&module.strings, *name);
                let path = Path::UnevaluatedAbsoluteString {
                    segments: smallvec![name],
                };
                let value = self.session.tree.insert_from_ast(
                    Expression::Path {
                        path,
                        static_arguments: None,
                    },
                    module.id,
                    argument_id,
                );
                self.session.tree.insert_from_ast(
                    Argument::UnevaluatedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    module.id,
                    argument_id,
                )
            }
            ast::Argument::Positional { modifiers, value } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let value = self.lower_expression(module, *value);
                self.session.tree.insert_from_ast(
                    Argument::UnevaluatedPositional { modifiers, value },
                    module.id,
                    argument_id,
                )
            }
            ast::Argument::Spread {
                modifiers,
                name,
                value,
            } => {
                let modifiers =
                    modifiers.map(|modifiers| self.lower_binding_modifier(module, modifiers));
                let name = name.map(|name| self.session.strings.intern_from(&module.strings, name));
                let value = self.lower_expression(module, *value);
                self.session.tree.insert_from_ast(
                    Argument::UnevaluatedSpread {
                        modifiers,
                        name,
                        value,
                    },
                    module.id,
                    argument_id,
                )
            }
        }
    }
}
