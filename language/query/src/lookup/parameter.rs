use destack_dir as dir;

use crate::{ModuleQueryContext, QueryContext, QueryResult};

impl ModuleQueryContext<'_> {
    /// Return the exact display name for one parameter.
    pub(crate) fn parameter_name(&self, parameter: &dir::Parameter) -> QueryResult<Option<String>> {
        match parameter {
            dir::Parameter::Named { name, .. } => Ok(Some(self.strings().get(*name).to_string())),
            dir::Parameter::Pattern { pattern, .. } => {
                let span = self.node_span(self.view(), (*pattern).into())?;
                let pattern = self.source_text(span)?;

                Ok(Some(pattern))
            }
            dir::Parameter::VariadicNamed { name, .. } => {
                let name = self.strings().get(*name);

                Ok(Some(format!("...{name}")))
            }
            dir::Parameter::VariadicPattern { pattern, .. } => {
                let span = self.node_span(self.view(), (*pattern).into())?;
                let pattern = self.source_text(span)?;

                Ok(Some(format!("...{pattern}")))
            }
            dir::Parameter::Error => Ok(None),
        }
    }

    /// Return the declared parameters for one callable symbol.
    pub(crate) fn callable_parameters(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<&[dir::LocalNodeId<dir::Parameter>]> {
        // read the symbol declaration
        let global_node_id = {
            let symbols = self.symbols();
            let symbol = symbols.get_symbol(symbol_id.local_id);
            symbol.declaration?
        };

        // resolve the parameter target based on the declaration node type
        let view = self.view();
        match global_node_id.local_id.ty {
            // collect parameters from function declarations
            dir::NodeType::Declaration => {
                let declaration_id =
                    dir::LocalNodeId::<dir::Declaration>::new(global_node_id.local_id.id);
                let declaration = view.get::<dir::Declaration>(declaration_id);
                let dir::Declaration::Function(declaration) = declaration else {
                    return None;
                };

                Some(&declaration.signature.parameters)
            }

            // collect parameters from method members
            dir::NodeType::Member => {
                let member_id = dir::LocalNodeId::<dir::Member>::new(global_node_id.local_id.id);
                let member = view.get::<dir::Member>(member_id);
                let signature = member.signature()?;

                Some(&signature.parameters)
            }
            dir::NodeType::TypeMember => {
                let member_id =
                    dir::LocalNodeId::<dir::TypeMember>::new(global_node_id.local_id.id);
                let member = view.get::<dir::TypeMember>(member_id);
                let signature = member.signature()?;

                Some(&signature.parameters)
            }

            _ => None,
        }
    }

    /// Return the function type parameters declared on a variable binding.
    pub(crate) fn variable_callable_parameters(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> Option<&[dir::LocalNodeId<dir::Parameter>]> {
        let symbol = self.symbols().get_symbol(symbol_id.local_id);
        let declaration = symbol.declaration?;
        if declaration.local_id.ty != dir::NodeType::Pattern {
            return None;
        }

        // select the variable declarator
        let view = self.view();
        let parent = view.get_parent_any(declaration.local_id)?;
        if parent.ty != dir::NodeType::Declarator {
            return None;
        }
        let declarator_id = dir::LocalNodeId::<dir::Declarator>::new(parent.id);
        let declarator = view.get(declarator_id);

        // prefer the declared function type
        if let Some(type_id) = declarator.ty
            && let dir::TypeExpression::Function(function) = view.get(type_id)
        {
            return Some(&function.parameters);
        }

        // otherwise use the authored lambda initializer
        let value_id = declarator.value?;
        let dir::Expression::Declaration(declaration_id) = view.get(value_id) else {
            return None;
        };
        let dir::Declaration::Function(function) = view.get(*declaration_id) else {
            return None;
        };

        Some(&function.signature.parameters)
    }
}

impl QueryContext<'_> {
    /// Return parameter names for one callable symbol.
    pub(crate) fn symbol_parameter_names(
        &self,
        symbol_id: dir::GlobalSymbolId,
    ) -> QueryResult<Option<Vec<String>>> {
        let module = self.module(symbol_id.module_id)?;
        let Some(parameters) = module.callable_parameters(symbol_id) else {
            return Ok(None);
        };
        let view = module.view();
        let names = parameters
            .iter()
            .map(|parameter_id| module.parameter_name(view.get(*parameter_id)))
            .collect::<QueryResult<Vec<_>>>()?
            .into_iter()
            .collect::<Option<Vec<_>>>();

        Ok(names)
    }
}
