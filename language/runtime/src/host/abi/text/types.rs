use crate::host::abi::describe::host_abi_types;

host_abi_types! {
    fn host_abi_types() {
        /// One host text-input type hint payload.
        enum HostTextInputType: i32 {
            /// Plain text entry.
            Text = 0,
            /// Numeric entry.
            Number = 1,
            /// Email entry.
            Email = 2,
            /// URL entry.
            Url = 3,
            /// Password entry.
            Password = 4,
            /// Phone entry.
            Phone = 5,
            /// Search entry.
            Search = 6,
        }

        /// One host text range payload.
        #[derive(Default)]
        struct HostTextInputRange {
            /// The inclusive selection start offset.
            start_offset: u32,
            /// The exclusive selection end offset.
            end_offset: u32,
        }

        /// One host text rectangle payload.
        #[derive(Default)]
        struct HostTextInputRectangle {
            /// The left edge in local logical units.
            x: f64,
            /// The top edge in local logical units.
            y: f64,
            /// The rectangle width in local logical units.
            width: f64,
            /// The rectangle height in local logical units.
            height: f64,
        }

        /// One host text transform payload.
        #[derive(Default)]
        struct HostTextInputTransform2D {
            /// The first-row X coefficient.
            xx: f64,
            /// The first-row Y coefficient.
            xy: f64,
            /// The second-row X coefficient.
            yx: f64,
            /// The second-row Y coefficient.
            yy: f64,
            /// The translation X component.
            tx: f64,
            /// The translation Y component.
            ty: f64,
        }

        /// One host text geometry payload.
        struct HostTextInputGeometry {
            /// The local-to-target transform.
            local_to_target_transform: HostTextInputTransform2D,
            /// The full editor rectangle.
            editor_rectangle: HostTextInputRectangle,
            /// Whether one caret rectangle is present.
            has_caret_rectangle: bool,
            /// The caret rectangle when present.
            caret_rectangle: HostTextInputRectangle,
            /// Whether one composing rectangle is present.
            has_composing_rectangle: bool,
            /// The composing rectangle when present.
            composing_rectangle: HostTextInputRectangle,
        }

        /// One host text session configuration payload.
        struct HostTextInputConfiguration {
            /// The stable runtime text session identifier.
            session_id: u64,
            /// The text input type hint.
            input_type: HostTextInputType,
            /// Whether the session is multiline.
            is_multiline: bool,
            /// Whether the session is secure or password-like.
            is_secure: bool,
        }

        /// One host text state payload.
        struct HostTextInputState {
            /// The current text payload.
            text: string_ref,
            /// The current selection range.
            selection: HostTextInputRange,
            /// Whether one composing range is present.
            has_composing: bool,
            /// The composing range when present.
            composing: HostTextInputRange,
        }

        /// One host text open request payload.
        struct HostTextInputOpenRequest {
            /// The text session configuration.
            configuration: HostTextInputConfiguration,
            /// The initial renderer-owned text state.
            state: HostTextInputState,
        }

        /// One host text close request payload.
        struct HostTextInputCloseRequest {
            /// The stable runtime text session identifier.
            session_id: u64,
        }

        /// One host text-geometry update payload.
        struct HostTextInputGeometryRequest {
            /// The stable runtime text session identifier.
            session_id: u64,
            /// The next renderer-owned geometry payload.
            geometry: HostTextInputGeometry,
        }

        /// One host text-state update payload.
        struct HostTextInputStateRequest {
            /// The stable runtime text session identifier.
            session_id: u64,
            /// The next renderer-owned text state.
            state: HostTextInputState,
        }

        /// One host text ingress event payload.
        struct HostTextInputEvent {
            /// The stable runtime text session identifier.
            session_id: u64,
            /// The current text-session state.
            state: HostTextInputState,
        }
    }
}
