use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{CheckState, GenericArgument, Origin, Reduction, TypeTerm, VariableId};

/// Runtime `type T` reflection term.
///
/// ```ts
/// type User
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct TypeValueTerm {
    /// The source type value expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The reflected type.
    pub(in crate::check) ty: VariableId,
}

/// Runtime `import.meta` value term.
///
/// ```ts
/// import.meta
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct ImportMetaTerm {
    /// The source import meta expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
}

/// Runtime contextual receiver term.
///
/// ```ts
/// this
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct ReceiverTerm {
    /// The source receiver expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The receiver syntax kind.
    pub(in crate::check) kind: dir::ReceiverKind,
    /// The receiver type variable.
    pub(in crate::check) ty: VariableId,
}

/// Runtime `super` context term.
///
/// ```ts
/// super
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::check) struct SuperTerm {
    /// The source super expression.
    pub(in crate::check) source: dir::GlobalNodeIdAny,
    /// The visible lexical receiver.
    pub(in crate::check) receiver: Option<VariableId>,
}

impl TypeValueTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        let mut variables = SmallVec::new();
        variables.push(self.ty);
        variables
    }
}

impl ReceiverTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        smallvec::smallvec![self.ty]
    }
}

impl SuperTerm {
    /// Return variables referenced by this term.
    pub(in crate::check) fn referenced_variables(&self) -> SmallVec<[VariableId; 4]> {
        self.receiver.into_iter().collect()
    }
}

impl CheckState<'_> {
    /// Reduce one reflected type value to `Type<T>`.
    pub(in crate::check) fn reduce_type_value_term(
        &mut self,
        module: ModuleId,
        value: &TypeValueTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let symbol = self.language_symbol(module, dir::LanguageItem::Type);
        let argument = GenericArgument::Type(value.ty.into());
        let term = TypeTerm::Reference {
            origin: Origin::Node(value.source),
            symbol,
            arguments: vec![argument].into(),
        };

        Ok(Reduction::value(term))
    }

    /// Reduce `import.meta` to the builtin import meta interface.
    pub(in crate::check) fn reduce_import_meta_term(
        &self,
        module: ModuleId,
        meta: &ImportMetaTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let symbol = self.language_symbol(module, dir::LanguageItem::ImportMeta);

        Ok(Reduction::value(TypeTerm::Reference {
            origin: Origin::Node(meta.source),
            symbol,
            arguments: Vec::new().into(),
        }))
    }

    /// Reduce one contextual receiver to its selected receiver type.
    pub(in crate::check) fn reduce_receiver_term(
        &self,
        term: &ReceiverTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        let ty = TypeTerm::Variable(term.ty);

        Ok(Reduction::value(ty))
    }

    /// Reduce `super` when class inheritance context is known.
    pub(in crate::check) fn reduce_super_term(
        &mut self,
        _module: ModuleId,
        _term: &SuperTerm,
    ) -> CompilerResult<Reduction<TypeTerm>> {
        Ok(Reduction::pending())
    }
}
