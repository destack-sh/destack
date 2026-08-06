use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use destack_rpc::MethodKind;

use crate::generate::core::write_text;
use crate::generate::schema::{Schema, Type};

use super::codec::{render_decode_type, render_encode_type};
use super::item::render_type;
use super::operation::WorkspaceOperation;
use super::path::TypeNames;
use super::text::{GENERATED_HEADER, Text};

/// Generate the typed workspace RPC client.
pub(super) fn generate_workspace_client(root: &Path, schema: &Schema) -> Result<()> {
    let content = render_workspace_client(schema)?;

    write_text(
        root,
        "client/typescript/src/_generated/workspace/client.ts",
        content,
    )
}

/// Render one complete workspace RPC client.
fn render_workspace_client(schema: &Schema) -> Result<String> {
    let operations = WorkspaceOperation::all(schema)?;
    let keys = operation_keys(&operations)?;
    let names = TypeNames::namespaced(schema, &keys);
    let mut text = Text::new();
    text.line(GENERATED_HEADER);
    text.blank();
    text.line("import type { Call, Encoder, Decoder, Method, RequestValue, RpcResponse } from \"../../rpc/index.js\";");
    text.line("import { Connection } from \"../../rpc/index.js\";");
    render_type_imports(schema, &keys, &names, &mut text);
    text.blank();

    render_service(schema, &mut text);
    for operation in &operations {
        render_method(schema, operation, &names, &mut text);
    }
    render_client(schema, &operations, &names, &mut text);

    Ok(text.finish())
}

/// Return all named type keys used by service methods.
fn operation_keys(operations: &[WorkspaceOperation]) -> Result<Vec<String>> {
    let mut keys = BTreeSet::new();

    for operation in operations {
        operation.visit_keys(&mut |key| {
            keys.insert(key.to_string());
        })?;
    }

    Ok(keys.into_iter().collect())
}

/// Render namespace imports for every referenced value module.
fn render_type_imports(schema: &Schema, keys: &[String], names: &TypeNames, text: &mut Text) {
    let mut imports = BTreeMap::new();

    for key in keys {
        let path = schema.module_path(key);
        let Some(namespace) = names.module(key) else {
            continue;
        };
        imports.insert(client_import_path(path.slash_path()), namespace.to_string());
    }
    for (path, namespace) in imports {
        text.line(format!("import * as {namespace} from \"{path}\";"));
    }
}

/// Return an import path from the generated workspace client.
fn client_import_path(target: String) -> String {
    if let Some(target) = target.strip_prefix("workspace/") {
        format!("./{target}.js")
    } else {
        format!("../{target}.js")
    }
}

/// Render the stable workspace service identifier.
fn render_service(schema: &Schema, text: &mut Text) {
    text.doc("Stable workspace RPC service identifier.", "");
    text.line(format!(
        "export const workspaceService = {}n;",
        schema.service.id().0
    ));
    text.blank();
}

/// Render one typed method descriptor and its codecs.
fn render_method(
    schema: &Schema,
    operation: &WorkspaceOperation,
    names: &TypeNames,
    text: &mut Text,
) {
    let request = render_type(schema, names, &operation.request);
    let response = render_type(schema, names, &operation.response);
    let input = operation
        .input
        .as_ref()
        .map(|ty| render_type(schema, names, ty))
        .unwrap_or_else(|| "never".to_string());
    let output = operation
        .output
        .as_ref()
        .map(|ty| render_type(schema, names, ty))
        .unwrap_or_else(|| "never".to_string());
    let constant = format!("{}Method", operation.method);

    render_encoder(schema, names, &operation.request, &constant, text);
    render_decoder(schema, names, &operation.response, &constant, text);
    if let Some(input) = &operation.input {
        render_input_encoder(schema, names, input, &constant, text);
    }
    if let Some(output) = &operation.output {
        render_output_decoder(schema, names, output, &constant, text);
    }

    text.doc(
        &format!("Descriptor for the {} RPC method.", operation.method),
        "",
    );
    text.line(format!(
        "const {constant}: Method<{request}, {response}, {input}, {output}> = {{"
    ));
    text.line(format!("    service: {}n,", schema.service.id().0));
    text.line(format!("    method: {}n,", operation.id.0));
    text.line(format!("    fingerprint: {}n,", operation.fingerprint));
    text.line(format!("    kind: {:?},", method_kind(operation.kind)));
    text.line(format!("    request: {constant}Request,"));
    text.line(format!("    response: {constant}Response,"));
    if operation.input.is_some() {
        text.line(format!("    input: {constant}Input,"));
    }
    if operation.output.is_some() {
        text.line(format!("    output: {constant}Output,"));
    }
    text.line("};");
    text.blank();
}

