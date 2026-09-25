use std::collections::BTreeSet;

use serde::Serialize;
use tspp_core::StableHasher;
use tspp_serde::{Field, Name, Payload, Schema, Type};

use super::{Idempotency, MethodFingerprint, MethodId, MethodKind, ServiceFingerprint, ServiceId};

/// Canonical description of one RPC service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceSchema {
    /// Stable service identifier.
    id: ServiceId,
    /// Canonical qualified service name.
    name: String,
    /// Canonical method descriptions.
    methods: Vec<MethodSchema>,
    /// Value schemas reachable from the methods.
    types: Schema,
    /// Canonical schema fingerprint.
    fingerprint: ServiceFingerprint,
}

impl ServiceSchema {
    /// Build and validate one canonical service schema.
    pub fn new(
        name: impl Into<String>,
        mut methods: Vec<MethodSchema>,
        mut types: Schema,
    ) -> Result<Self, ServiceSchemaError> {
        let name = name.into();
        let id = ServiceId::for_name(&name);
        let mut method_ids = BTreeSet::new();
        let mut method_names = BTreeSet::new();

        // reject duplicate method identities and names
        for method in &methods {
            let expected = MethodId::for_name(&name, &method.name);
            if method.id != expected {
                return Err(ServiceSchemaError::InvalidMethodId {
                    method: method.name.clone(),
                    expected,
                    actual: method.id,
                });
            }
            if !method_ids.insert(method.id) {
                return Err(ServiceSchemaError::DuplicateMethodId(method.id));
            }
            if !method_names.insert(method.name.clone()) {
                return Err(ServiceSchemaError::DuplicateMethodName(method.name.clone()));
            }

            // require stream types to agree exactly with the declared method kind
            let expected = match method.kind {
                MethodKind::Unary => (false, false),
                MethodKind::ServerStreaming => (false, true),
                MethodKind::ClientStreaming => (true, false),
                MethodKind::BidirectionalStreaming => (true, true),
            };
            let actual = (method.input.is_some(), method.output.is_some());
            if actual != expected {
                return Err(ServiceSchemaError::InvalidMethodKind {
                    method: method.name.clone(),
                    kind: method.kind,
                });
            }
        }

        // canonicalize declaration order before hashing and publishing the schema
        methods.sort_unstable_by(|left, right| left.name.cmp(&right.name));
        for names in types.modules.values_mut() {
            names.sort_unstable();
        }

        // fingerprint each method from only its reachable wire types
        for method in &mut methods {
            method.fingerprint = MethodSchema::calculate_fingerprint(
                method.id,
                &method.name,
                method.kind,
                method.idempotency,
                &method.request,
                &method.response,
                method.input.as_ref(),
                method.output.as_ref(),
                &types,
            )?;
        }

        // fingerprint only canonical behavior, excluding the fingerprint itself
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.rpc.service.v1");
        let canonical_types = Self::canonical_types(&types);
        tspp_serde::hash_into(&(id, &name, &methods, canonical_types), &mut hasher)
            .map_err(ServiceSchemaError::Encode)?;
        let fingerprint = ServiceFingerprint(hasher.finish_u128());

        Ok(Self {
            id,
            name,
            methods,
            types,
            fingerprint,
        })
    }

    /// Return this service's stable identifier.
    pub const fn id(&self) -> ServiceId {
        self.id
    }

    /// Return this service's canonical qualified name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return this service's canonical methods.
    pub fn methods(&self) -> &[MethodSchema] {
        &self.methods
    }

    /// Return the value schemas reachable from this service.
    pub const fn types(&self) -> &Schema {
        &self.types
    }

    /// Return this service's canonical fingerprint.
    pub const fn fingerprint(&self) -> ServiceFingerprint {
        self.fingerprint
    }

    /// Return one method by identifier.
    pub fn method(&self, id: MethodId) -> Option<&MethodSchema> {
        self.methods.iter().find(|method| method.id == id)
    }

    /// Return canonical wire types without descriptive reflection metadata.
    fn canonical_types(types: &Schema) -> Schema {
        let mut types = types.clone();

        // canonicalize declaration order before hashing the reachable graph
        for names in types.modules.values_mut() {
            names.sort_unstable();
        }

        // exclude documentation from the wire fingerprint
        for item in types.items.values_mut() {
            item.docs.clear();
            match &mut item.ty {
                Type::Struct(fields) => Self::clear_field_docs(fields),
                Type::Enum(variants) => {
                    for variant in variants {
                        variant.docs.clear();
                        match &mut variant.payload {
                            Payload::Struct(fields) => Self::clear_field_docs(fields),
                            Payload::Unit | Payload::Value(_) => {}
                        }
                    }
                }
                _ => {}
            }
        }

        types
    }

    /// Remove descriptive documentation from schema fields.
    fn clear_field_docs(fields: &mut [Field]) {
        for field in fields {
            field.docs.clear();
        }
    }
}

