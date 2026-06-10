use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{GenericArgument, TypeLiteralTerm, TypeTerm, WalkState};

impl WalkState<'_, '_> {
    /// Walk one runtime argument.
    ///
    /// Example:
    /// ```ds
    /// f(name: value, ...rest)
    /// ```
    pub(in crate::check) fn walk_argument(
        &mut self,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) -> CompilerResult<()> {
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any(), None)? else {
            return Ok(());
        };

        match argument {
            // f(name: value)
            dir::Argument::Named { value, .. } => {
                self.walk_expression(*value, self.tree.get(*value))?;
            }
            // f(label = value)
            dir::Argument::Labeled { value, .. } => {
                self.walk_expression(*value, self.tree.get(*value))?;
            }
            // f(value)
            dir::Argument::Positional { value } => {
                self.walk_expression(*value, self.tree.get(*value))?;
            }
            // f(...value)
            dir::Argument::Spread { value, .. } => {
                self.walk_expression(*value, self.tree.get(*value))?;
            }
            // ignore damaged syntax
            dir::Argument::Error => {}
        };

        Ok(())
    }

    /// Walk generic arguments and return their terms.
    ///
    /// Example:
    /// ```ds
    /// <T, U, comptime Size = 4>
    /// ```
    pub(in crate::check) fn walk_generic_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        let mut terms = SmallVec::new();

        // preserve generic argument order
        for argument in arguments {
            terms.push(self.walk_generic_argument_term(*argument)?);
        }

        Ok(terms)
    }

    /// Walk one generic argument and preserve ambiguous type or static syntax.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn walk_generic_argument_term(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> CompilerResult<GenericArgument> {
        let term = match self.tree.get(id) {
            // <T>
            dir::GenericArgument::Type { value } => {
                self.walk_type_expression(*value, self.tree.get(*value))?;
                let ty = self.node_type_operand(*value)?;
                let r#static = self.static_argument_value_operand(*value)?;

                GenericArgument::type_or_static(id.into_global(self.module), ty, r#static)
            }
            // <...T>
            dir::GenericArgument::SpreadType { value } => {
                self.walk_type_expression(*value, self.tree.get(*value))?;
                let ty = self.node_type_operand(*value)?;
                let r#static = self.static_argument_value_operand(*value)?;

                GenericArgument::spread_type_or_static(id.into_global(self.module), ty, r#static)
            }
            // <type Item = T>
            dir::GenericArgument::AssociatedType { name, value } => {
                self.walk_type_expression(*value, self.tree.get(*value))?;
                GenericArgument::AssociatedType {
                    name: *name,
                    value: self.node_type_operand(*value)?.into(),
                }
            }
            // <C>
            dir::GenericArgument::Value { value } => {
                GenericArgument::Static(self.walk_static_expression_operand(*value)?)
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                GenericArgument::AssociatedConst {
                    name: *name,
                    value: self.walk_static_expression_operand(*value)?,
                }
            }
            // <...C>
            dir::GenericArgument::SpreadValue { value } => {
                GenericArgument::SpreadStatic(self.walk_static_expression_operand(*value)?)
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                let variable = self
                    .check
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Error));

                GenericArgument::Type(variable.into())
            }
        };

        Ok(term)
    }
}
