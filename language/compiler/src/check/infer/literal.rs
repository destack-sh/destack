use destack_dir as dir;

use crate::CompilerResult;
use crate::check::CheckState;

/// Inference mode for literal-preserving expression contexts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum InferMode {
    /// Infer ordinary source expression types.
    Normal,
    /// Infer under `as const` literal-preserving rules.
    Const,
}

impl InferMode {
    /// Return whether object fields inferred in this mode are readonly.
    pub(in crate::check) fn is_readonly(self) -> bool {
        matches!(self, Self::Const)
    }
}

impl CheckState<'_> {
    /// Return the type of one scalar literal expression.
    pub(in crate::check) fn scalar_literal_type(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        value: dir::ScalarLiteral,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            dir::ScalarLiteral::RegexString { .. } => {
                self.language_type(node.module_id, dir::LanguageItem::RegExp, &[])
            }
            dir::ScalarLiteral::Null => self.intern_type(node.module_id, dir::Type::Null),
            dir::ScalarLiteral::Undefined => self.intern_type(node.module_id, dir::Type::Undefined),
            value => self.intern_type(node.module_id, dir::Type::Literal(value)),
        }
    }
}
