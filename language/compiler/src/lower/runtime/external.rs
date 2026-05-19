use std::sync::Arc;

use destack_artifact::{DirBound, DirChecked, DirParsed};
use destack_core::{StringId, StringPool};
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::{CompilerResult, LowerError};

use crate::lower::ModuleLowerer;

/// Visitor that collects call expressions from a subtree.
struct ExternalCallCollector {
    /// Call expressions found in the subtree.
    calls: Vec<dir::LocalNodeId<dir::Expression>>,
    /// Options for the node visitor.
    options: dir::NodeVisitorOptions,
}

impl ExternalCallCollector {
    /// Create a new collector.
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            options: dir::NodeVisitorOptions::default(),
        }
    }
}

impl dir::NodeVisitor for ExternalCallCollector {
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        // record direct call expressions
        if matches!(expression, dir::Expression::Call { .. }) {
            self.calls.push(id);
        }

        // continue walking the subtree
        destack_core::ensure_sufficient_stack(|| dir::walk_expression(self, tree, id, expression));
    }
}

pub(crate) struct BindingResolution {
    pub(crate) name: String,
    pub(crate) is_binding: bool,
}

impl ModuleLowerer<'_> {
    /// Declare call targets referenced by an expression subtree.
    pub(crate) fn declare_call_targets_for_expression(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // collect call expressions from the subtree
        let mut collector = ExternalCallCollector::new();
        let expression = self.dir_tree.get(expression_id);
        dir::NodeVisitor::visit_expression(
            &mut collector,
            self.dir_tree,
            expression_id,
            expression,
        );

        // declare each required call target lazily for this subtree
        for call_id in collector.calls {
            self.declare_call_target(call_id)?;
        }

        Ok(())
    }

    /// Declare the function referenced by a call, when needed.
    fn declare_call_target(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<()> {
        // resolve the static call candidate
        let node_id = expression_id.into_global_any(self.module_id);
        let Some(resolution) = self.resolutions.call_resolution(node_id) else {
            return Ok(());
        };
        let dir::CallTarget::Direct(candidate) = &resolution.target else {
            return Ok(());
        };

        // skip local targets and already declared symbols
        let target_symbol = candidate.symbol;
        if target_symbol.module_id == self.module_id {
            return Ok(());
        }
        if self.function_for_symbol(target_symbol).is_some() {
            return Ok(());
        }

        // skip intrinsic bindings, they are lowered directly
        if self
            .resolve_intrinsic_binding_name_id(target_symbol)?
            .is_some()
        {
            return Ok(());
        }

        // require a resolved signature
        // declare the call target on demand
        self.declare_external_function(expression_id, target_symbol, resolution)?;

        Ok(())
    }

    /// Declare the MIR function for an external call target.
    fn declare_external_function(
        &mut self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        target_symbol: dir::GlobalSymbolId,
        signature: &dir::CallResolution,
    ) -> CompilerResult<mir::LocalNodeId<mir::Function>> {
        // return the existing declaration when present
        if let Some(function_id) = self.function_for_symbol(target_symbol) {
            return Ok(function_id);
        }

        // resolve the extern/binding symbol name
        let binding = self
            .binding_name_for_symbol(expression_id, target_symbol)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "missing @binding or @extern decorator for call target".to_string(),
            })?;
        let extern_name = binding.name;

        // anchor diagnostic spans for type lowering
        let anchor = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        // lower parameter types
        let mut parameter_types = Vec::with_capacity(signature.parameters.len());
        for type_id in &signature.parameters {
            // lower the parameter type
            let parameter_type = self.lower_type(*type_id, anchor)?;
            parameter_types.push(parameter_type);
        }

        // lower the return type
        let return_type = match signature.return_type {
            Some(return_type) => self.lower_type(return_type, anchor)?,
            None => self.type_lowerer.ty_void,
        };

        if binding.is_binding && self.binding_abi_lowering {
            let binding_info = self.binding_result_info(signature, expression_id, target_symbol)?;
            let abi_info = self.runtime_status_layout(expression_id)?;
            let mut abi_parameters = Vec::with_capacity(parameter_types.len() + 1);

            self.declare_take_platform_error_function(expression_id, binding_info.err_value_type)?;

            if !binding_info.ok_is_void {
                let out_pointer = self.builder.type_reference(
                    mir::ReferenceKind::Raw,
                    binding_info.ok_mir_type,
                    mir::Access::Mutable,
                    mir::Space::Frame,
                    mir::Nullability::None,
                );
                abi_parameters.push(out_pointer);
            }
            abi_parameters.extend(parameter_types);

            let signature = self
                .builder
                .type_function_signature(abi_parameters.clone(), abi_info.ty);
            let signature_type = self.builder.type_function_pointer(signature);
            self.assign_signature_metadata_name(signature_type, target_symbol, anchor)?;

            let function_id =
                self.builder
                    .external_function(&extern_name, &abi_parameters, abi_info.ty);
            self.register_function_binding_for_symbol(target_symbol, function_id, signature)?;
            self.binding_symbols.insert(target_symbol);
            if extern_name == "destack.error.takePlatformError" {
                self.take_platform_error_function = Some(function_id);
            }

            return Ok(function_id);
        }

        // create a signature type for direct callsites
        let signature = self
            .builder
            .type_function_signature(parameter_types.clone(), return_type);
        let signature_type = self.builder.type_function_pointer(signature);
        // attach a metadata name for the signature type
        self.assign_signature_metadata_name(signature_type, target_symbol, anchor)?;

        // declare the external function
        let function_id =
            self.builder
                .external_function(&extern_name, &parameter_types, return_type);
        // register function binding
        self.register_function_binding_for_symbol(target_symbol, function_id, signature)?;
        if binding.is_binding {
            self.binding_symbols.insert(target_symbol);
        }

        // return after declaration
        Ok(function_id)
    }

    /// Resolve the extern or binding name for a symbol, if any.
    pub(crate) fn binding_name_for_symbol(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<BindingResolution>> {
        // require checked symbols for the referenced module
        self.require_checked_module(symbol.module_id)?;

        // read the symbol entry and binding metadata
        let dir =
            self.dir_bound_if_present(symbol.module_id)
                .ok_or_else(|| LowerError::Internal {
                    anchor: (self.module_id).into(),
                    module: self.module_id,
                    message: format!("missing bound DIR artifact for {:?}", symbol.module_id),
                })?;
        let parsed = self
            .dir_parsed_if_present(symbol.module_id)
            .ok_or_else(|| LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: format!("missing parsed DIR artifact for {:?}", symbol.module_id),
            })?;
        let checked = self
            .dir_checked_if_present(symbol.module_id)
            .ok_or_else(|| LowerError::Internal {
                anchor: (self.module_id).into(),
                module: self.module_id,
                message: format!("missing checked DIR artifact for {:?}", symbol.module_id),
            })?;
        let resolutions = checked.resolution_table();
        let bindings = dir.binding_table();
        let symbol_entry = bindings.get_symbol(symbol.local_id);

        if let Some(name) = self.host_decorator_name(
            expression_id,
            parsed.as_ref(),
            dir.as_ref(),
            &resolutions,
            symbol_entry,
            dir::LanguageItem::Binding,
        )? {
            return Ok(Some(BindingResolution {
                name,
                is_binding: true,
            }));
        }

        if let Some(name) = self.host_decorator_name(
            expression_id,
            parsed.as_ref(),
            dir.as_ref(),
            &resolutions,
            symbol_entry,
            dir::LanguageItem::Extern,
        )? {
            return Ok(Some(BindingResolution {
                name,
                is_binding: false,
            }));
        }

        Ok(None)
    }

    /// Resolve the host name carried by a host binding decorator.
    fn host_decorator_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        parsed: &DirParsed,
        bound: &DirBound,
        resolutions: &dir::ResolutionTable<'_>,
        symbol: &dir::Symbol,
        language_item: dir::LanguageItem,
    ) -> CompilerResult<Option<String>> {
        let Some(declaration) = symbol.declaration else {
            return Ok(None);
        };
        let Some(decorator_symbol) = self.language_item(language_item)? else {
            return Ok(None);
        };

        // inspect attached decorator expressions
        for decorator_id in parsed.tree.get_decorators(declaration.local_id.id) {
            let decorator = parsed.tree.get(decorator_id);
            let Some(name) = self.host_decorator_expression_name(
                expression_id,
                parsed,
                bound,
                resolutions,
                symbol,
                decorator.expression,
                language_item,
                decorator_symbol,
            )?
            else {
                continue;
            };

            return Ok(Some(name));
        }

        Ok(None)
    }

    /// Resolve one host decorator expression to its emitted name.
    fn host_decorator_expression_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        parsed: &DirParsed,
        bound: &DirBound,
        resolutions: &dir::ResolutionTable<'_>,
        symbol: &dir::Symbol,
        decorator_expression: dir::LocalNodeId<dir::Expression>,
        language_item: dir::LanguageItem,
        decorator_symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<String>> {
        let decorator_expression =
            unwrap_parenthesized_expression(&parsed.tree, decorator_expression);
        let expression = parsed.tree.get(decorator_expression);
        let (callee, arguments) = match expression {
            dir::Expression::Call {
                left, arguments, ..
            } => (
                unwrap_parenthesized_expression(&parsed.tree, *left),
                Some(arguments.as_slice()),
            ),
            _ => (decorator_expression, None),
        };
        if !host_decorator_matches(
            &parsed.tree,
            bound,
            resolutions,
            decorator_expression,
            callee,
            language_item,
            decorator_symbol,
        ) {
            return Ok(None);
        }

        let Some(arguments) = arguments else {
            return self
                .default_host_name(expression_id, self.strings, symbol)
                .map(Some);
        };

        self.explicit_host_name(expression_id, &parsed.tree, self.strings, arguments)
            .map(Some)
    }

    /// Resolve the default emitted name for a host binding symbol.
    fn default_host_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        strings: &StringPool,
        symbol: &dir::Symbol,
    ) -> CompilerResult<String> {
        symbol
            .name()
            .map(|name| strings.get(name).to_string())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "extern symbol is missing a name".to_string(),
            })
            .map_err(Into::into)
    }

    /// Resolve the explicit emitted name from host decorator arguments.
    fn explicit_host_name(
        &self,
        expression_id: dir::LocalNodeId<dir::Expression>,
        tree: &dir::Tree,
        strings: &StringPool,
        arguments: &[dir::LocalNodeId<dir::Argument>],
    ) -> CompilerResult<String> {
        let [argument_id] = arguments else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "host binding decorators accept at most one string argument".to_string(),
            }
            .into());
        };
        let argument = tree.get(*argument_id);
        let value = argument
            .value()
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "host binding decorator argument must have a value".to_string(),
            })?;
        let expression = tree.get(value);

        match expression {
            dir::Expression::ScalarLiteral(dir::ScalarLiteral::String(name)) => {
                Ok(strings.get(*name).to_string())
            }
            _ => Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(
                    expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                ),
                message: "host binding decorator argument must be a string".to_string(),
            }
            .into()),
        }
    }

    /// Ensure the module has been checked for this profile.
    pub(crate) fn require_checked_module(&self, module_id: ModuleId) -> CompilerResult<()> {
        // request checked DIR for the target module
        let result = self
            .compiler
            .artifact_reader(self.context)
            .dir_checked(module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        Err(error.into())
    }

    /// Read one committed checked DIR snapshot for a module when available.
    pub(crate) fn dir_checked_if_present(&self, module_id: ModuleId) -> Option<Arc<DirChecked>> {
        self.compiler
            .artifact_reader(self.context)
            .dir_checked(module_id, self.profile)
            .ok()
    }
}

