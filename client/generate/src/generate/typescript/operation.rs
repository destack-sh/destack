use anyhow::{Context, Result, bail};

use crate::generate::schema::{Field, Item, Payload, Schema, Shape, Type, Variant};

use super::codec::type_property_key;

/// Exact workspace request operation.
pub(super) struct WorkspaceRequestOperation {
    /// Method name.
    pub(super) method: String,
    /// Method documentation.
    pub(super) doc: String,
    /// Request expression.
    pub(super) request: String,
    /// Response variant kind.
    pub(super) response_kind: String,
    /// Response payload field.
    pub(super) response_field: String,
    /// Method parameters after the root handle.
    pub(super) parameters: Vec<WorkspaceParameter>,
    /// Method return type.
    pub(super) output: String,
}

/// One exact workspace client method parameter.
pub(super) struct WorkspaceParameter {
    /// Parameter name.
    pub(super) name: String,
    /// Parameter TypeScript type.
    pub(super) ty: String,
}

impl WorkspaceRequestOperation {
    /// Return exact request operations for one workspace client.
    pub(super) fn all(schema: &Schema) -> Result<Vec<Self>> {
        let request = schema.named_item("WorkspaceRequest");
        let Shape::Enum(variants) = &request.shape else {
            bail!("WorkspaceRequest must be an enum");
        };

        let mut operations = Vec::new();
        for variant in variants {
            if let Some(operation) = Self::from_variant(schema, variant)? {
                operations.push(operation);
            }
        }

        Ok(operations)
    }

    /// Return one exact request operation when the variant is root scoped.
    fn from_variant(schema: &Schema, variant: &Variant) -> Result<Option<Self>> {
        let kind = variant.label();
        if !workspace_request_is_root_scoped(schema, variant) {
            return Ok(None);
        }

        // require every root request to have an exact generated operation
        let parameters = workspace_request_parameters(schema, variant)
            .with_context(|| format!("failed to render {kind} request parameters"))?;
        let request = workspace_request_expression(schema, variant)
            .with_context(|| format!("failed to render {kind} request expression"))?;
        let response = workspace_response_kind(&kind)
            .with_context(|| format!("workspace request {kind} has no response mapping"))?;
        let response_field =
            enum_variant_payload_field(schema.named_item("WorkspaceResponse"), &response)
                .with_context(|| format!("workspace response {response} has no payload field"))?;
        let output = format!(
            "Response<{:?}>[{}]",
            response,
            type_property_key(&response_field)
        );

        Ok(Some(WorkspaceRequestOperation {
            method: kind,
            doc: variant.doc().to_string(),
            request,
            response_kind: response,
            response_field,
            parameters,
            output,
        }))
    }
}

/// Return whether one workspace request carries a root handle.
fn workspace_request_is_root_scoped(schema: &Schema, variant: &Variant) -> bool {
    workspace_request_fields(schema, variant)
        .is_some_and(|fields| fields.iter().any(|field| field.name == "handle"))
}

/// Return the fields carried by one workspace request variant.
fn workspace_request_fields<'schema>(
    schema: &'schema Schema,
    variant: &'schema Variant,
) -> Option<&'schema [Field]> {
    match &variant.payload {
        Payload::Struct(fields) => Some(fields),
        Payload::Tuple(Type::Named { key, .. }) => {
            let Shape::Struct(fields) = &schema.item(key).shape else {
                return None;
            };

            Some(fields)
        }
        Payload::Unit | Payload::Tuple(_) => None,
    }
}

/// Return the exact request expression for one root scoped request.
fn workspace_request_expression(schema: &Schema, variant: &Variant) -> Option<String> {
    let constructor = variant.label();

    match &variant.payload {
        Payload::Struct(fields) => {
            let fields = workspace_root_field_names(fields)?;
            let arguments = workspace_constructor_arguments(fields);

            Some(format!("WorkspaceRequest.{constructor}({arguments})"))
        }
        Payload::Tuple(Type::Named { .. }) => {
            let fields = workspace_request_fields(schema, variant)?;
            let fields = workspace_root_field_names(fields)?;
            let fields = workspace_object_fields(fields);

            Some(format!("WorkspaceRequest.{constructor}({{ {fields} }})"))
        }
        Payload::Unit | Payload::Tuple(_) => None,
    }
}