/// Render one request encoder.
fn render_encoder(schema: &Schema, names: &TypeNames, ty: &Type, name: &str, text: &mut Text) {
    let rendered = render_type(schema, names, ty);
    text.line(format!("const {name}Request: Encoder<{rendered}> = {{"));
    text.line(format!("    encode(writer, value: {rendered}): void {{"));
    render_encode_type(schema, names, text, ty, "value", "        ", 0);
    text.line("    },");
    text.line("};");
    text.blank();
}

/// Render one response decoder.
fn render_decoder(schema: &Schema, names: &TypeNames, ty: &Type, name: &str, text: &mut Text) {
    let rendered = render_type(schema, names, ty);
    let decode = render_decode_type(schema, names, ty, "reader", 0);
    text.line(format!("const {name}Response: Decoder<{rendered}> = {{"));
    text.line(format!("    decode(reader): {rendered} {{"));
    text.line(format!("        return {decode};"));
    text.line("    },");
    text.line("};");
    text.blank();
}

/// Render one caller stream encoder.
fn render_input_encoder(
    schema: &Schema,
    names: &TypeNames,
    ty: &Type,
    name: &str,
    text: &mut Text,
) {
    let rendered = render_type(schema, names, ty);
    text.line(format!("const {name}Input: Encoder<{rendered}> = {{"));
    text.line(format!("    encode(writer, value: {rendered}): void {{"));
    render_encode_type(schema, names, text, ty, "value", "        ", 0);
    text.line("    },");
    text.line("};");
    text.blank();
}

/// Render one service stream decoder.
fn render_output_decoder(
    schema: &Schema,
    names: &TypeNames,
    ty: &Type,
    name: &str,
    text: &mut Text,
) {
    let rendered = render_type(schema, names, ty);
    let decode = render_decode_type(schema, names, ty, "reader", 0);
    text.line(format!("const {name}Output: Decoder<{rendered}> = {{"));
    text.line(format!("    decode(reader): {rendered} {{"));
    text.line(format!("        return {decode};"));
    text.line("    },");
    text.line("};");
    text.blank();
}

/// Render the generated workspace client class.
fn render_client(
    schema: &Schema,
    operations: &[WorkspaceOperation],
    names: &TypeNames,
    text: &mut Text,
) {
    text.doc("Typed client for the Destack workspace service.", "");
    text.line("export class WorkspaceClient {");
    text.line("    readonly #connection: Connection;");
    text.blank();
    text.doc(
        "Create a workspace client over one negotiated connection.",
        "    ",
    );
    text.line("    constructor(connection: Connection) {");
    text.line("        this.#connection = connection;");
    for operation in operations {
        text.line(format!(
            "        connection.bind({}Method);",
            operation.method
        ));
    }
    text.line("    }");
    text.blank();

    for operation in operations {
        render_client_method(schema, operation, names, text);
    }
    text.line("}");
}

/// Render one typed workspace client method.
fn render_client_method(
    schema: &Schema,
    operation: &WorkspaceOperation,
    names: &TypeNames,
    text: &mut Text,
) {
    let request = render_type(schema, names, &operation.request);
    let response = render_type(schema, names, &operation.response);
    let method = &operation.method;
    let descriptor = format!("{method}Method");
    text.doc(&format!("Call the {} workspace operation.", method), "    ");

    if operation.kind == MethodKind::Unary {
        text.line(format!(
            "    {method}(request: RequestValue<{request}>): Promise<RpcResponse<{response}>> {{"
        ));
        text.line(format!(
            "        return this.#connection.call({descriptor}, request);"
        ));
    } else {
        let input = operation
            .input
            .as_ref()
            .map(|ty| render_type(schema, names, ty))
            .unwrap_or_else(|| "never".to_string());
        let output = operation
            .output
            .as_ref()
            .map(|ty| render_type(schema, names, ty))
            .unwrap_or_else(|| "never".to_string());
        text.line(format!(
            "    {method}(request: RequestValue<{request}>): Call<{response}, {input}, {output}> {{"
        ));
        text.line(format!(
            "        return this.#connection.start({descriptor}, request);"
        ));
    }
    text.line("    }");
    text.blank();
}

/// Return the serialized method kind label.
fn method_kind(kind: MethodKind) -> &'static str {
    match kind {
        MethodKind::Unary => "unary",
        MethodKind::ServerStreaming => "serverStreaming",
        MethodKind::ClientStreaming => "clientStreaming",
        MethodKind::BidirectionalStreaming => "bidirectionalStreaming",
    }
}
