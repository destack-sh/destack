//! destack.core.builtin.casing@2025.08.15.1

#![destack::partial(destack.core.builtin.casing, file)]

#[destack::generated(StringCasing, enum, block)]
/// StringCasing
pub enum StringCasing {
    Snake = 1,
    UpperCamel = 2,
    LowerCamel = 3,
    AllCaps = 4,
}
