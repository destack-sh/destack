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

/// One enclosing statement's control targets.
pub(in crate::lower) struct ControlFrame {
    /// The label naming this statement, when one does.
    pub(in crate::lower) label: Option<StringId>,
    /// The block `break` enters.
    pub(in crate::lower) break_target: mir::LocalNodeId<mir::Block>,
    /// The block `continue` enters when the statement is a loop.
    pub(in crate::lower) continue_target: Option<mir::LocalNodeId<mir::Block>>,
}

/// One declared function body awaiting lowering.
pub(in crate::lower) struct Body {
    /// The declared MIR function.
    pub(in crate::lower) function: mir::FunctionId,
    /// Whether the function receives this as its leading parameter.
    pub(in crate::lower) has_this: bool,
    /// The parameter symbols in order.
    pub(in crate::lower) parameters: Vec<dir::LocalSymbolId>,
    /// The lifetime slot declared for each induced lifetime parameter.
    pub(in crate::lower) lifetimes: FxIndexMap<dir::LocalGenericParameterId, u16>,
    /// The argument substituted for each generic parameter of this instance.
    pub(in crate::lower) substitution: FxIndexMap<dir::GlobalGenericParameterId, dir::GlobalTypeId>,
    /// The module whose DIR declares this body.
    pub(in crate::lower) source: destack_source::ModuleId,
    /// The DIR body expression.
    pub(in crate::lower) expression: dir::LocalNodeId<dir::Expression>,
}

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'a, 'b, 'c> {
    /// The module lowering state.
    pub(in crate::lower) lowerer: &'a mut ModuleLowerer<'b>,
    /// The MIR function builder.
    pub(in crate::lower) builder: mir::FunctionBuilder<'c>,
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The receiver reference of the enclosing method, when one exists.
    pub(in crate::lower) this: Option<mir::Value>,
    /// The enclosing control statements, innermost last.
    pub(in crate::lower) controls: Vec<ControlFrame>,
}

impl FunctionLowerer<'_, '_, '_> {
    /// Lower one declared function body.
    pub(in crate::lower) fn run(
        lowerer: &mut ModuleLowerer<'_>,
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
            this: None,
            controls: Vec::new(),
        };

        // bind the receiver and the parameters in header order
        let shift = body.has_this as usize;
        if body.has_this {
            function.this = Some(function.builder.function_parameter(0));
        }
        for (index, symbol) in body.parameters.iter().enumerate() {
            let value = function.builder.function_parameter(index + shift);
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
        let ty = self.lowerer.coerced_type_id(expression)?;
        let ty = self.lowerer.lower_type_id(self.builder.tree_mut(), ty)?;

        Ok(self.builder.local(ty, mir::Mutability::Mutable))
    }
}
