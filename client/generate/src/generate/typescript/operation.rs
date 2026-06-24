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

/// Exact workspace query operation.
pub(super) struct WorkspaceQueryOperation {
    /// Method name.
    pub(super) method: String,
    /// Method documentation.
    pub(super) doc: String,
    /// Query constructor name.
    pub(super) constructor: String,
    /// Query response variant kind.
    pub(super) response_kind: String,
    /// Query response payload field.
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
    pub(super) fn all(schema: &Schema) -> Vec<Self> {
        let request = schema.named_item("WorkspaceRequest");
        let Shape::Enum(variants) = &request.shape else {
            return Vec::new();
        };

        variants
            .iter()
            .filter_map(|variant| Self::from_variant(schema, variant))
            .collect()
    }

    /// Return one exact request operation when the variant is root scoped.
    fn from_variant(schema: &Schema, variant: &Variant) -> Option<Self> {
        let response = workspace_response_kind(&variant.label())?;
        let parameters = workspace_request_parameters(schema, variant)?;
        let request = workspace_request_expression(schema, variant)?;
        let response_field =
            enum_variant_payload_field(schema.named_item("WorkspaceResponse"), &response)?;
        let output = format!(
            "Response<{:?}>[{}]",
            response,
            type_property_key(&response_field)
        );

        Some(WorkspaceRequestOperation {
            method: variant.label(),
            doc: variant.doc().to_string(),
            request,
            response_kind: response,
            response_field,
            parameters,
            output,
        })
    }
}

impl WorkspaceQueryOperation {
    /// Return exact query operations for one workspace client.
    pub(super) fn all(schema: &Schema) -> Vec<Self> {
        let query = schema.named_item("WorkspaceQuery");
        let Shape::Enum(variants) = &query.shape else {
            return Vec::new();
        };

        variants
            .iter()
            .filter_map(|variant| Self::from_variant(schema, variant))
            .collect()
    }

    /// Return one exact query operation when the variant is root scoped.
    fn from_variant(schema: &Schema, variant: &Variant) -> Option<Self> {
        match &variant.payload {
            Payload::Struct(fields) => Self::from_fields(schema, variant, fields),
            Payload::Unit | Payload::Tuple(_) => None,
        }
    }

    /// Return one exact query operation from root scoped query fields.
    fn from_fields(schema: &Schema, variant: &Variant, fields: &[Field]) -> Option<Self> {
        let parameters = workspace_root_parameters("Query", &variant.label(), None, fields)?;
        let response = workspace_query_response_kind(&variant.label());
        let response_field =
            enum_variant_payload_field(schema.named_item("WorkspaceQueryResponse"), &response)?;
        let output = format!(
            "QueryResponse<{:?}>[{}]",
            response,
            type_property_key(&response_field)
        );

        Some(Self {
            method: variant.label(),
            doc: variant.doc().to_string(),
            constructor: variant.label(),
            response_kind: response,
            response_field,
            parameters,
            output,
        })
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
        Payload::Tuple(Type::Named { key, .. }) => {
            let Shape::Struct(fields) = &schema.item(key).shape else {
                return None;
            };
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
        Payload::Tuple(Type::Named { key, .. }) => {
            let Shape::Struct(fields) = &schema.item(key).shape else {
                return None;
            };
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
        "applyFileOperation" => "fileOperationApplied",
        "applySourceUpdate" => "sourceUpdated",
        "startWatch" => "watchStarted",
        "nextWatchBatch" => "watchBatchReady",
        "stopWatch" => "watchStopped",
        "artifact" => "artifactResult",
        "store" => "storeResult",
        "load" => "loadResult",
        "export" => "exportResult",
        "check" | "lint" | "format" | "build" | "run" | "test" | "doc" | "bench" | "info"
        | "inspect" | "manifest" | "targets" | "cache" | "settings" | "doctor" | "task"
        | "clean" => request,
        _ => return None,
    };

    Some(response.to_string())
}

/// Return the query response kind for one query kind.
fn workspace_query_response_kind(query: &str) -> String {
    match query {
        "execute" => "query".to_string(),
        "executeBatch" => "queryBatch".to_string(),
        _ => query.to_string(),
    }
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
