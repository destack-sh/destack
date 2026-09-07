use destack_artifact::Script;
use destack_core::{FxIndexMap, FxIndexSet};
use destack_dir::GlobalSymbolId;
use destack_js as js;
use destack_source::ModuleId;

use crate::JsLinker;

/// One symbol identity within a linked JavaScript output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum OutputSymbol {
    /// One symbol retained from DIR.
    Source(GlobalSymbolId),
    /// One generated resource module default.
    ModuleDefault(ModuleId),
    /// One symbol local to an emitted JavaScript module.
    Local(ModuleId, js::SymbolId),
}

impl JsLinker<'_> {
    /// Assign collision-free names to one linked output.
    pub(super) fn assign_output_names(&self, modules: &mut [(ModuleId, Script)]) {
        let mut groups = FxIndexMap::<OutputSymbol, Vec<(usize, js::SymbolId)>>::default();
        let mut reserved = FxIndexSet::<String>::default();

        // group linked symbols and reserve unresolved runtime globals
        for (module_index, (module_id, script)) in modules.iter().enumerate() {
            for (symbol, _) in script.module.symbols.iter() {
                let symbol = script.module.symbols.canonical(symbol);
                let entry = script.module.symbols.get(symbol);
                if script.is_external_symbol(symbol) {
                    let name = script.module.strings.get(entry.name).to_string();
                    reserved.insert(name);
                    continue;
                }

                let identity = self.output_symbol(*module_id, script, symbol);
                let occurrence = (module_index, symbol);
                let group = groups.entry(identity).or_default();
                if !group.contains(&occurrence) {
                    group.push(occurrence);
                }
            }
        }

        let mut renames = vec![Vec::<(js::SymbolId, String)>::new(); modules.len()];

        // assign one output name to each linked identity
        for (_, group) in groups {
            let (module_index, symbol) = group[0];
            let module = &modules[module_index].1.module;
            let original = module.strings.get(module.symbols.get(symbol).name);
            let name = Self::allocate_output_name(original, &mut reserved);

            for (module_index, symbol) in group {
                let module = &modules[module_index].1.module;
                let current = module.strings.get(module.symbols.get(symbol).name);
                if current != name {
                    renames[module_index].push((symbol, name.clone()));
                }
            }
        }

        // apply each module's names under one provenance transform
        for ((_, script), renames) in modules.iter_mut().zip(renames) {
            if renames.is_empty() {
                continue;
            }

            script.rewrite("rename-javascript-symbols", |script, provenance| {
                for (symbol, name) in renames {
                    let name = script.module.strings.intern(&name);
                    script.module.symbols.rename(symbol, name, provenance);
                }
            });
        }
    }

    /// Allocate one readable output name.
    fn allocate_output_name(original: &str, reserved: &mut FxIndexSet<String>) -> String {
        let mut suffix = 1usize;
        loop {
            let candidate = if suffix == 1 {
                original.to_string()
            } else {
                format!("{original}${suffix}")
            };
            if reserved.insert(candidate.clone()) {
                return candidate;
            }

            suffix += 1;
        }
    }

    /// Return one output-wide identity for a JavaScript symbol.
    fn output_symbol(
        &self,
        module: ModuleId,
        script: &Script,
        symbol: js::SymbolId,
    ) -> OutputSymbol {
        if let Some(source) = script.source_symbol(symbol) {
            OutputSymbol::Source(source)
        } else if let Some(module) = script.default_module(symbol) {
            OutputSymbol::ModuleDefault(module)
        } else {
            OutputSymbol::Local(module, symbol)
        }
    }
}
