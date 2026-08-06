use std::collections::{BTreeMap, BTreeSet};

use crate::generate::core::{lower_camel, upper_camel};
use crate::generate::schema::{ModulePath, Payload, Schema, SchemaModule, Shape, Type};

use super::codec::{decode_name, encode_name, from_json_name, to_json_name};
use super::name::identifier;
use super::text::Text;

const SERDE_IMPORT_ORDER: &[&str] = &[
    "BinaryReader",
    "BinaryWriter",
    "Json",
    "SerdeError",
    "bytesFromJson",
    "bytesToJson",
    "compareBytes",
    "jsonArray",
    "jsonBigint",
    "jsonBool",
    "jsonField",
    "jsonInteger",
    "jsonNull",
    "jsonNumber",
    "jsonObject",
    "jsonOptional",
    "jsonString",
    "nestedBytes",
];

/// Type names visible inside one generated TypeScript module.
pub(super) struct TypeNames {
    /// Module namespaces keyed by schema key.
    modules: BTreeMap<String, String>,
}

impl TypeNames {
    /// Return visible type names for one generated module.
    pub(super) fn new(schema: &Schema, keys: &[String]) -> Self {
        let local_names = schema
            .module_names(keys)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let referenced = schema.referenced_types(keys);
        let mut referenced_counts = BTreeMap::<String, usize>::new();
        let mut modules = BTreeMap::new();

        for item in &referenced {
            *referenced_counts.entry(item.name.clone()).or_default() += 1;
        }

        for item in referenced {
            let is_colliding = local_names.contains(&item.name)
                || referenced_counts
                    .get(&item.name)
                    .is_some_and(|count| *count > 1);
            if is_colliding {
                let path = schema.module_path(&item.key);
                modules.insert(item.key.clone(), module_namespace(path));
            }
        }

        Self { modules }
    }

    /// Return namespace-qualified names for an out-of-module generated file.
    pub(super) fn namespaced(schema: &Schema, keys: &[String]) -> Self {
        let modules = keys
            .iter()
            .map(|key| {
                let path = schema.module_path(key);

                (key.clone(), module_namespace(path))
            })
            .collect();

        Self { modules }
    }

    /// Return one visible TypeScript type name.
    pub(super) fn ty(&self, ty: &Type) -> String {
        match ty {
            Type::Named { key, name } => self
                .modules
                .get(key)
                .map(|module| format!("{module}.{name}"))
                .unwrap_or_else(|| name.clone()),
            _ => unreachable!("expected named TypeScript type"),
        }
    }

    /// Return one visible encoder function name.
    pub(super) fn encoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", encode_name(name)))
            .unwrap_or_else(|| encode_name(name))
    }

    /// Return one visible decoder function name.
    pub(super) fn decoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", decode_name(name)))
            .unwrap_or_else(|| decode_name(name))
    }

    /// Return one visible JSON encoder function name.
    pub(super) fn json_encoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", to_json_name(name)))
            .unwrap_or_else(|| to_json_name(name))
    }

    /// Return one visible JSON decoder function name.
    pub(super) fn json_decoder(&self, key: &str, name: &str) -> String {
        self.modules
            .get(key)
            .map(|module| format!("{module}.{}", from_json_name(name)))
            .unwrap_or_else(|| from_json_name(name))
    }

    /// Return this imported item module namespace when needed.
    pub(super) fn module(&self, key: &str) -> Option<&str> {
        self.modules.get(key).map(String::as_str)
    }
}

/// Render TypeScript imports referenced by one module.
pub(super) fn render_imports(
    schema: &Schema,
    module: &SchemaModule,
    names: &[String],
    type_names: &TypeNames,
) -> String {
    let mut text = Text::new();
    let source = generated_target_segments(&module.path);
    let serde = runtime_import_path(&source, &["protocol", "serde"]);
    let serde_imports = serde_imports(schema, names);

    text.line(format!(
        "import {{ {} }} from {:?};",
        serde_imports.join(", "),
        serde,
    ));

    let referenced = schema.referenced_types(names);
    let mut namespace_imports = BTreeMap::<String, String>::new();
    for item in &referenced {
        let path = generated_import_path(&module.path, schema.module_path(&item.key));
        if let Some(namespace) = type_names.module(&item.key) {
            namespace_imports.insert(path, namespace.to_string());
            continue;
        }

        text.line(format!("import type {{ {} }} from \"{path}\";", item.name));
    }

    for (path, namespace) in namespace_imports {
        text.line(format!("import * as {namespace} from \"{path}\";"));
    }

    for item in referenced {
        if type_names.module(&item.key).is_some() {
            continue;
        }

        let path = generated_import_path(&module.path, schema.module_path(&item.key));
        let source_encoder = encode_name(&item.name);
        let source_decoder = decode_name(&item.name);
        let source_json_encoder = to_json_name(&item.name);
        let source_json_decoder = from_json_name(&item.name);
        text.line(format!(
            "import {{ {source_decoder}, {source_encoder}, {source_json_decoder}, {source_json_encoder} }} from \"{path}\";"
        ));
    }

    text.finish()
}

/// Return the TypeScript import path between two generated semantic modules.
fn generated_import_path(source: &ModulePath, target: &ModulePath) -> String {
    let source = generated_target_segments(source);
    let target = generated_target_segments(target);

    module_import_path(&source, &target)
}

