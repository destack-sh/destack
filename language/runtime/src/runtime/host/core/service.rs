use std::sync::Arc;

/// Host lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostLifecycleState {
    /// Runtime has not received start events yet.
    Initializing,
    /// Runtime is active and can process host interactions.
    Running,
    /// Runtime is paused by the host.
    Paused,
    /// Runtime is stopped by the host.
    Stopped,
    /// Runtime host process is being destroyed.
    Destroyed,
}

/// Host lifecycle integration surface.
pub trait HostLifecycleService: std::fmt::Debug + Send + Sync {
    /// Return the current lifecycle state.
    fn state(&self) -> HostLifecycleState;
}

/// Host window integration surface.
pub trait HostWindowService: std::fmt::Debug + Send + Sync {
    /// Return whether one native window is currently available.
    fn has_window(&self) -> bool;
}

/// Host permission integration surface.
pub trait HostPermissionService: std::fmt::Debug + Send + Sync {
    /// Return whether the named permission currently has a pending request.
    fn is_request_in_flight(&self, permission: &str) -> bool;
}

/// Host asset integration surface.
pub trait HostAssetService: std::fmt::Debug + Send + Sync {
    /// Return whether one native asset manager is currently available.
    fn has_asset_manager(&self) -> bool;
}

/// Host JNI integration surface.
pub trait HostJniService: std::fmt::Debug + Send + Sync {
    /// Return whether this thread is attached to the JVM.
    fn is_thread_attached(&self) -> bool;
}

/// Host interruption integration surface.
pub trait HostInterruptionService: std::fmt::Debug + Send + Sync {
    /// Return whether host interruption is currently active.
    fn is_interrupted(&self) -> bool;
}

/// Host display integration surface.
pub trait HostDisplayService: std::fmt::Debug + Send + Sync {
    /// Return the active display scale factor in thousandths.
    fn scale_factor_thousandths(&self) -> u32;

    /// Return the active display refresh rate in millihertz when known.
    fn refresh_rate_millihz(&self) -> Option<u32>;
}

/// Host power integration surface.
pub trait HostPowerService: std::fmt::Debug + Send + Sync {
    /// Return whether host low power mode is currently active.
    fn is_low_power_mode(&self) -> bool;

    /// Return host battery level in percent when known.
    fn battery_percent(&self) -> Option<u8>;
}

/// Host text-input integration surface.
pub trait HostTextInputService: std::fmt::Debug + Send + Sync {
    /// Return whether host text input is currently active.
    fn is_text_input_active(&self) -> bool;
}

/// Host haptics integration surface.
pub trait HostHapticsService: std::fmt::Debug + Send + Sync {
    /// Return whether host haptics support is currently available.
    fn has_haptics_support(&self) -> bool;
}

/// Service surfaces exposed by one host adapter.
#[derive(Debug, Default, Clone)]
pub struct HostServices {
    /// Lifecycle service when the host provides one.
    lifecycle: Option<Arc<dyn HostLifecycleService>>,
    /// Window service when the host provides one.
    window: Option<Arc<dyn HostWindowService>>,
    /// Permission service when the host provides one.
    permission: Option<Arc<dyn HostPermissionService>>,
    /// Asset service when the host provides one.
    asset: Option<Arc<dyn HostAssetService>>,
    /// JNI service when the host provides one.
    jni: Option<Arc<dyn HostJniService>>,
    /// Interruption service when the host provides one.
    interruption: Option<Arc<dyn HostInterruptionService>>,
    /// Display service when the host provides one.
    display: Option<Arc<dyn HostDisplayService>>,
    /// Power service when the host provides one.
    power: Option<Arc<dyn HostPowerService>>,
    /// Text-input service when the host provides one.
    text_input: Option<Arc<dyn HostTextInputService>>,
    /// Haptics service when the host provides one.
    haptics: Option<Arc<dyn HostHapticsService>>,
}

impl HostServices {
    /// Set the lifecycle service.
    pub fn with_lifecycle(mut self, service: Arc<dyn HostLifecycleService>) -> Self {
        self.lifecycle = Some(service);
        self
    }

    /// Set the window service.
    pub fn with_window(mut self, service: Arc<dyn HostWindowService>) -> Self {
        self.window = Some(service);
        self
    }

    /// Set the permission service.
    pub fn with_permission(mut self, service: Arc<dyn HostPermissionService>) -> Self {
        self.permission = Some(service);
        self
    }

    /// Set the asset service.
    pub fn with_asset(mut self, service: Arc<dyn HostAssetService>) -> Self {
        self.asset = Some(service);
        self
    }

    /// Set the JNI service.
    pub fn with_jni(mut self, service: Arc<dyn HostJniService>) -> Self {
        self.jni = Some(service);
        self
    }

    /// Set the interruption service.
    pub fn with_interruption(mut self, service: Arc<dyn HostInterruptionService>) -> Self {
        self.interruption = Some(service);
        self
    }

    /// Set the display service.
    pub fn with_display(mut self, service: Arc<dyn HostDisplayService>) -> Self {
        self.display = Some(service);
        self
    }

    /// Set the power service.
    pub fn with_power(mut self, service: Arc<dyn HostPowerService>) -> Self {
        self.power = Some(service);
        self
    }

    /// Set the text-input service.
    pub fn with_text_input(mut self, service: Arc<dyn HostTextInputService>) -> Self {
        self.text_input = Some(service);
        self
    }

    /// Set the haptics service.
    pub fn with_haptics(mut self, service: Arc<dyn HostHapticsService>) -> Self {
        self.haptics = Some(service);
        self
    }

    /// Return the lifecycle service when available.
    pub fn lifecycle(&self) -> Option<&Arc<dyn HostLifecycleService>> {
        self.lifecycle.as_ref()
    }

    /// Return the window service when available.
    pub fn window(&self) -> Option<&Arc<dyn HostWindowService>> {
        self.window.as_ref()
    }

    /// Return the permission service when available.
    pub fn permission(&self) -> Option<&Arc<dyn HostPermissionService>> {
        self.permission.as_ref()
    }

    /// Return the asset service when available.
    pub fn asset(&self) -> Option<&Arc<dyn HostAssetService>> {
        self.asset.as_ref()
    }

    /// Return the JNI service when available.
    pub fn jni(&self) -> Option<&Arc<dyn HostJniService>> {
        self.jni.as_ref()
    }

    /// Return the interruption service when available.
    pub fn interruption(&self) -> Option<&Arc<dyn HostInterruptionService>> {
        self.interruption.as_ref()
    }

    /// Return the display service when available.
    pub fn display(&self) -> Option<&Arc<dyn HostDisplayService>> {
        self.display.as_ref()
    }

    /// Return the power service when available.
    pub fn power(&self) -> Option<&Arc<dyn HostPowerService>> {
        self.power.as_ref()
    }

    /// Return the text-input service when available.
    pub fn text_input(&self) -> Option<&Arc<dyn HostTextInputService>> {
        self.text_input.as_ref()
    }

    /// Return the haptics service when available.
    pub fn haptics(&self) -> Option<&Arc<dyn HostHapticsService>> {
        self.haptics.as_ref()
    }
}
