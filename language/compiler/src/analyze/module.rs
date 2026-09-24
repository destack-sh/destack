use std::sync::Arc;

use destack_artifact::{MirAnalyzed, MirElaborated};
use destack_mir as mir;

use crate::{CompilerError, CompilerResult};

/// Analyse symbol links and extract function effects for one elaborated module.
pub(super) fn analyse(elaborated: &MirElaborated) -> CompilerResult<MirAnalyzed> {
    // resolve calls once for symbol links and function extraction
    let resolution = mir::ResolutionTable::analyse(&elaborated.dispatch, None, &elaborated.tree);
    let calls = mir::CallTable::analyse(&resolution, &elaborated.tree);
    let links = mir::LinkTable::analyse(
        &calls,
        &elaborated.effects,
        &elaborated.drops,
        &elaborated.tree,
    );

    // extract the module's function effects and pointer flows
    let mut functions = Vec::new();
    for (function, declaration) in elaborated.tree.iter_nodes::<mir::Function>() {
        let body = mir::FunctionEffectBody::analyse(
            function,
            &resolution,
            &elaborated.effects,
            &elaborated.tree,
        )
        .map_err(|error| CompilerError::Internal {
            message: format!("failed to extract MIR function effects: {error}"),
        })?;
        functions.push((declaration.symbol, Arc::new(body)));
    }

    // record the module initializer symbol
    let initializer = elaborated
        .initializer
        .map(|function| elaborated.tree.get(function).symbol);

    Ok(MirAnalyzed {
        links,
        initializer,
        functions,
    })
}
