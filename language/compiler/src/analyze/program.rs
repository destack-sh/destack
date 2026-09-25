use std::sync::Arc;

use tspp_artifact::{MirAnalyzed, ProgramAnalysis};
use tspp_mir::{LinkSupergraph, ProgramEffectTable, Symbol};

use crate::{CompilerError, CompilerResult};

/// Analyse symbol references and function effects across the program's modules.
pub(super) fn analyse(
    modules: &[Arc<MirAnalyzed>],
    roots: &[Symbol],
    previous: Option<&ProgramAnalysis>,
) -> CompilerResult<ProgramAnalysis> {
    // collect symbol links and extracted function effects
    let supergraph = LinkSupergraph::build(modules.iter().map(|module| &module.links));
    let functions = modules
        .iter()
        .flat_map(|module| module.functions.iter().cloned())
        .collect();

    // solve function effects and reuse unchanged recursive components
    let effects =
        ProgramEffectTable::analyse(functions, previous.map(|previous| &previous.effects))
            .map_err(|error| CompilerError::Internal {
                message: format!("failed to analyse MIR program: {error}"),
            })?;

    Ok(ProgramAnalysis::analyze(&supergraph, roots, effects))
}
