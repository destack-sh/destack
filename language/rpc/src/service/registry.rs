use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::{Service, ServiceId};
use crate::protocol::{Code, MethodOffer, ServiceOffer, Status};

/// RPC services keyed by stable service identifier.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    /// Registered service implementations.
    services: HashMap<ServiceId, Arc<dyn Service>>,
}

impl Registry {
    /// Create one empty service registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one service implementation.
    pub fn insert<S: Service>(&mut self, service: S) -> Result<(), RegistryError> {
        let id = service.schema().id();
        if self.services.contains_key(&id) {
            return Err(RegistryError::DuplicateService(id));
        }

        self.services.insert(id, Arc::new(service));

        Ok(())
    }

    /// Return one registered service.
    pub fn get(&self, id: ServiceId) -> Option<Arc<dyn Service>> {
        self.services.get(&id).cloned()
    }

    /// Describe every requested service and its complete method contracts.
    pub fn offer(&self, requested: &[ServiceId]) -> Result<Vec<ServiceOffer>, Status> {
        let mut offers = Vec::with_capacity(requested.len());
        let mut service_ids = HashSet::with_capacity(requested.len());

        for id in requested {
            if !service_ids.insert(*id) {
                return Err(Status::new(
                    Code::InvalidArgument,
                    format!("service {id:?} is requested more than once"),
                ));
            }
            let Some(service) = self.services.get(id) else {
                return Err(Status::new(
                    Code::Unimplemented,
                    format!("service {id:?} is not registered"),
                ));
            };
            let schema = service.schema();
            let mut methods = schema
                .methods()
                .iter()
                .map(|method| MethodOffer {
                    method: method.id(),
                    fingerprint: method.fingerprint(),
                })
                .collect::<Vec<_>>();

            methods.sort_unstable_by_key(|offer| offer.method.0);
            offers.push(ServiceOffer {
                service: schema.id(),
                fingerprint: schema.fingerprint(),
                methods,
            });
        }

        offers.sort_unstable_by_key(|offer| offer.service.0);

        Ok(offers)
    }
}

/// Failure to change one service registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    /// One service identifier is already registered.
    DuplicateService(ServiceId),
}

impl std::fmt::Display for RegistryError {
    /// Format this registry failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateService(id) => write!(formatter, "service {id:?} is already registered"),
        }
    }
}

impl std::error::Error for RegistryError {}
