//! destack.simulation.perception.mouse@2025.08.14.0

#![destack::partial(destack.simulation.perception.mouse, file)]

#[destack::generated(MouseButton, enum, block)]
/// A MouseButton is a button on a mouse.
pub enum MouseButton {
    /// Left button
    LEFT = 1,
    /// Right button
    RIGHT = 2,
    /// Middle button
    MIDDLE = 3
}