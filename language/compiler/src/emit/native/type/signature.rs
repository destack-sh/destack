use cranelift_codegen::ir as cir;
use destack_mir as mir;

use crate::EmitError;

use super::{TypeEmitter, ValueType};

impl TypeEmitter<'_> {
    /// Build the internal native signature for one MIR function.
    pub(in crate::emit::native) fn signature(
        &self,
        function: &mir::Function,
    ) -> Result<cir::Signature, EmitError> {
        let environment = function
            .environment
            .map(|environment| self.value(environment))
            .transpose()?;
        let parameters = function.parameters.iter().map(|parameter| parameter.ty);
        let signature = self.build_signature(environment, parameters, function.return_type)?;

        Ok(signature)
    }

    /// Build one indirect native call signature.
    pub(in crate::emit::native) fn call_signature(
        &self,
        signature: mir::TypeId,
        has_environment: bool,
    ) -> Result<cir::Signature, EmitError> {
        // read the callable parameters and result type
        let (_, parameters, result) = self
            .optimized
            .tree
            .get(signature)
            .function_signature_parts()
            .ok_or_else(|| self.unsupported("native call signature is not callable"))?;
        let pointer = self.pointer();
        let environment = has_environment.then_some(ValueType::Direct {
            ty: pointer,
            byte_len: pointer.bytes(),
            alignment: pointer.bytes(),
        });
        let parameters = parameters.iter().map(|parameter| parameter.ty);
        let signature = self.build_signature(environment, parameters, result)?;

        Ok(signature)
    }

    /// Build one typed native signature.
    fn build_signature(
        &self,
        environment: Option<ValueType>,
        parameters: impl IntoIterator<Item = mir::TypeId>,
        result: mir::TypeId,
    ) -> Result<cir::Signature, EmitError> {
        let result = self.result(result)?;
        let mut signature = cir::Signature::new(self.call_conv());

        // pass the activation and optional indirect result address first
        signature.params.push(cir::AbiParam::special(
            self.pointer(),
            cir::ArgumentPurpose::VMContext,
        ));
        if result.is_some_and(ValueType::is_indirect) {
            signature.params.push(cir::AbiParam::new(self.pointer()));
        }

        // pass the captured environment before explicit parameters
        if let Some(environment) = environment {
            signature.params.extend(environment.abi(self.pointer()));
        }

        // flatten direct values and pass larger values by canonical address
        for parameter in parameters {
            let parameter = self.value(parameter)?;
            signature.params.extend(parameter.abi(self.pointer()));
        }

        // return direct results in their native physical representation
        if let Some(result) = result.filter(|result| !result.is_indirect()) {
            signature.returns.extend(result.abi(self.pointer()));
        }

        Ok(signature)
    }

    /// Build the canonical engine-transition signature.
    pub(in crate::emit::native) fn entry_signature(&self) -> cir::Signature {
        let mut signature = cir::Signature::new(self.call_conv());
        signature.params.extend([
            cir::AbiParam::new(self.pointer()),
            cir::AbiParam::new(self.pointer()),
            cir::AbiParam::new(self.pointer()),
        ]);
        signature
    }
}
