use std::sync::Arc;

use destack_core::{FxIndexMap, StringId};
use destack_dir as dir;
use destack_mir as mir;
use destack_source::ModuleId;

use crate::lower::{
    GenericInstanceKey, LifetimeParameters, LowerModuleState, Lowered, ModuleLowerer,
    NominalInstance, insert_reference_type,
};
use crate::{CompilerError, CompilerResult};

/// Lowering state for one function body.
pub(in crate::lower) struct FunctionLowerer<'lowerer, 'builder, 'module> {
    // context
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
    /// The class whose constructor this body runs, when it is one.
    pub(in crate::lower) constructs: Option<dir::GlobalSymbolId>,

    // body state
    /// The lowered binding for each symbol.
    pub(in crate::lower) values: FxIndexMap<dir::LocalSymbolId, Binding>,
    /// The allocated capture frame for each lifted scope.
    pub(in crate::lower) frames: FxIndexMap<dir::GlobalScopeId, mir::Value>,
    /// The receiver binding of the enclosing method, when one exists.
    pub(in crate::lower) this: Option<Binding>,
    /// The enclosing control statements, innermost last.
    pub(in crate::lower) controls: Vec<ControlFrame>,
}

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
        ty: mir::TypeId,
    },
}

/// One enclosing control statement's break and continue targets.
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
    pub(in crate::lower) source: ModuleId,
    /// The body expression.
    pub(in crate::lower) expression: dir::LocalNodeId<dir::Expression>,
    /// The class this constructor body initializes, when one exists.
    pub(in crate::lower) constructs: Option<dir::GlobalSymbolId>,
    /// The declared default expression of each parameter, in header order.
    pub(in crate::lower) defaults: Vec<Option<dir::LocalNodeId<dir::Expression>>>,
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
            constructs,
            defaults,
        } = definition;

        // open the declared body and stand up the lowering state around it
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
            constructs,
            controls: Vec::new(),
        };

        // bind the parameters in header order past any receiver
        let shift = has_this as usize;
        for (index, symbol) in parameters.iter().enumerate() {
            let value = function.builder.function_parameter(index + shift);
            function.values.insert(*symbol, Binding::Value(value));
        }

        // open the entry block
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);

        // home the receiver ahead of anything reading it
        if has_this {
            let value = function.builder.function_parameter(0);
            function.this = Some(function.bind_receiver(value)?);
        }

        // receive the closure environment before defaults can read captures
        function.bind_captures(symbol)?;

        // resolve the defaulted parameters before anything reads them
        function.lower_parameter_defaults(&parameters, &defaults)?;

        // lift every parameter a nested scope captures into its frame
        for symbol in &parameters {
            if let Some(Binding::Value(value)) = function.values.get(symbol).copied() {
                function.bind_lifted(symbol.into_global(source), value)?;
            }
        }

        // store the declared field initializers before a base class constructor body
        if let Some(owner) = constructs
            && !function.lowerer.class_extends_base(owner)?
        {
            function.lower_field_initializers(owner)?;
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

    /// Lower one synthesized default constructor to its initializer prologue.
    pub(in crate::lower) fn lower_default_constructor(
        lowerer: &mut ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        class: dir::GlobalSymbolId,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        function: mir::FunctionId,
    ) -> CompilerResult<()> {
        // open the synthesized body and stand up the lowering state around it
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("function body start failed: {error}"),
            })?;
        let mut function = FunctionLowerer {
            lowerer,
            builder,
            source: class.module_id,
            instance,
            lifetime_parameters: LifetimeParameters::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            constructs: None,
            controls: Vec::new(),
        };

        // open the entry block and home the receiver like any declared constructor
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);
        let value = function.builder.function_parameter(0);
        function.this = Some(function.bind_receiver(value)?);

        // store the declared field initializers the synthesized body stands in for
        function.lower_field_initializers(class)?;

        // close the constructor with a void return
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

    /// Lower one synthesized builtin clone to a copy of its receiver's pointee.
    pub(in crate::lower) fn lower_builtin_clone(
        lowerer: &mut ModuleLowerer<'_>,
        builder: &mut mir::ModuleBuilder,
        function: mir::FunctionId,
        result: mir::TypeId,
    ) -> CompilerResult<()> {
        // open the synthesized body and stand up the lowering state around it
        let module = lowerer.module;
        let builder = builder
            .function_body(function)
            .map_err(|error| CompilerError::Internal {
                message: format!("function body start failed: {error}"),
            })?;
        let mut lowering = FunctionLowerer {
            lowerer,
            builder,
            source: module,
            instance: None,
            lifetime_parameters: LifetimeParameters::default(),
            values: FxIndexMap::default(),
            frames: FxIndexMap::default(),
            this: None,
            constructs: None,
            controls: Vec::new(),
        };

        // copy the borrowed receiver's value and return it
        let entry = lowering.builder.block();
        lowering.builder.switch_to_block(entry);
        let receiver = lowering.builder.function_parameter(0);
        let value = lowering.builder.load(receiver, result);
        lowering.builder.return_(Some(value));
        lowering.builder.seal_all_blocks();
        lowering
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
        // open the initializer body and stand up the lowering state around it
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
            constructs: None,
            controls: Vec::new(),
        };

        // open the entry block
        let entry = function.builder.block();
        function.builder.switch_to_block(entry);

        // store each binding in declaration order
        for (global, expression) in initializers {
            function.lower_anchored(expression, |function| {
                let value = function.lower_expression(expression)?;
                function.builder.store_global(global, value);

                Ok(())
            })?;
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

    /// Lower one expression from another module's tree.
    pub(in crate::lower) fn lower_foreign_expression(
        &mut self,
        module: ModuleId,
        instance: Option<(ModuleId, dir::LocalInstanceId)>,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<mir::Value> {
        // swap into the declaring module with no local bindings in scope
        let source = std::mem::replace(&mut self.source, module);
        let outer_instance = std::mem::replace(&mut self.instance, instance);
        let values = std::mem::take(&mut self.values);
        let frames = std::mem::take(&mut self.frames);
        let this = self.this.take();

        // lower the expression against the declaring module
        let value = self.lower_expression(expression);

        // swap the enclosing body's state back in, keeping the outcome
        self.source = source;
        self.instance = outer_instance;
        self.values = values;
        self.frames = frames;
        self.this = this;

        value
    }

    /// Lower one expression with its emitted MIR attributed to that occurrence.
    pub(in crate::lower) fn lower_anchored<R>(
        &mut self,
        expression: dir::LocalNodeId<dir::Expression>,
        lower: impl FnOnce(&mut Self) -> CompilerResult<R>,
    ) -> CompilerResult<R> {
        let node = expression.into_global_any(self.source);
        let mut source = self.lowerer.node_provenance(node)?;

        // expand generic body occurrences through their closed instance
        if let Some(instance) = self.instance {
            let site = self.lowerer.instance_provenance(instance)?;
            let (_, mut provenance) = self.builder.split_mut();
            source = provenance.expand(source, site);
        }

        let previous = self.builder.replace_source(source);
        let result = lower(self);
        self.builder.replace_source(previous);

        result
    }

    /// Return the state of the module declaring this function.
    pub(in crate::lower) fn source(&self) -> &LowerModuleState {
        match self.lowerer.modules.get(&self.source) {
            Some(state) => state,
            None => unreachable!("the function source module is always loaded"),
        }
    }

    /// Return the representation type of one lowered value.
    pub(in crate::lower) fn value_representation(
        &self,
        value: mir::Value,
    ) -> CompilerResult<mir::TypeId> {
        self.builder
            .value_type(value)
            .ok_or_else(|| CompilerError::Internal {
                message: "a lowered value has no representation".to_string(),
            })
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
    ) -> CompilerResult<mir::TypeId> {
        // resolve the written id through its materialized types
        let id = self.lowerer.instance_type(self.instance, id)?;

        // lower the representation once for every body that reads it
        if !self.lowerer.representations.contains_key(&id) {
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lowerer
                .type_lowerer(
                    self.builder.split_mut(),
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
    ) -> CompilerResult<mir::TypeId> {
        // resolve the written id through its materialized types
        let id = self.lowerer.instance_type(self.instance, id)?;

        // lower the dispatch shape once for every body that reads it
        if !self.lowerer.constraints.contains_key(&id) {
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lowerer
                .type_lowerer(
                    self.builder.split_mut(),
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
        receiver: Option<dir::GlobalTypeId>,
        types: &[dir::GlobalTypeId],
    ) -> CompilerResult<GenericInstanceKey> {
        let receiver = match receiver {
            Some(ty) => {
                let ty = self.lower_type(ty)?;
                Some(self.builder.tree_mut().intern_static(mir::Static::Type(ty)))
            }
            None => None,
        };
        let mut arguments = Vec::with_capacity(types.len());
        for ty in types {
            // lifetime arguments never shape a specialization
            if self.lowerer.type_is_lifetime(*ty)? {
                continue;
            }

            match self.lowerer.place_space(*ty)? {
                // local place arguments canonicalize onto the plain declaration
                Some(dir::Space::Local) => {}
                // other place arguments bind their space
                Some(space) => {
                    let space = ModuleLowerer::mir_space(space);
                    let argument = self
                        .builder
                        .tree_mut()
                        .intern_static(mir::Static::Space(space));
                    arguments.push(argument);
                }
                // every other argument binds a type
                None => {
                    let ty = self.lower_type(*ty)?;
                    let argument = self.builder.tree_mut().intern_static(mir::Static::Type(ty));
                    arguments.push(argument);
                }
            }
        }

        Ok(GenericInstanceKey {
            symbol,
            receiver,
            arguments,
        })
    }

    /// Return the nominal instance beneath one value type, lowering it at first read.
    pub(in crate::lower) fn lower_nominal(
        &mut self,
        id: dir::GlobalTypeId,
    ) -> CompilerResult<NominalInstance> {
        // peel the value type down to the nominal it stores
        let stored = match self.lowerer.peel_indirection(id)? {
            Some(reference) => reference.stored,
            None => self.lowerer.peel_owned(id)?,
        };

        // lower the nominal instance once for every body that reads it
        if !self.lowerer.stored_nominals.contains_key(&stored) {
            let dir::Type::Application(_) = self.lowerer.ty(stored)? else {
                return Err(CompilerError::Internal {
                    message: format!(
                        "a nominal read outside an application type: {:?}",
                        self.lowerer.ty(stored)?
                    ),
                });
            };
            let pointer_bytes = self.builder.pointer_bytes();
            let outcome = self
                .lowerer
                .type_lowerer(
                    self.builder.split_mut(),
                    pointer_bytes,
                    &self.lifetime_parameters,
                )
                .with_instance(self.instance)
                .lower_nominal(stored);
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
            // keep the lowered value
            Ok(value) => {
                outcomes.insert(key, Ok(value));
            }
            // keep the diagnostic so every later read cascades it
            Err(CompilerError::Diagnostic(diagnostic)) => {
                outcomes.insert(key, Err(Arc::from(diagnostic)));
            }
            // raise every internal failure straight out
            Err(error) => return Err(error),
        }

        Ok(())
    }

    /// Intern one reference type over a lowered pointee in one storage.
    pub(in crate::lower) fn insert_reference(
        &mut self,
        kind: mir::ReferenceKind,
        access: mir::Access,
        storage: mir::Storage,
        pointee: mir::TypeId,
    ) -> mir::TypeId {
        insert_reference_type(self.builder.tree_mut(), kind, access, storage, pointee)
    }
}
