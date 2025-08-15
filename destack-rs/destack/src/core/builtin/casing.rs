//! destack.core.builtin.casing@2025.08.15.1

#![destack::partial(destack.core.builtin.casing, file)]

#[destack::generated(StringCasing, enum, block)]
/// StringCasing
pub enum StringCasing {
    SNAKE = 1,
    UPPER_CAMEL = 2,
    LOWER_CAMEL = 3,
    ALL_CAPS = 4
}