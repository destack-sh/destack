use destack_mir as mir;
use destack_program::{
    CoroutineKind, FunctionBuilder, FunctionExport, FunctionTableBuilder, Symbol,
};

use crate::LinkResult;

use super::ProgramLinker;

/// Link object functions into the Program function table.
#[derive(Debug)]
pub(crate) struct FunctionLinker<'a> {
    /// Dense program id projection.
    program: &'a ProgramLinker<'a>,
}

impl<'a> FunctionLinker<'a> {
    /// Create one function linker.
    pub(crate) fn new(program: &'a ProgramLinker<'a>) -> Self {
        Self { program }
    }

    /// Link program function declarations.
    pub(crate) fn link(&self) -> LinkResult<FunctionTableBuilder> {
        let mut functions = Vec::new();
        let mut exports = Vec::new();

        // project object function declarations into dense Program ids
        for (module, function_id) in self.program.functions_by_id() {
            let function = self
                .program
                .object(*module)
                .function(*function_id)
                .ok_or_else(|| {
                    self.program
                        .invalid_input(format!("missing function {function_id:?}"))
                })?;
            let program_function = self.program.function_id(*module, *function_id);
            let name = function.name;
            let signature = self.program.function_signature_id(*module, *function_id);
            let mut entry = FunctionBuilder::new(name, signature);
            if let Some(coroutine) = function.coroutine {
                entry = entry.coroutine(Self::coroutine(coroutine));
            }
            if let Some(environment) = function.environment {
                entry = entry.environment(self.program.type_id(*module, environment));
            }
            functions.push((Symbol::from_raw(function.symbol.raw()), entry));
            if function.linkage.is_exported() {
                exports.push(FunctionExport::new(name, program_function));
            }
        }

        Ok(FunctionTableBuilder::new()
            .signatures(self.program.signatures().iter().cloned())
            .functions(functions)
            .exports(exports))
    }

    /// Project MIR coroutine behavior into its durable Program tag.
    fn coroutine(coroutine: mir::CoroutineKind) -> CoroutineKind {
        match coroutine {
            mir::CoroutineKind::Async => CoroutineKind::ASYNC,
            mir::CoroutineKind::Generator => CoroutineKind::GENERATOR,
            mir::CoroutineKind::AsyncGenerator => CoroutineKind::ASYNC_GENERATOR,
        }
    }
}
