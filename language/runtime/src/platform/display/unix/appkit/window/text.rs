use std::sync::Arc;

use objc2::runtime::{AnyObject, ProtocolObject, Sel};
use objc2::{DefinedClass, MainThreadOnly, define_class};
use objc2_app_kit::{
    NSControl, NSControlTextEditingDelegate, NSSecureTextField, NSTextDelegate,
    NSTextFieldDelegate, NSTextInputClient, NSTextView, NSTextViewDelegate,
};
use objc2_foundation::{
    NSInteger, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRange, NSRect, NSSize,
    NSString,
};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::display::unix::appkit::core as appkit_core;
use crate::platform::input::host::{
    notify_appkit_window_clipboard_command, notify_appkit_window_edit_intent,
    notify_appkit_window_text_state, resolve_appkit_window_text_session,
};
use crate::platform::input::{
    InputClipboardCommandTypeValue, InputEditIntentTypeValue, InputTextGeometry, InputTextRange,
    InputTextSessionConfig, InputTextSessionStateValue,
};
use crate::platform::{PlatformError, core as core_platform, resource};

/// Retained native text editor payload for one AppKit window.
enum AppKitWindowTextEditor {
    /// One generic hidden text view host.
    TextView(objc2::rc::Retained<NSTextView>),
    /// One secure text field host that routes through the shared field editor.
    SecureField(objc2::rc::Retained<NSSecureTextField>),
}

/// Retained native text host for one AppKit window.
pub(crate) struct AppKitWindowTextHost {
    /// The hidden AppKit editor control.
    editor: AppKitWindowTextEditor,
    /// The delegate retained for callback routing.
    delegate: objc2::rc::Retained<AppKitWindowTextViewDelegate>,
}

impl AppKitWindowTextHost {
    /// Return whether this host uses one secure control lane.
    fn is_secure(&self) -> bool {
        matches!(self.editor, AppKitWindowTextEditor::SecureField(_))
    }

    /// Remove the native host view from its window.
    fn remove_from_superview(&self) {
        match &self.editor {
            AppKitWindowTextEditor::TextView(text_view) => text_view.removeFromSuperview(),
            AppKitWindowTextEditor::SecureField(text_field) => text_field.removeFromSuperview(),
        }
    }

    /// Focus the native editor control for one window.
    fn focus_in_window(&self, window: &objc2_app_kit::NSWindow) {
        match &self.editor {
            AppKitWindowTextEditor::TextView(text_view) => {
                window.makeFirstResponder(Some(text_view.as_ref()));
            }
            AppKitWindowTextEditor::SecureField(text_field) => {
                window.makeFirstResponder(Some(text_field.as_ref()));
            }
        }
    }

    /// Clear the native editor state during session close.
    fn clear(&self) {
        match &self.editor {
            AppKitWindowTextEditor::TextView(text_view) => {
                text_view.setString(&NSString::from_str(""));
                text_view.setSelectedRange(NSRange::from(0..0));
                if text_view.hasMarkedText() {
                    text_view.unmarkText();
                }
            }
            AppKitWindowTextEditor::SecureField(text_field) => {
                text_field.setStringValue(&NSString::from_str(""));

                if let Some(text_view) = secure_text_field_editor(text_field, &self.delegate) {
                    text_view.setSelectedRange(NSRange::from(0..0));

                    if text_view.hasMarkedText() {
                        text_view.unmarkText();
                    }
                }
            }
        }
    }

    /// Apply one renderer geometry hint to the native editor control.
    fn apply_geometry(&self, geometry: Option<InputTextGeometry>) {
        match &self.editor {
            AppKitWindowTextEditor::TextView(text_view) => {
                apply_view_geometry(text_view.as_ref(), geometry);
            }
            AppKitWindowTextEditor::SecureField(text_field) => {
                apply_view_geometry(text_field.as_ref(), geometry);
            }
        }
    }

    /// Apply one renderer-owned text configuration and state snapshot.
    fn synchronize(
        &self,
        configuration: InputTextSessionConfig,
        state: &InputTextSessionStateValue,
    ) -> RuntimeResult<()> {
        match &self.editor {
            AppKitWindowTextEditor::TextView(text_view) => {
                configure_text_view(text_view, configuration);
                apply_text_view_state(text_view, state);
            }
            AppKitWindowTextEditor::SecureField(text_field) => {
                configure_secure_text_field(text_field, configuration)?;
                apply_secure_text_field_state(text_field, &self.delegate, state);
            }
        }

        Ok(())
    }

