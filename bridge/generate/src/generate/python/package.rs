use std::collections::BTreeMap;

use crate::generate::schema::{Schema, Shape};

use super::codec::{decode_name, encode_name, from_json_name, to_json_name};
use super::item::protocol_variant_name;
use super::path::{module_names, python_segments};
use super::text::Text;

const PUBLIC_ROOTS: &[&str] = &[
    "artifact",
    "core",
    "dir",
    "heap",
    "js",
    "mir",
    "program",
    "qir",
    "query",
    "repository",
    "source",
];

pub(super) struct PythonPackage {
    /// Package path segments below `destack`.
    pub(super) segments: Vec<String>,
    /// Re-exported names in stable order.
    pub(super) names: Vec<String>,
    /// Re-export source module keyed by exported name.
    pub(super) modules: BTreeMap<String, String>,
}

impl PythonPackage {
    /// Return generated Python semantic packages.
    pub(super) fn all(schema: &Schema) -> BTreeMap<Vec<String>, Self> {
        let mut packages = BTreeMap::<Vec<String>, Self>::new();

        for module in &schema.modules {
            let names = public_module_names(schema, &module.keys);
            let segments = python_segments(schema, &module.path);
            Self::add(&mut packages, segments, &names);
        }

        packages
    }

    /// Return public Python semantic packages.
    pub(super) fn public(schema: &Schema) -> BTreeMap<Vec<String>, Self> {
        let mut packages = BTreeMap::<Vec<String>, Self>::new();

        for module in &schema.modules {
            let segments = python_segments(schema, &module.path);
            if !is_public_segments(&segments) {
                continue;
            }

            let names = public_module_names(schema, &module.keys);
            Self::add(&mut packages, segments, &names);
        }

        packages
    }

    /// Return generated Python protocol packages.
    pub(super) fn protocol(schema: &Schema) -> BTreeMap<Vec<String>, Self> {
        let mut packages = BTreeMap::<Vec<String>, Self>::new();

        for module in &schema.modules {
            let names = module_names(schema, module);
            let segments = python_segments(schema, &module.path);
            Self::add(&mut packages, segments, &names);
        }

        packages
    }

    /// Render one generated Python package facade.
    pub(super) fn render_generated_facade(&self) -> String {
        let mut text = Text::generated();
        self.render_reexports(&mut text);
        text.blank();
        render_all(&mut text, &self.names);

        text.finish()
    }

    /// Render one generated Python package stub.
    pub(super) fn render_generated_stub(&self) -> String {
        let mut text = Text::generated();
        text.line("from __future__ import annotations");
        text.blank();
        self.render_reexports(&mut text);
        text.blank();
        render_all(&mut text, &self.names);

        text.finish()
    }

    /// Render one public Python package facade.
    pub(super) fn render_public_facade(&self) -> String {
        let mut text = Text::generated();
        self.render_public_reexports(&mut text);
        self.render_handwritten_exports(&mut text);
        text.blank();
        render_all(&mut text, &self.public_names());

        text.finish()
    }

    /// Render one public Python package stub.
    pub(super) fn render_public_stub(&self) -> String {
        let mut text = Text::generated();
        text.line("from __future__ import annotations");
        text.blank();
        self.render_public_reexports(&mut text);
        self.render_handwritten_exports(&mut text);
        text.blank();
        render_all(&mut text, &self.public_names());

        text.finish()
    }

    /// Add package re-exports for one generated Python module path.
    fn add(packages: &mut BTreeMap<Vec<String>, Self>, segments: Vec<String>, names: &[String]) {
        if segments.is_empty() {
            return;
        }

        let module_name = segments[segments.len() - 1].clone();

        for depth in 1..segments.len() {
            let package_segments = segments[..depth].to_vec();
            let package = Self::get(packages, package_segments);
            let suffix = segments[depth..].join(".");

            for name in names {
                package.add_reexport(name, &suffix);
            }
        }

        let package_segments = segments[..segments.len() - 1].to_vec();
        if package_segments.is_empty() {
            return;
        }

        let package = Self::get(packages, package_segments);
        for name in names {
            package
                .modules
                .entry(name.clone())
                .or_insert(module_name.clone());
        }
    }

    /// Return one package entry.
    fn get(packages: &mut BTreeMap<Vec<String>, Self>, segments: Vec<String>) -> &mut Self {
        packages.entry(segments.clone()).or_insert_with(|| Self {
            segments,
            names: Vec::new(),
            modules: BTreeMap::new(),
        })
    }

    /// Add one re-export when it is not already declared.
    fn add_reexport(&mut self, name: &str, module: &str) {
        if self.modules.contains_key(name) {
            return;
        }

        self.names.push(name.to_string());
        self.modules.insert(name.to_string(), module.to_string());
    }

    /// Render package re-exports for generated DTO modules below this package.
    fn render_reexports(&self, text: &mut Text) {
        for (module, names) in self.modules() {
            text.line(format!("from .{module} import ("));

            for name in names {
                text.line(format!("    {name},"));
            }

            text.line(")");
        }
    }

