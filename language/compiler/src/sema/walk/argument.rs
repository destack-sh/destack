use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::WalkState;

/// One generic argument after walking its type or static value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct GenericArgument {
    /// The argument node.
    pub(in crate::sema) id: dir::LocalNodeId<dir::GenericArgument>,
    /// The associated argument name, when written.
    pub(in crate::sema) name: Option<dir::StringId>,
    /// The argument type or singleton static term.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The source node that produced the argument value.
    pub(in crate::sema) source: dir::GlobalNodeIdAny,
}

impl WalkState<'_, '_> {
    /// Walk one runtime argument.
    ///
    /// Example:
    /// ```tspp
    /// f(name: value, ...rest)
    /// ```
    pub(in crate::sema) fn walk_argument(
        &mut self,
        id: dir::LocalNodeId<dir::Argument>,
        argument: &dir::Argument,
    ) -> CompilerResult<()> {
        if !self.walk_decorators(id.into_any())? {
            return Ok(());
        }

        let Some(value) = argument.value() else {
            let error = self.intern_type(dir::Type::Error)?;
            self.commit_node_type(id, error)?;

            return Ok(());
        };

        self.walk_expression(value, self.tree.get(value))?;

        Ok(())
    }

    /// Walk generic arguments into named or positional applied types.
    ///
    /// Example:
    /// ```tspp
    /// <T, U, const Size = 4>
    /// ```
    pub(in crate::sema) fn walk_generic_arguments(
        &mut self,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<SmallVec<[GenericArgument; 2]>> {
        let mut applied = SmallVec::new();

        // preserve generic argument order
        for argument in arguments {
            applied.push(self.walk_generic_argument(*argument)?);
        }

        Ok(applied)
    }

    /// Walk one generic argument into its applied type.
    /// Static values become singleton and operation types directly.
    ///
    /// Example:
    /// ```tspp
    /// <T>
    /// ```
    fn walk_generic_argument(
        &mut self,
        id: dir::LocalNodeId<dir::GenericArgument>,
    ) -> CompilerResult<GenericArgument> {
        let (name, ty, source) = match self.tree.get(id) {
            // <T> and <...T>
            dir::GenericArgument::Type { value } | dir::GenericArgument::SpreadType { value } => (
                None,
                self.walk_argument_type_expression(*value)?,
                value.into_global_any(self.module),
            ),
            // <type Item = T> and <const Size = N>
            dir::GenericArgument::AssociatedType { name, value }
            | dir::GenericArgument::AssociatedConst { name, value } => (
                Some(*name),
                self.walk_argument_type_expression(*value)?,
                value.into_global_any(self.module),
            ),
            // keep the argument arity visible to solve
            dir::GenericArgument::Error => (
                None,
                self.intern_type(dir::Type::Error)?,
                id.into_global_any(self.module),
            ),
        };
        self.commit_node_type(id, ty)?;

        Ok(GenericArgument {
            id,
            name,
            ty,
            source,
        })
    }
}
