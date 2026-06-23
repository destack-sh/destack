use std::collections::BTreeSet;

use crate::generate::core::to_snake;
use crate::generate::schema::{
    Field, Item, ModulePath, PayloadNames, Schema, SchemaModule, Shape, Variant,
};

use super::payload::payload_storage_fields;
use super::projection::{ARTIFACT_KEY, Projection, artifact_key_fields};
use super::text::Text;
use super::ty::{c_declaration, c_enum_variant, c_type_name, header_guard};

pub(super) struct Header<'schema> {
    /// Bridge schema.
    schema: &'schema Schema,
    /// C ABI projection.
    projection: &'schema Projection<'schema>,
}

/// One generated C header.
pub(super) struct HeaderFile {
    /// Header path under the workspace root.
    pub(super) path: String,
    /// Header content.
    pub(super) content: String,
}

impl<'schema> Header<'schema> {
    /// Create one C ABI header generator.
    pub(super) fn new(schema: &'schema Schema, projection: &'schema Projection<'schema>) -> Self {
        Self { schema, projection }
    }

    /// Render C ABI headers.
    pub(super) fn render(&self) -> Vec<HeaderFile> {
        let mut outputs = Vec::new();

        outputs.push(HeaderFile {
            path: "bridge/capi/include/destack/generated.h".to_string(),
            content: self.render_umbrella(),
        });
        outputs.push(HeaderFile {
            path: "bridge/capi/include/destack/core.generated.h".to_string(),
            content: self.render_core(),
        });

        for module in &self.schema.modules {
            if !self.module_has_header(module) {
                continue;
            }

            outputs.push(HeaderFile {
                path: format!(
                    "bridge/capi/include/destack/{}.generated.h",
                    module.path.slash_path()
                ),
                content: self.render_module(module),
            });
        }

        outputs
    }

    /// Render the generated umbrella header.
    fn render_umbrella(&self) -> String {
        let mut text = Text::new();
        text.line("/* generated bridge target, do not edit */");
        text.blank();

        text.line("#ifndef DESTACK_GENERATED_H");
        text.line("#define DESTACK_GENERATED_H");
        text.blank();

        text.line("#include \"destack/core.generated.h\"");

        for module in &self.schema.modules {
            if !self.module_has_header(module) {
                continue;
            }

            text.line(format!(
                "#include \"destack/{}.generated.h\"",
                module.path.slash_path()
            ));
        }

        text.blank();
        text.line("#endif");

        text.finish()
    }

    /// Render C ABI core declarations.
    fn render_core(&self) -> String {
        let mut text = Text::new();
        text.line("/* generated bridge target, do not edit */");
        text.blank();

        text.line("#ifndef DESTACK_CORE_GENERATED_H");
        text.line("#define DESTACK_CORE_GENERATED_H");
        text.blank();
        text.line("#include \"destack/core.h\"");
        text.blank();
        text.line("#ifdef __cplusplus");
        text.line("extern \"C\" {");
        text.line("#endif");
        text.blank();

        for handle in &self.projection.handles {
            text.line(format!("typedef struct Destack{handle} Destack{handle};"));
        }
        text.blank();

        text.line("typedef struct DestackByteArray {");
        text.line("    uint8_t *ptr;");
        text.line("    size_t len;");
        text.line("} DestackByteArray;");
        text.blank();
        text.line("typedef struct DestackU128 {");
        text.line("    uint64_t high;");
        text.line("    uint64_t low;");
        text.line("} DestackU128;");
        text.blank();
        text.line("typedef struct DestackStringArray {");
        text.line("    char **ptr;");
        text.line("    size_t len;");
        text.line("} DestackStringArray;");
        text.blank();
        text.line("typedef struct DestackOptionalString {");
        text.line("    bool is_some;");
        text.line("    char *value;");
        text.line("} DestackOptionalString;");
        text.blank();

        text.line("void destack_byte_array_destroy(DestackByteArray array);");
        text.line("void destack_string_array_destroy(DestackStringArray array);");
        text.line("void destack_optional_string_destroy(DestackOptionalString value);");

        text.blank();
        text.line("#ifdef __cplusplus");
        text.line("}");
        text.line("#endif");
        text.blank();
        text.line("#endif");

        text.finish()
    }

    /// Render one module C ABI header.
    fn render_module(&self, module: &SchemaModule) -> String {
        let mut text = Text::new();
        let guard = header_guard(&module.path);
        text.line("/* generated bridge target, do not edit */");
        text.blank();
        text.line(format!("#ifndef {guard}"));
        text.line(format!("#define {guard}"));
        text.blank();

        text.line("#include \"destack/core.generated.h\"");
        for dependency in self.module_dependencies(module) {
            text.line(format!(
                "#include \"destack/{}.generated.h\"",
                dependency.slash_path()
            ));
        }
        text.blank();

        text.line("#ifdef __cplusplus");
        text.line("extern \"C\" {");
        text.line("#endif");
        text.blank();

        for name in self.module_values(module) {
            if !module.names.iter().any(|module_name| module_name == name) {
                continue;
            }

            self.render_item(&mut text, name.as_str());
            text.blank();
        }

        for name in &module.names {
            if self.projection.is_handle(name) {
                let snake = to_snake(name);
                text.line(format!(
                    "void destack_{snake}_destroy(Destack{name} *value);"
                ));
            }
        }
        for variant in &self.projection.artifact_key_variants {
            if module
                .names
                .iter()
                .any(|module_name| module_name == ARTIFACT_KEY)
            {
                self.render_artifact_key_function(&mut text, variant);
            }
        }

        for name in self.module_values(module) {
            if !module.names.iter().any(|module_name| module_name == name) {
                continue;
            }

            let snake = to_snake(name);
            text.line(format!(
                "void destack_{snake}_destroy(Destack{name} *value);"
            ));
            text.line(format!(
                "void destack_{snake}_array_destroy(Destack{name}Array array);"
            ));
        }

        text.blank();
        text.line("#ifdef __cplusplus");
        text.line("}");
        text.line("#endif");
        text.blank();
        text.line("#endif");

        text.finish()
    }

