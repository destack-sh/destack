use crate::Compiler;
use dyst_ast as ast;
use dyst_container::smallvec;
use dyst_dir::{
    Argument, BindingKind, BindingModifiers, DeclarationScope, Expression, Mutability, NodeId, Parameter, Path,
    Visibility,
};
use dyst_source::SourceId;

impl<'a> Compiler<'a> {
    /// Lower a binding modifiers into a DIR binding modifiers.
    pub fn lower_binding_modifiers(
        &mut self,
        _source_id: SourceId,
        _ast: &ast::NodeTree,
        modifiers: ast::BindingModifiers,
    ) -> BindingModifiers {
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
        BindingModifiers {
            kind,
            mutability,
            visibility,
        }
    }

    /// Lower a parameter into a DIR parameter.
    pub fn lower_parameter(
        &mut self,
        source_id: SourceId,
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
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = self.intern_string(source_id, *name);
                let ty = ty.map(|ty| self.lower_expression_to_type(source_id, ast, ty));
                let default = default.map(|default| self.lower_expression(source_id, ast, default));
                self.tree.insert_from_ast(
                    Parameter::Named {
                        modifiers,
                        name,
                        ty,
                        default,
                    },
                    source_id,
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
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let pattern = self.lower_pattern(source_id, ast, *pattern);
                let ty = ty.map(|ty| self.lower_expression_to_type(source_id, ast, ty));
                let default = default.map(|default| self.lower_expression(source_id, ast, default));
                self.tree.insert_from_ast(
                    Parameter::Pattern {
                        modifiers,
                        pattern,
                        ty,
                        default,
                    },
                    source_id,
                    parameter_id,
                )
            }
            ast::Parameter::Variadic {
                modifiers,
                name,
                ty,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = self.intern_string(source_id, *name);
                let ty = ty.map(|ty| self.lower_expression_to_type(source_id, ast, ty));
                self.tree.insert_from_ast(
                    Parameter::Variadic {
                        modifiers,
                        name,
                        ty,
                    },
                    source_id,
                    parameter_id,
                )
            }
        }
    }

    /// Lower an argument into a DIR argument.
    pub fn lower_argument(
        &mut self,
        source_id: SourceId,
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
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = self.intern_string(source_id, name.string());
                let value = self.lower_expression(source_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    source_id,
                    argument_id,
                )
            }
            ast::Argument::Shorthand { modifiers, name } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = self.intern_string(source_id, *name);
                let path = Path::UnresolvedAbsoluteString {
                    segments: smallvec![name],
                };
                let value =
                    self.tree
                        .insert_from_ast(Expression::Path { path }, source_id, argument_id);
                self.tree.insert_from_ast(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    source_id,
                    argument_id,
                )
            }
            ast::Argument::Function {
                modifiers,
                name,
                value,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = self.intern_string(source_id, name.string());
                let value = self.lower_expression(source_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnresolvedNamed {
                        modifiers,
                        name,
                        value,
                    },
                    source_id,
                    argument_id,
                )
            }
            ast::Argument::Positional { modifiers, value } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let value = self.lower_expression(source_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnresolvedPositional { modifiers, value },
                    source_id,
                    argument_id,
                )
            }
            ast::Argument::Spread {
                modifiers,
                name,
                value,
            } => {
                let modifiers = modifiers
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = name.map(|name| self.intern_string(source_id, name));
                let value = self.lower_expression(source_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnresolvedSpread {
                        modifiers,
                        name,
                        value,
                    },
                    source_id,
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
                    .map(|modifiers| self.lower_binding_modifiers(source_id, ast, modifiers));
                let name = name.map(|name| self.intern_string(source_id, name));
                let key = self.lower_expression(source_id, ast, *key);
                let value = self.lower_expression(source_id, ast, *value);
                self.tree.insert_from_ast(
                    Argument::UnresolvedDynamic {
                        modifiers,
                        name,
                        key,
                        value,
                    },
                    source_id,
                    argument_id,
                )
            }
        }
    }
}
