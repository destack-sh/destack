use crate::Compiler;
use dyst_ast as ast;
use dyst_dir::{
    Argument, BindingKind, BindingModifier, BindingOperator, Expression, Mutability, NodeId,
    Parameter, Path, Visibility,
};
use dyst_source::{FileId, smallvec};

impl<'a> Compiler<'a> {
    /// Lower a binding modifiers into a DIR binding modifiers.
    pub fn lower_binding_modifiers(
        &mut self,
        _file_id: FileId,
        _ast: &ast::NodeTree,
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
        file_id: FileId,
        ast: &ast::NodeTree,
        parameter_id: ast::NodeId<ast::Parameter>,
    ) -> NodeId<Parameter> {
        let parameter = ast.get(parameter_id);
        match parameter {
            ast::Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = self.intern_string(file_id, *name);
                let ty = ty.map(|ty| self.lower_expression_to_type(file_id, ast, ty));
                let default = default.map(|default| self.lower_expression(file_id, ast, default));
                self.tree.insert_from_ast(
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default,
                    },
                    file_id,
                    parameter_id,
                )
            }
            ast::Parameter::Pattern {
                modifiers,
                pattern,
                ty,
                default,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let pattern = self.lower_pattern(file_id, ast, *pattern);
                let ty = ty.map(|ty| self.lower_expression_to_type(file_id, ast, ty));
                let default = default.map(|default| self.lower_expression(file_id, ast, default));
                self.tree.insert_from_ast(
                    Parameter::Pattern {
                        modifiers,
                        pattern,
                        ty,
                        default,
                    },
                    file_id,
                    parameter_id,
                )
            }
            ast::Parameter::Variadic {
                modifiers,
                name,
                ty,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = self.intern_string(file_id, *name);
                let ty = ty.map(|ty| self.lower_expression_to_type(file_id, ast, ty));
                self.tree.insert_from_ast(
                    Parameter::Variadic {
                        modifiers,
                        name,
                        ty,
                    },
                    file_id,
                    parameter_id,
                )
            }
        }
    }

    /// Lower an argument into a DIR argument.
    pub fn lower_argument(
        &mut self,
        file_id: FileId,
        ast: &ast::NodeTree,
        argument_id: ast::NodeId<ast::Argument>,
    ) -> NodeId<Argument> {
        let argument = ast.get(argument_id);
        match argument {
            ast::Argument::Named {
                modifiers,
                name,
                value,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = self.intern_string(file_id, name.string());
                let value = self.lower_expression(file_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnevaluatedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    file_id,
                    argument_id,
                )
            }
            ast::Argument::Shorthand { modifiers, name } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = self.intern_string(file_id, *name);
                let path = Path::UnevaluatedAbsoluteString {
                    segments: smallvec![name],
                };
                let value =
                    self.tree
                        .insert_from_ast(Expression::Path { path }, file_id, argument_id);
                self.tree.insert_from_ast(
                    Argument::UnevaluatedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    file_id,
                    argument_id,
                )
            }
            ast::Argument::Positional { modifiers, value } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let value = self.lower_expression(file_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnevaluatedPositional { modifiers, value },
                    file_id,
                    argument_id,
                )
            }
            ast::Argument::Spread {
                modifiers,
                name,
                value,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = name.map(|name| self.intern_string(file_id, name));
                let value = self.lower_expression(file_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnevaluatedSpread {
                        modifiers,
                        name,
                        value,
                    },
                    file_id,
                    argument_id,
                )
            }
            ast::Argument::Dynamic {
                modifiers,
                name,
                key,
                value,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(file_id, ast, modifiers));
                let name = name.map(|name| self.intern_string(file_id, name));
                let key = self.lower_expression(file_id, ast, *key);
                let value = self.lower_expression(file_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnevaluatedDynamic {
                        modifiers,
                        name,
                        key,
                        value,
                    },
                    file_id,
                    argument_id,
                )
            }
            _ => todo!("lower_argument({argument:?})"),
        }
    }
}