    /// Build one runtime text-state snapshot from the native editor.
    fn text_state(&self) -> InputTextSessionStateValue {
        match &self.editor {
            AppKitWindowTextEditor::TextView(text_view) => text_state_from_text_view(text_view),
            AppKitWindowTextEditor::SecureField(text_field) => {
                text_state_from_secure_text_field(text_field, &self.delegate)
            }
        }
    }
}

/// Stored ivars for one AppKit text-view delegate.
#[derive(Debug)]
struct AppKitWindowTextViewDelegateState {
    /// The runtime state used for session routing.
    runtime_state: Arc<appkit_core::AppKitRuntimeState>,
    /// The owning runtime window handle.
    window: resource::WindowHandle,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "DestackAppKitWindowTextViewDelegate"]
    #[ivars = AppKitWindowTextViewDelegateState]
    struct AppKitWindowTextViewDelegate;

    unsafe impl NSObjectProtocol for AppKitWindowTextViewDelegate {}

    #[allow(non_snake_case)]
    unsafe impl NSTextDelegate for AppKitWindowTextViewDelegate {
        /// Publish one host-authoritative text-state change.
        #[unsafe(method(textDidChange:))]
        fn textDidChange(&self, notification: &NSNotification) {
            let state = self.ivars();

            if let Some(text_view) = notification
                .object()
                .and_then(|object| object.downcast::<NSTextView>().ok())
            {
                publish_text_view_state_best_effort(&state.runtime_state, state.window, &text_view);
                return;
            }

            publish_window_text_host_state(&state.runtime_state, state.window);
        }
    }

    #[allow(non_snake_case)]
    unsafe impl NSTextViewDelegate for AppKitWindowTextViewDelegate {
        /// Publish one selection-driven host text-state change.
        #[unsafe(method(textViewDidChangeSelection:))]
        fn textViewDidChangeSelection(&self, notification: &NSNotification) {
            let state = self.ivars();
            let Some(text_view) = notification
                .object()
                .and_then(|object| object.downcast::<NSTextView>().ok())
            else {
                publish_window_text_host_state(&state.runtime_state, state.window);
                return;
            };

            publish_text_view_state_best_effort(&state.runtime_state, state.window, &text_view);
        }

        /// Publish one edit or clipboard command while leaving AppKit behavior intact.
        #[unsafe(method(textView:doCommandBySelector:))]
        unsafe fn textView_doCommandBySelector(
            &self,
            text_view: &NSTextView,
            command_selector: Sel,
        ) -> bool {
            handle_command_selector(self.ivars(), text_view, command_selector)
        }
    }

    #[allow(non_snake_case)]
    unsafe impl NSControlTextEditingDelegate for AppKitWindowTextViewDelegate {
        /// Publish one field-editor command while leaving AppKit behavior intact.
        #[unsafe(method(control:textView:doCommandBySelector:))]
        unsafe fn control_textView_doCommandBySelector(
            &self,
            _control: &NSControl,
            text_view: &NSTextView,
            command_selector: Sel,
        ) -> bool {
            handle_command_selector(self.ivars(), text_view, command_selector)
        }
    }

    unsafe impl NSTextFieldDelegate for AppKitWindowTextViewDelegate {}
);

