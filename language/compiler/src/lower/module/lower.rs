use std::collections::HashMap;

use destack_base::StringPool;
use destack_dir::{Expression, GlobalNodeIdAny, GlobalSymbolId, LocalNodeId, LocalSymbolId};
use destack_source::ModuleId;
use destack_workspace::{Module, ProfileId, TargetId};
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult, TaskDependencyError};

use crate::lower::item::GlobalBinding;
use crate::lower::{BuiltinTypeLayouts, TypeLowerer};

/// Context for lowering a DIR module to MIR.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct ModuleLowerer<'a> {
    /// Provide access to the compiler for shared resources.
    pub(crate) compiler: &'a crate::Compiler,
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Identify the profile used for DIR access.
    pub(crate) profile: ProfileId,
    /// Provide access to the source module data.
    pub(crate) module: &'a Module,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to the root expressions for the module.
    pub(crate) dir_roots: &'a [LocalNodeId<dir::Expression>],
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Identify the target backend for lowering.
    pub(crate) target: &'a TargetId,

    /// Build MIR nodes for this module.
    pub(crate) builder: mir::ModuleBuilder,
    /// Map DIR symbols to MIR function ids.
    pub(crate) functions_by_symbol: HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Map DIR symbols to MIR global bindings.
    pub(crate) globals_by_symbol: HashMap<GlobalSymbolId, GlobalBinding>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: TypeLowerer,
}

#[allow(clippy::too_many_arguments)]
impl<'a> ModuleLowerer<'a> {
    /// Create a new module lowering context.
    pub(crate) fn new(
        compiler: &'a crate::Compiler,
        module: &'a Module,
        profile: ProfileId,
        dir_tree: &'a dir::NodeTree,
        dir_roots: &'a [LocalNodeId<dir::Expression>],
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        target: &'a TargetId,
        pointer_bytes: u8,
    ) -> Self {
        let mut builder = mir::ModuleBuilder::new();
        let type_lowerer = TypeLowerer::new(&mut builder, pointer_bytes);

        Self {
            compiler,
            module_id: module.id,
            profile,
            module,
            dir_tree,
            dir_roots,
            symbols,
            types,
            target,
            builder,
            functions_by_symbol: HashMap::new(),
            globals_by_symbol: HashMap::new(),
            type_lowerer,
        }
    }

    /// Lower this entire DIR module to MIR (in-place).
    ///
    /// Lowering proceeds in four phases:
    /// 1. Types: lower struct/class type layouts (cached on-demand)
    /// 2. Declarations: lower globals, function/method signatures and bodies
    /// 3. Tables: generate vtables, itabs, RTTI (currently stubbed)
    /// 4. Emit: (bodies are currently lowered inline with declarations)
    pub(crate) fn lower_module(&mut self) -> LowerResult<()> {
        // phase 1: types (lowered lazily as encountered)
        self.lower_types()?;

        // phase 2: declarations (globals + functions)
        self.lower_items()?;

        // phase 3: tables (vtables, itabs, RTTI)
        self.lower_tables()?;

        Ok(())
    }

    /// Phase 1: Lower type declarations.
    ///
    /// Type layouts are cached lazily when first encountered during lowering.
    /// This phase is a no-op since types are lowered on-demand.
    fn lower_types(&mut self) -> LowerResult<()> {
        // initialize builtin string layout for string literals and types
        self.initialize_string_type()?;

        // predeclare nominal layouts for struct and class instance types
        self.predeclare_nominal_layouts()?;

        // types are lowered lazily via TypeLowerer when first accessed
        Ok(())
    }

    /// Phase 2: Lower item declarations (globals, functions, methods).
    ///
    /// Processes all root expressions to lower globals and function bodies.
    fn lower_items(&mut self) -> LowerResult<()> {
        self.predeclare_external_calls()?;
        for expression_id in self.dir_roots.iter().copied() {
            self.lower_root_expression(expression_id)?;
        }
        Ok(())
    }

    /// Predeclare external functions referenced by this module.
    fn predeclare_external_calls(&mut self) -> LowerResult<()> {
        for (expression_id, expression) in self.dir_tree.iter_nodes_of_type::<dir::Expression>() {
            let dir::Expression::Call { .. } = expression else {
                continue;
            };

            let node_id = expression_id.into_global_any(self.module_id);
            let Some(resolution_id) = self.types.get_resolution_for_node(node_id) else {
                continue;
            };
            let resolution = self.types.get_resolution(resolution_id);
            let dir::Resolution::Static { candidate, .. } = resolution else {
                continue;
            };
            let target_symbol = candidate.target_symbol;
            if target_symbol.module_id == self.module_id {
                continue;
            }
            if self.functions_by_symbol.contains_key(&target_symbol) {
                continue;
            }

            let Some(signature) = candidate.resolved_signature.as_ref() else {
                return Err(LowerError::UnsupportedConstruct {
                    node: expression_id
                        .into_global_any(self.module_id)
                        .into_anchored(Some(self.profile)),
                    message: "missing resolved signature for external call".to_string(),
                });
            };

            self.ensure_external_function(expression_id, target_symbol, signature)?;
        }

        Ok(())
    }

