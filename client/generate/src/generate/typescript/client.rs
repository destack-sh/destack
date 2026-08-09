use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};
use destack_rpc as rpc;

use crate::generate::core::{lower_camel, upper_camel, write_text};
use crate::generate::schema::{ModulePath, Schema, Type};

use super::codec::{render_decode_type, render_encode_type};
use super::item::render_type;
use super::method::Method;
use super::path::{
    generated_import_path, generated_target_segments, output_path, runtime_import_path,
};
use super::scope::Scope;
use super::text::{GENERATED_HEADER, Text};

/// One generated TypeScript RPC client.
pub(super) struct Client<'a> {
    /// Complete reflected client schema.
    schema: &'a Schema,
    /// Service implemented by this client.
    service: &'a rpc::ServiceSchema,
    /// TypeScript service name.
    name: String,
    /// TypeScript service variable name.
    variable: String,
    /// Generated client module path.
    path: ModulePath,
    /// Typed service methods.
    methods: Vec<Method>,
    /// Named types referenced by the methods.
    keys: Vec<String>,
    /// Name resolution scope for the generated client module.
    scope: Scope,
}

impl<'a> Client<'a> {
    /// Generate typed clients for every reflected RPC service.
    pub(super) fn generate(root: &Path, schema: &'a Schema) -> Result<()> {
        for service in &schema.services {
            Self::new(schema, service)?.write(root)?;
        }

        Ok(())
    }

    /// Build one generated client from a reflected service.
    fn new(schema: &'a Schema, service: &'a rpc::ServiceSchema) -> Result<Self> {
        // derive the public client identity from the canonical service name
        let Some(name) = service
            .name()
            .rsplit('.')
            .next()
            .filter(|name| !name.is_empty())
        else {
            bail!("client service name is empty");
        };
        let name = upper_camel(name);
        let variable = lower_camel(&name);
        let path = ModulePath::from_service(service.name())?;

        // convert each reflected method into its TypeScript representation
        let methods = service
            .methods()
            .iter()
            .map(|method| Method::from_schema(method, schema))
            .collect::<Result<Vec<_>>>()?;

        // collect the exact generated types imported by this client
        let mut keys = BTreeSet::new();
        for method in &methods {
            method.visit_keys(&mut |key| {
                keys.insert(key.to_string());
            });
        }
        let keys = keys.into_iter().collect::<Vec<_>>();
        let scope = Scope::external(schema, &keys);

        Ok(Self {
            schema,
            service,
            name,
            variable,
            path,
            methods,
            keys,
            scope,
        })
    }

    /// Write this generated client.
    fn write(&self, root: &Path) -> Result<()> {
        let path = output_path(&self.path);
        let content = self.render();

        write_text(root, &path, content)
    }

    /// Render this complete generated client.
    fn render(&self) -> String {
        // resolve runtime imports required by this service
        let mut text = Text::new();
        let source = generated_target_segments(&self.path);
        let rpc_path = runtime_import_path(&source, &["rpc", "index"]);
        let has_stream = self
            .methods
            .iter()
            .any(|method| method.kind != rpc::MethodKind::Unary);
        let call_import = if has_stream { "Call, " } else { "" };

        // render imports before service declarations
        text.line(GENERATED_HEADER);
        text.blank();
        text.line(format!("import type {{ {call_import}Decoder, Encoder, Method, RequestValue, RpcResponse }} from \"{rpc_path}\";"));
        text.line(format!("import {{ Connection }} from \"{rpc_path}\";"));
        self.render_imports(&mut text);
        text.blank();

        // render the complete service definition and client
        self.render_service(&mut text);
        for method in &self.methods {
            self.render_method(method, &mut text);
        }
        self.render_client(&mut text);

        text.finish()
    }

    /// Render namespace imports for every referenced value module.
    fn render_imports(&self, text: &mut Text) {
        // collapse referenced types into one import per module
        let mut imports = BTreeMap::new();

        for key in &self.keys {
            let path = self.schema.module_path(key);
            let Some(namespace) = self.scope.module(key) else {
                continue;
            };
            let path = generated_import_path(&self.path, path);
            imports.insert(path, namespace.to_string());
        }

        // render imports in stable path order
        for (path, namespace) in imports {
            text.line(format!("import * as {namespace} from \"{path}\";"));
        }
    }

    /// Render this stable service identifier.
    fn render_service(&self, text: &mut Text) {
        text.doc(
            &format!("Stable {} RPC service identifier.", self.service.name()),
            "",
        );
        text.line(format!(
            "export const {}Service = {}n;",
            self.variable,
            self.service.id().0
        ));
        text.blank();
    }

