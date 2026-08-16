use std::sync::Arc;

use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    GenericInstanceKey, LifetimeParameters, LowerModuleState, Lowered, ModuleLowerer,
    NominalInstance, insert_local_reference,
};
use crate::{CompilerError, CompilerResult};

/// One lowered value bound to a symbol.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Binding {
    /// An immutable SSA value.
    Value(mir::Value),
    /// A mutable local.
    Local(mir::LocalNodeId<mir::Local>),
    /// A field of a managed capture frame.
    Captured {
        /// The frame reference holding the binding.
        frame: mir::Value,
        /// The field index within the frame.
        field: u32,
        /// The stored field type.
        ty: mir::LocalNodeId<mir::Type>,
    },
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
    /// The declared function.
    pub(in crate::lower) function: mir::FunctionId,
    /// The symbol declaring this function.
    pub(in crate::lower) symbol: dir::GlobalSymbolId,
    /// Whether the function receives this as its leading parameter.
    pub(in crate::lower) has_this: bool,
    /// The parameter symbols in order.
    pub(in crate::lower) parameters: Vec<dir::LocalSymbolId>,
    /// The sema instance this definition specializes, when generic.
    pub(in crate::lower) instance: Option<(ModuleId, dir::LocalInstanceId)>,
    /// The polymorphic lifetime parameters of this definition.
    pub(in crate::lower) lifetime_parameters: LifetimeParameters,
    /// The module declaring this body.
    pub(in crate::lower) source: destack_source::ModuleId,
    /// The body expression.
    pub(in crate::lower) expression: dir::LocalNodeId<dir::Expression>,
}

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'lowerer, 'builder, 'module> {
    /// The module lowering state.
    pub(in crate::lower) lowerer: &'lowerer mut ModuleLowerer<'module>,
    /// The function builder.
    pub(in crate::lower) builder: mir::FunctionBuilder<'builder>,
    /// The module declaring this function.
    pub(in crate::lower) source: ModuleId,
    /// The sema instance this function specializes, when generic.
    pub(in crate::lower) instance: Option<(ModuleId, dir::LocalInstanceId)>,
    /// The polymorphic lifetime parameters of this function.
    pub(in crate::lower) lifetime_parameters: LifetimeParameters,
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The allocated capture frame for each lifted scope.
    pub(in crate::lower) frames: FxIndexMap<dir::GlobalScopeId, mir::Value>,
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
            symbol,
            has_this,
            parameters,
            instance,
            lifetime_parameters,
            source,
            expression,
        } = definition;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("function body start failed: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lowerer,
            builder,
            source,
            instance,
            lifetime_parameters,
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
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

        // open the entry block
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);

        // receive the closure environment and lift any captured parameters
        function.bind_captures(symbol)?;
        for symbol in &parameters {
            if let Some(Binding::Value(value)) = function.values.get(symbol).copied() {
                function.bind_lifted(symbol.into_global(source), value)?;
            }
        }

        // lower the body and finalize its blocks
        function.lower_body(expression)?;
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("function build failed: {error}"),
            })?;

        Ok(())
    }

    /// Lower the module initializer storing each runtime binding.
    pub(in crate::lower) fn lower_initializer(
        lowerer: &mut ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        function: mir::FunctionId,
        initializers: Vec<(
            mir::LocalNodeId<mir::Global>,
            dir::LocalNodeId<dir::Expression>,
        )>,
    ) -> CompilerResult<()> {
        let source = lowerer.module;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("function body start failed: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lowerer,
            builder,
            source,
            instance: None,
            lifetime_parameters: LifetimeParameters::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            controls: Vec::new(),
        };

        // open the entry block
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);

        // store each binding in declaration order
        for (global, expression) in initializers {
            let value = function.lower_expression(expression)?;
            function.builder.store_global(global, value);
        }

        // close the initializer with a void return
        function.builder.return_(None);
        function.builder.seal_all_blocks();
        function
            .builder
            .finish()
            .map_err(|error| CompilerError::Internal {
                message: format!("function build failed: {error}"),
            })?;

        Ok(())
    }

    /// Return the state of the module declaring this function.
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

    /// Return the representation of one type, lowering it at first read.
    pub(in crate::lower) fn lower_type(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let id = self.lowerer.instance_type(self.instance, id)?;

        // lower the representation once for every body that reads it
        if !self.lowerer.representations.contains_key(&id) {
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lowerer
                .type_lowerer(
                    self.builder.tree_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower(id);
            Self::bank(&mut self.lowerer.representations, id, outcome)?;
        }

        // read the banked outcome, cascading the kept failure
        match &self.lowerer.representations[&id] {
            Ok(node) => Ok(*node),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Return the dispatch shape of one constraint, lowering it at first read.
    pub(in crate::lower) fn lower_constraint(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // resolve the written id through its materialized types
        let id = self.lowerer.instance_type(self.instance, id)?;

        // lower the dispatch shape once for every body that reads it
        if !self.lowerer.constraints.contains_key(&id) {
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lowerer
                .type_lowerer(
                    self.builder.tree_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower_dynamic_constraint(id);
            Self::bank(&mut self.lowerer.constraints, id, outcome)?;
        }

        // read the banked outcome, cascading the kept failure
        match &self.lowerer.constraints[&id] {
            Ok(shape) => Ok(*shape),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Return the declared instance key of one instantiated callable.
    pub(in crate::lower) fn generic_instance_key(
        &mut self,
        symbol: dir::GlobalSymbolId,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        let mut arguments = Vec::with_capacity(types.len());
        for ty in types {
            let node = self.lower_type(*ty)?;
            let argument = self
                .builder
                .tree_mut()
                .intern_static(mir::Static::Type(mir::TypeId::from(node)));
            arguments.push(argument);
        }

        Ok(GenericInstanceKey { symbol, arguments })
    }

    /// Return the nominal instance beneath one value type, lowering it at first read.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        let stored = match self.lowerer.peel_indirection(id)? {
            Some(reference) => reference.stored,
            None => self.lowerer.peel_owned(id)?,
        };

        // lower the nominal instance once for every body that reads it
        if !self.lowerer.stored_nominals.contains_key(&stored) {
            let dir::Type::Application(instance) = self.lowerer.ty(stored)? else {
                return Err(CompilerError::Internal {
                    message: format!("a nominal read outside an application type {stored:?}"),
                });
            };
            let arguments = self
                .lowerer
                .types(stored.module_id)?
                .type_ids(instance.arguments)
                .to_vec();
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lowerer
                .type_lowerer(
                    self.builder.tree_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower_nominal(instance.symbol, &arguments);
            Self::bank(&mut self.lowerer.stored_nominals, stored, outcome)?;
        }

        // read the banked outcome, cascading the kept failure
        match &self.lowerer.stored_nominals[&stored] {
            Ok(nominal) => Ok(nominal.clone()),
            Err(diagnostic) => Err(CompilerError::Diagnostic(Box::new(diagnostic.clone()))),
        }
    }

    /// Bank one lowering outcome under its key for every body that reads it.
    fn bank<T>(
        outcomes: &mut FxIndexMap<dir::GlobalTypeId, Lowered<T>>,
        key: dir::GlobalTypeId,
        outcome: CompilerResult<T>,
    ) -> CompilerResult<()> {
        match outcome {
            Ok(value) => {
                outcomes.insert(key, Ok(value));
            }
            Err(CompilerError::Diagnostic(diagnostic)) => {
                outcomes.insert(key, Err(Arc::from(diagnostic)));
            }
            Err(error) => return Err(error),
        }

        Ok(())
    }

    /// Intern one local reference type over a lowered pointee.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::ReferenceKind,
        access: mir::Access,
        pointee: mir::LocalNodeId<mir::Type>,
    ) -> mir::LocalNodeId<mir::Type> {
        insert_local_reference(self.builder.tree_mut(), kind, access, pointee)
    }
}
