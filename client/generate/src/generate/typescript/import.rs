use std::collections::{BTreeMap, BTreeSet};

use crate::generate::schema::{Item, Module, Payload, Schema, Type};

use super::codec::{decode_name, encode_name, from_json_name, to_json_name};
use super::path::{generated_import_path, generated_target_segments, runtime_import_path};
use super::scope::Scope;
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

/// Render TypeScript imports referenced by one module.
pub(super) fn render_imports(
    schema: &Schema,
    module: &Module,
    keys: &[String],
    scope: &Scope,
) -> String {
    // render the shared serde runtime import
    let mut text = Text::new();
    let source = generated_target_segments(&module.path);
    let serde = runtime_import_path(&source, &["protocol", "serde"]);
    let serde_imports = serde_imports(schema, keys);
    text.line(format!(
        "import {{ {} }} from {:?};",
        serde_imports.join(", "),
        serde,
    ));

    // collect type and namespace imports for referenced modules
    let referenced = schema.referenced_types(keys);
    let mut namespace_imports = BTreeMap::<String, String>::new();
    for item in &referenced {
        let path = generated_import_path(&module.path, schema.module_path(&item.key));
        if let Some(namespace) = scope.module(&item.key) {
            namespace_imports.insert(path, namespace.to_string());
        } else {
            text.line(format!("import type {{ {} }} from \"{path}\";", item.name));
        }
    }

    // render namespace imports in stable path order
    for (path, namespace) in namespace_imports {
        text.line(format!("import * as {namespace} from \"{path}\";"));
    }

    // render unqualified codec imports for referenced values
    for item in referenced {
        if scope.module(&item.key).is_some() {
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

/// Return serde runtime imports used by one module.
fn serde_imports(schema: &Schema, keys: &[String]) -> Vec<&'static str> {
    let mut imports = BTreeSet::new();
    imports.insert("BinaryReader");
    imports.insert("BinaryWriter");
    imports.insert("Json");

    for key in keys {
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
fn collect_item_imports(imports: &mut BTreeSet<&'static str>, item: &Item) {
    match &item.ty {
        Type::Struct(fields) => {
            imports.insert("jsonObject");
            if !fields.is_empty() {
                imports.insert("jsonField");
            }

            for field in fields {
                collect_type_imports(imports, &field.ty);
            }
        }
        Type::Enum(variants) => {
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
        _ => collect_type_imports(imports, &item.ty),
    }
}

/// Collect serde runtime imports for one enum payload.
fn collect_payload_imports(imports: &mut BTreeSet<&'static str>, payload: &Payload) {
    match payload {
        Payload::Unit => {}
        Payload::Value(ty) => collect_type_imports(imports, ty),
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
        Type::Signed(_) => ty.unsupported(),
        Type::Float(_) => {
            imports.insert("jsonNumber");
        }
        Type::Sequence(item) if matches!(item.as_ref(), Type::U8) => {
            imports.insert("bytesFromJson");
            imports.insert("bytesToJson");
        }
        Type::Sequence(item) => {
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
        Type::Named { .. } => {}
        Type::Struct(fields) => {
            for field in fields {
                collect_type_imports(imports, &field.ty);
            }
        }
        Type::Enum(variants) => {
            for variant in variants {
                collect_payload_imports(imports, &variant.payload);
            }
        }
    }
}
