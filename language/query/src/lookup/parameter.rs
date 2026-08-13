use destack_dir as dir;

use crate::{ModuleQueryContext, ProgramQueryContext, QueryError, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return the exact display name for one parameter.
    pub(crate) fn parameter_name(&self, parameter: &dir::Parameter) -> QueryResult<String> {
        match parameter {
            dir::Parameter::Named { name, .. } => Ok(self.strings().get(*name).to_string()),
            dir::Parameter::Pattern { pattern, .. } => {
                let span = self.node_span(self.view()?, (*pattern).into())?;
                let pattern = self.source_text(span)?;

                Ok(pattern)
            }
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = self.strings().get(*name);

                Ok(format!("...{name}"))
            }
            dir::Parameter::VariadicPattern { pattern, .. } => {
                let span = self.node_span(self.view()?, (*pattern).into())?;
                let pattern = self.source_text(span)?;

                Ok(format!("...{pattern}"))
            }
            dir::Parameter::Error => Err(QueryError::missing("parameter name")),
        }
    }

    /// Return the declared parameters for one callable symbol.
    pub(crate) fn callable_parameters(
        &self,
        symbol_id: dir::LocalSymbolId,
    ) -> QueryResult<Option<&[dir::LocalNodeId<dir::Parameter>]>> {
        // read the symbol declaration
        let global_node_id = {
            let symbols = self.bindings()?;
            let symbol = symbols.get_symbol(symbol_id);
            let Some(declaration) = symbol.declaration else {
                return Ok(None);
            };

            declaration
        };

        self.node_callable_parameters(global_node_id.local_id)
    }

    /// Return the declared parameters authored at one callable node.
    pub(crate) fn node_callable_parameters(
        &self,
        node_id: dir::LocalNodeIdAny,
    ) -> QueryResult<Option<&[dir::LocalNodeId<dir::Parameter>]>> {
        // read parameters from the exact authored declaration
        let view = self.view()?;
        let parameters: Option<&[dir::LocalNodeId<dir::Parameter>]> = match node_id.ty {
            dir::NodeType::Declaration => {
                let declaration_id = dir::LocalNodeId::<dir::Declaration>::new(node_id.id);
                let declaration = view.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return Ok(None);
                };

                Some(&declaration.signature.parameters)
            }
            dir::NodeType::Member => {
                let member_id = dir::LocalNodeId::<dir::Member>::new(node_id.id);
                let member = view.get::<dir::Member>(member_id);

                match member {
                    dir::Member::Method { signature, .. } => Some(&signature.parameters),
                    dir::Member::Field { declared_type, .. }
                    | dir::Member::AssociatedConst { declared_type, .. } => {
                        self.callable_type_parameters(*declared_type)?
                    }
                    dir::Member::AssociatedType { .. }
                    | dir::Member::StaticBlock { .. }
                    | dir::Member::ConstBlock { .. }
                    | dir::Member::Error => None,
                }
            }
            dir::NodeType::TypeMember => {
                let member_id = dir::LocalNodeId::<dir::TypeMember>::new(node_id.id);
                let member = view.get::<dir::TypeMember>(member_id);

                match member {
                    dir::TypeMember::Method { signature, .. } => Some(&signature.parameters),
                    dir::TypeMember::Field { declared_type, .. }
                    | dir::TypeMember::AssociatedConst { declared_type, .. } => {
                        self.callable_type_parameters(*declared_type)?
                    }
                    dir::TypeMember::CallSignature { signature } => Some(&signature.parameters),
                    dir::TypeMember::ConstructSignature { signature } => {
                        Some(&signature.parameters)
                    }
                    dir::TypeMember::IndexSignature { .. }
                    | dir::TypeMember::AssociatedType { .. }
                    | dir::TypeMember::Error => None,
                }
            }
            dir::NodeType::Parameter => {
                let parameter_id = dir::LocalNodeId::<dir::Parameter>::new(node_id.id);
                let parameter = view.get::<dir::Parameter>(parameter_id);

                self.callable_type_parameters(parameter.declared_type())?
            }
            dir::NodeType::Pattern => {
                let pattern_id = dir::LocalNodeId::<dir::Pattern>::new(node_id.id);

                self.binding_callable_parameters(pattern_id)?
            }
            _ => None,
        };

        Ok(parameters)
    }

    /// Return parameters declared by one callable type expression.
    fn callable_type_parameters(
        &self,
        type_id: Option<dir::LocalNodeId<dir::TypeExpression>>,
    ) -> QueryResult<Option<&[dir::LocalNodeId<dir::Parameter>]>> {
        let Some(type_id) = type_id else {
            return Ok(None);
        };
        let dir::TypeExpression::Function(function) = self.view()?.get(type_id) else {
            return Ok(None);
        };

        Ok(Some(&function.parameters))
    }

    /// Return parameters declared by one callable binding.
    fn binding_callable_parameters(
        &self,
        pattern_id: dir::LocalNodeId<dir::Pattern>,
    ) -> QueryResult<Option<&[dir::LocalNodeId<dir::Parameter>]>> {
        let view = self.view()?;
        let Some(parent) = view.get_parent_any(pattern_id.into_any()) else {
            return Ok(None);
        };
        if parent.ty != dir::NodeType::Declarator {
            return Ok(None);
        }
        let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(parent.id);
        let declarator = view.get(declarator_id);

        // prefer the declared function type
        if declarator.ty.is_some() {
            return self.callable_type_parameters(declarator.ty);
        }

        // otherwise use the authored lambda initializer
        let Some(value_id) = declarator.value else {
            return Ok(None);
        };
        let dir::Expression::Declaration(declaration_id) = view.get(value_id) else {
            return Ok(None);
        };
        let dir::Declaration::Function(function) = view.get(*declaration_id) else {
            return Ok(None);
        };

        Ok(Some(&function.signature.parameters))
    }
}

impl ProgramQueryContext<'_> {
    /// Return parameter names for one callable symbol.
    pub(crate) fn symbol_parameter_names(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Vec<String>>> {
        let module = self.module(symbol_id.module_id)?;
        let Some(parameters) = module.callable_parameters(symbol_id.local_id)? else {
            return Ok(None);
        };
        let view = module.view()?;
        let names = parameters
            .iter()
            .map(|parameter_id| module.parameter_name(view.get(*parameter_id)))
            .collect::<QueryResult<Vec<_>>>()?;

        Ok(Some(names))
    }

    /// Return parameter names authored at one callable signature node.
    pub(crate) fn node_parameter_names(
        &self,
        node_id: dir::GlobalNodeIdAny,
    ) -> QueryResult<Option<Vec<Option<String>>>> {
        let module = self.module(node_id.module_id)?;

        // read the single inline key parameter of index signatures
        if node_id.local_id.ty == dir::NodeType::TypeMember {
            let member_id = dir::LocalNodeId::<dir::TypeMember>::new(node_id.local_id.id);
            if let dir::TypeMember::IndexSignature { name, .. } = module.view()?.get(member_id) {
                let name = module.strings().get(*name).to_string();

                return Ok(Some(vec![Some(name)]));
            }
        }

        // read the authored parameter list of other signatures
        let Some(parameters) = module.node_callable_parameters(node_id.local_id)? else {
            return Ok(None);
        };
        let view = module.view()?;
        let names = parameters
            .iter()
            .map(|parameter_id| module.parameter_name(view.get(*parameter_id)))
            .collect::<QueryResult<Vec<_>>>()?;

        Ok(Some(names.into_iter().map(Some).collect()))
    }
}
