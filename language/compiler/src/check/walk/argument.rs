use destack_dir as dir;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::WalkState;

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
        let Some(_guard) = self.enter_decorated_static_guard(id.into_any())? else {
            return Ok(());
        };

        // the argument node carries its value's type for selection
        let value = match argument {
            // f(name: value), f(label = value), f(value), f(...value)
            dir::Argument::Named { value, .. }
            | dir::Argument::Labeled { value, .. }
            | dir::Argument::Positional { value }
            | dir::Argument::Spread { value, .. } => Some(*value),
            // ignore damaged syntax
            dir::Argument::Error => None,
        };

        let ty = match value {
            Some(value) => {
                self.walk_expression(value, self.tree.get(value))?;

                self.node_type(value)?
            }
            None => self.push_type(dir::Type::Error, id.into_any())?,
        };
        self.declare_node_type(id, ty)?;

        Ok(())
    }

    /// Walk generic arguments into named or positional applied types.
    ///
    /// Example:
    /// ```ds
    /// <T, U, comptime Size = 4>
    /// ```
    pub(in crate::check) fn walk_generic_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<SmallVec<[(Option<dir::StringId>, dir::GlobalTypeId); 2]>> {
        let mut applied = SmallVec::new();

        // preserve generic argument order
        for argument in arguments {
            applied.push(self.walk_generic_argument(*argument)?);
        }

        Ok(applied)
    }

    /// Walk one generic argument into its applied type.
    /// Static types lower to singleton and operation types directly.
    ///
    /// Example:
    /// ```ds
    /// <T>
    /// ```
    fn walk_generic_argument(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> CompilerResult<(Option<dir::StringId>, dir::GlobalTypeId)> {
        match self.tree.get(id) {
            // <T> and <...T>
            dir::GenericArgument::Type { value } | dir::GenericArgument::SpreadType { value } => {
                let value = *value;

                Ok((None, self.walk_type_expression(value)?))
            }
            // <type Item = T>
            dir::GenericArgument::AssociatedType { name, value } => {
                let (name, value) = (*name, *value);

                Ok((Some(name), self.walk_type_expression(value)?))
            }
            // <C> and <...C>
            dir::GenericArgument::Value { value } | dir::GenericArgument::SpreadValue { value } => {
                let value = *value;

                Ok((None, self.lower_static_predicate(value)?))
            }
            // <comptime Size = N>
            dir::GenericArgument::AssociatedConst { name, value } => {
                let (name, value) = (*name, *value);

                Ok((Some(name), self.lower_static_predicate(value)?))
            }
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => {
                Ok((None, self.push_type(dir::Type::Error, id.into_any())?))
            }
        }
    }
}
