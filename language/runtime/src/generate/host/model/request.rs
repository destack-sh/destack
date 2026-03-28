use destack_runtime::host::abi::describe::{HostAbiFunction, HostAbiParameter, HostAbiType};

/// One resolved host request callback surface.
#[derive(Clone)]
pub(crate) struct HostRequest {
    /// The authored request callback.
    abi: HostAbiFunction,
    /// The ordered input parameters.
    input_parameters: Vec<HostAbiParameter>,
    /// The optional output parameter.
    output_parameter: Option<HostAbiParameter>,
}

impl HostRequest {
    /// Build one resolved host request from one authored request callback.
    pub(crate) fn new(abi: HostAbiFunction) -> Self {
        // host session handle
        let session_handle_count = abi
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty, HostAbiType::HostSessionHandle))
            .count();
        assert_eq!(
            session_handle_count, 1,
            "unsupported host request session-handle shape for {}",
            abi.name
        );

        // input and output split
        let input_parameters = abi
            .parameters
            .iter()
            .filter(|parameter| {
                !matches!(
                    parameter.ty,
                    HostAbiType::HostSessionHandle | HostAbiType::OutputPointer(_)
                )
            })
            .cloned()
            .collect();

        let mut output_parameters = abi
            .parameters
            .iter()
            .filter(|parameter| matches!(parameter.ty, HostAbiType::OutputPointer(_)));
        let output_parameter = output_parameters.next().cloned();

        assert!(
            output_parameters.next().is_none(),
            "unsupported host request outputs for {}",
            abi.name
        );

        Self {
            abi,
            input_parameters,
            output_parameter,
        }
    }

    /// Return the authored request callback.
    pub(crate) fn abi(&self) -> &HostAbiFunction {
        &self.abi
    }

    /// Return the canonical request name.
    pub(crate) fn name(&self) -> &'static str {
        self.abi.name
    }

    /// Return the request documentation.
    pub(crate) fn documentation(&self) -> &'static str {
        self.abi.documentation
    }

    /// Return the ordered authored parameters.
    pub(crate) fn parameters(&self) -> &[HostAbiParameter] {
        &self.abi.parameters
    }

    /// Return the ordered input parameters.
    pub(crate) fn input_parameters(&self) -> &[HostAbiParameter] {
        &self.input_parameters
    }

    /// Return the optional output parameter.
    pub(crate) fn output_parameter(&self) -> Option<&HostAbiParameter> {
        self.output_parameter.as_ref()
    }

    /// Return whether this request produces one response payload.
    pub(crate) fn produces_response(&self) -> bool {
        self.output_parameter.is_some()
    }
}
