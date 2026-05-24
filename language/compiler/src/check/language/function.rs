use destack_dir as dir;

/// One type argument.
const TYPE_ARGUMENTS: &[IntrinsicFunctionValue] = &[IntrinsicFunctionValue::Type];

/// Two type arguments.
const TYPE_TYPE_ARGUMENTS: &[IntrinsicFunctionValue] =
    &[IntrinsicFunctionValue::Type, IntrinsicFunctionValue::Type];

/// One type argument followed by one static argument.
const TYPE_STATIC_ARGUMENTS: &[IntrinsicFunctionValue] =
    &[IntrinsicFunctionValue::Type, IntrinsicFunctionValue::Static];

/// Signature for one compiler-recognized intrinsic function language item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct IntrinsicFunctionSignature {
    /// The value produced by the intrinsic function.
    pub(in crate::check) result: IntrinsicFunctionValue,
    /// The values required by intrinsic function arguments.
    pub(in crate::check) arguments: &'static [IntrinsicFunctionValue],
}

impl IntrinsicFunctionSignature {
    /// Create a type-producing intrinsic function signature.
    const fn ty(arguments: &'static [IntrinsicFunctionValue]) -> Self {
        Self {
            result: IntrinsicFunctionValue::Type,
            arguments,
        }
    }

    /// Create a static-producing intrinsic function signature.
    const fn static_value(arguments: &'static [IntrinsicFunctionValue]) -> Self {
        Self {
            result: IntrinsicFunctionValue::Static,
            arguments,
        }
    }
}

/// Type or static value slot in an intrinsic function signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum IntrinsicFunctionValue {
    /// Type-level value.
    Type,
    /// Static-level value.
    Static,
}

/// Return the signature for one compiler-recognized intrinsic function language item.
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