    /// Render one typed method descriptor and its codecs.
    fn render_method(&self, method: &Method, text: &mut Text) {
        // resolve the complete visible method grammar
        let request = render_type(self.schema, &self.scope, &method.request);
        let response = render_type(self.schema, &self.scope, &method.response);
        let input = method
            .input
            .as_ref()
            .map(|ty| render_type(self.schema, &self.scope, ty))
            .unwrap_or_else(|| "never".to_string());
        let output = method
            .output
            .as_ref()
            .map(|ty| render_type(self.schema, &self.scope, ty))
            .unwrap_or_else(|| "never".to_string());
        let constant = format!("{}Method", method.name);

        // render codecs before their method descriptor
        self.render_encoder(&method.request, &format!("{constant}Request"), text);
        self.render_decoder(&method.response, &format!("{constant}Response"), text);
        if let Some(input) = &method.input {
            self.render_encoder(input, &format!("{constant}Input"), text);
        }
        if let Some(output) = &method.output {
            self.render_decoder(output, &format!("{constant}Output"), text);
        }

        // render the exact reflected method definition
        text.doc(
            &format!("Descriptor for the {} RPC method.", method.name),
            "",
        );
        text.line(format!(
            "const {constant}: Method<{request}, {response}, {input}, {output}> = {{"
        ));
        text.line(format!("    service: {}n,", self.service.id().0));
        text.line(format!("    method: {}n,", method.id.0));
        text.line(format!("    fingerprint: {}n,", method.fingerprint));
        text.line(format!("    kind: {:?},", Self::method_kind(method.kind)));
        text.line(format!(
            "    idempotency: {:?},",
            Self::idempotency(method.idempotency)
        ));
        text.line(format!("    request: {constant}Request,"));
        text.line(format!("    response: {constant}Response,"));
        if method.input.is_some() {
            text.line(format!("    input: {constant}Input,"));
        }
        if method.output.is_some() {
            text.line(format!("    output: {constant}Output,"));
        }
        text.line("};");
        text.blank();
    }

    /// Render one request encoder.
    fn render_encoder(&self, ty: &Type, name: &str, text: &mut Text) {
        let rendered = render_type(self.schema, &self.scope, ty);
        text.line(format!("const {name}: Encoder<{rendered}> = {{"));
        if matches!(ty, Type::Unit) {
            text.line("    encode(): void {},");
        } else {
            text.line(format!("    encode(writer, value: {rendered}): void {{"));
            render_encode_type(self.schema, &self.scope, text, ty, "value", "        ", 0);
            text.line("    },");
        }
        text.line("};");
        text.blank();
    }

    /// Render one response decoder.
    fn render_decoder(&self, ty: &Type, name: &str, text: &mut Text) {
        let rendered = render_type(self.schema, &self.scope, ty);
        text.line(format!("const {name}: Decoder<{rendered}> = {{"));
        if matches!(ty, Type::Unit) {
            text.line(format!("    decode(): {rendered} {{"));
            text.line("        return null;");
            text.line("    },");
        } else {
            let decode = render_decode_type(self.schema, &self.scope, ty, "reader", 0);
            text.line(format!("    decode(reader): {rendered} {{"));
            text.line(format!("        return {decode};"));
            text.line("    },");
        }
        text.line("};");
        text.blank();
    }

    /// Render this generated client class.
    fn render_client(&self, text: &mut Text) {
        text.doc(
            &format!("Typed client for the {} service.", self.service.name()),
            "",
        );
        text.line(format!("export class {}Client {{", self.name));
        text.line("    readonly #connection: Connection;");
        text.blank();
        text.doc(
            &format!(
                "Create a {} client over one negotiated connection.",
                self.variable
            ),
            "    ",
        );
        text.line("    constructor(connection: Connection) {");
        text.line("        this.#connection = connection;");
        for method in &self.methods {
            text.line(format!("        connection.bind({}Method);", method.name));
        }
        text.line("    }");
        text.blank();

        for (index, method) in self.methods.iter().enumerate() {
            if index > 0 {
                text.blank();
            }
            self.render_client_method(method, text);
        }
        text.line("}");
    }

    /// Render one typed client method.
    fn render_client_method(&self, method: &Method, text: &mut Text) {
        let request = render_type(self.schema, &self.scope, &method.request);
        let response = render_type(self.schema, &self.scope, &method.response);
        let name = &method.name;
        let descriptor = format!("{name}Method");
        text.doc(
            &format!("Call the {name} {} method.", self.variable),
            "    ",
        );

        if method.kind == rpc::MethodKind::Unary {
            text.line(format!(
                "    {name}(request: RequestValue<{request}>): Promise<RpcResponse<{response}>> {{"
            ));
            text.line(format!(
                "        return this.#connection.call({descriptor}, request);"
            ));
        } else {
            let input = method
                .input
                .as_ref()
                .map(|ty| render_type(self.schema, &self.scope, ty))
                .unwrap_or_else(|| "never".to_string());
            let output = method
                .output
                .as_ref()
                .map(|ty| render_type(self.schema, &self.scope, ty))
                .unwrap_or_else(|| "never".to_string());
            text.line(format!(
                "    {name}(request: RequestValue<{request}>): Call<{response}, {input}, {output}> {{"
            ));
            text.line(format!(
                "        return this.#connection.start({descriptor}, request);"
            ));
        }
        text.line("    }");
    }

    /// Return the serialized method kind label.
    fn method_kind(kind: rpc::MethodKind) -> &'static str {
        match kind {
            rpc::MethodKind::Unary => "unary",
            rpc::MethodKind::ServerStreaming => "serverStreaming",
            rpc::MethodKind::ClientStreaming => "clientStreaming",
            rpc::MethodKind::BidirectionalStreaming => "bidirectionalStreaming",
        }
    }

    /// Return the serialized method idempotency label.
    fn idempotency(idempotency: rpc::Idempotency) -> &'static str {
        match idempotency {
            rpc::Idempotency::Unknown => "unknown",
            rpc::Idempotency::Idempotent => "idempotent",
            rpc::Idempotency::NoSideEffects => "noSideEffects",
        }
    }
}