impl AppKitWindowTextViewDelegate {
    /// Create one AppKit text-view delegate.
    fn new(
        mtm: objc2::MainThreadMarker,
        runtime_state: Arc<appkit_core::AppKitRuntimeState>,
        window: resource::WindowHandle,
    ) -> objc2::rc::Retained<Self> {
        let value = Self::alloc(mtm).set_ivars(AppKitWindowTextViewDelegateState {
            runtime_state,
            window,
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return one protocol object for text-view delegate registration.
    fn as_text_view_protocol(&self) -> &ProtocolObject<dyn NSTextViewDelegate> {
        ProtocolObject::from_ref(self)
    }

    /// Return one protocol object for text-field delegate registration.
    fn as_text_field_protocol(&self) -> &ProtocolObject<dyn NSTextFieldDelegate> {
        ProtocolObject::from_ref(self)
    }
}

/// Publish one AppKit command selector through the runtime event queue.
fn handle_command_selector(
    delegate_state: &AppKitWindowTextViewDelegateState,
    text_view: &NSTextView,
    command_selector: Sel,
) -> bool {
    let selector_name = command_selector.name().to_str().unwrap_or_default();
    let session =
        resolve_appkit_window_text_session(&delegate_state.runtime_state, delegate_state.window);
    let Some((_, configuration, session_state)) = session.ok().flatten() else {
        return false;
    };
    let is_composing = session_state.composing.is_some();

    // clipboard commands
    if selector_name == "copy:" {
        if !allows_clipboard_command(configuration, InputClipboardCommandTypeValue::Copy) {
            return true;
        }

        publish_clipboard_command_best_effort(
            &delegate_state.runtime_state,
            delegate_state.window,
            InputClipboardCommandTypeValue::Copy,
            is_composing,
        );
        return false;
    }

    if selector_name == "cut:" {
        if !allows_clipboard_command(configuration, InputClipboardCommandTypeValue::Cut) {
            return true;
        }

        publish_clipboard_command_best_effort(
            &delegate_state.runtime_state,
            delegate_state.window,
            InputClipboardCommandTypeValue::Cut,
            is_composing,
        );
        return false;
    }

    if selector_name == "paste:" {
        if !allows_clipboard_command(configuration, InputClipboardCommandTypeValue::Paste) {
            return true;
        }

        publish_clipboard_command_best_effort(
            &delegate_state.runtime_state,
            delegate_state.window,
            InputClipboardCommandTypeValue::Paste,
            is_composing,
        );
        return false;
    }

    // edit intents
    if selector_name == "deleteBackward:" {
        publish_edit_intent_best_effort(
            &delegate_state.runtime_state,
            delegate_state.window,
            InputEditIntentTypeValue::DeleteContentBackward,
            None,
            is_composing,
        );
        return false;
    }

    if selector_name == "deleteForward:" {
        publish_edit_intent_best_effort(
            &delegate_state.runtime_state,
            delegate_state.window,
            InputEditIntentTypeValue::DeleteContentForward,
            None,
            is_composing,
        );
        return false;
    }

    if selector_name == "insertNewline:" || selector_name == "insertNewlineIgnoringFieldEditor:" {
        publish_edit_intent_best_effort(
            &delegate_state.runtime_state,
            delegate_state.window,
            InputEditIntentTypeValue::InsertLineBreak,
            None,
            is_composing,
        );

        // keep single-line sessions from inserting host line breaks
        if !configuration.is_multiline {
            publish_text_view_state_best_effort(
                &delegate_state.runtime_state,
                delegate_state.window,
                text_view,
            );
            return true;
        }
    }

    false
}

/// Return whether one AppKit session allows one clipboard command.
fn allows_clipboard_command(
    configuration: InputTextSessionConfig,
    command: InputClipboardCommandTypeValue,
) -> bool {
    if configuration.is_secure
        && matches!(
            command,
            InputClipboardCommandTypeValue::Copy | InputClipboardCommandTypeValue::Cut
        )
    {
        return false;
    }

    true
}

/// Activate one AppKit window-backed text session.
pub(crate) fn activate_window_text_session(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
    configuration: InputTextSessionConfig,
    state: InputTextSessionStateValue,
    geometry: Option<InputTextGeometry>,
) -> RuntimeResult<()> {
    appkit_core::with_window_host(runtime_state, window, "destack.input.text.open", |host| {
        ensure_window_text_host(host, runtime_state, window, configuration)?;
        Ok(())
    })?;

    // route host callbacks for this window to the active session
    runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .insert(window, session);

    // seed host state before focusing the native editor
    synchronize_window_text_session(
        runtime_state,
        window,
        configuration,
        state.clone(),
        geometry,
    )?;

    appkit_core::with_window_host(runtime_state, window, "destack.input.text.open", |host| {
        let text_input = host.text_input.borrow();
        let Some(text_host) = text_input.as_ref() else {
            return Err(core_platform::io_not_found(
                "destack.input.text.open",
                format!("window handle {} lost its text host", window.0.local_id),
            ));
        };

        text_host.focus_in_window(&host.window);
        Ok(())
    })?;

    // secure fields need one focused field editor before selection and marked text can be restored
    if configuration.is_secure {
        synchronize_window_text_session(runtime_state, window, configuration, state, geometry)?;
    }

    Ok(())
}

/// Deactivate one AppKit window-backed text session.
pub(crate) fn deactivate_window_text_session(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    session: resource::InputTextSessionHandle,
) -> RuntimeResult<()> {
    // clear routing only when the closing session is still active
    let mut active_sessions = runtime_state
        .active_text_sessions
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    if active_sessions.get(&window).copied() != Some(session) {
        return Ok(());
    }
    active_sessions.remove(&window);
    drop(active_sessions);

    appkit_core::with_window_host(runtime_state, window, "destack.input.text.close", |host| {
        let text_input = host.text_input.borrow();
        let Some(text_host) = text_input.as_ref() else {
            return Ok(());
        };

        host.window.makeFirstResponder(None);
        text_host.clear();
        Ok(())
    })
}

/// Synchronize one AppKit window-backed text session into the hidden host editor.
pub(crate) fn synchronize_window_text_session(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    configuration: InputTextSessionConfig,
    state: InputTextSessionStateValue,
    geometry: Option<InputTextGeometry>,
) -> RuntimeResult<()> {
    appkit_core::with_window_host(
        runtime_state,
        window,
        "destack.input.text.syncWindowRepository",
        |host| {
            ensure_window_text_host(host, runtime_state, window, configuration)?;

            let text_input = host.text_input.borrow();
            let Some(text_host) = text_input.as_ref() else {
                return Err(core_platform::io_not_found(
                    "destack.input.text.syncWindowRepository",
                    format!("window handle {} lost its text host", window.0.local_id),
                ));
            };

            text_host.synchronize(configuration, &state)?;
            text_host.apply_geometry(geometry);

            Ok(())
        },
    )
}

/// Ensure one retained AppKit text host exists for one window.
fn ensure_window_text_host(
    host: &appkit_core::AppKitWindowHost,
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    configuration: InputTextSessionConfig,
) -> RuntimeResult<()> {
    // secure AppKit editing is single-line only
    if configuration.is_secure && configuration.is_multiline {
        return Err(core_platform::not_supported("destack.input.text.open"));
    }

    // reuse one existing host object when the window already owns the right control kind
    if host
        .text_input
        .borrow()
        .as_ref()
        .is_some_and(|text_host| text_host.is_secure() == configuration.is_secure)
    {
        return Ok(());
    }

    let mtm = objc2::MainThreadMarker::new()
        .ok_or_else(|| core_platform::not_supported("destack.input.text.open"))?;
    let content_view = host.window.contentView().ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported("destack.input.text.open")).boxed()
    })?;
    let delegate = AppKitWindowTextViewDelegate::new(mtm, Arc::clone(runtime_state), window);

