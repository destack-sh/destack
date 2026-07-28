use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    LifetimeParameters, LowerModuleState, ModuleLowerer, NominalInstance, TypeLowerer,
    TypeSubstitution,
};
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
pub(in crate::lower) struct ControlFrame {
    /// The label naming this statement, when one does.
    pub(in crate::lower) label: Option<StringId>,
    /// The block `break` enters.
    pub(in crate::lower) break_target: mir::LocalNodeId<mir::Block>,
    /// The block `continue` enters when the statement is a loop.
    pub(in crate::lower) continue_target: Option<mir::LocalNodeId<mir::Block>>,
}

/// One concrete function definition awaiting lowering.
pub(in crate::lower) struct FunctionDefinition {
    /// The declared MIR function.
    pub(in crate::lower) function: mir::FunctionId,
    /// Whether the function receives this as its leading parameter.
    pub(in crate::lower) has_this: bool,
    /// The parameter symbols in order.
    pub(in crate::lower) parameters: Vec<dir::LocalSymbolId>,
    /// The concrete type substitutions of this definition.
    pub(in crate::lower) type_substitution: TypeSubstitution,
    /// The polymorphic lifetime parameters of this definition.
    pub(in crate::lower) lifetime_parameters: LifetimeParameters,
    /// The module whose DIR declares this body.
    pub(in crate::lower) source: destack_source::ModuleId,
    /// The DIR body expression.
    pub(in crate::lower) expression: dir::LocalNodeId<dir::Expression>,
}

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'lowerer, 'builder, 'module> {
    /// The module lowering state.
    pub(in crate::lower) lowerer: &'lowerer mut ModuleLowerer<'module>,
    /// The MIR function builder.
    pub(in crate::lower) builder: mir::FunctionBuilder<'builder>,
    /// The module whose DIR declares this function.
    pub(in crate::lower) source: ModuleId,
    /// The concrete type substitutions of this function.
    pub(in crate::lower) type_substitution: TypeSubstitution,
    /// The polymorphic lifetime parameters of this function.
    pub(in crate::lower) lifetime_parameters: LifetimeParameters,
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The receiver reference of the enclosing method, when one exists.
    pub(in crate::lower) this: Option<mir::Value>,
    /// The enclosing control statements, innermost last.
    pub(in crate::lower) controls: Vec<ControlFrame>,
}

impl<'module> FunctionLowerer<'_, '_, 'module> {
    /// Lower one declared function body.
    pub(in crate::lower) fn lower(
        lowerer: &mut ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        definition: FunctionDefinition,
    ) -> CompilerResult<()> {
        let FunctionDefinition {
            function,
            has_this,
            parameters,
            type_substitution,
            lifetime_parameters,
            source,
            expression,
        } = definition;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("MIR body start failed: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lowerer,
            builder,
            source,
            type_substitution,
            lifetime_parameters,
            values: FxIndexMap::default(),
            this: None,
            controls: Vec::new(),
        };

        // bind the receiver and the parameters in header order
        let shift = has_this as usize;
        if has_this {
            function.this = Some(function.builder.function_parameter(0));
        }
        for (index, symbol) in parameters.iter().enumerate() {
            let value = function.builder.function_parameter(index + shift);
            function.values.insert(*symbol, Binding::Value(value));
        }
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);
        function.lower_body(expression)?;
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("MIR function build failed: {error}"),
            })?;

        Ok(())
    }

    /// Return the sealed check output declaring this function.
    pub(in crate::lower) fn source(&self) -> &LowerModuleState {
        match self.lowerer.modules.get(&self.source) {
            Some(state) => state,
            None => unreachable!("the function source module is always loaded"),
        }
    }

    /// Allocate one join local typed as one expression's runtime type.
    pub(in crate::lower) fn value_slot(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Local>> {
        let ty = self.node_type_id(expression)?;
        let ty = self.lower_type(ty)?;

        Ok(self.builder.local(ty, mir::Mutability::Mutable))
    }

    /// Lower one checked type into this function's MIR tree.
    pub(in crate::lower) fn lower_type(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        self.type_lowerer().lower(id)
    }

    /// Return recursive type lowering for this function.
    pub(in crate::lower) fn type_lowerer(&mut self) -> TypeLowerer<'_, 'module> {
        let pointer_bytes = (self.builder.pointer_bits() / 8) as u8;

        self.lowerer.type_lowerer(
            self.builder.tree_mut(),
            pointer_bytes,
            &self.type_substitution,
            &self.lifetime_parameters,
        )
    }

    /// Lower the nominal representation beneath one checked value type.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        let stored = match self.lowerer.peel_reference(id)? {
            Some(reference) => reference.stored,
            None => self.lowerer.peel_owned(id)?,
        };
        let dir::Type::Application(instance) = self.lowerer.ty(stored)? else {
            return Err(CompilerError::Internal {
                message: "checked value does not carry a nominal representation".to_string(),
            });
        };
        let arguments = self
            .lowerer
            .types(stored.module_id)?
            .type_ids(instance.arguments)
            .to_vec();
        self.type_lowerer()
            .lower_nominal(instance.symbol, &arguments)
    }
}
