use crate::{Compiler, ResolveResult};

use dyst_dir::{Annotation, Argument, Expression, ModuleId, NodeId, Type};

/// Task to statically resolve something in-place.
#[derive(Debug, Clone)]
pub enum ResolveTask {
    /// Resolve an Expression fully (in-place).
    ResolveExpression {
        module_id: ModuleId,
        expression: NodeId<Expression>,
    },
    /// Resolve a Type to its Type value (in-place).
    ResolveType {
        module_id: ModuleId,
        ty: NodeId<Type>,
    },
    /// Resolve an Argument (in-place).
    ResolveArgument {
        module_id: ModuleId,
        argument: NodeId<Argument>,
    },
    /// Resolve an Annotation fully (in-place).
    ResolveAnnotation {
        module_id: ModuleId,
        annotation: NodeId<Annotation>,
    },
}

impl<'a> Compiler<'a> {
    /// Resolve a node.
    pub fn process_resolve(&mut self, task: ResolveTask) -> ResolveResult<()> {
        todo!("process_resolve({task:?})")
    }
}
