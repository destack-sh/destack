use destack_runtime::host::abi::describe::{HostAbiFunction, HostAbiParameter, HostAbiType};

/// One resolved host runtime ingress surface.
#[derive(Clone)]
pub(crate) struct HostIngress {
    /// The authored ingress callback.
    abi: HostAbiFunction,
    /// The encoded payload parameter.
    encoded_parameter: HostAbiParameter,
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

        // encoded payload
        let mut encoded_parameters = abi.parameters.iter().filter(|parameter| {
            matches!(
                parameter.ty,
                HostAbiType::Named(_) | HostAbiType::NativeSlice(_)
            )
        });
        let encoded_parameter = encoded_parameters
            .next()
            .cloned()
            .unwrap_or_else(|| panic!("missing encoded ingress parameter for {}", abi.name));

        assert!(
            encoded_parameters.next().is_none(),
            "unsupported ingress payload shape for {}",
            abi.name
        );

        Self {
            abi,
            encoded_parameter,
        }
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

    /// Return the encoded payload parameter.
    pub(crate) fn encoded_parameter(&self) -> &HostAbiParameter {
        &self.encoded_parameter
    }
}
