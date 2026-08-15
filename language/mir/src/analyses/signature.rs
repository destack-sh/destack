use crate as mir;

/// Signature key used for matching function types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureKey {
    /// Parameter types for the signature.
    pub parameters: Vec<mir::TypeId>,
    /// Return type for the signature.
    pub result: mir::TypeId,
}

impl SignatureKey {
    /// Build a signature key from a function definition.
    pub fn from_function(function: &mir::Function) -> Self {
        // collect parameter types
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect();

        // collect the result type
        let result = function.return_type;

        Self { parameters, result }
    }

    /// Build a signature key from a function reference type.
    pub fn from_signature_type(tree: &mir::Tree, signature: &mir::TypeId) -> Option<Self> {
        // resolve the function pointer signature
        let (_, parameters, result) = tree.get(*signature).function_signature_parts()?;

        // collect parameter types
        let parameters = parameters.iter().map(|parameter| parameter.ty).collect();

        Some(Self { parameters, result })
    }

    /// Insert a function pointer signature type for a function.
    pub fn insert_function_type(
        function_id: mir::LocalNodeId<mir::Function>,
        tree: &mut mir::Tree,
    ) -> mir::LocalNodeId<mir::Type> {
        let function = tree.get(function_id);
        let lifetimes = function.lifetimes.clone();
        let parameters = function
            .parameters
            .iter()
            .map(mir::FunctionParameter::signature_parameter)
            .collect();

        tree.intern_type(mir::Type::FunctionSignature {
            lifetimes,
            parameters,
            result: function.return_type,
        })
    }
}
