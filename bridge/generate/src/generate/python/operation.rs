use crate::generate::core::{to_snake, upper_camel};
use crate::generate::schema::{Field, Item, Payload, Schema, Shape, Type, Variant};

use super::item::render_protocol_type;
use super::name::{python_field_name, python_parameter_name};

/// One exact Python workspace client operation.
pub(super) struct WorkspaceOperation {
    /// Method name.
    pub(super) method: String,
    /// Method documentation.
    pub(super) doc: String,
    /// Request expression.
    pub(super) request: String,
    /// Response variant class.
    pub(super) response_class: String,
    /// Response payload field.
    pub(super) response_field: String,
    /// Method parameters after the root handle.
    pub(super) parameters: Vec<WorkspaceParameter>,
    /// Method return type.
    pub(super) output: String,
}

/// One exact Python workspace client method parameter.
pub(super) struct WorkspaceParameter {
    /// Parameter name.
    pub(super) name: String,
    /// Parameter Python type.
    pub(super) ty: String,
}

impl WorkspaceOperation {
    /// Return exact Python request operations for one workspace client.
    pub(super) fn requests(schema: &Schema) -> Vec<Self> {
        let request = schema.item("WorkspaceRequest");
        let Shape::Enum(variants) = &request.shape else {
            return Vec::new();
        };

        variants
            .iter()
            .filter_map(|variant| Self::request(schema, variant))
            .collect()
    }

    /// Return exact Python query operations for one workspace client.
    pub(super) fn queries(schema: &Schema) -> Vec<Self> {
        let query = schema.item("WorkspaceQuery");
        let Shape::Enum(variants) = &query.shape else {
            return Vec::new();
        };

        variants
            .iter()
            .filter_map(|variant| Self::query(schema, variant))
            .collect()
    }

    /// Return one exact Python request operation when the variant is root scoped.
    fn request(schema: &Schema, variant: &Variant) -> Option<Self> {
        let response = python_workspace_response_kind(&variant.label())?;
        let parameters = python_workspace_request_parameters(schema, variant)?;
        let request = python_workspace_request_expression(schema, variant)?;
        let response_field =
            python_enum_variant_payload_field(schema.item("WorkspaceResponse"), &response)?;
        let output =
            python_enum_variant_payload_type(schema, schema.item("WorkspaceResponse"), &response)?;

        Some(Self {
            method: to_snake(&variant.label()),
            doc: variant.doc().to_string(),
            request,
            response_class: format!("WorkspaceResponse{}", upper_camel(&response)),
            response_field,
            parameters,
            output,
        })
    }

    /// Return one exact Python query operation when the variant is root scoped.
    fn query(schema: &Schema, variant: &Variant) -> Option<Self> {
        let fields = match &variant.payload {
            Payload::Struct(fields) => fields,
            Payload::Unit | Payload::Tuple(_) => return None,
        };
        let parameters = python_workspace_root_parameters(schema, fields)?;
        let fields = python_workspace_root_field_names(fields)?;
        let fields = python_workspace_constructor_fields(fields);
        let response = python_workspace_query_response_kind(&variant.label());
        let response_field =
            python_enum_variant_payload_field(schema.item("WorkspaceQueryResponse"), &response)?;
        let output = python_enum_variant_payload_type(
            schema,
            schema.item("WorkspaceQueryResponse"),
            &response,
        )?;

        Some(Self {
            method: to_snake(&variant.label()),
            doc: variant.doc().to_string(),
            request: format!("WorkspaceQuery{}({fields})", upper_camel(&variant.label())),
            response_class: format!("WorkspaceQueryResponse{}", upper_camel(&response)),
            response_field,
            parameters,
            output,
        })
    }
}