/// Return whether one host decorator resolves to the requested language item.
fn host_decorator_matches(
    tree: &dir::Tree,
    bound: &DirBound,
    resolutions: &dir::ResolutionTable<'_>,
    decorator_expression: dir::LocalNodeId<dir::Expression>,
    callee: dir::LocalNodeId<dir::Expression>,
    language_item: dir::LanguageItem,
    decorator_symbol: dir::GlobalSymbolId,
) -> bool {
    let module_id = bound.bindings.module_id;
    let decorator_node = decorator_expression.into_global_any(module_id);
    if resolutions
        .symbol_resolution(decorator_node)
        .is_some_and(|symbol| symbol == decorator_symbol)
    {
        return true;
    }

    let callee_node = callee.into_global_any(module_id);
    if resolutions
        .symbol_resolution(callee_node)
        .is_some_and(|symbol| symbol == decorator_symbol)
    {
        return true;
    }

    let expression = tree.get(callee);
    expression_is_unqualified_name(expression, language_item.export_name())
}

/// Unwrap parenthesized decorator expressions.
fn unwrap_parenthesized_expression(
    tree: &dir::Tree,
    expression_id: dir::LocalNodeId<dir::Expression>,
) -> dir::LocalNodeId<dir::Expression> {
    let mut current = expression_id;
    loop {
        let expression = tree.get(current);
        let dir::Expression::Parenthesized { expression } = expression else {
            return current;
        };
        current = *expression;
    }
}

/// Return whether one expression is an unqualified reference to a name.
fn expression_is_unqualified_name(expression: &dir::Expression, name: &str) -> bool {
    let name = StringId::for_text(name);
    match expression {
        dir::Expression::Identifier { name: actual } => *actual == name,
        dir::Expression::QualifiedReference { path, .. } => {
            path.segments.len() == 1 && path.segments.first().copied() == Some(name)
        }
        _ => false,
    }
}
