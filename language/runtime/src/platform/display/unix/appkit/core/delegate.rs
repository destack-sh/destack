use std::sync::{Arc, Mutex};

use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{DefinedClass, MainThreadOnly, define_class};
use objc2_app_kit::{
    NSDragOperation, NSDraggingDestination, NSDraggingInfo, NSWindow, NSWindowDelegate,
};
use objc2_foundation::{NSNotification, NSObject, NSObjectProtocol};

use crate::platform::resource;

use super::runtime::AppKitRuntimeState;
use crate::platform::display::unix::appkit::model::AppKitWindowHostState;
use crate::platform::display::unix::appkit::{event, window};

/// Stored ivars for one AppKit window delegate instance.
#[derive(Debug)]
pub(crate) struct AppKitWindowDelegateState {
    /// The runtime state used for event publication.
    pub(crate) runtime_state: Arc<AppKitRuntimeState>,
    /// The runtime window handle associated with this delegate.
    pub(crate) window: resource::WindowHandle,
    /// The runtime host state for this window.
    pub(crate) host_state: Arc<Mutex<AppKitWindowHostState>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "DestackAppKitWindowDelegate"]
    #[ivars = AppKitWindowDelegateState]
    pub(crate) struct AppKitWindowDelegate;

    unsafe impl NSObjectProtocol for AppKitWindowDelegate {}

    unsafe impl NSWindowDelegate for AppKitWindowDelegate {
        /// Handle one close-request callback from AppKit.
        #[unsafe(method(windowShouldClose:))]
        fn window_should_close(&self, _: Option<&AnyObject>) -> bool {
            let state = self.ivars();
            event::publish_window_close_requested(&state.runtime_state, state.window);
            false
        }

        /// Handle one will-close callback from AppKit.
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _: Option<&NSNotification>) {
            let state = self.ivars();
            event::publish_window_destroyed(&state.runtime_state, state.window);
        }

        /// Handle one move callback from AppKit.
        #[unsafe(method(windowDidMove:))]
        fn window_did_move(&self, notification: Option<&NSNotification>) {
            let state = self.ivars();
            let Some(window) = notification.and_then(window_from_notification) else {
                return;
            };

            window::apply_host_position_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                &window,
            );
        }

        /// Handle one resize callback from AppKit.
        #[unsafe(method(windowDidResize:))]
        fn window_did_resize(&self, notification: Option<&NSNotification>) {
            let state = self.ivars();
            let Some(window) = notification.and_then(window_from_notification) else {
                return;
            };

            window::apply_host_size_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                &window,
            );
        }

        /// Handle one screen-change callback from AppKit.
        #[unsafe(method(windowDidChangeScreen:))]
        fn window_did_change_screen(&self, notification: &NSNotification) {
            let state = self.ivars();
            let Some(window) = window_from_notification(notification) else {
                return;
            };

            window::apply_host_size_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                &window,
            );
        }

        /// Handle one backing-properties callback from AppKit.
        #[unsafe(method(windowDidChangeBackingProperties:))]
        fn window_did_change_backing_properties(&self, notification: &NSNotification) {
            let state = self.ivars();
            let Some(window) = window_from_notification(notification) else {
                return;
            };

            window::apply_host_size_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                &window,
            );
        }

        /// Handle one focus-gained callback from AppKit.
        #[unsafe(method(windowDidBecomeKey:))]
        fn window_did_become_key(&self, _: Option<&NSNotification>) {
            let state = self.ivars();
            window::apply_host_focus_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                true,
            );
        }

        /// Handle one focus-lost callback from AppKit.
        #[unsafe(method(windowDidResignKey:))]
        fn window_did_resign_key(&self, _: Option<&NSNotification>) {
            let state = self.ivars();
            window::apply_host_focus_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                false,
            );
        }

        /// Handle one occlusion-change callback from AppKit.
        #[unsafe(method(windowDidChangeOcclusionState:))]
        fn window_did_change_occlusion_state(&self, notification: Option<&NSNotification>) {
            let state = self.ivars();
            let Some(window) = notification.and_then(window_from_notification) else {
                return;
            };

            window::apply_host_occlusion_change(
                &state.runtime_state,
                state.window,
                &state.host_state,
                &window,
            );
        }
    }

    unsafe impl NSDraggingDestination for AppKitWindowDelegate {
        /// Handle one drag-enter callback from AppKit.
        #[unsafe(method(draggingEntered:))]
        fn dragging_entered(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            let state = self.ivars();

            window::handle_dragging_entered(&state.runtime_state, state.window, sender)
        }

        /// Handle one drag-update callback from AppKit.
        #[unsafe(method(draggingUpdated:))]
        fn dragging_updated(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> NSDragOperation {
            let state = self.ivars();

            window::handle_dragging_updated(&state.runtime_state, state.window, sender)
        }

        /// Handle one drag-exit callback from AppKit.
        #[unsafe(method(draggingExited:))]
        fn dragging_exited(&self, _: Option<&ProtocolObject<dyn NSDraggingInfo>>) {
            let state = self.ivars();

            window::handle_dragging_exited(&state.runtime_state, state.window);
        }

        /// Handle one drag-prepare callback from AppKit.
        #[unsafe(method(prepareForDragOperation:))]
        fn prepare_for_drag_operation(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            window::handle_prepare_for_drag_operation(sender)
        }

        /// Handle one drag-perform callback from AppKit.
        #[unsafe(method(performDragOperation:))]
        fn perform_drag_operation(&self, sender: &ProtocolObject<dyn NSDraggingInfo>) -> bool {
            let state = self.ivars();

            window::handle_perform_drag_operation(&state.runtime_state, state.window, sender)
        }
    }
);

impl AppKitWindowDelegate {
    /// Create one AppKit window delegate.
    pub(crate) fn new(
        mtm: objc2::MainThreadMarker,
        runtime_state: Arc<AppKitRuntimeState>,
        window: resource::WindowHandle,
        binding: Arc<Mutex<AppKitWindowHostState>>,
    ) -> objc2::rc::Retained<Self> {
        let value = Self::alloc(mtm).set_ivars(AppKitWindowDelegateState {
            runtime_state,
            window,
            host_state: binding,
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return one protocol object for AppKit delegate registration.
    pub(crate) fn as_protocol(&self) -> &ProtocolObject<dyn NSWindowDelegate> {
        ProtocolObject::from_ref(self)
    }
}

/// Resolve one native window payload from one AppKit notification.
fn window_from_notification(
    notification: &NSNotification,
) -> Option<objc2::rc::Retained<NSWindow>> {
    let object = notification.object()?;

    object.downcast::<NSWindow>().ok()
}
