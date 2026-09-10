use crate::analyses::{LoanTable, ResolutionTable};
use crate::{
    Access, Block, BlockTarget, Edge, Function, Instruction, Lifetime, LifetimeParameter,
    LifetimeTerm, LocalNodeId, Path, PlaceTable, Point, ReferenceKind, Successor, Terminator, Tree,
    Type, TypeId, Value,
};

use super::region::{Origin, Region};
use super::state::OriginState;

/// The per-function analyses one origin transfer reads.
#[derive(Debug, Clone, Copy)]
pub struct OriginContext<'a> {
    /// The function being transferred.
    pub function: &'a Function,
    /// The MIR tree.
    pub tree: &'a Tree,
    /// Places derived by address values.
    pub places: &'a PlaceTable,
    /// Statically resolved call targets.
    pub resolution: &'a ResolutionTable,
    /// The loans of the function.
    pub loans: &'a LoanTable,
}

impl<'a> OriginContext<'a> {
    /// Create the context for one function.
    pub fn new(
        function: &'a Function,
        tree: &'a Tree,
        places: &'a PlaceTable,
        resolution: &'a ResolutionTable,
        loans: &'a LoanTable,
    ) -> Self {
        Self {
            function,
            tree,
            places,
            resolution,
            loans,
        }
    }

    /// Return initial origin for the function parameters.
    pub(super) fn initial_state(&self) -> OriginState {
        let mut state = OriginState::new();

        // seed parameter origin from declared types
        for parameter in &self.function.parameters {
            for (path, mut origin) in self.parameter_bindings(parameter.ty) {
                if let Some(loan) = self.loans.carried(parameter.value, &path) {
                    origin = origin.with_loan(loan);
                }
                state.insert_at(parameter.value, path, origin);
            }
        }

        state
    }

    /// Return origin implied by one parameter type.
    pub(super) fn parameter_bindings(&self, ty: TypeId) -> Vec<(Path, Origin)> {
        // derive each reference path from its declared lifetime
        self.tree
            .type_origin_paths(ty)
            .into_iter()
            .map(|borrowed| {
                let origin = Origin::from_path(&borrowed);
                (borrowed.path, origin)
            })
            .collect()
    }

    /// Transfer one block from its entry state.
    pub(super) fn solve_block(&self, block_id: LocalNodeId<Block>, state: &mut OriginState) {
        let block = self.tree.get(block_id);
        for &instruction_id in &block.instructions {
            state.advance(self, instruction_id);
        }
    }

    /// Bind predecessor values to successor parameters.
    pub(super) fn bind_edge(&self, edge: Edge, target: &BlockTarget, state: &mut OriginState) {
        let predecessor = self.tree.get(edge.source);
        let terminator = self.tree.get(predecessor.terminator);
        let arguments = target.arguments(self.tree);
        let Some(parameters) = terminator.target_parameters(self.tree, edge.successor, target)
        else {
            return;
        };

        // bind edge arguments to successor parameters
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            state.bind(self, argument, parameter.value);
        }

        // bind one normal invoke result to its successor parameter
        let Some(bindings) = self.invoke_result(edge.source, edge.successor, terminator, state)
        else {
            return;
        };
        let Some(parameter) = self
            .tree
            .get(target.block)
            .parameters
            .first()
            .map(|parameter| parameter.value)
        else {
            return;
        };

