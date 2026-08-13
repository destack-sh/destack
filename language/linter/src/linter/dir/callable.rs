use destack_dir as dir;
use destack_repository::ProviderError;

use super::DirModule;

impl DirModule<'_> {
    /// Return the body expression owned by one callable node.
    pub(crate) fn callable_body(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();

        match node.ty {
            // function declaration
            dir::NodeType::Declaration => {
                let declaration = dir::LocalNodeId::<dir::Declaration>::new(node.id);
                let dir::Declaration::Function(function) = view.get(declaration) else {
                    return None;
                };

                function.body
            }
            // class or interface method
            dir::NodeType::Member => {
                let member = dir::LocalNodeId::<dir::Member>::new(node.id);
                let dir::Member::Method { body, .. } = view.get(member) else {
                    return None;
                };

                *body
            }
            // object method
            dir::NodeType::Property => {
                let property = dir::LocalNodeId::<dir::Property>::new(node.id);
                let dir::Property::Method { body, .. } = view.get(property) else {
                    return None;
                };

                *body
            }
            // structural type method
            dir::NodeType::TypeMember => {
                let member = dir::LocalNodeId::<dir::TypeMember>::new(node.id);
                let dir::TypeMember::Method { body, .. } = view.get(member) else {
                    return None;
                };

                *body
            }
            // non-callable node
            _ => None,
        }
    }

    /// Return the nearest callable body containing one node.
    pub(crate) fn enclosing_callable_body(
        &self,
        node: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalNodeId<dir::Expression>> {
        let view = self.view();
        let mut current = Some(node);

        // climb to the first callable owner
        while let Some(node) = current {
            if let Some(body) = self.callable_body(node) {
                return Some(body);
            }

            current = view.get_parent_any(node);
        }

        None
    }

    /// Return the function declaration authored as one lambda expression.
    pub(crate) fn lambda(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Option<&dir::FunctionDeclaration> {
        // select the wrapped function declaration
        let view = self.view();
        let dir::Expression::Declaration(declaration) = view.get(expression) else {
            return None;
        };
        let dir::Declaration::Function(function) = view.get(*declaration) else {
            return None;
        };

        // require the lambda function form
        if function.signature.form != dir::FunctionForm::Lambda {
            return None;
        }

        Some(function)
    }

    /// Return the checked result type id of one callable expression.
    pub fn callable_return_type_id(
        &self,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> Result<dir::GlobalTypeId, ProviderError> {
        // inspect the callable value beneath placement forms
        let type_id = self.node_type_id(expression.into_any())?;
        let type_id = self.dir.strip_form(type_id)?;
        let ty = self.dir.get_type(type_id)?;

        // select the callable signature
        let signature = match ty {
            dir::Type::FunctionSignature(_) => type_id,
            dir::Type::Function(function) => function.signature,
            dir::Type::FunctionPointer(function) => function.signature,
            _ => {
                return Err(ProviderError::internal(format!(
                    "checked callback expression {expression:?} has non-callable type {ty:?}"
                )));
            }
        };

        self.dir.signature_return_type_id(signature)
    }
}
