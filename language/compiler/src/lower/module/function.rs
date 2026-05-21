use std::collections::{HashMap, HashSet};
use {destack_dir as dir, destack_mir as mir};

use crate::{
    AddressTakenBindings, CompilerError, CompilerResult, FunctionLowerer, FunctionLoweringContext,
    FunctionState, LowerError, LowerResult, Terminates,
};

use crate::lower::ModuleLowerer;

/// Visitor that collects expression ids from a subtree.
#[derive(Default)]
struct ExpressionTypeCollector {
    /// dir::Expression ids encountered during traversal.
    expression_ids: Vec<dir::LocalNodeId<dir::Expression>>,
    /// dir::Expression ids used as call or constructor callees.
    callee_expression_ids: HashSet<u32>,
    /// Options for the node visitor.
    options: dir::NodeVisitorOptions,
}

impl ExpressionTypeCollector {
    /// Create a new collector.
    fn new() -> Self {
        Self {
            expression_ids: Vec::new(),
            callee_expression_ids: HashSet::new(),
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Return collected expression ids.
    fn expression_ids(&self) -> &[dir::LocalNodeId<dir::Expression>] {
        &self.expression_ids
    }

    /// Return expression ids used as callees.
    fn callee_expression_ids(&self) -> &HashSet<u32> {
        &self.callee_expression_ids
    }
}

impl dir::NodeVisitor for ExpressionTypeCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // skip lowering callee types for direct calls
        if let dir::Expression::Call { left, .. } | dir::Expression::New { left, .. } = expression {
            self.callee_expression_ids.insert(left.id);
        }
        self.expression_ids.push(id);
        destack_core::ensure_sufficient_stack(|| dir::walk_expression(self, tree, id, expression));
    }
}

/// Visitor that collects address taken bindings.
struct AddressTakenCollector<'a> {
    /// Provide access to inferred type information.
    types: &'a dir::TypeTable<'a>,
    /// Provide access to checked resolutions.
    resolutions: &'a dir::ResolutionTable<'a>,
    /// Provide access to declaration forms.
    symbols: &'a dir::BindingTable<'a>,
    /// Identify the module for expression lookups.
    module_id: destack_source::ModuleId,
    /// Symbols that require addressable locals.
    locals: HashSet<dir::GlobalSymbolId>,
    /// Whether `this` is address taken.
    takes_this: bool,
    /// Options for the node visitor.
    options: dir::NodeVisitorOptions,
}

impl<'a> AddressTakenCollector<'a> {
    /// Create a new address-taken collector.
    fn new(
        types: &'a dir::TypeTable<'a>,
        resolutions: &'a dir::ResolutionTable<'a>,
        symbols: &'a dir::BindingTable<'a>,
        module_id: destack_source::ModuleId,
    ) -> Self {
        Self {
            types,
            resolutions,
            symbols,
            module_id,
            locals: HashSet::new(),
            takes_this: false,
            options: dir::NodeVisitorOptions::default(),
        }
    }

    /// Convert the collector into address taken bindings.
    fn into_bindings(self) -> AddressTakenBindings {
        AddressTakenBindings {
            locals: self.locals,
            takes_this: self.takes_this,
        }
    }

    /// Record a reference target for address taken tracking.
    fn record_reference_target(
        &mut self,
        tree: &dir::Tree,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) {
        // unwrap reference targets that can yield addressable bases
        let expression = tree.get(expression_id);
        match expression {
            dir::Expression::Parenthesized { expression } => {
                self.record_reference_target(tree, *expression);
            }
            dir::Expression::As {
                expression: value, ..
            }
            | dir::Expression::Satisfies {
                expression: value, ..
            } => {
                self.record_reference_target(tree, *value);
            }
            dir::Expression::Member { left, .. } | dir::Expression::PrivateMember { left, .. } => {
                if !self.expression_is_reference_like(*left) {
                    self.record_reference_target(tree, *left);
                }
            }
            dir::Expression::Index { left, .. } => {
                if !self.expression_is_reference_like(*left) {
                    self.record_reference_target(tree, *left);
                }
            }
            dir::Expression::Identifier { .. } | dir::Expression::QualifiedReference { .. } => {
                let node_id = expression_id.into_global_any(self.module_id);
                if let Some(symbol) = self.resolutions.symbol_resolution(node_id) {
                    self.locals.insert(symbol);
                }
            }
            dir::Expression::This => {
                self.takes_this = true;
            }
            _ => {}
        }
    }

    /// Check whether an expression lowers to a reference-like value.
    fn expression_is_reference_like(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        let node_id = expression_id.into_global_any(self.module_id);
        let Some(type_id) = self.types.get_declared_or_inferred_type_id(node_id) else {
            return false;
        };
        let mut visited = HashSet::new();
        self.type_is_reference_like(type_id, &mut visited)
    }

