use destack_dir as dir;

/// Return whether one language item names a compiler-evaluated type computation.
pub(in crate::sema) fn is_type_computation(item: dir::LanguageItem) -> bool {
    matches!(
        item,
        dir::LanguageItem::Uppercase
            | dir::LanguageItem::Lowercase
            | dir::LanguageItem::Capitalize
            | dir::LanguageItem::Uncapitalize
            | dir::LanguageItem::NoInfer
            | dir::LanguageItem::Unsigned
            | dir::LanguageItem::Awaited
            | dir::LanguageItem::AccessOf
            | dir::LanguageItem::PlaceOf
            | dir::LanguageItem::WithAccess
    )
}
