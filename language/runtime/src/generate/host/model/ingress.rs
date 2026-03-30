use destack_runtime::host::abi::describe::{HostAbiFunction, HostAbiParameter, HostAbiType};

/// One resolved host runtime ingress surface.
#[derive(Clone)]
pub(crate) struct HostIngress {
    /// The authored ingress callback.
    abi: HostAbiFunction,
}

impl HostIngress {
    /// Build one resolved host ingress from one authored ingress callback.
    pub(crate) fn new(abi: HostAbiFunction) -> Self {
        // host session handle
        let session_handle_count = abi
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty, HostAbiType::HostSessionHandle))
            .count();
        assert_eq!(
            session_handle_count, 1,
            "unsupported ingress session-handle shape for {}",
            abi.name
        );

        assert!(
            abi.parameters
                .iter()
                .any(|parameter| !matches!(parameter.ty, HostAbiType::HostSessionHandle)),
            "missing ingress payload parameter for {}",
            abi.name
        );

        Self { abi }
    }

    /// Return the canonical ingress name.
    pub(crate) fn name(&self) -> &'static str {
        self.abi.name
    }

    /// Return the ingress documentation.
    pub(crate) fn documentation(&self) -> &'static str {
        self.abi.documentation
    }

    /// Return the ordered authored parameters.
    pub(crate) fn parameters(&self) -> &[HostAbiParameter] {
        &self.abi.parameters
    }
}
