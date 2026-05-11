use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;

use crate::diagnostic::{RuntimeError, RuntimeResult};

/// One process-global registry of typed platform services.
#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
struct ServiceRegistry {
    /// Live services keyed by concrete type.
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

/// Global typed platform-service registry.
static SERVICE_REGISTRY: OnceLock<ServiceRegistry> = OnceLock::new();

/// One worker-local cached handle for one typed platform service.
#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
pub(crate) struct ServiceHandle<S> {
    /// Cached typed service handle.
    service: OnceLock<Arc<S>>,
}

#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
impl<S> Default for ServiceHandle<S> {
    /// Create one empty service handle.
    fn default() -> Self {
        Self {
            service: OnceLock::new(),
        }
    }
}

#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
impl<S> ServiceHandle<S> {
    /// Return one typed service handle from one infallible builder.
    pub(crate) fn get_or_init(&self, builder: impl FnOnce() -> Arc<S>) -> Arc<S> {
        self.service.get_or_init(builder).clone()
    }

    /// Return one typed service handle from one fallible builder.
    pub(crate) fn get_or_try_init(
        &self,
        builder: impl FnOnce() -> RuntimeResult<Arc<S>>,
    ) -> RuntimeResult<Arc<S>> {
        if let Some(service) = self.service.get() {
            return Ok(service.clone());
        }

        let service = builder()?;
        match self.service.set(service.clone()) {
            Ok(()) => Ok(service),
            Err(service) => Ok(self.service.get().cloned().unwrap_or(service)),
        }
    }
}

#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
impl ServiceRegistry {
    /// Return the process-global service registry.
    fn shared() -> &'static ServiceRegistry {
        SERVICE_REGISTRY.get_or_init(|| ServiceRegistry {
            services: Mutex::new(HashMap::new()),
        })
    }

    /// Return one live typed service when it has already been initialized.
    fn get<S>(&self) -> Option<Arc<S>>
    where
        S: Any + Send + Sync + 'static,
    {
        // resolve one existing typed service
        let services = self.services.lock();
        let service = services.get(&TypeId::of::<S>())?;

        service.clone().downcast::<S>().ok()
    }

    /// Return one shared typed service, initializing it on first access.
    fn get_or_init<S>(&self, builder: impl FnOnce() -> RuntimeResult<S>) -> RuntimeResult<Arc<S>>
    where
        S: Any + Send + Sync + 'static,
    {
        // reuse one existing typed service
        {
            let services = self.services.lock();
            if let Some(service) = services.get(&TypeId::of::<S>()) {
                return downcast_service(service.clone());
            }
        }

        // build one new service without holding the registry lock
        let service = Arc::new(builder()?);

        // publish the new service unless another thread won the race
        let mut services = self.services.lock();
        if let Some(existing) = services.get(&TypeId::of::<S>()) {
            return downcast_service(existing.clone());
        }

        services.insert(TypeId::of::<S>(), service.clone());

        Ok(service)
    }
}

/// Return one shared process-global typed service.
#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
pub(crate) fn global_service<S>(builder: impl FnOnce() -> RuntimeResult<S>) -> RuntimeResult<Arc<S>>
where
    S: Any + Send + Sync + 'static,
{
    ServiceRegistry::shared().get_or_init(builder)
}

/// Return one shared process-global typed service when it is already live.
///
/// This is for callback ingress paths that may race service teardown.
/// Normal binding and registration flows should use `global_service(...)`.
#[cfg_attr(any(target_os = "ios", target_os = "android"), allow(dead_code))]
pub(crate) fn global_service_if_initialized<S>() -> Option<Arc<S>>
where
    S: Any + Send + Sync + 'static,
{
    ServiceRegistry::shared().get::<S>()
}

/// Downcast one erased service to its concrete service type.
fn downcast_service<S>(service: Arc<dyn Any + Send + Sync>) -> RuntimeResult<Arc<S>>
where
    S: Any + Send + Sync + 'static,
{
    service.downcast::<S>().map_err(|_| {
        RuntimeError::Internal {
            message: "typed platform service registry stored an invalid service type".to_string(),
        }
        .boxed()
    })
}
