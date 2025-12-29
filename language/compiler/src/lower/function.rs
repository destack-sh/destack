use std::collections::HashMap;

use destack_dir::{Declaration, GlobalNodeId, GlobalSymbolId};
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::block::{BlockLowerer, LocalBinding, Terminates};
use super::{ModuleLowerer, TypeLowerer};

/// Lower a single function body into MIR.
pub(crate) struct FunctionLowerer<'a> {
    /// Identify the module being lowered.
    pub(crate) module_id: ModuleId,
    /// Provide access to the DIR tree for expression lookup.
    pub(crate) dir_tree: &'a dir::NodeTree,
    /// Provide access to symbol metadata for type resolution.
    pub(crate) symbols: &'a dir::SymbolTable,
    /// Provide access to inferred and declared types.
    pub(crate) types: &'a dir::TypeTable,
    /// Resolve direct calls for known function symbols.
    pub(crate) functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
    /// Lower and cache DIR types into MIR types.
    pub(crate) type_lowerer: &'a TypeLowerer,
    /// Emit MIR into the current function builder.
    pub(crate) builder: mir::FunctionBuilder<'a>,
    /// Track locals by symbol for variable resolution.
    pub(crate) locals_by_symbol: HashMap<GlobalSymbolId, LocalBinding>,
}

impl<'a> FunctionLowerer<'a> {
    /// Create a new function lowerer with the given builder.
    fn new(
        module_id: ModuleId,
        dir_tree: &'a dir::NodeTree,
        symbols: &'a dir::SymbolTable,
        types: &'a dir::TypeTable,
        functions_by_symbol: &'a HashMap<GlobalSymbolId, mir::LocalNodeId<mir::Function>>,
        type_lowerer: &'a TypeLowerer,
        builder: mir::FunctionBuilder<'a>,
    ) -> Self {
        Self {
            module_id,
            dir_tree,
            symbols,
            types,
            functions_by_symbol,
            type_lowerer,
            builder,
            locals_by_symbol: HashMap::new(),
        }
    }

    /// Lower a function body to MIR and return whether it terminates.
    fn lower_body(
        &mut self,
        body_id: dir::LocalNodeId<dir::Expression>,
    ) -> LowerResult<Terminates> {
        let mut block_lowerer = BlockLowerer {
            module_id: self.module_id,
            dir_tree: self.dir_tree,
            symbols: self.symbols,
            types: self.types,
            functions_by_symbol: self.functions_by_symbol,
            type_lowerer: self.type_lowerer,
            builder: &mut self.builder,
            locals_by_symbol: &mut self.locals_by_symbol,
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
                node: declaration_id.into_global_any(self.module_id),
                message: format!(
                    "unsupported non-function declaration '{}'",
                    declaration.kind_name()
                ),
            })?;
        };

        let name_id = descriptor.name.ok_or(LowerError::UnsupportedConstruct {
            node: declaration_id.into_global_any(self.module_id),
            message: "missing name".to_string(),
        })?;
        let name = self.compiler.program.strings.get(name_id).to_string();

        let symbol_id = descriptor.symbol.into_global(self.module_id);

        // return type
        let return_type =
            self.resolve_function_return_type(symbol_id, signature.return_type, declaration_id)?;

        // parameter types
        let mut parameter_types = Vec::new();
        for parameter_id in &signature.dynamic_parameters {
            let parameter_node = GlobalNodeId::new(self.module_id, *parameter_id).into();
            let parameter_ty = self
                .types
                .get_declared_or_inferred_type_id(parameter_node)
                .ok_or(LowerError::MissingType {
                    node: parameter_node,
                })?;
            let parameter_ty = self.type_lowerer.lower_type(
                self.types,
                parameter_ty,
                self.module_id,
                parameter_node,
                &mut self.builder,
            )?;
            parameter_types.push(parameter_ty);
        }

        // build the function
        let builder = self.builder.function(&name, &parameter_types, return_type);
        let function_id = builder.function_id();
        self.functions_by_symbol.insert(symbol_id, function_id);
        let mut function_lowerer = FunctionLowerer::new(
            self.module_id,
            self.dir_tree,
            self.symbols,
            self.types,
            &self.functions_by_symbol,
            &self.type_lowerer,
            builder,
        );

        // create entry block
        let entry_block = function_lowerer.builder.create_block();
        function_lowerer.builder.switch_to_block(entry_block);

        // add parameter locals
        for (index, parameter_id) in signature.dynamic_parameters.iter().enumerate() {
            let parameter = self.dir_tree.get(*parameter_id);
            let symbol = parameter.symbol().into_global(self.module_id);
            let ty = parameter_types[index];
            let variable = function_lowerer.builder.create_variable(ty);
            let value = function_lowerer.builder.function_parameter(index);
            function_lowerer.builder.define_variable(variable, value);
            function_lowerer
                .locals_by_symbol
                .insert(symbol, LocalBinding { variable, ty });
        }

        // lower body
        if let Some(body_id) = body {
            let terminated = function_lowerer.lower_body(*body_id)?;
            if terminated == Terminates::No {
                if return_type == self.type_lowerer.ty_void {
                    function_lowerer.builder.return_(None);
                } else {
                    return Err(LowerError::UnsupportedConstruct {
                        node: declaration_id.into_global_any(self.module_id),
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
    /// FUGU #Suspicious #Cleanup: why don't all functions have value types post-Analyze? #FunctionType
    fn resolve_function_return_type(
        &mut self,
        function_symbol: GlobalSymbolId,
        return_type_expr: Option<dir::LocalNodeId<dir::Expression>>,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        if let Some(function_type_id) = self.types.get_value_type_id(function_symbol) {
            let function_type = self.types.get_type(function_type_id);
            if let dir::Type::Function { return_type, .. } = function_type {
                if let Some(return_type) = return_type {
                    return self.type_lowerer.lower_type(
                        self.types,
                        *return_type,
                        self.module_id,
                        declaration_id.into_global_any(self.module_id),
                        &mut self.builder,
                    );
                }
                return Ok(self.type_lowerer.ty_void);
            }
        }

        let return_type_expr = return_type_expr.ok_or(LowerError::UnsupportedConstruct {
            node: declaration_id.into_global_any(self.module_id),
            message: "missing return type".to_string(),
        })?;
        let return_node = GlobalNodeId::new(self.module_id, return_type_expr).into();
        let declared = self
            .types
            .get_declared_or_inferred_type_id(return_node)
            .ok_or(LowerError::MissingType { node: return_node })?;
        self.type_lowerer.lower_type(
            self.types,
            declared,
            self.module_id,
            return_node,
            &mut self.builder,
        )
    }
}
