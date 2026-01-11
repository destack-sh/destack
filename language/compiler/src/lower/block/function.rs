use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::{Declaration, GlobalNodeId, GlobalSymbolId};
use destack_source::ModuleId;
use destack_workspace::ProfileId;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::{GlobalBinding, ModuleLowerer, TypeLowerer};
use super::block::{BlockLowerer, LocalBinding, LoopContext, Terminates};

/// Lower a single function body into MIR.
pub(crate) struct FunctionLowerer<'a> {
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Provide access to the program string pool for name resolution.
    pub(crate) strings: &'a StringPool,
    /// Resolve direct calls for known function symbols.
    pub(crate) functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Resolve globals by symbol for module-level variable references.
    pub(crate) globals_by_symbol: &'a HashMap<GlobalSymbolId, GlobalBinding>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
    /// Emit MIR into the current function builder.
    pub(crate) builder: mir::FunctionBuilder<'a>,
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: HashMap<GlobalSymbolId, LocalBinding>,
    /// Track loop contexts by symbol for labeled break/continue.
    pub(crate) loops_by_symbol: HashMap<GlobalSymbolId, LoopContext>,
    /// Track loop nesting for unlabeled break/continue.
    pub(crate) loop_stack: Vec<LoopContext>,
    /// Binding for `this` in method bodies.
    pub(crate) this_binding: Option<LocalBinding>,
}

#[allow(clippy::too_many_arguments)]
impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer with the given builder.
    pub(crate) fn new(
        module_id: ModuleId,
        profile: ProfileId,
        dir_tree: &'a dir::NodeTree,
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        strings: &'a StringPool,
        functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
        globals_by_symbol: &'a HashMap<GlobalSymbolId, GlobalBinding>,
        type_lowerer: &'a TypeLowerer,
        builder: mir::FunctionBuilder<'a>,
    ) -> Self {
        Self {
            module_id,
            profile,
            dir_tree,
            symbols,
            types,
            strings,
            functions_by_symbol,
            globals_by_symbol,
            type_lowerer,
            builder,
            locals_by_symbol: HashMap::new(),
            loops_by_symbol: HashMap::new(),
            loop_stack: Vec::new(),
            this_binding: None,
        }
    }

    /// Lower a function body to MIR and return whether it terminates.
    pub(crate) fn lower_body(
        &mut self,
        body_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<Terminates> {
        let mut block_lowerer = BlockLowerer {
            module_id: self.module_id,
            profile: self.profile,
            dir_tree: self.dir_tree,
            symbols: self.symbols,
            types: self.types,
            strings: self.strings,
            functions_by_symbol: self.functions_by_symbol,
            globals_by_symbol: self.globals_by_symbol,
            type_lowerer: self.type_lowerer,
            builder: &mut self.builder,
            locals_by_symbol: &mut self.locals_by_symbol,
            loops_by_symbol: &mut self.loops_by_symbol,
            loop_stack: &mut self.loop_stack,
            this_binding: self.this_binding,
        };

        block_lowerer.lower_statement_expression(body_id)
    }
}

