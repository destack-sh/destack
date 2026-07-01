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
            dir::ScalarLiteral::RegexString { .. } => self.push_language_type(
                node.module_id,
                node.local_id.into_any(),
                dir::LanguageItem::RegExp,
                Vec::new(),
            ),
            dir::ScalarLiteral::Null => {
                self.push_type(node.module_id, dir::Type::Null, node.local_id.into_any())
            }
            dir::ScalarLiteral::Undefined => self.push_type(
                node.module_id,
                dir::Type::Undefined,
                node.local_id.into_any(),
            ),
            value => self.push_type(
                node.module_id,
                dir::Type::Literal(value),
                node.local_id.into_any(),
            ),
        }
    }
}