    // native host objects
    let text_host = if configuration.is_secure {
        let text_field = NSSecureTextField::initWithFrame(
            NSSecureTextField::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)),
        );

        // secure field setup
        text_field.setEditable(true);
        text_field.setSelectable(true);
        text_field.setBordered(false);
        text_field.setBezeled(false);
        text_field.setDrawsBackground(false);
        text_field.setHidden(false);
        unsafe {
            text_field.setDelegate(Some(delegate.as_text_field_protocol()));
        }
        content_view.addSubview(&text_field);

        AppKitWindowTextHost {
            editor: AppKitWindowTextEditor::SecureField(text_field),
            delegate,
        }
    } else {
        let text_view = NSTextView::initWithFrame(
            NSTextView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)),
        );

        // editor setup
        text_view.setEditable(true);
        text_view.setSelectable(true);
        text_view.setFieldEditor(false);
        text_view.setDrawsBackground(false);
        text_view.setHidden(false);
        text_view.setDelegate(Some(delegate.as_text_view_protocol()));
        content_view.addSubview(&text_view);

        AppKitWindowTextHost {
            editor: AppKitWindowTextEditor::TextView(text_view),
            delegate,
        }
    };

    let previous_host = host.text_input.replace(Some(text_host));

    if let Some(previous_host) = previous_host {
        previous_host.remove_from_superview();
    }

    Ok(())
}

