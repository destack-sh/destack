mod asset;
mod dependency;
mod link;
mod linker;
mod manifest;
mod module;
mod output;
mod plan;

#[cfg(test)]
mod tests;

const SCRIPT_SEPARATOR: &str = "\n\n";

pub(crate) use asset::AssetReference;
pub(crate) use dependency::{JsDependencyTarget, static_js_dependencies};
pub(crate) use linker::JsLinker;
pub(crate) use plan::{ModuleSet, OutputGraph, OutputId, OutputLayout};
