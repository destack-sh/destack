use std::path::Path;

use anyhow::Result;

use crate::generate::core::write_text;
use crate::generate::schema::Schema;

use super::codec::property_access;
use super::operation::{WorkspaceQueryOperation, WorkspaceRequestOperation};
use super::text::{GENERATED_HEADER, Text};

pub(super) fn generate_protocol_workspace_client(root: &Path, schema: &Schema) -> Result<()> {
    let content = render_protocol_workspace_client(schema);

    write_text(
        root,
        "client/typescript/src/_generated/protocol/workspace/client.ts",
        content,
    )
}

/// Render the exact TypeScript workspace protocol client.
fn render_protocol_workspace_client(schema: &Schema) -> String {
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();
    text.line("import type { ProtocolError } from \"../error.js\";");
    text.line("import type { WorkspaceQueryResponse } from \"../query.js\";");
    text.line("import { WorkspaceQuery } from \"../query.js\";");
    text.line("import { WorkspaceRequest } from \"../request.js\";");
    text.line("import type { WorkspaceResponse } from \"../response.js\";");
    text.line("import type { RootId } from \"../root.js\";");
    text.line("import type { Connection } from \"../../../protocol/connection/index.js\";");
    text.blank();
    text.line("type Request<K extends WorkspaceRequest[\"kind\"]> = Extract<WorkspaceRequest, { readonly kind: K }>;");
    text.line("type Response<K extends WorkspaceResponse[\"kind\"]> = Extract<WorkspaceResponse, { readonly kind: K }>;");
    text.line("type Query<K extends WorkspaceQuery[\"kind\"]> = Extract<WorkspaceQuery, { readonly kind: K }>;");
    text.line("type QueryResponse<K extends WorkspaceQueryResponse[\"kind\"]> = Extract<WorkspaceQueryResponse, { readonly kind: K }>;");
    text.blank();

    render_workspace_client_class(schema, &mut text);
    render_workspace_client_response_functions(&mut text);

    text.finish()
}

/// Render the exact workspace client class.
fn render_workspace_client_class(schema: &Schema, text: &mut Text) {
    text.doc("Exact workspace protocol client for one opened root.", "");
    text.line("export class WorkspaceClient {");
    text.line("    readonly #connection: Connection;");
    text.line("    readonly #handle: RootId;");
    text.blank();
    text.doc("Create one exact workspace protocol client.", "    ");
    text.line("    constructor(connection: Connection, handle: RootId) {");
    text.line("        this.#connection = connection;");
    text.line("        this.#handle = handle;");
    text.line("    }");
    text.blank();
    text.doc("Return the backing protocol connection.", "    ");
    text.line("    connection(): Connection {");
    text.line("        return this.#connection;");
    text.line("    }");
    text.blank();
    text.doc("Return the opened root handle.", "    ");
    text.line("    handle(): RootId {");
    text.line("        return this.#handle;");
    text.line("    }");
    text.blank();

    for operation in WorkspaceRequestOperation::all(schema) {
        render_workspace_request_method(text, &operation);
    }

    for operation in WorkspaceQueryOperation::all(schema) {
        render_workspace_query_method(text, &operation);
    }

    text.doc("Run one exact workspace query.", "    ");
    text.line("    async query(query: WorkspaceQuery): Promise<WorkspaceQueryResponse> {");
    text.line(
        "        const response = await this.#connection.request(WorkspaceRequest.query(query));",
    );
    text.blank();
    text.line("        return expectResponse(response, \"queryResult\").query_result;");
    text.line("    }");
    text.line("}");
    text.blank();
}

/// Render one exact workspace request method.
fn render_workspace_request_method(text: &mut Text, operation: &WorkspaceRequestOperation) {
    let parameters = operation
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");

    text.doc(&operation.doc, "    ");
    text.line(format!(
        "    async {}({parameters}): Promise<{}> {{",
        operation.method, operation.output
    ));
    text.line(format!(
        "        const response = await this.#connection.request({});",
        operation.request
    ));
    text.blank();
    let response = format!("expectResponse(response, {:?})", operation.response_kind);
    let value = property_access(&response, &operation.response_field);

    text.line(format!("        return {value};"));
    text.line("    }");
    text.blank();
}

/// Render one exact workspace query method.
fn render_workspace_query_method(text: &mut Text, operation: &WorkspaceQueryOperation) {
    let parameters = operation
        .parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, parameter.ty))
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = operation
        .parameters
        .iter()
        .map(|parameter| parameter.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = if arguments.is_empty() {
        "this.#handle".to_string()
    } else {
        format!("this.#handle, {arguments}")
    };

    text.doc(&operation.doc, "    ");
    text.line(format!(
        "    async {}({parameters}): Promise<{}> {{",
        operation.method, operation.output
    ));
    text.line(format!(
        "        const response = await this.query(WorkspaceQuery.{}({arguments}));",
        operation.constructor
    ));
    text.blank();
    let response = format!("expectQuery(response, {:?})", operation.response_kind);
    let value = property_access(&response, &operation.response_field);

    text.line(format!("        return {value};"));
    text.line("    }");
    text.blank();
}

/// Render exact workspace response functions.
fn render_workspace_client_response_functions(text: &mut Text) {
    text.doc(
        "Return one response variant or throw its protocol error.",
        "",
    );
    text.line("function expectResponse<K extends WorkspaceResponse[\"kind\"]>(");
    text.line("    response: WorkspaceResponse,");
    text.line("    kind: K,");
    text.line("): Response<K> {");
    text.line("    if (response.kind === kind) {");
    text.line("        return response as Response<K>;");
    text.line("    }");
    text.blank();
    text.line("    if (response.kind === \"error\") {");
    text.line("        throw protocolError(response.error);");
    text.line("    }");
    text.blank();
    text.line("    throw new Error(`expected ${kind} response, got ${response.kind}`);");
    text.line("}");
    text.blank();
    text.doc("Return one query response variant.", "");
    text.line("function expectQuery<K extends WorkspaceQueryResponse[\"kind\"]>(");
    text.line("    response: WorkspaceQueryResponse,");
    text.line("    kind: K,");
    text.line("): QueryResponse<K> {");
    text.line("    if (response.kind === kind) {");
    text.line("        return response as QueryResponse<K>;");
    text.line("    }");
    text.blank();
    text.line("    throw new Error(`expected ${kind} query response, got ${response.kind}`);");
    text.line("}");
    text.blank();
    text.doc("Convert one protocol error into a JavaScript error.", "");
    text.line("function protocolError(error: ProtocolError): Error {");
    text.line("    return new Error(`${error.code}: ${error.message}`);");
    text.line("}");
}