    /// Render public package re-exports for generated DTO modules below this package.
    fn render_public_reexports(&self, text: &mut Text) {
        let dots = ".".repeat(self.segments.len() + 1);
        let base = format!("{dots}_generated.{}", self.segments.join("."));

        for (module, names) in self.modules() {
            text.line(format!("from {base}.{module} import ("));

            for name in names {
                text.line(format!("    {name},"));
            }

            text.line(")");
        }
    }

    /// Render handwritten public exports for this package.
    fn render_handwritten_exports(&self, text: &mut Text) {
        if self.segments.as_slice() == ["core"] {
            text.line("from .string import (");
            text.line("    StringPool,");
            text.line(")");
        }
        if self.segments.as_slice() == ["dir"] {
            text.line("from .binding import (");
            text.line("    BindingTable,");
            text.line(")");
        }
    }

    /// Return public exported names for this package.
    fn public_names(&self) -> Vec<String> {
        let mut names = self.names.clone();

        if self.segments.as_slice() == ["core"] {
            names.push("StringPool".to_string());
        }
        if self.segments.as_slice() == ["dir"] {
            names.push("BindingTable".to_string());
        }

        names
    }

    /// Return re-exported names grouped by source module.
    fn modules(&self) -> BTreeMap<&str, Vec<&str>> {
        let mut modules = BTreeMap::<&str, Vec<&str>>::new();

        for (name, module) in &self.modules {
            modules
                .entry(module.as_str())
                .or_default()
                .push(name.as_str());
        }

        modules
    }
}

/// One Python stub method declaration.
pub(super) struct StubMethod {
    /// Method decorators.
    decorators: Vec<String>,
    /// Method name.
    name: String,
    /// Method arguments.
    arguments: Vec<String>,
    /// Return type.
    output: String,
}

impl StubMethod {
    /// Create one method declaration.
    pub(super) fn new(name: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            decorators: Vec::new(),
            name: name.into(),
            arguments: Vec::new(),
            output: output.into(),
        }
    }

    /// Add one decorator.
    pub(super) fn with_decorator(mut self, decorator: impl Into<String>) -> Self {
        self.decorators.push(decorator.into());

        self
    }

    /// Add one argument.
    pub(super) fn with_argument(mut self, argument: impl Into<String>) -> Self {
        self.arguments.push(argument.into());

        self
    }

    /// Render this method declaration.
    pub(super) fn render(&self, text: &mut Text) {
        for decorator in &self.decorators {
            text.line(format!("    {decorator}"));
        }

        let mut arguments = Vec::new();
        if self
            .decorators
            .iter()
            .any(|decorator| decorator == "@staticmethod")
        {
            arguments.extend(self.arguments.iter().map(String::as_str));
        } else {
            arguments.push("self");
            arguments.extend(self.arguments.iter().map(String::as_str));
        }
        let arguments = arguments.join(", ");

        text.line(format!(
            "    def {}({arguments}) -> {}: ...",
            self.name, self.output
        ));
        text.blank();
    }
}

/// Render one Python `__all__` block.
pub(super) fn render_all(text: &mut Text, names: &[String]) {
    text.line("__all__ = [");
    for name in names {
        text.line(format!("    {name:?},"));
    }
    text.line("]");
}

/// Return names exported by the root package.
pub(super) fn root_names() -> Vec<String> {
    let mut names = public_root_names();
    names.push("Workspace".to_string());
    names.push("RemoteWorkspace".to_string());
    names.push("MemoryContent".to_string());
    names.push("MemoryFile".to_string());
    names.push("MemoryWorkspace".to_string());
    names.push("open_workspace".to_string());
    names.push("VERSION".to_string());
    names.push("version".to_string());

    names
}

/// Return public root namespace names.
pub(super) fn public_root_names() -> Vec<String> {
    PUBLIC_ROOTS.iter().map(|name| name.to_string()).collect()
}

/// Return whether one Python package path is part of the public facade.
fn is_public_segments(segments: &[String]) -> bool {
    segments
        .first()
        .is_some_and(|segment| PUBLIC_ROOTS.contains(&segment.as_str()))
}

/// Return public names emitted by one generated Python module.
fn public_module_names(schema: &Schema, keys: &[String]) -> Vec<String> {
    let mut names = Vec::new();

    for key in keys {
        let item = schema.item(key);
        names.push(item.name.clone());
        names.push(encode_name(&item.name));
        names.push(decode_name(&item.name));
        names.push(to_json_name(&item.name));
        names.push(from_json_name(&item.name));

        if let Shape::Enum(variants) = &item.shape
            && !schema.is_unit_enum(&item.key)
        {
            names.extend(
                variants
                    .iter()
                    .map(|variant| protocol_variant_name(schema, item, variant)),
            );
        }
    }

    names
}
