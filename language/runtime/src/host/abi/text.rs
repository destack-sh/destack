use crate::platform::input::{InputTextSessionConfig, InputTextSessionStateValue};
use crate::runtime::NativeStringRef;

/// One host text range payload.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct HostTextRange {
    /// The inclusive selection start offset.
    pub start_offset: u32,
    /// The exclusive selection end offset.
    pub end_offset: u32,
}

/// One host text rectangle payload.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct HostTextRectangle {
    /// The left edge in local logical units.
    pub x: f64,
    /// The top edge in local logical units.
    pub y: f64,
    /// The rectangle width in local logical units.
    pub width: f64,
    /// The rectangle height in local logical units.
    pub height: f64,
}

/// One host text transform payload.
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub(crate) struct HostTextTransform2D {
    /// The first-row X coefficient.
    pub xx: f64,
    /// The first-row Y coefficient.
    pub xy: f64,
    /// The second-row X coefficient.
    pub yx: f64,
    /// The second-row Y coefficient.
    pub yy: f64,
    /// The translation X component.
    pub tx: f64,
    /// The translation Y component.
    pub ty: f64,
}

/// One host text session configuration payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostTextSessionConfig {
    /// The stable runtime text session identifier.
    pub session_id: u64,
    /// The text input type hint.
    pub input_type: i32,
    /// Whether the session is multiline.
    pub is_multiline: bool,
    /// Whether the session is secure or password-like.
    pub is_secure: bool,
}

/// One host text state payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostTextSessionState {
    /// The current text payload.
    pub text: NativeStringRef,
    /// The current selection range.
    pub selection: HostTextRange,
    /// Whether one composing range is present.
    pub has_composing: bool,
    /// The composing range when present.
    pub composing: HostTextRange,
}

/// One host text open request payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostTextOpenRequest {
    /// The text session configuration.
    pub config: HostTextSessionConfig,
    /// The initial renderer-owned text state.
    pub state: HostTextSessionState,
}

/// One host text-geometry update payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostTextGeometryRequest {
    /// The stable runtime text session identifier.
    pub session_id: u64,
    /// The local-to-target transform.
    pub local_to_target_transform: HostTextTransform2D,
    /// The full editor rectangle.
    pub editor_rectangle: HostTextRectangle,
    /// Whether one caret rectangle is present.
    pub has_caret_rectangle: bool,
    /// The caret rectangle when present.
    pub caret_rectangle: HostTextRectangle,
    /// Whether one composing rectangle is present.
    pub has_composing_rectangle: bool,
    /// The composing rectangle when present.
    pub composing_rectangle: HostTextRectangle,
}

/// One host text-state update payload.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(crate) struct HostTextStateRequest {
    /// The stable runtime text session identifier.
    pub session_id: u64,
    /// The next renderer-owned text state.
    pub state: HostTextSessionState,
}

/// One owned host text open request payload.
#[derive(Debug)]
pub(crate) struct HostTextOpenRequestPayload {
    /// The owned state text storage.
    text_storage: String,
    /// The borrowed ABI payload.
    abi: HostTextOpenRequest,
}

impl HostTextOpenRequestPayload {
    /// Build one owned host text open request payload.
    pub(crate) fn new(
        session_id: u64,
        config: InputTextSessionConfig,
        state: &InputTextSessionStateValue,
    ) -> Self {
        let text_storage = state.text.clone();
        let abi = HostTextOpenRequest {
            config: HostTextSessionConfig {
                session_id,
                input_type: config.input_type as i32,
                is_multiline: config.is_multiline,
                is_secure: config.is_secure,
            },
            state: host_text_state(&text_storage, state),
        };

        Self { text_storage, abi }
    }

    /// Return the ABI request view.
    pub(crate) fn abi(&self) -> HostTextOpenRequest {
        let _ = &self.text_storage;

        self.abi
    }
}

/// One owned host text-state update payload.
#[derive(Debug)]
pub(crate) struct HostTextStateRequestPayload {
    /// The owned state text storage.
    text_storage: String,
    /// The borrowed ABI payload.
    abi: HostTextStateRequest,
}

impl HostTextStateRequestPayload {
    /// Build one owned host text-state update payload.
    pub(crate) fn new(session_id: u64, state: &InputTextSessionStateValue) -> Self {
        let text_storage = state.text.clone();
        let abi = HostTextStateRequest {
            session_id,
            state: host_text_state(&text_storage, state),
        };

        Self { text_storage, abi }
    }

    /// Return the ABI request view.
    pub(crate) fn abi(&self) -> HostTextStateRequest {
        let _ = &self.text_storage;

        self.abi
    }
}

/// Convert one runtime text state into one host payload.
fn host_text_state(text: &str, state: &InputTextSessionStateValue) -> HostTextSessionState {
    let composing = if let Some(composing) = state.composing {
        host_text_range(composing)
    } else {
        HostTextRange::default()
    };

    HostTextSessionState {
        text: NativeStringRef::from(text),
        selection: host_text_range(state.selection),
        has_composing: state.composing.is_some(),
        composing,
    }
}

/// Convert one runtime text range into one host payload.
fn host_text_range(range: crate::platform::input::InputTextRange) -> HostTextRange {
    HostTextRange {
        start_offset: range.start_offset,
        end_offset: range.end_offset,
    }
}

/// Convert one runtime text rectangle into one host payload.
pub(crate) fn host_text_rectangle(
    rectangle: crate::platform::input::InputTextRectangle,
) -> HostTextRectangle {
    HostTextRectangle {
        x: rectangle.x,
        y: rectangle.y,
        width: rectangle.width,
        height: rectangle.height,
    }
}

/// Convert one runtime text transform into one host payload.
pub(crate) fn host_text_transform(
    transform: crate::platform::input::InputTextTransform2D,
) -> HostTextTransform2D {
    HostTextTransform2D {
        xx: transform.xx,
        xy: transform.xy,
        yx: transform.yx,
        yy: transform.yy,
        tx: transform.tx,
        ty: transform.ty,
    }
}

/// Convert one runtime text rectangle into one host payload.
pub(crate) fn host_text_rect(
    rect: crate::platform::input::InputTextRectangle,
) -> HostTextRectangle {
    HostTextRectangle {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}