impl ModuleLowerer<'_> {
    /// Lower a function declaration to a MIR function.
    pub(crate) fn lower_function(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        declaration: &Declaration,
    ) -> LowerResult<mir::LocalNodeId<mir::Function>> {
        let Declaration::Function {
            descriptor,
            signature,
            body,
            ..
        } = declaration
        else {
            return Err(LowerError::UnsupportedConstruct {
                node: declaration_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!(
                    "unsupported non-function declaration '{}'",
                    declaration.kind_name()
                ),
            })?;
        };

        let name_id =
            descriptor
                .name
                .map(|name| name.string())
                .ok_or(LowerError::UnsupportedConstruct {
                    node: declaration_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "missing name".to_string(),
                })?;
        let name = self.compiler.program.strings.get(name_id).to_string();

        let symbol_id = descriptor.symbol.into_global(self.module_id);

        // return type
        let return_type = self.resolve_function_return_type(declaration_id)?;

        // parameter types
        let mut parameter_types = Vec::new();
        for parameter_id in &signature.dynamic_parameters {
            let parameter_node = GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty = self
                .types
                .get_declared_or_inferred_type_id(parameter_node)
                .ok_or(LowerError::MissingType {
                    node: parameter_node.into_anchored(Some(self.profile)),
                })?;
            let parameter_ty = self.type_lowerer.lower_type(
                self.types,
                parameter_ty,
                self.module_id,
                parameter_node.into_anchored(Some(self.profile)),
                &mut self.builder,
            )?;
            parameter_types.push(parameter_ty);
        }

        // extract return lifetime from @lifetime decorator before borrowing self.builder
        let return_lifetime = self.extract_lifetime_annotation(declaration_id, signature);

        // build the function
        let mut builder = self.builder.function(&name, &parameter_types, return_type);
        let function_id = builder.function_id();
        self.functions_by_symbol.insert(symbol_id, function_id);

        // set return lifetime
        builder.set_return_lifetime(return_lifetime);

        let mut function_lowerer = FunctionLowerer::new(
            self.module_id,
            self.profile,
            self.dir_tree,
            self.symbols,
            self.types,
            &self.compiler.program.strings,
            &self.functions_by_symbol,
            &self.globals_by_symbol,
            &self.type_lowerer,
            builder,
        );

        // create entry block
        let entry_block = function_lowerer.builder.create_block();
        function_lowerer.builder.switch_to_block(entry_block);

        // add parameter locals
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let parameter = self.dir_tree.get(*parameter_id);
            let symbol_id = parameter.symbol().into_global(self.module_id);
            let ty = parameter_types[index];
            let variable = function_lowerer.builder.create_variable(ty);
            let value = function_lowerer.builder.function_parameter(index);
            function_lowerer.builder.define_variable(variable, value);
            function_lowerer
                .locals_by_symbol
                .insert(symbol_id, LocalBinding { variable, ty });
        }

        // lower body
        if let Some(body_id) = body {
            let terminated = function_lowerer.lower_body(*body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_lowerer.builder.return_(None);
                } else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: declaration_id
                            .into_global_any(self.module_id)
                            .into_anchored(Some(self.profile)),
                        message: "missing terminator".to_string(),
                    })?;
                }
            }
        } else {
            function_lowerer.builder.return_(None);
        }

        function_lowerer.builder.finish();
        Ok(function_id)
    }

    /// Resolve a function return type for lowering.
    fn resolve_function_return_type(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let node_id = declaration_id.into_global_any(self.module_id);

        let signature_type_id =
            self.types
                .get_inferred_type_id(node_id)
                .ok_or(LowerError::MissingType {
                    node: node_id.into_anchored(Some(self.profile)),
                })?;
        let return_type_id = match self.types.get_type(signature_type_id) {
            dir::Type::Function { return_type, .. } => {
                return_type.ok_or(LowerError::MissingType {
                    node: node_id.into_anchored(Some(self.profile)),
                })?
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: node_id.into_anchored(Some(self.profile)),
                    message: "missing function signature type".to_string(),
                })?;
            }
        };

        self.type_lowerer.lower_type(
            self.types,
            return_type_id,
            self.module_id,
            node_id.into_anchored(Some(self.profile)),
            &mut self.builder,
        )
    }

    /// Extract return lifetime from @lifetime decorator annotations on a function.
    ///
    /// Supports:
    /// - `@lifetime("static")` - static lifetime
    /// - `@lifetime(param1, param2, ...)` - borrows from named parameters
    fn extract_lifetime_annotation(
        &self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
        signature: &dir::FunctionSignature,
    ) -> mir::Lifetime {
        let annotations = self.dir_tree.get_annotations(declaration_id.id);

        for annotation_id in annotations {
            let annotation = self.dir_tree.get(annotation_id);

            // look for decorator annotations
            let dir::Annotation::Decorator {
                left, arguments, ..
            } = annotation
            else {
                continue;
            };

            // check if the decorator is named "lifetime"
            let left_expr = self.dir_tree.get(*left);
            let is_lifetime = match left_expr {
                dir::Expression::UnresolvedPath { path, .. }
                | dir::Expression::LocalReference { path, .. }
                | dir::Expression::ModuleReference { path, .. }
                | dir::Expression::GlobalReference { path, .. } => {
                    if let Some(first) = path.first_segment() {
                        self.compiler.program.strings.get(first) == "lifetime"
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if !is_lifetime {
                continue;
            }

            // parse the arguments
            let Some(args) = arguments else {
                // @lifetime with no args: defaults to inferred
                continue;
            };
            if args.is_empty() {
                continue;
            }

            // check for @lifetime("static")
            if args.len() == 1 {
                let arg = self.dir_tree.get(args[0]);
                if let dir::Argument::Positional { value }
                | dir::Argument::Named { value, .. }
                | dir::Argument::Labeled { value, .. } = arg
                {
                    let expr = self.dir_tree.get(*value);
                    if let dir::Expression::ScalarLiteral {
                        value: dir::ScalarLiteral::String(string_id),
                    } = expr
                        && self.compiler.program.strings.get(*string_id) == "static"
                    {
                        return mir::Lifetime::Static;
                    }
                }
            }

            // build mapping from parameter names to indices
            let mut param_name_to_index: HashMap<String, u32> = HashMap::new();
            for (index, param_id) in signature.dynamic_parameters.iter().enumerate() {
                let param: &dir::Parameter = self.dir_tree.get(*param_id);
                let param_name = match param {
                    dir::Parameter::Named { name, .. } => Some(*name),
                    dir::Parameter::Variadic { name, .. } => Some(*name),
                    dir::Parameter::Pattern { .. } => None,
                };
                if let Some(name_id) = param_name {
                    let name_str = self.compiler.program.strings.get(name_id).to_string();
                    param_name_to_index.insert(name_str, index as u32);
                }
            }

            // parse parameter references from arguments
            let mut param_indices = Vec::new();
            for arg_id in args {
                let arg = self.dir_tree.get(*arg_id);
                let value_id = match arg {
                    dir::Argument::Positional { value } => value,
                    dir::Argument::Named { value, .. } => value,
                    dir::Argument::Labeled { value, .. } => value,
                    dir::Argument::Spread { value, .. } => value,
                };
                let expr = self.dir_tree.get(*value_id);

                // look for identifier references that match parameter names
                let param_name: Option<String> = match expr {
                    // single-segment path is a parameter reference
                    dir::Expression::UnresolvedPath { path, .. }
                    | dir::Expression::LocalReference { path, .. } => {
                        if path.segments.len() == 1 {
                            Some(
                                self.compiler
                                    .program
                                    .strings
                                    .get(path.segments[0])
                                    .to_string(),
                            )
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(name) = param_name
                    && let Some(&index) = param_name_to_index.get(&name)
                    && !param_indices.contains(&index)
                {
                    param_indices.push(index);
                }
            }

            if !param_indices.is_empty() {
                return mir::Lifetime::Parameters(param_indices);
            }
        }

        mir::Lifetime::Inferred
    }
}
