use anyhow::Result;
use destack_rpc as rpc;

use crate::generate::core::lower_camel;
use crate::generate::schema::{Schema, Type};

/// One generated TypeScript RPC method.
pub(super) struct Method {
    /// Stable method identifier.
    pub(super) id: rpc::MethodId,
    /// Exact method fingerprint.
    pub(super) fingerprint: u128,
    /// TypeScript method name.
    pub(super) name: String,
    /// Method streaming behavior.
    pub(super) kind: rpc::MethodKind,
    /// Method behavior under repeated calls.
    pub(super) idempotency: rpc::Idempotency,
    /// Initial request type.
    pub(super) request: Type,
    /// Terminal response type.
    pub(super) response: Type,
    /// Caller stream item type.
    pub(super) input: Option<Type>,
    /// Service stream item type.
    pub(super) output: Option<Type>,
}

impl Method {
    /// Convert one reflected RPC method.
    pub(super) fn from_schema(method: &rpc::MethodSchema, schema: &Schema) -> Result<Self> {
        // convert the complete method value grammar
        let request = schema.ty(method.request().clone())?;
        let response = schema.ty(method.response().clone())?;
        let input = method
            .input()
            .cloned()
            .map(|input| schema.ty(input))
            .transpose()?;
        let output = method
            .output()
            .cloned()
            .map(|output| schema.ty(output))
            .transpose()?;

        Ok(Self {
            id: method.id(),
            fingerprint: method.fingerprint().0,
            name: lower_camel(method.name()),
            kind: method.kind(),
            idempotency: method.idempotency(),
            request,
            response,
            input,
            output,
        })
    }

    /// Visit every named generator key referenced by this method.
    pub(super) fn visit_keys(&self, visit: &mut impl FnMut(&str)) {
        let types = [
            Some(&self.request),
            Some(&self.response),
            self.input.as_ref(),
            self.output.as_ref(),
        ];

        for ty in types.into_iter().flatten() {
            ty.visit_refs(visit);
        }
    }
}
