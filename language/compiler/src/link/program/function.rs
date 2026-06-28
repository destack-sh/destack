use std::collections::HashMap;

use destack_core::StringPool;
use destack_mir as mir;
use destack_program::{Function, FunctionTable, Signature};

use super::ProgramLinker;

/// Link MIR functions into executable function tables.
#[derive(Debug)]
pub(crate) struct FunctionLinker<'a> {
    /// MIR tree being linked.
    tree: &'a mir::Tree,
    /// Program string pool.
    strings: &'a StringPool,
    /// Dense executable id projection for this program.
    program: &'a ProgramLinker,
}

impl<'a> FunctionLinker<'a> {
    /// Create one function linker.
    pub(crate) fn new(
        tree: &'a mir::Tree,
        strings: &'a StringPool,
        program: &'a ProgramLinker,
    ) -> Self {
        Self {
            tree,
            strings,
            program,
        }
    }

    /// Link executable function declarations.
    pub(crate) fn link(&self) -> FunctionTable {
        let mut functions = Vec::new();
        let mut function_by_name = HashMap::new();

        // project MIR function declarations into dense program ids
        for (function_id, function) in self.tree.iter_nodes::<mir::Function>() {
            let program_function = self.program.function_id(function_id);
            let slot = program_function.index();
            if slot >= functions.len() {
                functions.resize_with(slot + 1, || None);
            }

            let name = self.strings.get(function.name).to_string();
            let signature = Signature {
                parameters: function
                    .parameters
                    .iter()
                    .map(|parameter| self.program.type_id(parameter.ty))
                    .collect(),
                result: self.program.type_id(function.return_type),
            };
            let record = Function {
                name: name.clone(),
                signature,
                environment: function.environment.map(|ty| self.program.type_id(ty)),
            };
            functions[slot] = Some(record);
            function_by_name.entry(name).or_insert(program_function);
        }

        FunctionTable::new(functions, function_by_name)
    }
}
