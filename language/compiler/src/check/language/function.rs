use destack_dir as dir;

const TYPE_ARGUMENTS: &[IntrinsicFunctionArgument] = &[IntrinsicFunctionArgument::Type];
const TYPE_TYPE_ARGUMENTS: &[IntrinsicFunctionArgument] = &[
    IntrinsicFunctionArgument::Type,
    IntrinsicFunctionArgument::Type,
];
const TYPE_STATIC_ARGUMENTS: &[IntrinsicFunctionArgument] = &[
    IntrinsicFunctionArgument::Type,
    IntrinsicFunctionArgument::Static,
];

/// Solver signature for one compiler-recognized intrinsic language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct IntrinsicFunctionSignature {
    /// The solver space produced by the intrinsic.
    pub(in crate::check) result: IntrinsicFunctionResult,
    /// The solver spaces required by intrinsic arguments.
    pub(in crate::check) arguments: &'static [IntrinsicFunctionArgument],
}

impl IntrinsicFunctionSignature {
    /// Create a type-producing intrinsic signature.
    const fn ty(arguments: &'static [IntrinsicFunctionArgument]) -> Self {
        Self {
            result: IntrinsicFunctionResult::Type,
            arguments,
        }
    }

    /// Create a static-producing intrinsic signature.
    const fn static_value(arguments: &'static [IntrinsicFunctionArgument]) -> Self {
        Self {
            result: IntrinsicFunctionResult::Static,
            arguments,
        }
    }
}

/// Solver space produced by one compiler-recognized intrinsic language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IntrinsicFunctionResult {
    /// The intrinsic produces a type.
    Type,
    /// The intrinsic produces a static value.
    Static,
}

/// Solver space required by one compiler-recognized intrinsic language item argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IntrinsicFunctionArgument {
    /// The argument is a type.
    Type,
    /// The argument is a static value.
    Static,
}

/// Return the solver signature for one compiler-recognized intrinsic language item.
pub(in crate::check) fn intrinsic_function_signature(
    item: dir::LanguageItem,
) -> Option<IntrinsicFunctionSignature> {
    let signature = match item {
        // PayloadOf<T>
        dir::LanguageItem::PayloadOf => IntrinsicFunctionSignature::ty(TYPE_ARGUMENTS),

        // BaseOf<T>
        dir::LanguageItem::BaseOf => IntrinsicFunctionSignature::ty(TYPE_ARGUMENTS),

        // WithBase<T, Base>
        dir::LanguageItem::WithBase => IntrinsicFunctionSignature::ty(TYPE_TYPE_ARGUMENTS),

        // WithOwnership<T, O>
        dir::LanguageItem::WithOwnership => IntrinsicFunctionSignature::ty(TYPE_STATIC_ARGUMENTS),

        // WithPlace<T, P>
        dir::LanguageItem::WithPlace => IntrinsicFunctionSignature::ty(TYPE_STATIC_ARGUMENTS),

        // WithSpace<T, S>
        dir::LanguageItem::WithSpace => IntrinsicFunctionSignature::ty(TYPE_STATIC_ARGUMENTS),

        // WithLifetime<T, L>
        dir::LanguageItem::WithLifetime => IntrinsicFunctionSignature::ty(TYPE_STATIC_ARGUMENTS),

        // WithAccess<T, A>
        dir::LanguageItem::WithAccess => IntrinsicFunctionSignature::ty(TYPE_STATIC_ARGUMENTS),

        // OwnershipOf<T>
        dir::LanguageItem::OwnershipOf => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // OwnershipOr<T, O>
        dir::LanguageItem::OwnershipOr => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        // PlaceOf<T>
        dir::LanguageItem::PlaceOf => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // PlaceOr<T, P>
        dir::LanguageItem::PlaceOr => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        // PlaceIn<T, P>
        dir::LanguageItem::PlaceIn => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        // SpaceOf<T>
        dir::LanguageItem::SpaceOf => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // SpaceOr<T, S>
        dir::LanguageItem::SpaceOr => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        // LifetimeOf<T>
        dir::LanguageItem::LifetimeOf => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // LifetimeOr<T, L>
        dir::LanguageItem::LifetimeOr => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        // AccessOf<T>
        dir::LanguageItem::AccessOf => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // AccessOr<T, A>
        dir::LanguageItem::AccessOr => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        // IsManaged<T>
        dir::LanguageItem::IsManaged => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // IsOwned<T>
        dir::LanguageItem::IsOwned => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // IsBorrowed<T>
        dir::LanguageItem::IsBorrowed => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // IsRaw<T>
        dir::LanguageItem::IsRaw => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // IsShared<T>
        dir::LanguageItem::IsShared => IntrinsicFunctionSignature::static_value(TYPE_ARGUMENTS),

        // IsSharedIn<T, S>
        dir::LanguageItem::IsSharedIn => {
            IntrinsicFunctionSignature::static_value(TYPE_STATIC_ARGUMENTS)
        }

        _ => return None,
    };

    Some(signature)
}
