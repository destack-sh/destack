#[path = "abi.generated.rs"]
mod abi_generated;
#[path = "bindings.generated.rs"]
mod bindings_generated;

#[allow(unused_imports, unreachable_pub)]
pub use abi_generated::*;
#[allow(unused_imports, unreachable_pub)]
pub use bindings_generated::*;

/// Internal event kind used by host backends when constructing input events.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputEventKind {
    /// Key event lane.
    Key,
    /// Pointer-motion event lane.
    PointerMotion,
    /// Pointer-button event lane.
    PointerButton,
    /// Scroll event lane.
    Scroll,
    /// Touch event lane.
    Touch,
    /// Gamepad event lane.
    Gamepad,
    /// Text event lane.
    Text,
    /// Device event lane.
    Device,
    /// Sensor event lane.
    Sensor,
    /// Composition event lane.
    Composition,
}

/// Internal event payload container used by host backends before ABI union projection.
#[derive(Debug, Clone, Copy)]
pub(crate) struct InputEventPayload {
    /// Key payload.
    pub(crate) key: InputKeyEventPayload,
    /// Pointer-motion payload.
    pub(crate) pointer_motion: InputPointerMotionEventPayload,
    /// Pointer-button payload.
    pub(crate) pointer_button: InputPointerButtonEventPayload,
    /// Scroll payload.
    pub(crate) scroll: InputScrollEventPayload,
    /// Touch payload.
    pub(crate) touch: InputTouchEventPayload,
    /// Gamepad payload.
    pub(crate) gamepad: InputGamepadEventPayload,
    /// Text payload.
    pub(crate) text: InputTextEventPayload,
    /// Device payload.
    pub(crate) device: InputDeviceEventPayload,
    /// Sensor payload.
    pub(crate) sensor: InputSensorEventPayload,
    /// Composition payload.
    pub(crate) composition: InputCompositionEventPayload,
}

/// Internal monitor event kind used by host backends when constructing monitor events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InputMonitorEventKind {
    /// Device connected.
    Connect,
    /// Device disconnected.
    Disconnect,
    /// Device metadata changed.
    Change,
}

mod host;
pub mod native;
pub(crate) mod simulation;
mod state;
#[cfg(test)]
mod tests;
pub(crate) mod validation;
pub mod vm;

pub(crate) use state::*;
