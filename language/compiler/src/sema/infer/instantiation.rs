use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_dir::TypeFold;

use crate::CompilerResult;
use crate::sema::{CheckState, FlowSite, TypeSubstitution};

impl CheckState<'_> {
    /// Infer explicit generic arguments applied to one expression.
    pub(in crate::sema) fn infer_instantiation(
        &mut self,
        node: dir::GlobalNodeId<dir::Expression>,
        left: FlowSite,
        target: dir::GlobalTypeId,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> CompilerResult<dir::GlobalTypeId> {
        let module = node.module_id;
        let node = node.into_any();
        let site = self.visit_site(node)?;

        // read the explicitly written arguments
        self.walk_body_generic_arguments(module, arguments)?;
        let mut written = SmallVec::<[_; 4]>::new();
        for argument in arguments {
            written.push(self.require_node_type(argument.into_global_any(module))?);
        }

        // keep an application symbolic while its operand type is being declared
        let Some((ty, substitution)) = self.instantiate_type(site.origin(), target, &written)?
        else {
            let arguments = self.intern_type_ids(&written)?;
            let ty =
                self.intern_operation(dir::TypeOperation::Instantiation(dir::InstantiationType {
                    target,
                    arguments,
                }))?;
            return Ok(ty);
        };

        // record the selected declaration or callable after applying its arguments
        match self.ty(ty)? {
            dir::Type::Error => self.commit_decision(node, dir::Decision::Rejected)?,
            dir::Type::Reference(reference) => {
                self.commit_name(node, dir::NameResolution::new(reference.symbol))?;
            }
            dir::Type::Function(_)
            | dir::Type::FunctionPointer(_)
            | dir::Type::FunctionSignature(_) => {
                // preserve a selected function's receiver and dispatch
                let mut target = match self.decision(left.node) {
                    Some(dir::Decision::Function(dir::OperationResolution::One(function))) => {
                        function.target.clone()
                    }
                    _ => dir::CallableTarget::Expression {
                        generic_arguments: Vec::new(),
                    },
                };
                let arguments = match &mut target {
                    dir::CallableTarget::Constructor(construction) => {
                        construction
                            .map_types(&mut |ty| self.substitute_type(ty, &substitution))?;
                        match &mut construction.target {
                            dir::ConstructTarget::Class { arguments, .. } => arguments,
                            dir::ConstructTarget::Newtype { key, .. } => &mut key.arguments,
                        }
                    }
                    dir::CallableTarget::Symbol { function, .. } => &mut function.key.arguments,
                    dir::CallableTarget::Expression { generic_arguments }
                    | dir::CallableTarget::Dynamic {
                        generic_arguments, ..
                    } => generic_arguments,
                };
                let substitution = TypeSubstitution::default()
                    .with_carried(arguments)?
                    .with_carried(&substitution.bindings)?;
                *arguments = substitution.bindings.into_vec();

                // keep the class key at the parameters of the class
                if let dir::CallableTarget::Constructor(construction) = &mut target
                    && let dir::ConstructTarget::Class { key, arguments, .. } =
                        &mut construction.target
                {
                    key.arguments = self.instance_key_bindings(key.symbol, arguments)?;
                }
                let value = dir::FunctionValue {
                    target,
                    callable_type: ty,
                };
                self.commit_decision(
                    node,
                    dir::Decision::Function(dir::OperationResolution::One(value)),
                )?;
            }
            _ => {}
        }
        Ok(ty)
    }
}
