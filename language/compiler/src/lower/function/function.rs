use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;

use crate::lower::ModuleLowerer;
use crate::{CompilerError, CompilerResult};

/// One lowered value bound to a symbol.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Binding {
    /// An immutable SSA value.
    Value(mir::Value),
    /// A mutable local.
    Local(mir::LocalNodeId<mir::Local>),
}

/// One enclosing loop's control targets.
pub(in crate::lower) struct LoopFrame {
    /// The label naming this loop, when one does.
    pub(in crate::lower) label: Option<StringId>,
    /// The block continue re-enters.
    pub(in crate::lower) continue_target: mir::LocalNodeId<mir::Block>,
    /// The exit block break jumps to.
    pub(in crate::lower) exit: mir::LocalNodeId<mir::Block>,
}

/// One declared function body awaiting lowering.
pub(in crate::lower) struct Body {
    /// The declared MIR function.
    pub(in crate::lower) function: mir::FunctionId,
    /// The parameter symbols in order.
    pub(in crate::lower) parameters: Vec<dir::LocalSymbolId>,
    /// The DIR body expression.
    pub(in crate::lower) expression: dir::LocalNodeId<dir::Expression>,
}

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'a, 'b> {
    /// The module lowering state.
    pub(in crate::lower) lowerer: &'a ModuleLowerer<'a>,
    /// The MIR function builder.
    pub(in crate::lower) builder: mir::FunctionBuilder<'b>,
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The enclosing loops, innermost last.
    pub(in crate::lower) loops: Vec<LoopFrame>,
}

impl ModuleLowerer<'_> {
    /// Declare the MIR header for one function declaration with a body.
    pub(in crate::lower) fn declare_function(
        &mut self,
        builder: &mut mir::ModuleBuilder,
        declaration: dir::LocalNodeId<dir::Declaration>,
        function: &dir::FunctionDeclaration,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Body> {
        let module = self.module;

        // resolve the checked parameter types alongside their symbols
        let mut parameters = Vec::with_capacity(function.signature.parameters.len());
        let mut symbols = Vec::with_capacity(function.signature.parameters.len());
        for parameter in &function.signature.parameters {
            let node = parameter.into_global_any(module);
            let Some(symbol) = self.symbol_declared_at(node) else {
                return Err(CompilerError::Internal {
                    message: "checked DIR is missing a symbol for one parameter".to_string(),
                });
            };
            let ty = self.ty(self.symbol_type(symbol)?)?;
            let ty = self.lower_type(&ty)?;
            parameters.push(builder.insert_type(ty));
            symbols.push(symbol.local_id);
        }

        // resolve the checked return type from the function's own symbol
        let node = declaration.into_global_any(module);
        let Some(symbol) = self.symbol_declared_at(node) else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a symbol for one function declaration".to_string(),
            });
        };
        let declared = self.symbol_type(symbol)?;
        let result = match self.signature_return(declared)? {
            Some(return_type) => {
                let ty = self.ty(return_type)?;
                let ty = self.lower_type(&ty)?;

                builder.insert_type(ty)
            }
            None => builder.type_void(),
        };

        // declare the header under the function's name
        let Some(name) = function.name else {
            return Err(CompilerError::Internal {
                message: "checked DIR is missing a name on one lowered function declaration"
                    .to_string(),
            });
        };
        let name = self.strings.get(name.string()).to_string();
        let header = builder
            .function_header(&name)
            .parameters(parameters)
            .result(result);
        let function = builder.declare_function(header);
        self.functions.insert(symbol.local_id, function);

        Ok(Body {
            function,
            parameters: symbols,
            expression,
        })
    }
}

impl FunctionLowerer<'_, '_> {
    /// Lower one declared function body.
    pub(in crate::lower) fn run(
        lowerer: &ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        body: Body,
    ) -> CompilerResult<()> {
        let builder =
            builder
                .function_body(body.function)
                .map_err(|error| CompilerError::Internal {
                    message: format!("MIR body start failed: {error}"),
                })?;
        let mut function = FunctionLowerer {
            lowerer,
            builder,
            values: FxIndexMap::default(),
            loops: Vec::new(),
        };

        // bind the parameters and lower the body block by block
        for (index, symbol) in body.parameters.iter().enumerate() {
            let value = function.builder.function_parameter(index);
            function.values.insert(*symbol, Binding::Value(value));
        }
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);
        function.lower_body(body.expression)?;
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("MIR function build failed: {error}"),
            })?;

        Ok(())
    }

    /// Allocate one join local typed as one expression's runtime type.
    pub(in crate::lower) fn value_slot(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Local>> {
        let ty = self.lowerer.coerced_type(expression)?;
        let ty = self.lowerer.lower_type(&ty)?;
        let ty = self.builder.tree_mut().insert_type(ty);

        Ok(self.builder.local(ty, mir::Mutability::Mutable))
    }
}
