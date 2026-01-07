use std::collections::HashMap;

use destack_dir::{Declaration, GlobalNodeId, GlobalSymbolId};
use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use crate::{LowerError, LowerResult};

use super::super::{ModuleLowerer, TypeLowerer};
use super::block::{BlockLowerer, LocalBinding, Terminates};

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

        let name_id =
            descriptor
                .name
                .map(|name| name.string())
                .ok_or(LowerError::UnsupportedConstruct {
                    node: declaration_id.into_global_any(self.module_id),
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
    fn resolve_function_return_type(
        &mut self,
        declaration_id: dir::LocalNodeId<dir::Declaration>,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        let node_id = declaration_id.into_global_any(self.module_id);

        let signature_type_id = self
            .types
            .get_inferred_type_id(node_id)
            .ok_or(LowerError::MissingType { node: node_id })?;
        let return_type_id = match self.types.get_type(signature_type_id) {
            dir::Type::Function { return_type, .. } => {
                return_type.ok_or(LowerError::MissingType { node: node_id })?
            }
            _ => {
                return Err(LowerError::UnsupportedConstruct {
                    node: node_id,
                    message: "missing function signature type".to_string(),
                })?;
            }
        };

        self.type_lowerer.lower_type(
            self.types,
            return_type_id,
            self.module_id,
            node_id,
            &mut self.builder,
        )
    }
}

#[cfg(test)]
mod tests {
    use destack_vm::Value;

    use crate::TestProgram;

    /// Lower a simple add function into MIR and execute it.
    #[test]
    fn test_lower_add_function() {
        // setup module
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function add(a: int32, b: int32): int32 {
    return a + b;
}
"#,
        );

        // lower and typecheck
        test.lower_module(module_id, "native");
        test.compile_check_clean();

        // assert mir output
        test.assert_mir(
            module_id,
            "native",
            r#"
function @add(v0: i32, v1: i32) -> i32 {
block0:
    v2 = iadd v0, v1
    return v2
}
        "#,
        );

        // assert execution
        test.assert_mir_function_output(
            module_id,
            "native",
            "add",
            &[Value::int32(1), Value::int32(2)],
            Value::int32(3),
        );
    }

    /// Lower fibonacci with recursion and execute it.
    #[test]
    fn test_lower_fibonacci_function() {
        // setup module
        let test = TestProgram::memory_sequential();
        let module_id = test.add_module(
            "test.ds",
            r#"
function fibonacci(n: number): number {
    if (n < 2) {
        return n;
    }
    return fibonacci(n - 1) + fibonacci(n - 2);
}
"#,
        );

        // lower and typecheck
        test.lower_module(module_id, "native");
        test.compile_check_clean();

        // assert mir output
        test.assert_mir(
            module_id,
            "native",
            r#"
function @fibonacci(v0: f64) -> f64 {
block0:
    v1 = iconst 2i32
    v2 = scvt_to_float v1 -> f64
    v3 = fcmp_lt v0, v2
    branch v3, block1, block2
block1:
    return v0
block2:
    jump block3
block3:
    v6 = iconst 1f64
    v7 = fsub v0, v6
    v8 = call @fibonacci(v7)
    v9 = iconst 2f64
    v10 = fsub v0, v9
    v11 = call @fibonacci(v10)
    v12 = fadd v8, v11
    return v12
}
"#,
        );

        // assert execution
        test.assert_mir_function_output(
            module_id,
            "native",
            "fibonacci",
            &[Value::float64(10.0)],
            Value::float64(55.0),
        );
    }
}
