use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, GenericArgument, Origin, TypeOperand, TypeTerm, VariableId};

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

impl TypeValueTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(
        &self,
        state: &CheckState<'_>,
    ) -> SmallVec<[VariableId; 2]> {
        let mut variables = SmallVec::new();
        variables.extend(self.ty.referenced_variables(state));

        variables
    }
}

impl CheckState<'_> {
    /// Reduce one reflected type value to `Type<T>`.
    pub(in crate::check) fn reduce_type_value_term(
        &mut self,
        module: ModuleId,
        value: &TypeValueTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let symbol = self.language_symbol(module, dir::LanguageItem::Type);
        let argument = GenericArgument::Type(value.ty);

        let term = TypeTerm::Reference {
            origin: Origin::Node(value.source),
            symbol,
            arguments: vec![argument].into(),
        };

        Ok(Some(term))
    }

    /// Reduce `import.meta` to the builtin import meta interface.
    pub(in crate::check) fn reduce_import_meta_term(
        &self,
        module: ModuleId,
        meta: &ImportMetaTerm,
    ) -> CompilerResult<Option<TypeTerm>> {
        let symbol = self.language_symbol(module, dir::LanguageItem::ImportMeta);

        Ok(Some(TypeTerm::Reference {
            origin: Origin::Node(meta.source),
            symbol,
            arguments: Vec::new().into(),
        }))
    }
}