        state.insert_bindings(parameter, bindings);
    }

    /// Return origin carried by one normal invoke result.
    fn invoke_result(
        &self,
        block: LocalNodeId<Block>,
        successor: Successor,
        terminator: &Terminator,
        state: &OriginState,
    ) -> Option<Vec<(Path, Origin)>> {
        let Terminator::Invoke { call, .. } = terminator else {
            return None;
        };
        if successor != Successor::InvokeNormal {
            return None;
        }

        let target = terminator
            .call_direct_target()
            .or_else(|| self.resolution.target(Point::Terminator(block)));
        let arguments = self.tree.get_values(call.arguments);

        Some(self.call_result(state, target, call.signature, arguments))
    }

    /// Bind origin carried by one instruction call result.
    pub(super) fn define_call_result(
        &self,
        state: &mut OriginState,
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
    ) {
        let Instruction::Call { call, .. } = instruction else {
            return;
        };
        let Some(destination) = instruction.destination() else {
            return;
        };
        let ty = self.function.expect_value_type(destination);
        if self.tree.type_lifetime(ty).is_none() && !self.tree.type_contains_borrowed_refs(ty) {
            return;
        }

        let target = instruction
            .call_direct_target()
            .or_else(|| self.resolution.target(Point::Instruction(instruction_id)));
        let arguments = self.tree.get_values(call.arguments);
        let bindings = self.call_result(state, target, call.signature, arguments);
        state.insert_bindings(destination, bindings);
    }

    /// Return the borrowed arguments one call result reborrows, with the access each takes.
    pub(super) fn call_reborrows(
        &self,
        target: Option<LocalNodeId<Function>>,
        signature: TypeId,
        arguments: &[Value],
    ) -> Vec<(Value, Access)> {
        let Type::FunctionSignature {
            lifetimes: signature_lifetimes,
            parameters: signature_parameters,
            result: signature_result,
        } = self.tree.get(signature)
        else {
            unreachable!("call has no function signature")
        };

        // use the resolved declaration as the direct call lifetime environment
        let target = target.map(|target| self.tree.get(target));
        let result = target
            .map(|function| function.return_type)
            .unwrap_or(*signature_result);
        let lifetimes = target
            .map(|function| function.lifetimes.as_slice())
            .unwrap_or(signature_lifetimes);
        let parameter_types = target
            .map(|function| {
                function
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                signature_parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .collect()
            });

        // collect the regions the result names
        let mut result_lifetimes = self
            .tree
            .type_borrowed_paths(result)
            .into_iter()
            .map(|borrowed| borrowed.lifetime)
            .collect::<Vec<_>>();
        result_lifetimes.extend(self.tree.type_lifetime(result));
        result_lifetimes.retain(|lifetime| !lifetime.is_empty());

        // a result over no region reborrows nothing
        if result_lifetimes.is_empty() {
            return Vec::new();
        }

        // keep each borrowed argument whose region the result covers
        let mut reborrows = Vec::new();
        for (index, parameter) in parameter_types.iter().enumerate() {
            // keep the borrowed parameters this call passes an argument for
            let Some(argument) = arguments.get(index).copied() else {
                continue;
            };
            let parameter_type = self.tree.get(*parameter);
            if parameter_type.reference_kind() != Some(ReferenceKind::Borrowed) {
                continue;
            }
            let Some(access) = parameter_type.reference_access() else {
                continue;
            };

            // reborrow when the result covers the region the parameter binds
            let is_reborrowed = self
                .parameter_bindings(*parameter)
                .iter()
                .any(|(_, callee)| {
                    result_lifetimes
                        .iter()
                        .any(|lifetime| callee.is_covered_by(lifetime, lifetimes))
                });
            if is_reborrowed {
                reborrows.push((argument, access));
            }
        }

        reborrows
    }

    /// Return origin carried by one call result.
    pub fn call_result(
        &self,
        state: &OriginState,
        target: Option<LocalNodeId<Function>>,
        signature: TypeId,
        arguments: &[Value],
    ) -> Vec<(Path, Origin)> {
        let Type::FunctionSignature {
            lifetimes: signature_lifetimes,
            parameters: signature_parameters,
            result: signature_result,
        } = self.tree.get(signature)
        else {
            unreachable!("call has no function signature")
        };

        // use the resolved declaration as the direct call lifetime environment
        let target = target.map(|target| self.tree.get(target));
        let result = target
            .map(|function| function.return_type)
            .unwrap_or(*signature_result);
        let lifetimes = target
            .map(|function| function.lifetimes.as_slice())
            .unwrap_or(signature_lifetimes);
        let parameter_types = target
            .map(|function| {
                function
                    .parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| {
                signature_parameters
                    .iter()
                    .map(|parameter| parameter.ty)
                    .collect()
            });
        let paths = self.tree.type_borrowed_paths(result);
        if !paths.is_empty() {
            return paths
                .into_iter()
                .map(|borrowed_path| {
                    let origin = self.map_lifetime(
                        state,
                        &borrowed_path.lifetime,
                        lifetimes,
                        &parameter_types,
                        arguments,
                    );

                    (borrowed_path.path, origin)
                })
                .collect();
        }

        let lifetime = self
            .tree
            .type_lifetime(result)
            .filter(|lifetime| !lifetime.is_empty());
        let Some(lifetime) = lifetime else {
            return Vec::new();
        };
        let origin = self.map_lifetime(state, &lifetime, lifetimes, &parameter_types, arguments);

        vec![(Path::root(), origin)]
    }

    /// Map callee lifetime regions to caller origin.
    pub fn map_lifetime(
        &self,
        state: &OriginState,
        lifetime: &Lifetime,
        lifetimes: &[LifetimeParameter],
        parameters: &[TypeId],
        arguments: &[Value],
    ) -> Origin {
        let mut origin = Origin::none();

        // map the storage extents directly
        for term in &lifetime.terms {
            match term {
                LifetimeTerm::Static => origin = origin.merge(&Origin::one(Region::Static)),
                LifetimeTerm::Managed => origin = origin.merge(&Origin::one(Region::Managed)),
                LifetimeTerm::Frame | LifetimeTerm::Slot(_) | LifetimeTerm::Parameter(_) => {}
            }
        }

        // map lifetime slots through actual argument paths
        for (index, parameter) in parameters.iter().enumerate() {
            let Some(argument) = arguments.get(index) else {
                continue;
            };
            for (path, callee) in self.parameter_bindings(*parameter) {
                if !callee.is_covered_by(lifetime, lifetimes) {
                    continue;
                }

                let argument = state.value_path(*argument, &path);
                origin = origin.merge(&argument);
            }

            // merge pointee borrows when the result covers the pointee lifetime
            let Type::Reference {
                kind: ReferenceKind::Borrowed,
                pointee,
                ..
            } = self.tree.get(*parameter)
            else {
                continue;
            };
            let Some(nested) = self.tree.type_lifetime(*pointee) else {
                continue;
            };
            if !Origin::from_lifetime(&nested).is_covered_by(lifetime, lifetimes) {
                continue;
            }
            let place = self.places.get(*argument);
            origin = origin.merge(&state.place(self, place));
        }

        origin
    }
}
