use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;

/// One process-global registry of typed platform services.
struct ServiceRegistry {
    /// Live services keyed by concrete type.
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

/// Global typed platform-service registry.
static SERVICE_REGISTRY: OnceLock<ServiceRegistry> = OnceLock::new();

/// One agent-local cached handle for one typed platform service.
pub(crate) struct CachedServiceHandle<S> {
    /// Cached typed service handle.
    service: OnceLock<Arc<S>>,
}

impl<S> Default for CachedServiceHandle<S> {
    /// Create one empty cached service handle.
    fn default() -> Self {
        Self {
            service: OnceLock::new(),
        }
    }
}

impl<S> CachedServiceHandle<S> {
    /// Return one cached typed service handle from one infallible builder.
    pub(crate) fn get_or_init(&self, builder: impl FnOnce() -> Arc<S>) -> Arc<S> {
        if let Some(service) = self.service.get() {
            return service.clone();
        }

        let service = builder();
        drop(self.service.set(service.clone()));

        self.service.get().cloned().unwrap_or(service)
    }

    /// Return one cached typed service handle from one fallible builder.
    pub(crate) fn get_or_try_init(
        &self,
        builder: impl FnOnce() -> RuntimeResult<Arc<S>>,
    ) -> RuntimeResult<Arc<S>> {
        if let Some(service) = self.service.get() {
            return Ok(service.clone());
        }

        let service = builder()?;
        drop(self.service.set(service.clone()));

        Ok(self.service.get().cloned().unwrap_or(service))
    }
}

impl ServiceRegistry {
    /// Return the process-global service registry.
    fn shared() -> &'static ServiceRegistry {
        SERVICE_REGISTRY.get_or_init(|| ServiceRegistry {
            services: Mutex::new(HashMap::new()),
        })
    }

    /// Return one live typed service when it has already been initialized.
    #[cfg(target_os = "macos")]
    fn get<S>(&self) -> Option<Arc<S>>
    where
        S: Any + Send + Sync + 'static,
    {
        // resolve one existing typed service
        let services = self.services.lock();
        let service = services.get(&TypeId::of::<S>())?;

        Some(
            service
                .clone()
                .downcast::<S>()
                .expect("typed platform service should downcast"),
        )
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
                return Ok(service
                    .clone()
                    .downcast::<S>()
                    .expect("typed platform service should downcast"));
            }
        }

        // build one new service without holding the registry lock
        let service = Arc::new(builder()?);

        // publish the new service unless another thread won the race
        let mut services = self.services.lock();
        if let Some(existing) = services.get(&TypeId::of::<S>()) {
            return Ok(existing
                .clone()
                .downcast::<S>()
                .expect("typed platform service should downcast"));
        }

        services.insert(TypeId::of::<S>(), service.clone());

        Ok(service)
    }
}

/// Return one shared process-global typed service.
pub(crate) fn global_service<S>(builder: impl FnOnce() -> RuntimeResult<S>) -> RuntimeResult<Arc<S>>
where
    S: Any + Send + Sync + 'static,
{
    ServiceRegistry::shared().get_or_init(builder)
}

/// Return one shared process-global typed service when it is already live.
#[cfg(target_os = "macos")]
pub(crate) fn global_service_if_initialized<S>() -> Option<Arc<S>>
where
    S: Any + Send + Sync + 'static,
{
    ServiceRegistry::shared().get::<S>()
}