    /// Check whether a type id lowers to a reference-like MIR value.
    fn type_is_reference_like(
        &self,
        type_id: dir::LocalTypeId,
        visited: &mut HashSet<dir::LocalTypeId>,
    ) -> bool {
        if !visited.insert(type_id) {
            return false;
        }

        let ty = self.types.get_type(type_id);
        match ty {
            dir::Type::Form(form) => self.type_is_reference_like(form.value, visited),
            dir::Type::Named(reference) => {
                match self.symbols.get_symbol(reference.symbol.local_id).form {
                    dir::SymbolForm::Class | dir::SymbolForm::Interface => true,
                    dir::SymbolForm::TypeAlias => self
                        .types
                        .get_alias_target_type_id(reference.symbol)
                        .is_some_and(|target| self.type_is_reference_like(target, visited)),
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

impl dir::NodeVisitor for AddressTakenCollector<'_> {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        if let dir::Expression::BorrowOf { right, .. } = expression {
            self.record_reference_target(tree, *right);
        }
        destack_core::ensure_sufficient_stack(|| dir::walk_expression(self, tree, id, expression));
    }
}

impl ModuleLowerer<'_> {
    /// Declare all function declarations in the module.
    pub(crate) fn declare_functions(&mut self) -> CompilerResult<()> {
        // scan function declarations
        for (declaration_id, declaration) in self.dir_tree.iter_nodes_of_type::<dir::Declaration>()
        {
            // skip non function declarations
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };

            // skip type-only lambda signatures
            if declaration.body.is_none() && declaration.signature.form == dir::FunctionForm::Lambda
            {
                continue;
            }

            // predeclare the function binding
            self.declare_function(declaration_id, declaration)?;

            // resolve capture layouts early for closure values
            if declaration.body.is_some() {
                let Some(symbol_id) = self.symbol_for_node(declaration_id) else {
                    continue;
                };
                self.function_environment_layout_for_symbol(symbol_id)?;
                self.enqueue_function_declaration(declaration_id);
            }
        }

        Ok(())
    }

    /// Lower any queued function declarations that do not yet have bodies.
    pub(crate) fn lower_pending_functions(&mut self) -> CompilerResult<()> {
        // drain the pending declaration queue to fixpoint
        while let Some(declaration_id) = self.pending_function_bodies.pop_front() {
            self.queued_function_bodies.remove(&declaration_id.id);

            // require a function declaration
            let declaration = self.dir_tree.get(declaration_id);
            let dir::Declaration::Function(declaration) = declaration else {
                continue;
            };

            // skip declaration-only functions
            if declaration.body.is_none() {
                continue;
            }

            // skip functions without bindings
            let Some(symbol_id) = self.symbol_for_node(declaration_id) else {
                continue;
            };
            let Some(function_id) = self.function_for_symbol(symbol_id) else {
                continue;
            };

            // skip functions already lowered
            let function = self.builder.tree().get(function_id);
            if function.entry.is_some() {
                continue;
            }

            // lower the queued body
            self.lower_function(declaration_id, declaration)?;
        }

        Ok(())
    }

    /// Predeclare a function declaration and register it for call resolution.
    pub(crate) fn declare_function(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
    ) -> CompilerResult<mir::LocalNodeId<mir::Function>> {
        // resolve function name and symbol
        let symbol_id = self.require_symbol_for_node(declaration_id)?;
        let name = self.function_name_for_declaration(symbol_id, declaration.name)?;

        // skip when the function is already registered
        if let Some(function_id) = self.function_for_symbol(symbol_id) {
            return Ok(function_id);
        }

        // resolve parameter types
        let mut parameter_types = Vec::new();
        let mut parameter_names = Vec::new();
        for parameter_id in &declaration.signature.parameters {
            let parameter_node = dir::GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty =
                self.declared_or_inferred_type_id_for_node_or_error(parameter_node)?;
            let parameter_ty = self.lower_type(
                parameter_ty,
                parameter_node.into_anchored(Some(self.profile)),
            )?;
            parameter_types.push(parameter_ty);

            // track parameter names for diagnostics
            let parameter = self.dir_tree.get(*parameter_id);
            let name = match parameter {
                dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                    Some(*name)
                }
                dir::Parameter::Pattern { .. }
                | dir::Parameter::VariadicPattern { .. }
                | dir::Parameter::Error { .. } => None,
            };
            parameter_names.push(name);
        }

        // resolve return type
        let return_type_id = self.resolve_function_return_type_id(declaration_id)?;
        let return_type = self.lower_function_return_type(declaration_id, return_type_id)?;

        // build a MIR signature type aligned with the lowered parameters
        let mir_signature = self
            .builder
            .type_function_signature(parameter_types.clone(), return_type);
        self.builder.type_function_pointer(mir_signature);

