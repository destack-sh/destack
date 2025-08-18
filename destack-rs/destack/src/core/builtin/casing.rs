//! destack.core.builtin.casing

#![destack::partial(destack.core.builtin.casing, file)]

#[destack::generated(StringCasing, -, block)]
/// StringCasing
pub enum StringCasing {
    Snake = 1,
    UpperCamel = 2,
    LowerCamel = 3,
    AllCaps = 4,
}