/// Project one local rectangle into one target-space rectangle.
fn project_target_rectangle(geometry: InputTextGeometry) -> NSRect {
    let rectangle = geometry.editor_rectangle;
    let transform = geometry.local_to_target_transform;

    let first_x = transform.xx * rectangle.x + transform.xy * rectangle.y + transform.tx;
    let first_y = transform.yx * rectangle.x + transform.yy * rectangle.y + transform.ty;
    let second_x =
        transform.xx * (rectangle.x + rectangle.width) + transform.xy * rectangle.y + transform.tx;
    let second_y =
        transform.yx * (rectangle.x + rectangle.width) + transform.yy * rectangle.y + transform.ty;
    let third_x =
        transform.xx * rectangle.x + transform.xy * (rectangle.y + rectangle.height) + transform.tx;
    let third_y =
        transform.yx * rectangle.x + transform.yy * (rectangle.y + rectangle.height) + transform.ty;
    let fourth_x = transform.xx * (rectangle.x + rectangle.width)
        + transform.xy * (rectangle.y + rectangle.height)
        + transform.tx;
    let fourth_y = transform.yx * (rectangle.x + rectangle.width)
        + transform.yy * (rectangle.y + rectangle.height)
        + transform.ty;

    let min_x = first_x.min(second_x).min(third_x).min(fourth_x);
    let min_y = first_y.min(second_y).min(third_y).min(fourth_y);
    let max_x = first_x.max(second_x).max(third_x).max(fourth_x);
    let max_y = first_y.max(second_y).max(third_y).max(fourth_y);

    NSRect::new(
        NSPoint::new(min_x.floor(), min_y.floor()),
        NSSize::new(
            (max_x - min_x).ceil().max(1.0),
            (max_y - min_y).ceil().max(1.0),
        ),
    )
}

/// Apply one renderer geometry hint to one AppKit view.
fn apply_view_geometry(view: &objc2_app_kit::NSView, geometry: Option<InputTextGeometry>) {
    if let Some(geometry) = geometry {
        view.setFrame(project_target_rectangle(geometry));
    } else {
        view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)));
    }
}

/// Configure one AppKit text view from one session configuration.
fn configure_text_view(text_view: &NSTextView, configuration: InputTextSessionConfig) {
    text_view.setEditable(true);
    text_view.setSelectable(true);
    text_view.setFieldEditor(false);
    text_view.setDrawsBackground(false);

    // single-line sessions should still avoid host-created line breaks
    if !configuration.is_multiline && text_view.isHorizontallyResizable() {
        text_view.setHorizontallyResizable(true);
    }
}

/// Configure one AppKit secure text field from one session configuration.
fn configure_secure_text_field(
    text_field: &NSSecureTextField,
    configuration: InputTextSessionConfig,
) -> RuntimeResult<()> {
    if configuration.is_multiline {
        return Err(core_platform::not_supported(
            "destack.input.text.syncWindowRepository",
        ));
    }

    text_field.setEditable(true);
    text_field.setSelectable(true);
    text_field.setBordered(false);
    text_field.setBezeled(false);
    text_field.setDrawsBackground(false);
    text_field.setMaximumNumberOfLines(1 as NSInteger);

    Ok(())
}

/// Apply one renderer-owned text state to one AppKit text view.
fn apply_text_view_state(text_view: &NSTextView, state: &InputTextSessionStateValue) {
    let string = NSString::from_str(&state.text);
    text_view.setString(&string);
    text_view.setSelectedRange(ns_range_from_text_range(state.selection));

    // preserve renderer-owned marked text when one composition is active
    if let Some(composing_range) = state.composing {
        if let Some(composing_text) = text_range_string(&state.text, composing_range) {
            let composing_text = NSString::from_str(&composing_text);
            let selection_range = selection_range_within_composition(state, composing_range);

            unsafe {
                text_view.setMarkedText_selectedRange_replacementRange(
                    composing_text.as_ref() as &AnyObject,
                    selection_range,
                    ns_range_from_text_range(composing_range),
                );
            }
        } else if text_view.hasMarkedText() {
            text_view.unmarkText();
        }
    } else if text_view.hasMarkedText() {
        text_view.unmarkText();
    }
}

