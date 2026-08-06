use anyhow::Result;
use destack_rpc::{MethodId, MethodKind};

use crate::generate::core::lower_camel;
use crate::generate::schema::{Schema, Type};

/// One generated TypeScript RPC operation.
pub(super) struct WorkspaceOperation {
    /// Stable method identifier.
    pub id: MethodId,
    /// Exact method contract fingerprint.
    pub fingerprint: u128,
    /// TypeScript method name.
    pub method: String,
    /// Method streaming behavior.
    pub kind: MethodKind,
    /// Initial request type.
    pub request: Type,
    /// Terminal response type.
    pub response: Type,
    /// Caller stream item type.
    pub input: Option<Type>,
    /// Service stream item type.
    pub output: Option<Type>,
}

impl WorkspaceOperation {
    /// Convert every workspace method schema.
    pub(super) fn all(schema: &Schema) -> Result<Vec<Self>> {
        schema
            .service
            .methods()
            .iter()
            .map(|method| {
                Ok(Self {
                    id: method.id(),
                    fingerprint: method.fingerprint().0,
                    method: lower_camel(method.name()),
                    kind: method.kind(),
                    request: schema.ty(method.request().clone())?,
                    response: schema.ty(method.response().clone())?,
                    input: method
                        .input()
                        .cloned()
                        .map(|input| schema.ty(input))
                        .transpose()?,
                    output: method
                        .output()
                        .cloned()
                        .map(|output| schema.ty(output))
                        .transpose()?,
                })
            })
            .collect()
    }

    /// Visit every named generator key referenced by this operation.
    pub(super) fn visit_keys(&self, visit: &mut impl FnMut(&str)) -> Result<()> {
        self.request.visit_refs(&mut |key| {
            visit(key);

            Ok(())
        })?;
        self.response.visit_refs(&mut |key| {
            visit(key);

            Ok(())
        })?;
        if let Some(input) = &self.input {
            input.visit_refs(&mut |key| {
                visit(key);

                Ok(())
            })?;
        }
        if let Some(output) = &self.output {
            output.visit_refs(&mut |key| {
                visit(key);

                Ok(())
            })?;
        }

        Ok(())
    }
}