    /// Return whether one module emits any C ABI header declarations.
    fn module_has_header(&self, module: &SchemaModule) -> bool {
        module.names.iter().any(|name| {
            self.projection.values.iter().any(|value| value == name)
                || self.projection.is_handle(name)
        })
    }

    /// Return generated header dependencies for one module.
    fn module_dependencies(&self, module: &SchemaModule) -> Vec<&ModulePath> {
        let mut dependencies = BTreeSet::<&ModulePath>::new();
        let current = &module.path;
        let values = module
            .names
            .iter()
            .filter(|name| self.projection.values.iter().any(|value| value == *name))
            .cloned()
            .collect::<Vec<_>>();

        for item in self.schema.referenced_types(&values) {
            if !self
                .projection
                .values
                .iter()
                .any(|value| value == &item.name)
                && !self.projection.is_handle(&item.name)
            {
                continue;
            }

            let dependency = self.schema.module_path(&item.name);
            if dependency != current {
                dependencies.insert(dependency);
            }
        }

        dependencies.into_iter().collect()
    }

    /// Return projected module value names in dependency order.
    fn module_values(&self, module: &SchemaModule) -> Vec<&String> {
        self.projection
            .values
            .iter()
            .filter(|value| module.names.iter().any(|name| name == *value))
            .collect()
    }

    /// Render one projected item.
    fn render_item(&self, text: &mut Text, name: &str) {
        let item = self.schema.item(name);

        match &item.shape {
            Shape::Struct(fields) => self.render_struct(text, item, fields),
            Shape::Enum(variants) if self.schema.is_unit_enum(name) => {
                self.render_unit_enum(text, item, variants);
            }
            Shape::Enum(variants) => self.render_payload_enum(text, item, variants),
        }

        text.blank();
        text.line(format!("typedef struct Destack{name}Array {{"));
        text.line(format!("    Destack{name} *ptr;"));
        text.line("    size_t len;");
        text.line(format!("}} Destack{name}Array;"));
        text.blank();
        text.line(format!("typedef struct DestackOptional{name} {{"));
        text.line("    bool is_some;");
        text.line(format!("    Destack{name} value;"));
        text.line(format!("}} DestackOptional{name};"));
    }

    /// Render one C struct.
    fn render_struct(&self, text: &mut Text, item: &Item, fields: &[Field]) {
        let name = c_type_name(&item.name);

        text.line(format!("typedef struct {name} {{"));
        for field in fields {
            let declaration = c_declaration(&field.name, &field.ty, self.projection);
            text.line(format!("    {declaration};"));
        }
        text.line(format!("}} {name};"));
    }

    /// Render one C unit enum.
    fn render_unit_enum(&self, text: &mut Text, item: &Item, variants: &[Variant]) {
        let name = c_type_name(&item.name);

        text.line(format!("typedef enum {name} {{"));
        for (index, variant) in variants.iter().enumerate() {
            let variant = c_enum_variant(&item.name, &variant.name);
            text.line(format!("    {variant} = {index},"));
        }
        text.line(format!("}} {name};"));
    }

    /// Render one C payload enum.
    fn render_payload_enum(&self, text: &mut Text, item: &Item, variants: &[Variant]) {
        let kind = format!("{}Kind", c_type_name(&item.name));

        text.line(format!("typedef enum {kind} {{"));
        for (index, variant) in variants.iter().enumerate() {
            let variant = c_enum_variant(&format!("{}Kind", item.name), &variant.name);
            text.line(format!("    {variant} = {index},"));
        }
        text.line(format!("}} {kind};"));
        text.blank();

        let names = PayloadNames::new(variants);
        let name = c_type_name(&item.name);
        text.line(format!("typedef struct {name} {{"));
        text.line(format!("    {kind} kind;"));
        for field in payload_storage_fields(&names, variants) {
            let declaration = c_declaration(&field.name, field.ty, self.projection);
            text.line(format!("    {declaration};"));
        }
        text.line(format!("}} {name};"));
    }

    /// Render one artifact key constructor declaration.
    fn render_artifact_key_function(&self, text: &mut Text, variant: &Variant) {
        let name = to_snake(&variant.name);
        text.line(format!("DestackStatus destack_artifact_key_{name}("));

        let fields = artifact_key_fields(variant);
        for field in &fields {
            let field = c_declaration(field.name, field.ty, self.projection);
            text.line(format!("    {field},"));
        }

        text.line("    DestackArtifactKey **out,");
        text.line("    DestackError **error");
        text.line(");");
    }
}
