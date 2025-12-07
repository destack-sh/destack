use crate::{
    Asynchrony, CodegenJsError, CodegenJsResult, FunctionAbstraction, FunctionCardinality,
    FunctionKind, FunctionMode, FunctionSignature, ModuleLowerer,
};
use destack_dir as dir;

impl ModuleLowerer<'_> {
    /// Lower asynchrony from DIR into JS AST.
    pub fn lower_asynchrony(&self, asynchrony: dir::Asynchrony) -> Asynchrony {
        match asynchrony {
            dir::Asynchrony::Sync => Asynchrony::Sync,
            dir::Asynchrony::Async => Asynchrony::Async,
        }
    }

    /// Lower a function abstraction from DIR into JS AST.
    pub fn lower_function_abstraction(
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

    /// Lower function cardinality from DIR into JS AST.
    pub fn lower_function_cardinality(
        &self,
        cardinality: dir::FunctionCardinality,
    ) -> FunctionCardinality {
        match cardinality {
            dir::FunctionCardinality::Scalar => FunctionCardinality::Scalar,
            dir::FunctionCardinality::Generator => FunctionCardinality::Generator,
        }
    }

    /// Lower function mode from DIR into JS AST.
    pub fn lower_function_mode(&self, mode: dir::FunctionMode) -> FunctionMode {
        match mode {
            dir::FunctionMode::Getter => FunctionMode::Getter,
            dir::FunctionMode::Setter => FunctionMode::Setter,
            dir::FunctionMode::Constructor => FunctionMode::Constructor,
            dir::FunctionMode::New => FunctionMode::New,
            dir::FunctionMode::Call => FunctionMode::Call,
        }
    }

    /// Lower function kind from DIR into JS AST.
    pub fn lower_function_kind(&self, kind: dir::FunctionKind) -> FunctionKind {
        match kind {
            dir::FunctionKind::Function => FunctionKind::Function,
            dir::FunctionKind::Lambda => FunctionKind::Lambda,
        }
    }

    /// Lower a function signature from DIR into JS AST.
    pub fn lower_function_signature(
        &mut self,
        function_signature: &dir::FunctionSignature,
    ) -> CodegenJsResult<FunctionSignature> {
        let abstraction = self.lower_function_abstraction(function_signature.abstraction);
        let asynchrony = self.lower_asynchrony(function_signature.asynchrony);
        let cardinality = self.lower_function_cardinality(function_signature.cardinality);
        let mode = function_signature
            .mode
            .map(|mode| self.lower_function_mode(mode));
        let kind = self.lower_function_kind(function_signature.kind);
        let generics = function_signature
            .generics
            .as_ref()
            .map(|generics| self.lower_generics(generics))
            .transpose()?;
        let dynamic_parameters = function_signature
            .dynamic_parameters
            .iter()
            .map(|parameter| self.lower_parameter(*parameter))
            .collect::<Result<Vec<_>, CodegenJsError>>()?;
        let return_type = function_signature.return_type.map(|_| {
            todo!("generate return type");
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