/// Return the TypeScript import path between two generated module paths.
fn module_import_path(source: &[String], target: &[String]) -> String {
    let source_directory = &source[..source.len() - 1];
    let mut shared = 0;

    while shared < source_directory.len()
        && shared < target.len()
        && source_directory[shared] == target[shared]
    {
        shared += 1;
    }

    let mut segments = Vec::new();
    for _ in shared..source_directory.len() {
        segments.push("..".to_string());
    }
    segments.extend(target[shared..].iter().cloned());

    let path = segments.join("/");
    if path.starts_with('.') {
        format!("{path}.js")
    } else {
        format!("./{path}.js")
    }
}

/// Return one TypeScript import path from a generated module to a runtime module.
fn runtime_import_path(source: &[String], target: &[&str]) -> String {
    let source_directory = &source[..source.len() - 1];
    let mut shared = 0;

    while shared < source_directory.len()
        && shared < target.len()
        && source_directory[shared] == target[shared]
    {
        shared += 1;
    }

    let mut segments = Vec::new();
    for _ in shared..source_directory.len() {
        segments.push("..".to_string());
    }
    segments.extend(target[shared..].iter().map(|segment| segment.to_string()));

    let path = segments.join("/");
    if path.starts_with('.') {
        format!("{path}.js")
    } else {
        format!("./{path}.js")
    }
}

/// Return TypeScript generated semantic target segments.
pub(super) fn generated_target_segments(path: &ModulePath) -> Vec<String> {
    let mut segments = Vec::with_capacity(path.segments().len() + 1);
    segments.push("_generated".to_string());
    segments.extend(path.segments().iter().cloned());

    segments
}

/// Return one generated module namespace binding.
fn module_namespace(path: &ModulePath) -> String {
    let mut segments = path.segments().iter();
    let Some(first) = segments.next() else {
        return "module".to_string();
    };

    let mut name = lower_camel(first);
    for segment in segments {
        name.push_str(&upper_camel(segment));
    }

    identifier(&name)
}

/// Return serde runtime imports used by one module.
fn serde_imports(schema: &Schema, names: &[String]) -> Vec<&'static str> {
    let mut imports = BTreeSet::new();
    imports.insert("BinaryReader");
    imports.insert("BinaryWriter");
    imports.insert("Json");

    for key in names {
        let item = schema.item(key);
        collect_item_imports(&mut imports, item);
    }

    SERDE_IMPORT_ORDER
        .iter()
        .copied()
        .filter(|name| imports.contains(name))
        .collect()
}

/// Collect serde runtime imports for one schema item.
fn collect_item_imports(
    imports: &mut BTreeSet<&'static str>,
    item: &crate::generate::schema::Item,
) {
    match &item.shape {
        Shape::Struct(fields) => {
            imports.insert("jsonObject");
            if !fields.is_empty() {
                imports.insert("jsonField");
            }

            for field in fields {
                collect_type_imports(imports, &field.ty);
            }
        }
        Shape::Enum(variants) => {
            imports.insert("SerdeError");

            for variant in variants {
                collect_payload_imports(imports, &variant.payload);
            }

            if variants
                .iter()
                .all(|variant| matches!(variant.payload, Payload::Unit))
            {
                imports.insert("jsonString");
            } else {
                imports.insert("jsonObject");
                imports.insert("jsonField");
                imports.insert("jsonString");
            }
        }
    }
}

/// Collect serde runtime imports for one enum payload.
fn collect_payload_imports(imports: &mut BTreeSet<&'static str>, payload: &Payload) {
    match payload {
        Payload::Unit => {}
        Payload::Tuple(ty) => collect_type_imports(imports, ty),
        Payload::Struct(fields) => {
            for field in fields {
                collect_type_imports(imports, &field.ty);
            }
        }
    }
}

/// Collect serde runtime imports for one type reference.
fn collect_type_imports(imports: &mut BTreeSet<&'static str>, ty: &Type) {
    match ty {
        Type::Unit => {
            imports.insert("jsonNull");
        }
        Type::String | Type::Char => {
            imports.insert("jsonString");
        }
        Type::Bool => {
            imports.insert("jsonBool");
        }
        Type::U8 | Type::U32 | Type::Usize | Type::Signed(8 | 16 | 32) => {
            imports.insert("jsonInteger");
        }
        Type::U64 | Type::U128 | Type::Signed(64 | 128) => {
            imports.insert("jsonBigint");
        }
        Type::Signed(_) => ty.unsupported_client_type(),
        Type::Float(_) => {
            imports.insert("jsonNumber");
        }
        Type::Vec(item) if matches!(item.as_ref(), Type::U8) => {
            imports.insert("bytesFromJson");
            imports.insert("bytesToJson");
        }
        Type::Vec(item) => {
            imports.insert("jsonArray");
            collect_type_imports(imports, item);
        }
        Type::Option(item) => {
            imports.insert("jsonOptional");
            collect_type_imports(imports, item);
        }
        Type::Array(item, _) if matches!(item.as_ref(), Type::U8) => {
            imports.insert("bytesFromJson");
            imports.insert("bytesToJson");
        }
        Type::Array(item, _) => {
            imports.insert("SerdeError");
            imports.insert("jsonArray");
            collect_type_imports(imports, item);
        }
        Type::Tuple(items) => {
            imports.insert("SerdeError");
            imports.insert("jsonArray");

            for item in items {
                collect_type_imports(imports, item);
            }
        }
        Type::Map(key, item) => {
            imports.insert("SerdeError");
            imports.insert("compareBytes");
            imports.insert("jsonArray");
            imports.insert("nestedBytes");
            collect_type_imports(imports, key);
            collect_type_imports(imports, item);
        }
        Type::Json | Type::Named { .. } => {}
    }
}