/// Apply one renderer-owned text state to one AppKit secure field.
fn apply_secure_text_field_state(
    text_field: &NSSecureTextField,
    delegate: &AppKitWindowTextViewDelegate,
    state: &InputTextSessionStateValue,
) {
    let string = NSString::from_str(&state.text);
    text_field.setStringValue(&string);

    if let Some(text_view) = secure_text_field_editor(text_field, delegate) {
        apply_text_view_state(&text_view, state);
    }
}

/// Return the active secure-field editor when one exists.
fn secure_text_field_editor(
    text_field: &NSSecureTextField,
    delegate: &AppKitWindowTextViewDelegate,
) -> Option<objc2::rc::Retained<NSTextView>> {
    let text = text_field.currentEditor()?;
    let text_view = text.downcast::<NSTextView>().ok()?;

    text_view.setDelegate(Some(delegate.as_text_view_protocol()));

    Some(text_view)
}

/// Build one runtime text state snapshot from one AppKit text view.
fn text_state_from_text_view(text_view: &NSTextView) -> InputTextSessionStateValue {
    let text = text_view.string().to_string();
    let selection = input_text_range_from_ns_range(text_view.selectedRange());
    let composing = if text_view.hasMarkedText() {
        Some(input_text_range_from_ns_range(text_view.markedRange()))
    } else {
        None
    };

    InputTextSessionStateValue {
        text,
        selection,
        composing,
    }
}

/// Build one runtime text state snapshot from one AppKit secure field.
fn text_state_from_secure_text_field(
    text_field: &NSSecureTextField,
    delegate: &AppKitWindowTextViewDelegate,
) -> InputTextSessionStateValue {
    if let Some(text_view) = secure_text_field_editor(text_field, delegate) {
        return text_state_from_text_view(&text_view);
    }

    let text = text_field.stringValue().to_string();
    let end_offset = text.encode_utf16().count() as u32;

    InputTextSessionStateValue {
        text,
        selection: InputTextRange {
            start_offset: end_offset,
            end_offset,
        },
        composing: None,
    }
}

/// Publish one host-authoritative text state from one AppKit text view.
fn publish_text_view_state(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    text_view: &NSTextView,
) -> RuntimeResult<()> {
    let state = text_state_from_text_view(text_view);

    notify_appkit_window_text_state(runtime_state, window, state)
}

/// Publish one host-authoritative text state and suppress best-effort failures.
fn publish_text_view_state_best_effort(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    text_view: &NSTextView,
) {
    if let Err(error) = publish_text_view_state(runtime_state, window, text_view) {
        runtime_state.diagnostics.warn(
            "display",
            "destack.input.text.publishWindowState",
            format!("appkit text state publish failed: {error}"),
            None,
        );
    }
}

/// Publish one clipboard command and log best-effort failures.
fn publish_clipboard_command_best_effort(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    command: InputClipboardCommandTypeValue,
    is_composing: bool,
) {
    if let Err(error) =
        notify_appkit_window_clipboard_command(runtime_state, window, command, is_composing)
    {
        runtime_state.diagnostics.warn(
            "display",
            "destack.input.text.handleWindowCommand",
            format!("appkit clipboard command publish failed: {error}"),
            None,
        );
    }
}

/// Publish one edit intent and log best-effort failures.
fn publish_edit_intent_best_effort(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
    input_type: InputEditIntentTypeValue,
    data: Option<String>,
    is_composing: bool,
) {
    if let Err(error) =
        notify_appkit_window_edit_intent(runtime_state, window, input_type, data, is_composing)
    {
        runtime_state.diagnostics.warn(
            "display",
            "destack.input.text.handleWindowCommand",
            format!("appkit edit intent publish failed: {error}"),
            None,
        );
    }
}

