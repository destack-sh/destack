use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{Answer, CheckState, GenericArgument, Origin, TypeOperand, TypeTerm};

/// Runtime `type T` reflection term.
///
/// ```ds
/// type User
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TypeValueTerm {
    /// The source type value expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The reflected type.
    pub(in crate::check) ty: TypeOperand,
}

/// Runtime `import.meta` value term.
///
/// ```ds
/// import.meta
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct ImportMetaTerm {
    /// The source import meta expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

impl CheckState<'_> {
    /// Reduce one reflected type value to `Type<T>`.
    pub(in crate::check) fn reduce_type_value_term(
        &mut self,
        value: TypeValueTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let symbol = self.language_symbol(dir::LanguageItem::Type);
        let argument = GenericArgument::Type(value.ty);

        let term = TypeTerm::Reference {
            origin: Origin::Node(value.source),
            symbol,
            arguments: vec![argument].into(),
        };
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
    }

    /// Reduce `import.meta` to the builtin import meta interface.
    pub(in crate::check) fn reduce_import_meta_term(
        &mut self,
        meta: ImportMetaTerm,
    ) -> CompilerResult<Answer<TypeOperand>> {
        let symbol = self.language_symbol(dir::LanguageItem::ImportMeta);
        let term = TypeTerm::Reference {
            origin: Origin::Node(meta.source),
            symbol,
            arguments: Vec::new().into(),
        };
        let operand = self.type_term_operand(term);

        Ok(Answer::Ready(operand))
    }
}