/// Canonical description of one RPC method.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MethodSchema {
    /// Stable method identifier.
    id: MethodId,
    /// Stable method name within its service.
    name: String,
    /// Method streaming behavior.
    kind: MethodKind,
    /// Method behavior under repeated calls.
    idempotency: Idempotency,
    /// Initial request value type.
    request: Type,
    /// Terminal response value type.
    response: Type,
    /// Caller-to-callee stream item type when present.
    input: Option<Type>,
    /// Callee-to-caller stream item type when present.
    output: Option<Type>,
    /// Exact canonical method contract fingerprint.
    fingerprint: MethodFingerprint,
}

impl MethodSchema {
    /// Create one method schema from its exact value types.
    fn new(
        service: &str,
        name: impl Into<String>,
        kind: MethodKind,
        request: Type,
        response: Type,
        input: Option<Type>,
        output: Option<Type>,
        types: &Schema,
    ) -> Result<Self, ServiceSchemaError> {
        let name = name.into();
        let id = MethodId::for_name(service, &name);
        let fingerprint = Self::calculate_fingerprint(
            id,
            &name,
            kind,
            Idempotency::Unknown,
            &request,
            &response,
            input.as_ref(),
            output.as_ref(),
            types,
        )?;

        Ok(Self {
            id,
            name,
            kind,
            idempotency: Idempotency::Unknown,
            request,
            response,
            input,
            output,
            fingerprint,
        })
    }

    /// Set this method's behavior under repeated calls.
    pub fn with_idempotency(
        mut self,
        idempotency: Idempotency,
        types: &Schema,
    ) -> Result<Self, ServiceSchemaError> {
        self.idempotency = idempotency;

        // keep the published contract and its fingerprint inseparable
        self.fingerprint = Self::calculate_fingerprint(
            self.id,
            &self.name,
            self.kind,
            self.idempotency,
            &self.request,
            &self.response,
            self.input.as_ref(),
            self.output.as_ref(),
            types,
        )?;

        Ok(self)
    }

    /// Return this method's stable identifier.
    pub const fn id(&self) -> MethodId {
        self.id
    }

    /// Return this method's stable name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return this method's streaming behavior.
    pub const fn kind(&self) -> MethodKind {
        self.kind
    }

    /// Return this method's repeated-call behavior.
    pub const fn idempotency(&self) -> Idempotency {
        self.idempotency
    }

    /// Return this method's initial request type.
    pub const fn request(&self) -> &Type {
        &self.request
    }

    /// Return this method's terminal response type.
    pub const fn response(&self) -> &Type {
        &self.response
    }

    /// Return this method's caller stream item type.
    pub fn input(&self) -> Option<&Type> {
        self.input.as_ref()
    }

    /// Return this method's service stream item type.
    pub fn output(&self) -> Option<&Type> {
        self.output.as_ref()
    }

    /// Return this method's exact canonical contract fingerprint.
    pub const fn fingerprint(&self) -> MethodFingerprint {
        self.fingerprint
    }

    /// Create one unary method schema.
    pub fn unary(
        service: &str,
        name: impl Into<String>,
        request: Type,
        response: Type,
        types: &Schema,
    ) -> Result<Self, ServiceSchemaError> {
        Self::new(
            service,
            name,
            MethodKind::Unary,
            request,
            response,
            None,
            None,
            types,
        )
    }

    /// Create one server-streaming method schema.
    pub fn server_streaming(
        service: &str,
        name: impl Into<String>,
        request: Type,
        response: Type,
        output: Type,
        types: &Schema,
    ) -> Result<Self, ServiceSchemaError> {
        Self::new(
            service,
            name,
            MethodKind::ServerStreaming,
            request,
            response,
            None,
            Some(output),
            types,
        )
    }

    /// Create one client-streaming method schema.
    pub fn client_streaming(
        service: &str,
        name: impl Into<String>,
        request: Type,
        response: Type,
        input: Type,
        types: &Schema,
    ) -> Result<Self, ServiceSchemaError> {
        Self::new(
            service,
            name,
            MethodKind::ClientStreaming,
            request,
            response,
            Some(input),
            None,
            types,
        )
    }

    /// Create one bidirectional-streaming method schema.
    pub fn bidirectional_streaming(
        service: &str,
        name: impl Into<String>,
        request: Type,
        response: Type,
        input: Type,
        output: Type,
        types: &Schema,
    ) -> Result<Self, ServiceSchemaError> {
        Self::new(
            service,
            name,
            MethodKind::BidirectionalStreaming,
            request,
            response,
            Some(input),
            Some(output),
            types,
        )
    }

