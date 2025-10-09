use dyst_ast as ast;
use dyst_dir::{Expression, Mutability, NodeId, Runtime, ScopedMutability, Visibility};
use dyst_source::SourceId;

use crate::Compiler;

impl<'a> Compiler<'a> {
    /// Lower visibility into a DIR visibility.
    #[inline]
    pub fn lower_visibility(&self, visibility: ast::Visibility) -> Visibility {
        match visibility {
            ast::Visibility::Public => Visibility::Public,
            ast::Visibility::Private => Visibility::Private,
        }
    }

    /// Lower runtime into a DIR runtime.
    #[inline]
    pub fn lower_runtime(&self, runtime: ast::Runtime) -> Runtime {
        match runtime {
            ast::Runtime::Dynamic => Runtime::Dynamic,
            ast::Runtime::Static => Runtime::Static,
        }
    }

    /// Lower mutability into a DIR mutability.
    #[inline]
    pub fn lower_mutability(&self, mutability: ast::Mutability) -> Mutability {
        match mutability {
            ast::Mutability::Immutable => Mutability::Immutable,
            ast::Mutability::Mutable => Mutability::Mutable,
        }
    }

    /// Lower scoped mutability into a DIR scoped mutability.
    #[inline]
    pub fn lower_scoped_mutability(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        scoped_mutability: ast::ScopedMutability,
    ) -> ScopedMutability {
        match scoped_mutability {
            ast::ScopedMutability::Unscoped { mutability } => ScopedMutability::Unscoped {
                mutability: self.lower_mutability(mutability),
            },
            ast::ScopedMutability::Scoped { mutability, scopes } => ScopedMutability::Scoped {
                mutability: self.lower_mutability(mutability),
                scopes: scopes
                    .iter()
                    .map(|scope| self.lower_path(source_id, ast, scope))
                    .collect(),
            },
        }
    }

    /// Lower an expression to a DIR expression.
    pub fn lower_expression(
        &mut self,
        source_id: SourceId,
        ast: &ast::NodeTree,
        expression_id: ast::NodeId<ast::Expression>,
    ) -> NodeId<Expression> {
        todo!("Compiler::lower_expression");
    }
}