/// Publish one host-authoritative text state from one retained window host.
fn publish_window_text_host_state(
    runtime_state: &Arc<appkit_core::AppKitRuntimeState>,
    window: resource::WindowHandle,
) {
    if let Err(error) = appkit_core::with_window_host(
        runtime_state,
        window,
        "destack.input.text.publishWindowState",
        |host| {
            let text_input = host.text_input.borrow();
            let Some(text_host) = text_input.as_ref() else {
                return Ok(());
            };

            let state = text_host.text_state();
            notify_appkit_window_text_state(runtime_state, window, state)
        },
    ) {
        runtime_state.diagnostics.warn(
            "display",
            "destack.input.text.publishWindowState",
            format!("appkit text host state publish failed: {error}"),
            None,
        );
    }
}

/// Return one runtime selection range relative to one composition span.
fn selection_range_within_composition(
    state: &InputTextSessionStateValue,
    composing_range: InputTextRange,
) -> NSRange {
    let selection_start = state
        .selection
        .start_offset
        .clamp(composing_range.start_offset, composing_range.end_offset)
        - composing_range.start_offset;
    let selection_end = state
        .selection
        .end_offset
        .clamp(composing_range.start_offset, composing_range.end_offset)
        - composing_range.start_offset;

    NSRange::from((selection_start as usize)..(selection_end as usize))
}

/// Return one utf-8 byte offset for one utf-16 code-unit offset.
fn utf16_offset_to_utf8_offset(text: &str, offset: u32) -> Option<usize> {
    let mut utf16_offset = 0u32;

    for (byte_offset, character) in text.char_indices() {
        if utf16_offset == offset {
            return Some(byte_offset);
        }

        utf16_offset = utf16_offset.checked_add(character.len_utf16() as u32)?;
        if utf16_offset > offset {
            return None;
        }
    }

    if utf16_offset == offset {
        Some(text.len())
    } else {
        None
    }
}

/// Return one utf-8 string slice for one runtime text range.
fn text_range_string(text: &str, range: InputTextRange) -> Option<String> {
    let start = utf16_offset_to_utf8_offset(text, range.start_offset)?;
    let end = utf16_offset_to_utf8_offset(text, range.end_offset)?;

    Some(text.get(start..end)?.to_string())
}

/// Convert one runtime text range into one AppKit range.
fn ns_range_from_text_range(range: InputTextRange) -> NSRange {
    NSRange::from((range.start_offset as usize)..(range.end_offset as usize))
}

/// Convert one AppKit range into one runtime text range.
fn input_text_range_from_ns_range(range: NSRange) -> InputTextRange {
    let start_offset = range.location as u32;
    let end_offset = range.location.saturating_add(range.length) as u32;

    InputTextRange {
        start_offset,
        end_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::allows_clipboard_command;
    use crate::platform::input::{
        InputClipboardCommandTypeValue, InputTextInputType, InputTextSessionConfig,
        InputWindowTarget,
    };

    /// Reject copy and cut commands for secure AppKit sessions.
    #[test]
    fn test_secure_appkit_sessions_reject_copy_and_cut() {
        let configuration = InputTextSessionConfig {
            target: InputWindowTarget { window: None },
            input_type: InputTextInputType::Password,
            is_multiline: false,
            is_secure: true,
        };

        assert!(!allows_clipboard_command(
            configuration,
            InputClipboardCommandTypeValue::Copy,
        ));
        assert!(!allows_clipboard_command(
            configuration,
            InputClipboardCommandTypeValue::Cut,
        ));
        assert!(allows_clipboard_command(
            configuration,
            InputClipboardCommandTypeValue::Paste,
        ));
    }

    /// Allow clipboard commands for non-secure AppKit sessions.
    #[test]
    fn test_non_secure_appkit_sessions_allow_clipboard_commands() {
        let configuration = InputTextSessionConfig {
            target: InputWindowTarget { window: None },
            input_type: InputTextInputType::Text,
            is_multiline: false,
            is_secure: false,
        };

        assert!(allows_clipboard_command(
            configuration,
            InputClipboardCommandTypeValue::Copy,
        ));
        assert!(allows_clipboard_command(
            configuration,
            InputClipboardCommandTypeValue::Cut,
        ));
        assert!(allows_clipboard_command(
            configuration,
            InputClipboardCommandTypeValue::Paste,
        ));
    }
}
