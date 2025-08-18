//! destack.simulation.perception.mouse

#![destack::partial(destack.simulation.perception.mouse, file)]

#[destack::generated(MouseButton, -, block)]
/// A MouseButton is a button on a mouse.
pub enum MouseButton {
    /// Left button
    Left = 1,
    /// Right button
    Right = 2,
    /// Middle button
    Middle = 3,
}