        // declare the function and register bindings
        let allocation_mode = self.allocation_mode_for_symbol(symbol_id);
        let function_id = self
            .builder
            .declare_function(&name, &parameter_types, return_type);
        {
            let function = self.builder.tree_mut().get_mut(function_id);
            function.parameter_names = parameter_names.clone();
            function.allocation = allocation_mode;
        }

        self.register_function_binding_for_symbol(symbol_id, function_id, mir_signature)?;

        Ok(function_id)
    }

    /// Queue a function declaration for later body lowering.
    fn enqueue_function_declaration(&mut self, declaration_id: dir::LocalNodeId<dir::Declaration>) {
        // deduplicate queued declarations
        if !self.queued_function_bodies.insert(declaration_id.id) {
            return;
        }

        // enqueue the declaration once
        self.pending_function_bodies.push_back(declaration_id);
    }

    /// Prelower types required by a function body expression.
    fn prelower_expression_types(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // collect expression ids for this subtree
        let mut collector = ExpressionTypeCollector::new();
        let expression = self.dir_tree.get(expression_id);
        dir::NodeVisitor::visit_expression(
            &mut collector,
            self.dir_tree,
            expression_id,
            expression,
        );

        // collect type ids referenced by expressions
        let mut type_sources: HashMap<dir::LocalTypeId, dir::GlobalNodeIdAny> = HashMap::new();
        for expression_id in collector.expression_ids() {
            if collector
                .callee_expression_ids()
                .contains(&expression_id.id)
            {
                continue;
            }
            let node_id = expression_id.into_global_any(self.module_id);
            if let Some(type_id) = self.types.get_declared_or_inferred_type_id(node_id) {
                let dir_type = self.types.get_type(type_id);
                if matches!(dir_type, dir::Type::Never | dir::Type::Form(_)) {
                    continue;
                }
                type_sources.entry(type_id).or_insert(node_id);
            }

            if let dir::Expression::Type { value } = self.dir_tree.get(*expression_id) {
                let Some(resolved_type) = self
                    .types
                    .get_declared_or_inferred_type_id(value.into_global_any(self.module_id))
                else {
                    continue;
                };
                let dir_type = self.types.get_type(resolved_type);
                if matches!(dir_type, dir::Type::Never | dir::Type::Form(_)) {
                    continue;
                }
                type_sources.entry(resolved_type).or_insert(node_id);
            }

            // include local binding symbol types for uninitialized lets
            if let dir::Expression::Let { declarators, .. }
            | dir::Expression::Using { declarators, .. } = self.dir_tree.get(*expression_id)
            {
                for declarator_id in declarators {
                    let declarator = self.dir_tree.get(*declarator_id);
                    let Some(symbol) = self.symbol_for_node(declarator.pattern) else {
                        continue;
                    };

                    let Some(type_id) = self.types.get_value_type_id(symbol) else {
                        continue;
                    };
                    let type_id = self.types.unwrap_form_payload_type_id(type_id);
                    let dir_type = self.types.get_type(type_id);
                    if matches!(dir_type, dir::Type::Never | dir::Type::Form(_)) {
                        continue;
                    }

                    let type_node = declarator.pattern.into_global_any(self.module_id);
                    type_sources.entry(type_id).or_insert(type_node);
                }
            }
        }

        // lower each type in deterministic order
        let mut type_entries: Vec<_> = type_sources.into_iter().collect();
        type_entries.sort_by_key(|(type_id, _)| type_id.0);
        for (type_id, node_id) in type_entries {
            let anchor = node_id.into_anchored(Some(self.profile));
            self.lower_type(type_id, anchor)?;
        }

        Ok(())
    }

    /// Collect address taken bindings within a function body.
    fn collect_address_taken_bindings(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> AddressTakenBindings {
        // walk the function body to find reference targets
        let mut collector =
            AddressTakenCollector::new(self.types, self.resolutions, self.symbols, self.module_id);
        let expression = self.dir_tree.get(expression_id);
        dir::NodeVisitor::visit_expression(
            &mut collector,
            self.dir_tree,
            expression_id,
            expression,
        );
        collector.into_bindings()
    }

    /// Resolve the signature type id for a declaration or member node.
    pub(crate) fn signature_type_id_for_node(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> LowerResult<dir::LocalTypeId> {
        // resolve the signature type id
        self.types
            .signature_type_id(node_id)
            .ok_or_else(|| self.missing_type_error(node_id))
    }

    /// Lower a function declaration to a MIR function.
    pub(crate) fn lower_function(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::FunctionDeclaration,
    ) -> CompilerResult<mir::LocalNodeId<mir::Function>> {
        // resolve function name and symbol
        let symbol_id = self.require_symbol_for_node(declaration_id)?;
        let name = self.function_name_for_declaration(symbol_id, declaration.name)?;

        // resolve capture layout
        let capture_layout = self.function_environment_layout_for_symbol(symbol_id)?;

        // resolve parameter types
        let mut parameter_types = Vec::new();
        let mut parameter_names = Vec::new();
        for parameter_id in &declaration.signature.parameters {
            let parameter_node = dir::GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty =
                self.declared_or_inferred_type_id_for_node_or_error(parameter_node)?;
            let parameter_ty = self.lower_type(
                parameter_ty,
                parameter_node.into_anchored(Some(self.profile)),
            )?;
            parameter_types.push(parameter_ty);

            // track parameter names for diagnostics
            let parameter = self.dir_tree.get(*parameter_id);
            let name = match parameter {
                dir::Parameter::Named { name, .. } | dir::Parameter::VariadicNamed { name, .. } => {
                    Some(*name)
                }
                dir::Parameter::Pattern { .. }
                | dir::Parameter::VariadicPattern { .. }
                | dir::Parameter::Error { .. } => None,
            };
            parameter_names.push(name);
        }

        // resolve return type
        let return_type_id = self.resolve_function_return_type_id(declaration_id)?;
        let return_type = self.lower_function_return_type(declaration_id, return_type_id)?;

        // build a MIR signature type aligned with the lowered parameters
        let mir_signature = self
            .builder
            .type_function_signature(parameter_types.clone(), return_type);
        self.builder.type_function_pointer(mir_signature);

        // prelower body expression types
        if let Some(body_id) = declaration.body {
            self.declare_call_targets_for_expression(body_id)?;
            self.prelower_expression_types(body_id)?;
        }

        // collect address taken locals
        let address_taken = declaration
            .body
            .map(|body_id| self.collect_address_taken_bindings(body_id))
            .unwrap_or_else(AddressTakenBindings::empty);

        // resolve the implicit this symbol
        let this_symbol = self.resolve_this_symbol_for_function(symbol_id, &declaration.signature);

        // resolve allocation mode
        let allocation_mode = self.allocation_mode_for_symbol(symbol_id);

        // declare or reuse the function id
        let function_id = if let Some(function_id) = self.function_for_symbol(symbol_id) {
            function_id
        } else {
            let function_id = self
                .builder
                .declare_function(&name, &parameter_types, return_type);
            {
                let function = self.builder.tree_mut().get_mut(function_id);
                function.parameter_names = parameter_names.clone();
                function.allocation = allocation_mode;
            }

            self.register_function_binding_for_symbol(symbol_id, function_id, mir_signature)?;

            function_id
        };

        // skip declared functions without bodies
        if declaration.body.is_none() {
            return Ok(function_id);
        }

        // resolve shared function environment metadata
        let empty_function_environment_pointer_type =
            self.empty_function_environment_pointer_type();

        // build the function body
        let mut builder = self.builder.function_body(function_id);
        for (index, name) in parameter_names.iter().enumerate() {
            if let Some(name_id) = name {
                builder.set_parameter_name(index, *name_id);
            }
        }
        builder.set_allocation_mode(allocation_mode);

        // build lowering context
        let context = FunctionLoweringContext {
            module_id: self.module_id,
            profile: self.profile,
            compiler: self.compiler,
            provider: self.context,
            dir_tree: self.dir_tree,
            symbols: self.symbols,
            types: self.types,
            resolutions: self.resolutions,
            guards: self.guards,
            captures: self.captures,
            strings: &self.strings,
            language_intrinsics: self.language_intrinsics.as_ref(),
            functions_by_instance: &self.functions_by_instance,
            function_signature_types: &self.function_signature_types,
            binding_symbols: &self.binding_symbols,
            binding_abi_lowering: self.binding_abi_lowering,
            runtime_status_layout: self.runtime_status_layout,
            take_platform_error_function: self.take_platform_error_function,
            globals_by_symbol: &self.globals_by_symbol,
            string_literal_globals: &self.string_literal_globals,
            interface_slots_by_symbol: &self.interface_slots_by_symbol,
            interface_table_globals_by_pair: &self.interface_table_globals_by_pair,
            virtual_method_slots_by_key: &self.virtual_method_slots_by_key,
            vtable_globals_by_symbol: &self.vtable_globals_by_symbol,
            dispatch_call_name: self.dispatch_call_name,
            dispatch_construct_name: self.dispatch_construct_name,
            checks: self.runtime_checks,
            type_lowerer: &self.type_lowerer,
            return_type,
            return_type_id: Some(return_type_id),
            symbol: symbol_id,
            function_environment_layouts: &self.function_environment_layouts,
            empty_function_environment_pointer_type,
        };
        let state = FunctionState::new(builder, address_taken);
        let mut function_lowerer = FunctionLowerer::new(context, state);

        // capture explicit or captured this symbols when present
        function_lowerer.state.bindings.this_symbol = this_symbol;

        // track bindings captured by borrow
        let capture = self.captures.capture(symbol_id);
        let mut reference_bindings = capture
            .into_iter()
            .flat_map(|capture| capture.captures.iter())
            .filter_map(|capture| {
                (capture.mode() == dir::CaptureMode::Borrow).then_some(capture.symbol())
            })
            .collect::<HashSet<_>>();

        // include borrowed lexical this
        if let Some(this) = capture.and_then(|capture| capture.this)
            && this.mode == dir::CaptureMode::Borrow
        {
            reference_bindings.insert(this.symbol);
        }
        function_lowerer.state.bindings.reference_bindings = reference_bindings;

        // create entry block
        let entry_block = function_lowerer.state.builder.block();
        function_lowerer.state.builder.switch_to_block(entry_block);

        // seed function environment when captured
        if let Some(layout) = capture_layout {
            let env_ref_type = layout.env_pointer_type;
            let env_value = function_lowerer
                .state
                .builder
                .callable_environment(env_ref_type);
            function_lowerer.state.bindings.environment = Some(env_value);
        }

        // add parameter locals
        for (index, parameter_id) in declaration.signature.parameters.iter().enumerate() {
            let ty = parameter_types[index];
            let value = function_lowerer.state.builder.function_parameter(index);
            let parameter_symbol = function_lowerer
                .context
                .require_symbol_for_node(*parameter_id)?
                .local_id;
            function_lowerer.define_local_binding(
                parameter_id.into_any(),
                parameter_symbol,
                None,
                value,
                ty,
            )?;
        }

        // lower body
        if let Some(body_id) = declaration.body {
            let terminated = function_lowerer.lower_body(body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_lowerer.state.builder.return_(None);
                } else {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            declaration_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                        ),
                        message: "missing terminator".to_string(),
                    }
                    .into());
                }
            }
        } else {
            function_lowerer.state.builder.return_(None);
        }

        // finish the function builder
        function_lowerer.state.builder.finish();
        Ok(function_id)
    }

    /// Resolve the name used for MIR functions, including anonymous lambdas.
    fn function_name_for_declaration(
        &self,
        symbol_id: dir::GlobalSymbolId,
        name: Option<dir::Name>,
    ) -> CompilerResult<String> {
        // prefer explicit declaration names
        let symbol_data = self.symbols.get_symbol(symbol_id.local_id);
        let name_id = name
            .map(|name| name.string())
            .or_else(|| symbol_data.name());
        if let Some(name_id) = name_id {
            let name = self.strings.get(name_id).to_string();
            return Ok(name);
        }

        // synthesize a deterministic name for anonymous lambdas
        let owner_name = self.lambda_owner_name(symbol_id);
        let suffix = symbol_id.local_id.id;
        let name = if let Some(owner_name) = owner_name {
            format!("{owner_name}.lambda#{suffix}")
        } else {
            format!("lambda#{suffix}")
        };

        Ok(name)
    }

    /// Resolve a `this` symbol for explicit parameters or captured bindings.
    fn resolve_this_symbol_for_function(
        &self,
        symbol_id: dir::GlobalSymbolId,
        signature: &dir::FunctionSignature,
    ) -> Option<dir::GlobalSymbolId> {
        // prefer explicit this parameters
        if let Some(parameter_id) = signature.this_parameter {
            return self.symbol_for_node(parameter_id);
        }

        // resolve implicit this for member methods
        let symbol_data = self.symbols.get_symbol(symbol_id.local_id);
        let is_member = symbol_data
            .declaration
            .is_some_and(|primary| primary.local_id.ty == dir::NodeType::Member);
        if is_member {
            let scope = self.symbols.get_scope_by_id(symbol_data.scope.id);
            let this_name = self.strings.intern("this");
            if let Some(symbol) = scope.find_symbol(dir::StaticKey::Name(this_name)) {
                return Some(symbol.into_global(self.module_id));
            }
        }

        // fall back to captured this bindings
        self.captures
            .capture(symbol_id)
            .and_then(|capture| capture.this.map(|this| this.symbol))
    }

    /// Resolve the module-local owner path for an anonymous lambda.
    fn lambda_owner_name(&self, symbol_id: dir::GlobalSymbolId) -> Option<String> {
        let symbol_data = self.symbols.get_symbol(symbol_id.local_id);
        let mut scope_id = symbol_data.scope.id;
        let mut seen_scopes = HashSet::new();
        loop {
            if !seen_scopes.insert(scope_id) {
                return None;
            }

            let scope = self.symbols.get_scope_by_id(scope_id);
            if let Some(owner_id) = scope.owner {
                let owner_symbol = owner_id.into_global(self.module_id);
                if let Some(owner_path) = self.symbol_path_name(owner_symbol) {
                    return Some(owner_path);
                }
            }

            scope_id = scope.parent?.id;
        }
    }

    /// Resolve a function return type id for lowering.
    fn resolve_function_return_type_id(
        &self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> LowerResult<dir::LocalTypeId> {
        // resolve the declaration node id
        let node_id = declaration_id.into_global_any(self.module_id);

        // resolve the function signature type
        let signature_type_id = self.signature_type_id_for_node(node_id)?;

        // extract the return type id from the signature
        let return_type_id = match self.types.get_type(signature_type_id) {
            dir::Type::Function(function) => function
                .return_type
                .ok_or_else(|| self.missing_type_error(node_id))?,
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(node_id.into_anchored(Some(self.profile))),
                    message: "missing function signature type".to_string(),
                });
            }
        };

        Ok(return_type_id)
    }

    /// Lower a resolved function return type.
    fn lower_function_return_type(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        return_type_id: dir::LocalTypeId,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        let node_id = declaration_id.into_global_any(self.module_id);

        Ok(self.lower_type(return_type_id, node_id.into_anchored(Some(self.profile)))?)
    }

    /// Lower a method member to a MIR function.
    pub(crate) fn lower_method(
        &mut self,
        member_id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
        this_type: Option<mir::LocalNodeId<mir::Type>>,
        owner_symbol: dir::GlobalSymbolId,
        parent_declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> CompilerResult<()> {
        // require a method member
        let dir::Member::Method {
            key,
            signature,
            body,
            is_static,
            ..
        } = member
        else {
            return Ok(());
        };
        let is_constructor = matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor) | Some(dir::FunctionRole::New)
        );
        let is_static = *is_static;

        // track constructor declaration symbol when needed
        let mut constructor_symbol = None;

        // resolve the method symbol
        let method_symbol = self.require_symbol_for_node(member_id)?;

        // resolve the method name
        let name_str = if is_constructor {
            // reject constructor keys
            if key.is_some() {
                return Err(LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "constructor cannot have a name".to_string(),
                }
                .into());
            }

            // resolve the nominal declaration descriptor
            let declaration = self.dir_tree.get(parent_declaration_id);
            let symbol =
                self.nominal_symbol_for_declaration_or_error(parent_declaration_id, declaration)?;
            constructor_symbol = Some(symbol);

            // require a declaration name for constructor
            let name = declaration
                .name()
                .ok_or_else(|| LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        parent_declaration_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "constructor must have a declaration name".to_string(),
                })
                .map_err(CompilerError::from)?;

            // format the constructor name
            let type_name = self.strings.get(name.string()).to_string();
            format!("{type_name}.constructor")
        } else if is_static {
            self.static_member_name(owner_symbol, *key).ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "static method requires a static member name".to_string(),
                }
            })?
        } else {
            // resolve the static method key or dispatch name
            let method_name = self.member_dispatch_name_or_error(
                key.as_ref(),
                signature.role,
                member_id.into_any(),
            )?;
            let method_name = self.strings.get(method_name).to_string();

            // prefix instance methods with the owner type name when available
            if let Some(owner_name) = self.symbol_path_name(owner_symbol) {
                format!("{owner_name}.{method_name}")
            } else {
                method_name
            }
        };

        // capture 'this' type for constructor initialization
        let constructor_this_type = if is_constructor {
            // require an instance type for constructors
            Some(this_type.ok_or_else(|| {
                LowerError::UnsupportedConstruct {
                    anchor: self.diagnostic_anchor(
                        member_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                    ),
                    message: "constructor missing instance type".to_string(),
                }
            })?)
        }
        // skip constructor state for non constructors
        else {
            None
        };

        // drop 'this' type for static methods
        let method_this_type = if is_constructor {
            this_type
        } else if is_static {
            None
        } else {
            this_type
        };

        // resolve parameter types
        let parameter_types = self.method_parameter_types(signature, method_this_type)?;

        // resolve return type from the method's inferred signature
        let member_node = member_id.into_global_any(self.module_id);
        let (return_type, return_type_id) = if let Some(constructor_type) = constructor_this_type {
            (constructor_type, None)
        } else {
            let return_type_id = self.resolve_method_return_type_id(member_id, member_node)?;
            let return_type = self.lower_method_return_type(member_node, return_type_id)?;

            (return_type, return_type_id)
        };

        // build a MIR signature type aligned with the lowered parameters
        let mir_signature = self
            .builder
            .type_function_signature(parameter_types.clone(), return_type);
        self.builder.type_function_pointer(mir_signature);

        // prelower body expression types
        if let Some(body_id) = body {
            self.prelower_expression_types(*body_id)?;
        }

        // collect address taken locals
        let address_taken = body
            .map(|body_id| self.collect_address_taken_bindings(body_id))
            .unwrap_or_else(AddressTakenBindings::empty);

        // resolve the implicit this symbol
        let this_symbol = self.resolve_this_symbol_for_function(method_symbol, signature);

        // resolve shared function environment metadata
        let empty_function_environment_pointer_type =
            self.empty_function_environment_pointer_type();
        let instance = self.symbol_instance_key(method_symbol);

        // prepare constructor diagnostics before borrowing the MIR builder
        let constructor_this = constructor_this_type.map(|this_ty| {
            let node = member_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));
            let anchor = self.diagnostic_anchor(node);

            (this_ty, node, anchor)
        });

        // build the function and register bindings
        let builder = {
            let functions_by_instance = &mut self.functions_by_instance;
            let function_signature_types = &mut self.function_signature_types;
            let mut builder = self
                .builder
                .function(&name_str, &parameter_types, return_type);

            // track parameter names for diagnostics
            let mut parameter_names = Vec::new();
            if !is_constructor && method_this_type.is_some() {
                let name_id = self.strings.intern("this");
                parameter_names.push(Some(name_id));
            }
            for parameter_id in &signature.parameters {
                let parameter = self.dir_tree.get(*parameter_id);
                let name = match parameter {
                    dir::Parameter::Named { name, .. }
                    | dir::Parameter::VariadicNamed { name, .. } => Some(*name),
                    dir::Parameter::Pattern { .. }
                    | dir::Parameter::VariadicPattern { .. }
                    | dir::Parameter::Error { .. } => None,
                };
                parameter_names.push(name);
            }

            // apply parameter names to the MIR function
            for (index, name) in parameter_names.iter().enumerate() {
                if let Some(name_id) = name {
                    builder.set_parameter_name(index, *name_id);
                }
            }
            let function_id = builder.function_id();

            // register function bindings
            Self::register_function_binding_in_maps(
                self.module_id,
                functions_by_instance,
                function_signature_types,
                instance,
                function_id,
                mir_signature,
            )?;

            builder
        };

        // create function lowerer
        let context = FunctionLoweringContext {
            module_id: self.module_id,
            profile: self.profile,
            compiler: self.compiler,
            provider: self.context,
            dir_tree: self.dir_tree,
            symbols: self.symbols,
            types: self.types,
            resolutions: self.resolutions,
            guards: self.guards,
            captures: self.captures,
            strings: &self.strings,
            language_intrinsics: self.language_intrinsics.as_ref(),
            functions_by_instance: &self.functions_by_instance,
            function_signature_types: &self.function_signature_types,
            binding_symbols: &self.binding_symbols,
            binding_abi_lowering: self.binding_abi_lowering,
            runtime_status_layout: self.runtime_status_layout,
            take_platform_error_function: self.take_platform_error_function,
            globals_by_symbol: &self.globals_by_symbol,
            string_literal_globals: &self.string_literal_globals,
            interface_slots_by_symbol: &self.interface_slots_by_symbol,
            interface_table_globals_by_pair: &self.interface_table_globals_by_pair,
            virtual_method_slots_by_key: &self.virtual_method_slots_by_key,
            vtable_globals_by_symbol: &self.vtable_globals_by_symbol,
            dispatch_call_name: self.dispatch_call_name,
            dispatch_construct_name: self.dispatch_construct_name,
            checks: self.runtime_checks,
            type_lowerer: &self.type_lowerer,
            return_type,
            return_type_id,
            symbol: method_symbol,
            function_environment_layouts: &self.function_environment_layouts,
            empty_function_environment_pointer_type,
        };
        let state = FunctionState::new(builder, address_taken);
        let mut function_lowerer = FunctionLowerer::new(context, state);

        // capture explicit or captured this symbols when present
        function_lowerer.state.bindings.this_symbol = this_symbol;

        // track bindings captured by borrow
        let capture = self.captures.capture(method_symbol);
        let mut reference_bindings = capture
            .into_iter()
            .flat_map(|capture| capture.captures.iter())
            .filter_map(|capture| {
                (capture.mode() == dir::CaptureMode::Borrow).then_some(capture.symbol())
            })
            .collect::<HashSet<_>>();

        // include borrowed lexical this
        if let Some(this) = capture.and_then(|capture| capture.this)
            && this.mode == dir::CaptureMode::Borrow
        {
            reference_bindings.insert(this.symbol);
        }
        function_lowerer.state.bindings.reference_bindings = reference_bindings;

        // create entry block
        let entry_block = function_lowerer.state.builder.block();
        function_lowerer.state.builder.switch_to_block(entry_block);

        // initialize constructor state before parameter locals
        if let Some((this_ty, node, anchor)) = constructor_this {
            // select the layout type
            let layout_type = match function_lowerer.state.builder.tree().get(this_ty) {
                mir::Type::Reference { pointee, .. } => pointee
                    .ty()
                    .ok_or_else(|| LowerError::UnsupportedConstruct {
                        anchor,
                        message: "constructor receiver pointee type is not concrete".to_string(),
                    })
                    .map_err(CompilerError::from)?,
                _ => this_ty,
            };

            // initialize constructor state
            let layout = self
                .type_lowerer
                .layout_for_type_or_error(layout_type, node)?;
            let class_symbol = constructor_symbol.filter(|symbol| {
                function_lowerer
                    .context
                    .symbol_is(*symbol, dir::SymbolForm::Class)
            });
            function_lowerer.initialize_constructor(this_ty, layout.clone(), node, class_symbol)?;
        }

        // track parameter index for locals
        let mut param_index = 0;

        // add 'this' parameter as first local for instance methods
        if let Some(this_ty) = method_this_type
            && !is_constructor
        {
            function_lowerer.bind_this_parameter(member_id.into_any(), this_symbol, this_ty)?;
            param_index += 1;
        }

        // add declared parameter locals
        for parameter_id in &signature.parameters {
            let parameter_symbol = function_lowerer
                .context
                .require_symbol_for_node(*parameter_id)?
                .local_id;

            // bind the parameter local
            let ty = parameter_types[param_index];
            let value = function_lowerer
                .state
                .builder
                .function_parameter(param_index);
            function_lowerer.define_local_binding(
                parameter_id.into_any(),
                parameter_symbol,
                None,
                value,
                ty,
            )?;

            // advance the parameter index
            param_index += 1;
        }

        // lower the body when present
        if let Some(body_id) = body {
            // lower the body and handle fallthrough
            let terminated = function_lowerer.lower_body(*body_id)?;
            if terminated == Terminates::No {
                // return constructed value when constructor falls through
                if is_constructor {
                    let node = member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile));
                    function_lowerer.return_constructor_value(node)?;
                }
                // return void when allowed
                else if return_type == self.type_lowerer.ty_void {
                    function_lowerer.state.builder.return_(None);
                }
                // error on missing terminator
                else {
                    return Err(LowerError::UnsupportedConstruct {
                        anchor: self.diagnostic_anchor(
                            parent_declaration_id
                                .into_global_any(self.module_id)
                                .into_anchored(Some(self.profile)),
                        ),
                        message: "method body missing terminator".to_string(),
                    }
                    .into());
                }
            }
        }
        // synthesize constructor return when body is missing
        else if is_constructor {
            let node = member_id
                .into_global_any(self.module_id)
                .into_anchored(Some(self.profile));
            function_lowerer.return_constructor_value(node)?;
        }
        // synthesize void return when body is missing
        else {
            function_lowerer.state.builder.return_(None);
        }

        // finish the function builder
        function_lowerer.state.builder.finish();

        Ok(())
    }

    /// Resolve a method return type id for lowering.
    pub(crate) fn resolve_method_return_type_id(
        &self,
        member_id: dir::LocalNodeId<dir::Member>,
        member_node: dir::GlobalNodeIdAny,
    ) -> LowerResult<Option<dir::LocalTypeId>> {
        // get signature type from analyzed metadata
        let signature_type_id = self.signature_type_id_for_node(member_node)?;

        // extract return type from function signature
        let dir::Type::Function(function) = self.types.get_type(signature_type_id) else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    member_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "method signature is not a function type".to_string(),
            }
            .into());
        };

        // lower return type or default to void
        let Some(return_type_id) = function.return_type else {
            return Ok(None);
        };

        Ok(Some(return_type_id))
    }

    /// Lower a resolved method return type.
    pub(crate) fn lower_method_return_type(
        &mut self,
        member_node: dir::GlobalNodeIdAny,
        return_type_id: Option<dir::LocalTypeId>,
    ) -> CompilerResult<mir::LocalNodeId<mir::Type>> {
        // lower return type or default to void
        let Some(return_type_id) = return_type_id else {
            return Ok(self.type_lowerer.ty_void);
        };

        Ok(self.lower_type(
            return_type_id,
            member_node.into_anchored(Some(self.profile)),
        )?)
    }

    /// Resolve parameter types for a method signature.
    pub(crate) fn method_parameter_types(
        &mut self,
        signature: &dir::FunctionSignature,
        this_type: Option<mir::LocalNodeId<mir::Type>>,
    ) -> LowerResult<Vec<mir::LocalNodeId<mir::Type>>> {
        // decide whether this method is a constructor
        let is_constructor = matches!(
            signature.role,
            Some(dir::FunctionRole::Constructor) | Some(dir::FunctionRole::New)
        );

        // initialize parameter types
        let mut parameter_types = Vec::new();

        // add this parameter when lowering an instance method
        if !is_constructor && let Some(this_ty) = this_type {
            parameter_types.push(this_ty);
        }

        // lower declared parameter types
        for parameter_id in &signature.parameters {
            // resolve the parameter type id
            let parameter_node = dir::GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty_id =
                self.declared_or_inferred_type_id_for_node_or_error(parameter_node)?;

            // lower the parameter type
            let parameter_ty = self.lower_type(
                parameter_ty_id,
                parameter_node.into_anchored(Some(self.profile)),
            )?;
            parameter_types.push(parameter_ty);
        }

        Ok(parameter_types)
    }
}
