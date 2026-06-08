use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, StaticOperand, StaticTerm, TypeOperand, TypeTerm};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import one dependency symbol type operand.
    pub(in crate::check) fn import_symbol_type_operand(
        &mut self,
        requesting_module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        // return cached imported operand
        if let Some(operand) = self.inputs.symbol_type(symbol) {
            return Ok(operand);
        }

        // reject component symbols
        if self.is_component_module(symbol.module_id) {
            let symbol = self.dump_in_module(requesting_module, &symbol);

            return Err(CompilerError::Internal {
                message: format!("check component symbol {symbol} reached dependency type import"),
            });
        }

        // require an imported dependency edge
        self.require_imported_dependency(requesting_module, symbol.module_id)?;

        self.import_dependency_symbol_type_operand(requesting_module, symbol)
    }

    /// Return one committed type id as a check type operand.
    pub(in crate::check) fn import_type_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalTypeId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(operand) = self.inputs.r#type(source) {
            return Ok(operand);
        }

        // import generic parameters as check parameters
        let parameter = match self.r#type(source) {
            dir::Type::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            let parameter = self.import_generic_parameter_id(module, parameter)?;
            let operand =
                TypeOperand::Term(self.inference.push_term(TypeTerm::Parameter(parameter)));

            self.inputs.upsert_type(source, operand)?;

            return Ok(operand);
        }

        Ok(TypeOperand::Type(source))
    }

    /// Return one committed static id as a check static operand.
    pub(in crate::check) fn import_static_operand(
        &mut self,
        module: ModuleId,
        source: dir::GlobalStaticId,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self.inputs.r#static(source) {
            return Ok(operand);
        }

        // import generic parameters as check parameters
        let parameter = match self.r#static(source) {
            dir::StaticTerm::Parameter(parameter) => Some(*parameter),
            _ => None,
        };
        if let Some(parameter) = parameter {
            let parameter = self.import_generic_parameter_id(module, parameter)?;
            let operand =
                StaticOperand::Term(self.inference.push_term(StaticTerm::Parameter(parameter)));

            self.inputs.upsert_static(source, operand)?;

            return Ok(operand);
        }

        Ok(StaticOperand::Static(source))
    }

    /// Import one committed dependency symbol type operand.
    fn import_dependency_symbol_type_operand(
        &mut self,
        requesting_module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        let dependency = self.dependency(symbol.module_id);
        let Some(type_id) = dependency.types.get_symbol_type_id(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("dependency symbol {symbol:?} has no committed type"),
            });
        };
        let operand = self.import_type_operand(requesting_module, type_id)?;

        self.inputs.insert_symbol_type(symbol, operand)
    }

    /// Require one dependency module imported by the requesting module.
    fn require_imported_dependency(
        &self,
        requesting_module: ModuleId,
        dependency_module: ModuleId,
    ) -> CompilerResult<()> {
        if self
            .module(requesting_module)
            .imports_dependency(dependency_module)
        {
            return Ok(());
        }

        let module = requesting_module;
        let requesting_module = self.dump_in_module(module, &requesting_module);
        let dependency_module = self.dump_in_module(module, &dependency_module);

        Err(CompilerError::Internal {
            message: format!(
                "check module {requesting_module} cannot import dependency {dependency_module}"
            ),
        })
    }
}
