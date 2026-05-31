use destack_dir as dir;

use crate::check::WalkState;

impl WalkState<'_, '_> {
    /// Return whether one language item is a type returning memory intrinsic.
    pub(in crate::check) fn is_memory_type_intrinsic(item: dir::LanguageItem) -> bool {
        matches!(
            item,
            dir::LanguageItem::PayloadOf
                | dir::LanguageItem::BaseOf
                | dir::LanguageItem::WithBase
                | dir::LanguageItem::WithPlace
                | dir::LanguageItem::WithSpace
                | dir::LanguageItem::WithLifetime
                | dir::LanguageItem::WithAccess
                | dir::LanguageItem::WithOwnership
        )
    }

    /// Return whether one language item is a static returning memory intrinsic.
    pub(in crate::check) fn is_memory_static_intrinsic(item: dir::LanguageItem) -> bool {
        matches!(
            item,
            dir::LanguageItem::OwnershipOf
                | dir::LanguageItem::OwnershipOr
                | dir::LanguageItem::PlaceOf
                | dir::LanguageItem::PlaceOr
                | dir::LanguageItem::PlaceIn
                | dir::LanguageItem::SpaceOf
                | dir::LanguageItem::SpaceOr
                | dir::LanguageItem::LifetimeOf
                | dir::LanguageItem::LifetimeOr
                | dir::LanguageItem::AccessOf
                | dir::LanguageItem::AccessOr
                | dir::LanguageItem::IsManaged
                | dir::LanguageItem::IsOwned
                | dir::LanguageItem::IsBorrowed
                | dir::LanguageItem::IsRaw
                | dir::LanguageItem::IsShared
                | dir::LanguageItem::IsSharedIn
        )
    }
}
