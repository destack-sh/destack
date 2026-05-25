mod asset;
mod dependency;
mod link;
mod linker;
mod manifest;
mod output;
mod plan;
mod script;

#[cfg(test)]
mod tests;

pub(crate) use asset::AssetReference;
pub(crate) use dependency::{
    ScriptDependencyTarget, dynamic_script_dependencies, static_script_dependencies,
};
pub(crate) use linker::ScriptLinker;
pub(crate) use plan::{ModuleSet, OutputGraph, OutputId, OutputLayout};
