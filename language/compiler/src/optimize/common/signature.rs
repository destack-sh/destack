use destack_mir as mir;

use crate::optimize::common::TypeKey;

/// Signature key used for matching function types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureKey {
    /// Parameter type keys for the signature.
    pub parameters: Vec<TypeKey>,
    /// Return type key for the signature.
    pub result: TypeKey,
}

impl SignatureKey {
    /// Build a signature key from a function definition.
    pub fn from_function(tree: &mir::NodeTree, function: &mir::Function) -> Self {
        // collect parameter type keys
        let parameters = function
            .parameters
            .iter()
            .map(|param| TypeKey::from_type(param.ty, tree))
            .collect();

        // collect result type key
        let result = TypeKey::from_type(function.return_type, tree);

        Self { parameters, result }
    }

    /// Build a signature key from a function pointer type.
    pub fn from_signature_type(
        tree: &mir::NodeTree,
        signature: mir::LocalNodeId<mir::Type>,
    ) -> Option<Self> {
        // resolve the function pointer signature
        let mir::Type::FunctionPointer { parameters, result } = tree.get(signature) else {
            return None;
        };

        // collect parameter type keys
        let parameters = parameters
            .iter()
            .map(|param| TypeKey::from_type(*param, tree))
            .collect();

        // collect result type key
        let result = TypeKey::from_type(*result, tree);

        Some(Self { parameters, result })
    }
}
