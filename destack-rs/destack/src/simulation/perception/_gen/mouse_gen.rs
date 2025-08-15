//! destack.simulation.perception.mouse@2025.08.15.1

#![destack::generated(destack.simulation.perception.mouse, file)]

#[destack::generated(MouseButton, Debug, block)]
impl std::fmt::Debug for MouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MouseButton::Left => write!(f, "LEFT"),
            MouseButton::Right => write!(f, "RIGHT"),
            MouseButton::Middle => write!(f, "MIDDLE"),
        }
    }
}