    /// Ensure an external function is declared for a call target.
    fn ensure_external_function(
        &mut self,
        expression_id: LocalNodeId<Expression>,
        target_symbol: GlobalSymbolId,
        signature: &dir::ResolvedSignature,
    ) -> LowerResult<()> {
        if self.functions_by_symbol.contains_key(&target_symbol) {
            return Ok(());
        }

        let extern_name = self
            .extern_name_for_symbol(expression_id, target_symbol)?
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "missing @extern binding for call target".to_string(),
            })?;

        let anchor = expression_id
            .into_global_any(self.module_id)
            .into_anchored(Some(self.profile));

        let mut parameter_types = Vec::with_capacity(signature.dynamic_parameters.len());
        for type_id in &signature.dynamic_parameters {
            let parameter_type = self.type_lowerer.lower_type(
                self.types,
                *type_id,
                self.module_id,
                anchor,
                &mut self.builder,
            )?;
            parameter_types.push(parameter_type);
        }

        let return_type = match signature.return_type {
            Some(return_type) => self.type_lowerer.lower_type(
                self.types,
                return_type,
                self.module_id,
                anchor,
                &mut self.builder,
            )?,
            None => self.type_lowerer.ty_void,
        };

        let function_id = self
            .builder
            .extern_function(&extern_name, &parameter_types, return_type);
        self.functions_by_symbol.insert(target_symbol, function_id);

        Ok(())
    }

    /// Resolve the extern binding name for a symbol, if any.
    fn extern_name_for_symbol(
        &self,
        expression_id: LocalNodeId<Expression>,
        symbol: GlobalSymbolId,
    ) -> LowerResult<Option<String>> {
        self.require_analyzed_module(symbol.module_id)?;

        let module = self.compiler.program.modules.get(symbol.module_id);
        let module = module.read();
        let dir = module.dir(self.profile);
        let symbols = dir.symbols.read();
        let symbol_entry = symbols.get_symbol(symbol.local_id);
        let Some(binding) = symbol_entry.decorators.extern_binding.as_ref() else {
            return Ok(None);
        };

        if let Some(name) = binding.name {
            return Ok(Some(self.compiler.program.strings.get(name).to_string()));
        }

        let default_name = symbol_entry
            .name()
            .map(|name| self.compiler.program.strings.get(name).to_string())
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: "extern symbol is missing a name".to_string(),
            })?;

        Ok(Some(default_name))
    }

    /// Ensure the module has been analyzed for this profile.
    fn require_analyzed_module(&self, module_id: ModuleId) -> LowerResult<()> {
        let result = self
            .compiler
            .require_analyze_module(module_id, self.profile);
        let Err(error) = result else {
            return Ok(());
        };

        match error {
            TaskDependencyError::NotReady { dependency } => Err(LowerError::Yield { dependency }),
            TaskDependencyError::Failed { dependency } => {
                Err(LowerError::UnsatisfiedDependency { dependency })
            }
        }
    }

    /// Finish the module lowering process and return the resulting MIR tree and string pool.
    pub(crate) fn finish(self) -> (mir::NodeTree, StringPool) {
        self.builder.finish_mutable()
    }

    /// Get the name of a symbol as a String.
    pub(crate) fn get_symbol_name(&self, symbol_id: LocalSymbolId) -> Option<String> {
        let symbol = self.symbols.get_symbol(symbol_id);
        symbol
            .name()
            .map(|n| self.compiler.program.strings.get(n).to_string())
    }

    /// Get the name of a symbol, returning an error if it has no name.
    pub(crate) fn symbol_name(
        &self,
        symbol_id: LocalSymbolId,
        node: GlobalNodeIdAny,
    ) -> LowerResult<String> {
        self.get_symbol_name(symbol_id)
            .ok_or_else(|| LowerError::UnsupportedConstruct {
                node: node.into_anchored(Some(self.profile)),
                message: "symbol must have a name".to_string(),
            })
    }

    /// Lower a root expression.
    pub(crate) fn lower_root_expression(
        &mut self,
        expression_id: LocalNodeId<Expression>,
    ) -> LowerResult<()> {
        let expression = self.dir_tree.get(expression_id);
        match expression {
            Expression::Declaration { declaration } => {
                let declaration_id = *declaration;
                let declaration = self.dir_tree.get(declaration_id);
                self.lower_declaration(declaration_id, declaration)
            }
            Expression::Statement { statement } => {
                // unwrap statement wrapper and process the inner expression
                self.lower_root_expression(*statement)
            }
            Expression::Let {
                mutability,
                declarators,
                ..
            } => self.lower_module_let(expression_id, *mutability, declarators),
            _ => Err(LowerError::UnsupportedConstruct {
                node: expression_id
                    .into_global_any(self.module_id)
                    .into_anchored(Some(self.profile)),
                message: format!("unsupported root expression `{}`", expression.kind_name()),
            })?,
        }
    }

    /// Initialize the canonical string type from builtin definitions.
    fn initialize_string_type(&mut self) -> LowerResult<()> {
        // resolve a stable anchor for type lowering
        let Some(anchor) = self
            .dir_roots
            .first()
            .copied()
            .map(|root| root.into_global_any(self.module_id))
            .map(|root| root.into_anchored(Some(self.profile)))
        else {
            return Ok(());
        };

        // ensure builtin layouts are installed
        let mut builtin_layouts = BuiltinTypeLayouts::new(
            self.compiler,
            self.profile,
            &mut self.builder,
            &mut self.type_lowerer,
        );
        builtin_layouts.ensure_string_layout(anchor)?;

        Ok(())
    }
}
