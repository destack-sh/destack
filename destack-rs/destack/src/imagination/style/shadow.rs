//! destack.imagination.style.shadow@2025.08.15.1

#![destack::partial(destack.imagination.style.shadow, file)]

#[destack::generated(Shadow, struct, block)]
/// A shadow value.
pub struct Shadow {

}

#[destack::generated(ShadowType, enum, block)]
/// Built-in shadow types.
pub enum ShadowType {
    /// A box shadow
    BOX = 10,
    /// A realistic shadow
    REALISTIC = 11
}

#[destack::generated(ShadowPosition, enum, block)]
/// Built-in shadow positions.
pub enum ShadowPosition {
    /// An outside shadow
    OUTSIDE = 1,
    /// An inside shadow
    INSIDE = 2
}