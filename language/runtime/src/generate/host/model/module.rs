use destack_runtime::host::abi::describe::{
    HostAbiField, HostAbiModule, HostAbiNamedType, HostAbiNamedTypeDefinition,
};

use super::{HostIngress, HostRequest};

/// One resolved host named type.
#[derive(Clone)]
pub(crate) struct HostType {
    /// The authored named type.
    abi: HostAbiNamedType,
    /// Whether this type is one enum.
    is_enum: bool,
    /// The struct fields for this type.
    fields: Vec<HostAbiField>,
}

impl HostType {
    /// Build one resolved host type from one authored named type.
    pub(crate) fn new(abi: HostAbiNamedType) -> Self {
        // type shape
        let (is_enum, fields) = match &abi.definition {
            HostAbiNamedTypeDefinition::Struct { fields } => (false, fields.clone()),
            HostAbiNamedTypeDefinition::Enum { .. } => (true, Vec::new()),
        };

        Self {
            abi,
            is_enum,
            fields,
        }
    }

    /// Return the authored named type.
    pub(crate) fn abi(&self) -> &HostAbiNamedType {
        &self.abi
    }

    /// Return the canonical type name.
    pub(crate) fn name(&self) -> &'static str {
        self.abi.name
    }

    /// Return whether this type is one enum.
    pub(crate) fn is_enum(&self) -> bool {
        self.is_enum
    }

    /// Return the struct fields for this type.
    pub(crate) fn fields(&self) -> &[HostAbiField] {
        &self.fields
    }

    /// Validate the resolved named-type shape.
    fn validate(&self) {
        // enum shape
        if self.is_enum() {
            assert!(
                self.fields().is_empty(),
                "unexpected struct fields for enum host type {}",
                self.name()
            );

            return;
        }

        // struct shape
        let HostAbiNamedTypeDefinition::Struct { fields } = &self.abi().definition else {
            panic!("missing struct definition for host type {}", self.name());
        };

        assert_eq!(
            self.fields().len(),
            fields.len(),
            "mismatched field count for host type {}",
            self.name()
        );
    }
}

/// One resolved host generator module.
#[derive(Clone)]
pub(crate) struct HostModule {
    /// The authored host ABI module.
    abi: HostAbiModule,
    /// The resolved request surfaces.
    requests: Vec<HostRequest>,
    /// The resolved ingress surfaces.
    ingresses: Vec<HostIngress>,
    /// The resolved named types.
    types: Vec<HostType>,
}

impl HostModule {
    /// Build one resolved host module from one authored host ABI module.
    pub(crate) fn new(abi: HostAbiModule) -> Self {
        // resolved surfaces
        let requests = abi.requests.iter().cloned().map(HostRequest::new).collect();
        let ingresses = abi.ingress.iter().cloned().map(HostIngress::new).collect();
        let types = abi.types.iter().cloned().map(HostType::new).collect();

        let module = Self {
            abi,
            requests,
            ingresses,
            types,
        };

        // resolved invariants
        module.validate();
        module
    }

    /// Return the authored host ABI module.
    pub(crate) fn abi(&self) -> &HostAbiModule {
        &self.abi
    }

    /// Return the canonical module name.
    pub(crate) fn name(&self) -> &'static str {
        self.abi.name
    }

    /// Return the resolved request surfaces.
    pub(crate) fn requests(&self) -> &[HostRequest] {
        &self.requests
    }

    /// Return the resolved named types.
    pub(crate) fn types(&self) -> &[HostType] {
        &self.types
    }

    /// Return whether this module has one ingress surface.
    pub(crate) fn has_ingress(&self) -> bool {
        !self.ingresses.is_empty()
    }

    /// Return the single ingress surface when present.
    pub(crate) fn single_ingress(&self) -> Option<&HostIngress> {
        self.ingresses.first()
    }

    /// Validate the resolved module shape.
    fn validate(&self) {
        use std::collections::BTreeSet;

        // named-type shape
        let mut names = BTreeSet::new();
        for named_type in self.types() {
            named_type.validate();

            assert!(
                names.insert(named_type.name()),
                "duplicate host type {} in module {}",
                named_type.name(),
                self.name()
            );
        }

        // ingress shape
        assert!(
            self.ingresses.len() <= 1,
            "unsupported ingress count for host module {}",
            self.name()
        );
    }
}