/// Return root scoped request fields excluding the root handle.
fn workspace_request_parameters(
    schema: &Schema,
    variant: &Variant,
) -> Option<Vec<WorkspaceParameter>> {
    let kind = variant.label();
    match &variant.payload {
        Payload::Struct(fields) => workspace_root_parameters("Request", &kind, None, fields),
        Payload::Tuple(Type::Named { .. }) => {
            let fields = workspace_request_fields(schema, variant)?;
            let payload = variant.payload_field_name();

            workspace_root_parameters("Request", &kind, Some(&payload), fields)
        }
        Payload::Unit | Payload::Tuple(_) => None,
    }
}

/// Return root scoped field names excluding the root handle.
fn workspace_root_field_names(fields: &[Field]) -> Option<Vec<String>> {
    if !fields.iter().any(|field| field.name == "handle") {
        return None;
    }

    Some(
        fields
            .iter()
            .filter(|field| field.name != "handle")
            .map(Field::label)
            .collect(),
    )
}

/// Return positional constructor arguments for one struct request variant.
fn workspace_constructor_arguments(fields: Vec<String>) -> String {
    let mut arguments = vec!["this.#handle".to_string()];
    arguments.extend(fields);

    arguments.join(", ")
}

/// Return object constructor fields for one tuple request variant.
fn workspace_object_fields(fields: Vec<String>) -> String {
    let mut object_fields = vec!["handle: this.#handle".to_string()];
    object_fields.extend(fields);

    object_fields.join(", ")
}

/// Return parameters excluding a root handle.
fn workspace_root_parameters(
    root: &str,
    kind: &str,
    payload: Option<&str>,
    fields: &[Field],
) -> Option<Vec<WorkspaceParameter>> {
    if !fields.iter().any(|field| field.name == "handle") {
        return None;
    }

    Some(
        fields
            .iter()
            .filter(|field| field.name != "handle")
            .map(|field| {
                let name = field.label();
                let ty = if let Some(payload) = payload {
                    format!(
                        "{root}<{kind:?}>[{}][{}]",
                        type_property_key(payload),
                        type_property_key(&name)
                    )
                } else {
                    format!("{root}<{kind:?}>[{}]", type_property_key(&name))
                };

                WorkspaceParameter { name, ty }
            })
            .collect(),
    )
}

/// Return the response kind for one request kind.
fn workspace_response_kind(request: &str) -> Option<String> {
    let response = match request {
        "closeRoot" => "rootClosed",
        "reloadRoot" => "rootReloaded",
        "readRevision" => "readRevision",
        "applyFileOperation" => "fileOperationApplied",
        "applySourceUpdate" => "sourceUpdated",
        "isFileOpen" => "isFileOpen",
        "formatFile" => "formatFile",
        "readFiles" => "readFiles",
        "startWatch" => "watchStarted",
        "nextWatchBatch" => "watchBatchReady",
        "stopWatch" => "watchStopped",
        "artifact" => "artifactResult",
        "store" => "storeResult",
        "load" => "loadResult",
        "export" => "exportResult",
        "check" | "format" | "query" | "rewrite" | "build" | "run" | "test" | "doc" | "bench"
        | "info" | "targets" | "cache" | "settings" | "doctor" | "task" | "clean" | "diagnose"
        | "diagnoseFile" | "resolveQueryFile" | "runQuery" => request,
        _ => return None,
    };

    Some(response.to_string())
}

/// Return one enum variant payload field by variant kind.
fn enum_variant_payload_field(item: &Item, kind: &str) -> Option<String> {
    let Shape::Enum(variants) = &item.shape else {
        return None;
    };
    let variant = variants.iter().find(|variant| variant.label() == kind)?;

    match &variant.payload {
        Payload::Tuple(_) => Some(variant.payload_field_name()),
        Payload::Struct(fields) if fields.len() == 1 => Some(fields[0].label()),
        Payload::Unit | Payload::Struct(_) => None,
    }
}
