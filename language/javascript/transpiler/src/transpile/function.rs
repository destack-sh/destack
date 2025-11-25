use dyst_dir::{self as dir, Module, NodeTree, SymbolTable, TypeTable};
use dyst_javascript_ast::{
    Asynchrony, FunctionAbstraction, FunctionCardinality, FunctionKind, FunctionMode,
    FunctionSignature,
};

use crate::{TranspileError, TranspileResult, Transpiler, TranspilerUnit};

impl Transpiler {
    /// Transpile asynchrony from DIR into JS AST.
    pub fn transpile_asynchrony(&self, asynchrony: dir::Asynchrony) -> Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => Asynchrony::Sync,
            dir::Asynchrony::Async => Asynchrony::Async,
        }
    }

    /// Transpile a function abstraction from DIR into JS AST.
    pub fn transpile_function_abstraction(
        &self,
        abstraction: dir::FunctionAbstraction,
    ) -> FunctionAbstraction {
        match abstraction {
            dir::FunctionAbstraction::Abstract => FunctionAbstraction::Abstract,
            dir::FunctionAbstraction::AbstractOverride => FunctionAbstraction::AbstractOverride,
            dir::FunctionAbstraction::ConcreteOverride => FunctionAbstraction::ConcreteOverride,
            dir::FunctionAbstraction::Concrete => FunctionAbstraction::Concrete,
        }
    }

    /// Transpile function cardinality from DIR into JS AST.
    pub fn transpile_function_cardinality(
        &self,
        cardinality: dir::FunctionCardinality,
    ) -> FunctionCardinality {
        match cardinality {
            dir::FunctionCardinality::Scalar => FunctionCardinality::Scalar,
            dir::FunctionCardinality::Generator => FunctionCardinality::Generator,
        }
    }

    /// Transpile function mode from DIR into JS AST.
    pub fn transpile_function_mode(&self, mode: dir::FunctionMode) -> FunctionMode {
        match mode {
            dir::FunctionMode::Getter => FunctionMode::Getter,
            dir::FunctionMode::Setter => FunctionMode::Setter,
            dir::FunctionMode::Constructor => FunctionMode::Constructor,
            dir::FunctionMode::New => FunctionMode::New,
            dir::FunctionMode::Call => FunctionMode::Call,
        }
    }

    /// Transpile function kind from DIR into JS AST.
    pub fn transpile_function_kind(&self, kind: dir::FunctionKind) -> FunctionKind {
        match kind {
            dir::FunctionKind::Function => FunctionKind::Function,
            dir::FunctionKind::Lambda => FunctionKind::Lambda,
        }
    }

    /// Transpile a function signature from DIR into JS AST.
    pub fn transpile_function_signature(
        &self,
        module: &Module,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &TypeTable,
        function_signature: &dir::FunctionSignature,
        unit: &mut TranspilerUnit,
    ) -> TranspileResult<FunctionSignature> {
        let abstraction = self.transpile_function_abstraction(function_signature.abstraction);
        let asynchrony = self.transpile_asynchrony(function_signature.asynchrony);
        let cardinality = self.transpile_function_cardinality(function_signature.cardinality);
        let mode = function_signature
            .mode
            .map(|mode| self.transpile_function_mode(mode));
        let kind = self.transpile_function_kind(function_signature.kind);
        let generics = function_signature
            .generics
            .as_ref()
            .map(|generics| self.transpile_generics(module, tree, symbols, types, generics, unit))
            .transpose()?;
        let dynamic_parameters = function_signature
            .dynamic_parameters
            .iter()
            .map(|parameter| {
                self.transpile_parameter(module, tree, symbols, types, *parameter, unit)
            })
            .collect::<Result<Vec<_>, TranspileError>>()?;
        let return_type = function_signature.return_type.map(|_| {
            todo!("transpile return type");
        });
        Ok(FunctionSignature {
            abstraction,
            asynchrony,
            cardinality,
            mode,
            kind,
            generics,
            dynamic_parameters,
            return_type,
        })
    }
}
