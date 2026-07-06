use destack_core::SectionPacker;
use destack_mir as mir;
use destack_program::{BindingId, FunctionBuilder, FunctionExport, FunctionTable, Signature};

use crate::LinkResult;

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
    pub(crate) fn link(&self, sections: &mut SectionPacker) -> LinkResult<FunctionTable> {
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
            let binding = self.binding_id(function)?;
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
                binding,
            };
            functions[slot] = Some(record);
            exports.push(FunctionExport {
                name,
                function: program_function,
            });
        }

        Ok(FunctionTable::pack(sections, functions, exports))
    }

    /// Return the runtime binding id for one imported function.
    fn binding_id(&self, function: &mir::Function) -> LinkResult<Option<BindingId>> {
        if !function.is_import() {
            return Ok(None);
        }

        let Some(binding) = function.binding_name() else {
            let function_name = self.program.string(function.name);
            return Err(self.program.invalid_input(format!(
                "imported function '{function_name}' has no binding"
            )));
        };

        let name = self.program.string(binding);

        Ok(Some(BindingId::from_name(name)))
    }
}
