use destack_compiler::Compiler;
use destack_core::StringPool;
use destack_dir::Declaration;
use destack_source::ModuleId;
use destack_workspace::{ProfileId, Program};

use super::domain::module_platform_domain;
use crate::platform::model::{
    BindingType, BindingTypeContext, binding_type_symbols, patched_dir_artifact,
};

/// Collect exported platform type declarations from builtin modules.
pub(crate) fn collect_platform_types(
    compiler: &Compiler,
    program: &Program,
    strings: &StringPool,
    profile_id: ProfileId,
    platform_modules: &[ModuleId],
) -> Vec<BindingType> {
    // collect binding type symbols for declaration lowering
    let binding_symbols = binding_type_symbols(compiler, compiler.artifacts.as_ref(), profile_id);

    // accumulate exported platform types
    let mut binding_types = Vec::new();

    // scan each platform module for exported type declarations
    for module_id in platform_modules {
        let module = program.modules.get(*module_id);
        let module = module.as_ref();

        let dir =
            patched_dir_artifact(compiler, compiler.artifacts.as_ref(), module.id, profile_id);
        let tree = &dir.tree;
        let types = &dir.types;
        let symbols = &dir.symbols;
        let domain =
            module_platform_domain(module.uri.as_ref()).unwrap_or_else(|| "global".to_string());

        // collect exported type-like declarations
        for (_declaration_id, declaration) in tree.iter_nodes_of_type::<Declaration>() {
            let descriptor = declaration.descriptor();
            if descriptor.export.is_none() {
                continue;
            }

            // lower exported nominal binding types
            let binding_type = match declaration {
                Declaration::Type { .. }
                | Declaration::Struct { .. }
                | Declaration::Enum { .. } => {
                    let symbol_id = declaration.symbol().into_global(module.id);
                    let binding_context = BindingTypeContext::new(
                        compiler,
                        compiler.artifacts.as_ref(),
                        tree,
                        types,
                        symbols,
                        &program.modules,
                        strings,
                        profile_id,
                        &binding_symbols,
                        &domain,
                    );
                    binding_context.binding_type_from_symbol(symbol_id)
                }

                _ => continue,
            };
            binding_types.push(binding_type);
        }
    }

    binding_types
}