    /// Fingerprint one method and exactly the type graph it can exchange.
    fn calculate_fingerprint(
        id: MethodId,
        name: &str,
        kind: MethodKind,
        idempotency: Idempotency,
        request: &Type,
        response: &Type,
        input: Option<&Type>,
        output: Option<&Type>,
        types: &Schema,
    ) -> Result<MethodFingerprint, ServiceSchemaError> {
        let mut names = BTreeSet::new();
        Self::collect_type(request, types, &mut names)?;
        Self::collect_type(response, types, &mut names)?;
        if let Some(input) = input {
            Self::collect_type(input, types, &mut names)?;
        }
        if let Some(output) = output {
            Self::collect_type(output, types, &mut names)?;
        }

        // retain only named types reachable from this method
        let mut reachable = types.clone();
        reachable.items.retain(|name, _item| names.contains(name));
        reachable.modules.retain(|_module, module_names| {
            module_names.retain(|name| names.contains(name));

            !module_names.is_empty()
        });
        let reachable = ServiceSchema::canonical_types(&reachable);

        // hash exact behavior independently from unrelated service methods
        let mut hasher = StableHasher::new();
        hasher.update_len_prefixed(b"tspp.rpc.method.v1");
        tspp_serde::hash_into(
            &(
                id,
                name,
                kind,
                idempotency,
                request,
                response,
                input,
                output,
                reachable,
            ),
            &mut hasher,
        )
        .map_err(ServiceSchemaError::Encode)?;

        Ok(MethodFingerprint(hasher.finish_u128()))
    }

    /// Collect every named type reachable from one type.
    fn collect_type(
        ty: &Type,
        types: &Schema,
        names: &mut BTreeSet<Name>,
    ) -> Result<(), ServiceSchemaError> {
        match ty {
            Type::Option(inner) | Type::Sequence(inner) => Self::collect_type(inner, types, names),
            Type::Array { item, .. } => Self::collect_type(item, types, names),
            Type::Tuple(elements) => {
                for element in elements {
                    Self::collect_type(element, types, names)?;
                }

                Ok(())
            }
            Type::Map { key, value } => {
                Self::collect_type(key, types, names)?;
                Self::collect_type(value, types, names)
            }
            Type::Named(name) => Self::collect_named_type(name, types, names),
            Type::Struct(fields) => Self::collect_fields(fields, types, names),
            Type::Enum(variants) => {
                for variant in variants {
                    match &variant.payload {
                        Payload::Unit => {}
                        Payload::Value(ty) => Self::collect_type(ty, types, names)?,
                        Payload::Struct(fields) => Self::collect_fields(fields, types, names)?,
                    }
                }

                Ok(())
            }
            Type::Unit
            | Type::Bool
            | Type::Signed { .. }
            | Type::Unsigned { .. }
            | Type::Usize
            | Type::Float { .. }
            | Type::Char
            | Type::String => Ok(()),
        }
    }

    /// Collect one named type and its transitive field types.
    fn collect_named_type(
        name: &Name,
        types: &Schema,
        names: &mut BTreeSet<Name>,
    ) -> Result<(), ServiceSchemaError> {
        if !names.insert(name.clone()) {
            return Ok(());
        }
        let item = types
            .items
            .get(name)
            .ok_or_else(|| ServiceSchemaError::MissingType(name.clone()))?;

        Self::collect_type(&item.ty, types, names)
    }

    /// Collect named types reachable through schema fields.
    fn collect_fields(
        fields: &[Field],
        types: &Schema,
        names: &mut BTreeSet<Name>,
    ) -> Result<(), ServiceSchemaError> {
        for field in fields {
            Self::collect_type(&field.ty, types, names)?;
        }

        Ok(())
    }
}

/// Failure to build one service schema.
#[derive(Debug)]
pub enum ServiceSchemaError {
    /// One method identifier does not match its canonical name.
    InvalidMethodId {
        /// Method name.
        method: String,
        /// Expected canonical identifier.
        expected: MethodId,
        /// Actual identifier.
        actual: MethodId,
    },
    /// Two methods have the same identifier.
    DuplicateMethodId(MethodId),
    /// Two methods have the same name.
    DuplicateMethodName(String),
    /// A method's stream types do not match its declared kind.
    InvalidMethodKind {
        /// Method name.
        method: String,
        /// Declared method kind.
        kind: MethodKind,
    },
    /// One method references a named type absent from its service schema.
    MissingType(Name),
    /// Canonical schema encoding failed.
    Encode(tspp_serde::Error),
}

impl std::fmt::Display for ServiceSchemaError {
    /// Format this service schema failure.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMethodId {
                method,
                expected,
                actual,
            } => write!(
                formatter,
                "method {method} has id {actual:?}, expected {expected:?}"
            ),
            Self::DuplicateMethodId(id) => write!(formatter, "duplicate method id {id:?}"),
            Self::DuplicateMethodName(name) => write!(formatter, "duplicate method name {name}"),
            Self::InvalidMethodKind { method, kind } => {
                write!(
                    formatter,
                    "method {method} has invalid {kind:?} stream types"
                )
            }
            Self::MissingType(name) => {
                write!(formatter, "service schema is missing type {name:?}")
            }
            Self::Encode(error) => write!(formatter, "service schema encoding failed: {error}"),
        }
    }
}

impl std::error::Error for ServiceSchemaError {
    /// Return the underlying serialization failure when one exists.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            _ => None,
        }
    }
}
