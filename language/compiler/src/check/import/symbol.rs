use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{CheckState, StaticOperand, TypeOperand};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Import one dependency symbol type operand.
    pub(in crate::check) fn import_symbol_type_operand(
        &mut self,
        requesting_module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<TypeOperand> {
        if let Some(operand) = self.inputs.symbol_type(symbol) {
            return Ok(operand);
        }

        if self.is_component_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message: format!("check component symbol {symbol:?} has no type operand"),
            });
        }

        if !self
            .module(requesting_module)
            .dependencies
            .contains(&symbol.module_id)
        {
            return Err(CompilerError::Internal {
                message: "dependency symbol module must be imported by requesting module"
                    .to_owned(),
            });
        }

        let dependency = self.dependency(symbol.module_id);
        let Some(source) = dependency.types.get_symbol_type_id(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("dependency symbol {symbol:?} has no committed type"),
            });
        };
        let operand = self.import_type_operand(requesting_module, source)?;

        self.inputs.insert_symbol_type(symbol, operand)
    }

    /// Import one dependency symbol static operand.
    pub(in crate::check) fn import_symbol_static_operand(
        &mut self,
        requesting_module: ModuleId,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<StaticOperand> {
        if let Some(operand) = self.inputs.symbol_static(symbol) {
            return Ok(operand);
        }

        if self.is_component_module(symbol.module_id) {
            return Err(CompilerError::Internal {
                message: format!("check component symbol {symbol:?} has no static operand"),
            });
        }

        if !self
            .module(requesting_module)
            .dependencies
            .contains(&symbol.module_id)
        {
            return Err(CompilerError::Internal {
                message: "dependency static symbol module must be imported by requesting module"
                    .to_owned(),
            });
        }

        let dependency = self.dependency(symbol.module_id);
        let Some(source) = dependency.statics.get_symbol_static_id(symbol) else {
            return Err(CompilerError::Internal {
                message: format!("dependency static symbol {symbol:?} has no committed static"),
            });
        };
        let operand = self.import_static_operand(requesting_module, source)?;

        self.inputs.insert_symbol_static(symbol, operand)
    }
}
