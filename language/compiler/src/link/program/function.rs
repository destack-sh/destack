use destack_core::SectionPacker;
use destack_mir as mir;
use destack_program::{FunctionBuilder, FunctionExport, FunctionTable, Signature};

use super::ProgramLinker;

/// Link MIR functions into program function tables.
#[derive(Debug)]
pub(crate) struct FunctionLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Dense program id projection.
    program: &'a ProgramLinker,
}

impl<'a> FunctionLinker<'a> {
    /// Create one function linker.
    pub(crate) fn new(tree: &'a mir::Tree, program: &'a ProgramLinker) -> Self {
        Self { tree, program }
    }

    /// Link program function declarations.
    pub(crate) fn link(&self, sections: &mut SectionPacker) -> FunctionTable {
        let mut functions = Vec::new();
        let mut exports = Vec::new();

        // project MIR function declarations into dense program ids
        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            let program_function = self.program.function_id(function_id);
            let slot = program_function.index();
            if slot >= functions.len() {
                functions.resize_with(slot + 1, || None);
            }

            let name = function.name;
            let signature = Signature {
                parameters: function
                    .parameters
                    .iter()
                    .map(|parameter| self.program.type_id(parameter.ty))
                    .collect(),
                result: self.program.type_id(function.return_type),
            };
            let record = FunctionBuilder {
                name,
                signature,
                environment: function.environment.map(|ty| self.program.type_id(ty)),
            };
            functions[slot] = Some(record);
            exports.push(FunctionExport {
                name,
                function: program_function,
            });
        }

        FunctionTable::pack(sections, functions, exports)
    }
}
