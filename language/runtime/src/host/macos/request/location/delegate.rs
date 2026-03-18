use std::cell::Cell;

use objc2::runtime::ProtocolObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class};
use objc2_core_location::{
    CLAuthorizationStatus, CLHeading, CLLocation, CLLocationManager, CLLocationManagerDelegate,
};
use objc2_foundation::{NSArray, NSError, NSObject, NSObjectProtocol};
use tracing::error;

use crate::host::core::HostRuntimeId;
use crate::host::macos::{macos_notify_location_sample, macos_notify_permission_result};

use super::permission::{is_location_authorized, location_permission_name_for_status};
use super::sample::{heading_degrees_from_native, location_sample_value_from_native};

/// Stored ivars for one Core Location delegate instance.
#[derive(Debug)]
pub(super) struct MacosLocationDelegateState {
    /// Host runtime id that should receive location events.
    pub(super) host_runtime_id: HostRuntimeId,
    /// Runtime-scoped watch id for active watch callbacks.
    pub(super) watch_id: Option<String>,
    /// Whether heading updates should override course values.
    pub(super) include_heading: bool,
    /// Last known authorization status for synchronous waiters.
    pub(super) authorization_status: Cell<Option<CLAuthorizationStatus>>,
    /// Last heading sample seen from heading callbacks.
    pub(super) heading_degrees: Cell<Option<f64>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "DestackMacosLocationManagerDelegate"]
    #[ivars = MacosLocationDelegateState]
    pub(super) struct MacosLocationManagerDelegate;

    unsafe impl NSObjectProtocol for MacosLocationManagerDelegate {}

    unsafe impl CLLocationManagerDelegate for MacosLocationManagerDelegate {
        /// Handle one native location update callback.
        #[unsafe(method(locationManager:didUpdateLocations:))]
        fn location_manager_did_update_locations(
            &self,
            _manager: &CLLocationManager,
            locations: &NSArray<CLLocation>,
        ) {
            let state = self.ivars();
            let Some(watch_id) = state.watch_id.as_deref() else {
                return;
            };
            let Some(location) = locations.lastObject() else {
                return;
            };
            let sample =
                location_sample_value_from_native(location.as_ref(), state.heading_degrees.get());

            if let Err(error) =
                macos_notify_location_sample(state.host_runtime_id.0, watch_id, sample)
            {
                error!(?error, "failed to publish macOS location sample");
            }
        }

        /// Handle one native heading update callback.
        #[unsafe(method(locationManager:didUpdateHeading:))]
        fn location_manager_did_update_heading(
            &self,
            manager: &CLLocationManager,
            heading: &CLHeading,
        ) {
            let state = self.ivars();
            if !state.include_heading {
                return;
            }

            let heading_degrees = heading_degrees_from_native(heading);
            state.heading_degrees.set(heading_degrees);

            let Some(watch_id) = state.watch_id.as_deref() else {
                return;
            };
            let Some(location) = (unsafe { manager.location() }) else {
                return;
            };
            let sample = location_sample_value_from_native(location.as_ref(), heading_degrees);

            if let Err(error) =
                macos_notify_location_sample(state.host_runtime_id.0, watch_id, sample)
            {
                error!(
                    ?error,
                    "failed to publish macOS heading-backed location sample"
                );
            }
        }

        /// Handle one native authorization change callback.
        #[unsafe(method(locationManagerDidChangeAuthorization:))]
        fn location_manager_did_change_authorization(&self, manager: &CLLocationManager) {
            let status = unsafe { manager.authorizationStatus() };
            let state = self.ivars();
            state.authorization_status.set(Some(status));

            if let Err(error) = macos_notify_permission_result(
                state.host_runtime_id.0,
                location_permission_name_for_status(status),
                is_location_authorized(status),
            ) {
                error!(?error, "failed to publish macOS location permission change");
            }
        }

        /// Handle one native authorization change callback on older APIs.
        #[unsafe(method(locationManager:didChangeAuthorizationStatus:))]
        fn location_manager_did_change_authorization_status(
            &self,
            _manager: &CLLocationManager,
            status: CLAuthorizationStatus,
        ) {
            let state = self.ivars();
            state.authorization_status.set(Some(status));

            if let Err(error) = macos_notify_permission_result(
                state.host_runtime_id.0,
                location_permission_name_for_status(status),
                is_location_authorized(status),
            ) {
                error!(?error, "failed to publish macOS location permission change");
            }
        }

        /// Handle one native failure callback.
        #[unsafe(method(locationManager:didFailWithError:))]
        fn location_manager_did_fail_with_error(
            &self,
            _manager: &CLLocationManager,
            error_value: &NSError,
        ) {
            error!("CoreLocation update failed: {error_value:?}");
        }
    }
);

impl MacosLocationManagerDelegate {
    /// Create one Core Location delegate for one runtime and optional watch id.
    pub(super) fn new(
        mtm: MainThreadMarker,
        host_runtime_id: HostRuntimeId,
        watch_id: Option<String>,
        include_heading: bool,
        authorization_status: CLAuthorizationStatus,
    ) -> objc2::rc::Retained<Self> {
        let value = Self::alloc(mtm).set_ivars(MacosLocationDelegateState {
            host_runtime_id,
            watch_id,
            include_heading,
            authorization_status: Cell::new(Some(authorization_status)),
            heading_degrees: Cell::new(None),
        });

        unsafe { objc2::msg_send![super(value), init] }
    }

    /// Return the latest authorization status observed by this delegate.
    pub(super) fn authorization_status(&self) -> Option<CLAuthorizationStatus> {
        self.ivars().authorization_status.get()
    }

    /// Return one protocol object for Core Location delegate registration.
    pub(super) fn as_protocol(&self) -> &ProtocolObject<dyn CLLocationManagerDelegate> {
        ProtocolObject::from_ref(self)
    }
}
