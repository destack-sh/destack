use std::collections::BTreeMap;

use crate::generate::schema::Schema;

use super::path::{protocol_module_names, protocol_python_segments};
use super::text::Text;

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
            Self::add(
                &mut packages,
                module.path.segments().to_vec(),
                &module.names,
            );
        }

        packages
    }

    /// Return generated Python protocol packages.
    pub(super) fn protocol(schema: &Schema) -> BTreeMap<Vec<String>, Self> {
        let mut packages = BTreeMap::<Vec<String>, Self>::new();

        for module in &schema.modules {
            let names = protocol_module_names(schema, module);
            let segments = protocol_python_segments(schema, module);
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

        text.finish()
    }

    /// Render one public Python package facade.
    pub(super) fn render_public_facade(&self) -> String {
        let mut text = Text::generated();
        self.render_public_reexports(&mut text);
        text.blank();
        render_all(&mut text, &self.names);

        text.finish()
    }

    /// Render one public Python package stub.
    pub(super) fn render_public_stub(&self) -> String {
        let mut text = Text::generated();
        text.line("from __future__ import annotations");
        text.blank();
        self.render_public_reexports(&mut text);

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

/// Render root re-exports for all generated DTO modules.
pub(super) fn render_reexports(text: &mut Text, schema: &Schema, indent: &str) {
    for module in &schema.modules {
        let path = module.path.slash_path().replace('/', ".");
        text.line(format!("{indent}from ._generated.{path} import ("));

        for name in &module.names {
            text.line(format!("{indent}    {name},"));
        }

        text.line(format!("{indent})"));
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
pub(super) fn root_names(schema: &Schema) -> Vec<String> {
    let mut names = Vec::new();
    names.push("Workspace".to_string());
    names.push("RemoteWorkspace".to_string());
    names.push("open_workspace".to_string());

    for module in &schema.modules {
        names.extend(module.names.iter().cloned());
    }

    names.push("VERSION".to_string());
    names.push("version".to_string());

    names
}