/// Return the exact Python request expression for one root scoped request.
fn python_workspace_request_expression(schema: &Schema, variant: &Variant) -> Option<String> {
    let class = format!("WorkspaceRequest{}", upper_camel(&variant.label()));

    match &variant.payload {
        Payload::Struct(fields) => {
            let fields = python_workspace_root_field_names(fields)?;
            let fields = python_workspace_constructor_fields(fields);

            Some(format!("{class}({fields})"))
        }
        Payload::Tuple(Type::Named(name)) => {
            let Shape::Struct(fields) = &schema.item(name).shape else {
                return None;
            };
            let payload = python_parameter_name(&variant.payload_field_name());
            let payload_class = name.as_str();
            let fields = python_workspace_root_field_names(fields)?;
            let fields = python_workspace_constructor_fields(fields);

            Some(format!("{class}({payload}={payload_class}({fields}))"))
        }
        Payload::Unit | Payload::Tuple(_) => None,
    }
}

/// Return root scoped request fields excluding the root handle.
fn python_workspace_request_parameters(
    schema: &Schema,
    variant: &Variant,
) -> Option<Vec<WorkspaceParameter>> {
    match &variant.payload {
        Payload::Struct(fields) => python_workspace_root_parameters(schema, fields),
        Payload::Tuple(Type::Named(name)) => {
            let Shape::Struct(fields) = &schema.item(name).shape else {
                return None;
            };

            python_workspace_root_parameters(schema, fields)
        }
        Payload::Unit | Payload::Tuple(_) => None,
    }
}

/// Return root scoped field names excluding the root handle.
fn python_workspace_root_field_names(fields: &[Field]) -> Option<Vec<String>> {
    if !fields.iter().any(|field| field.name == "handle") {
        return None;
    }

    Some(
        fields
            .iter()
            .filter(|field| field.name != "handle")
            .map(|field| python_field_name(&field.name))
            .collect(),
    )
}

/// Return root scoped Python parameters excluding the root handle.
fn python_workspace_root_parameters(
    schema: &Schema,
    fields: &[Field],
) -> Option<Vec<WorkspaceParameter>> {
    if !fields.iter().any(|field| field.name == "handle") {
        return None;
    }

    Some(
        fields
            .iter()
            .filter(|field| field.name != "handle")
            .map(|field| WorkspaceParameter {
                name: python_field_name(&field.name),
                ty: render_protocol_type(schema, &field.ty),
            })
            .collect(),
    )
}

/// Return constructor fields for one Python root scoped request.
fn python_workspace_constructor_fields(fields: Vec<String>) -> String {
    let mut constructor_fields = vec!["handle=self._handle".to_string()];
    constructor_fields.extend(fields.into_iter().map(|field| format!("{field}={field}")));

    constructor_fields.join(", ")
}

/// Return the response kind for one workspace request kind.
fn python_workspace_response_kind(request: &str) -> Option<String> {
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
fn python_workspace_query_response_kind(query: &str) -> String {
    match query {
        "execute" => "query".to_string(),
        "executeBatch" => "queryBatch".to_string(),
        _ => query.to_string(),
    }
}

/// Return one enum variant payload field by variant kind.
fn python_enum_variant_payload_field(item: &Item, kind: &str) -> Option<String> {
    let Shape::Enum(variants) = &item.shape else {
        return None;
    };
    let variant = variants.iter().find(|variant| variant.label() == kind)?;

    match &variant.payload {
        Payload::Tuple(_) => Some(python_parameter_name(&variant.payload_field_name())),
        Payload::Struct(fields) if fields.len() == 1 => Some(python_field_name(&fields[0].name)),
        Payload::Unit | Payload::Struct(_) => None,
    }
}

/// Return one enum variant payload type by variant kind.
fn python_enum_variant_payload_type(schema: &Schema, item: &Item, kind: &str) -> Option<String> {
    let Shape::Enum(variants) = &item.shape else {
        return None;
    };
    let variant = variants.iter().find(|variant| variant.label() == kind)?;

    match &variant.payload {
        Payload::Tuple(ty) => Some(render_protocol_type(schema, ty)),
        Payload::Struct(fields) if fields.len() == 1 => {
            Some(render_protocol_type(schema, &fields[0].ty))
        }
        Payload::Unit | Payload::Struct(_) => None,
    }
}
