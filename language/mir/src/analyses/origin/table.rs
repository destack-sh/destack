use crate::analyses::{
    Analysis, Dataflow, ForwardTransfer, FunctionCache, Loan, LoanTable, Mutation, ResolutionTable,
};
use crate::{
    AddressKind, Block, ControlTable, Function, Instruction, LocalNodeId, Path, Place, PlaceTable,
    Point, ReferenceKind, Tree, Value,
};

use super::context::OriginContext;
use super::state::OriginState;

/// Borrow origin across one MIR function.
#[derive(Debug)]
pub struct OriginTable {
    /// Origin at reachable block entries and exits.
    flow: Dataflow<OriginState>,
    /// Loans issued by borrowed references.
    loans: LoanTable,
}

impl OriginTable {
    /// Compute borrow origin for one function.
    pub(crate) fn compute(
        function: &Function,
        tree: &Tree,
        analyses: &mut FunctionCache,
        resolution: &ResolutionTable,
    ) -> Self {
        let control = analyses.control(function, tree);
        let places = analyses.place(function, tree);
        let mut builder = OriginBuilder {
            function,
            tree,
            places: &places,
            resolution,
            loans: LoanTable::new(),
        };

        builder.declare_loans();
        let flow = builder.solve(&control);
        builder.bind_parents(&flow);

        Self {
            flow,
            loans: builder.loans,
        }
    }

    /// Return origin at one reachable block entry.
    pub fn entry(&self, block: LocalNodeId<Block>) -> Option<&OriginState> {
        self.flow.entry(block)
    }

    /// Return loans issued in the function.
    pub fn loans(&self) -> &LoanTable {
        &self.loans
    }
}

impl Analysis for OriginTable {
    const INVALIDATED_BY: Mutation = Mutation::VALUE
        .union(Mutation::CONTROL)
        .union(Mutation::LAYOUT)
        .union(Mutation::DISPATCH)
        .union(Mutation::SYMBOL);
}

/// The per-function analyses one origin table is built from.
struct OriginBuilder<'a> {
    /// The function being analyzed.
    function: &'a Function,
    /// The MIR tree.
    tree: &'a Tree,
    /// Places derived by address values.
    places: &'a PlaceTable,
    /// Statically resolved call targets.
    resolution: &'a ResolutionTable,
    /// The loans declared so far.
    loans: LoanTable,
}

impl OriginBuilder<'_> {
    /// Return the transfer context over the loans declared so far.
    fn context(&self) -> OriginContext<'_> {
        OriginContext::new(
            self.function,
            self.tree,
            self.places,
            self.resolution,
            &self.loans,
        )
    }

    /// Solve origin flow to its fixed point.
    fn solve(&self, control: &ControlTable) -> Dataflow<OriginState> {
        let cx = self.context();
        let entry = cx.initial_state();
        Dataflow::forward(
            self.function,
            self.tree,
            control,
            entry,
            |transfer, mut state, _| {
                match transfer {
                    // transfer each instruction in the block
                    ForwardTransfer::Block(block) => cx.solve_block(block, &mut state),
                    // bind origin carried through the edge
                    ForwardTransfer::Edge { edge, target } => {
                        cx.bind_edge(edge, target, &mut state)
                    }
                }

                state
            },
        )
    }

    /// Declare every loan identity before origin flow.
    fn declare_loans(&mut self) {
        let entry = self
            .function
            .entry()
            .unwrap_or_else(|| unreachable!("defined function has no entry block"));

        // declare borrowed paths entering through function parameters
        for parameter in &self.function.parameters {
            for borrowed in self.tree.type_origin_paths(parameter.ty) {
                if borrowed.kind != ReferenceKind::Borrowed {
                    continue;
                }
                let loan = Loan::parameter(
                    parameter.value,
                    borrowed.path.clone(),
                    borrowed.access,
                    entry.into_any(),
                );
                self.loans.insert(parameter.value, borrowed.path, loan);
            }
        }

        // declare every loan a borrowing instruction issues
        for &block_id in self.function.blocks() {
            let block = self.tree.get(block_id);
            for &instruction_id in &block.instructions {
                if let Some((representation, loan)) = self.issued_loan(instruction_id) {
                    self.loans.insert(representation, Path::root(), loan);
                }
                for (representation, loan) in self.call_reborrow_loans(instruction_id) {
                    self.loans.insert(representation, Path::root(), loan);
                }
            }
        }

        self.loans.sort();
    }

    /// Return the loans one call result issues.
    ///
    /// The result issues one reborrow per argument whose region it covers.
    fn call_reborrow_loans(&self, instruction_id: LocalNodeId<Instruction>) -> Vec<(Value, Loan)> {
        let instruction = self.tree.get(instruction_id);
        let Instruction::Call { call, .. } = instruction else {
            return Vec::new();
        };
        let Some(destination) = instruction.destination() else {
            return Vec::new();
        };

        // reborrow through the resolved declaration's regions
        let target = instruction
            .call_direct_target()
            .or_else(|| self.resolution.target(Point::Instruction(instruction_id)));
        let arguments = self.tree.get_values(call.arguments);
        let cx = self.context();

        cx.call_reborrows(target, call.signature, arguments)
            .into_iter()
            .map(|(argument, access)| {
                let loan = Loan::new(
                    self.places.get(argument).clone(),
                    Some(argument),
                    access,
                    destination,
                    [],
                    instruction_id.into_any(),
                );

                (destination, loan)
            })
            .collect()
    }

    /// Return the loan one instruction issues: a borrow address, or a reference cast off an owning handle.
    fn issued_loan(&self, instruction_id: LocalNodeId<Instruction>) -> Option<(Value, Loan)> {
        let (representation, place, source, kind) = match self.tree.get(instruction_id) {
            Instruction::FieldAddr {
                destination,
                aggregate,
                kind,
                ..
            } => (
                *destination,
                self.places.get(*destination).clone(),
                Some(*aggregate),
                *kind,
            ),
            Instruction::ElementAddr {
                destination,
                base,
                kind,
                ..
            } => (
                *destination,
                self.places.get(*destination).clone(),
                Some(*base),
                *kind,
            ),
            Instruction::VariantPayloadAddr {
                destination,
                variant,
                kind,
                ..
            } => (
                *destination,
                self.places.get(*destination).clone(),
                Some(*variant),
                *kind,
            ),
            Instruction::SliceView {
                destination,
                source,
                ..
            } => (
                *destination,
                self.places.get(*destination).clone(),
                Some(*source),
                AddressKind::Borrow,
            ),
            Instruction::LocalAddr {
                destination,
                local,
                kind,
                ..
            } => (*destination, Place::local(*local), None, *kind),
            Instruction::GlobalAddr {
                destination,
                global,
                kind,
                ..
            } => (*destination, Place::global(*global), None, *kind),
            Instruction::Cast {
                destination,
                argument,
                ..
            } if matches!(
                self.function.reference_kind(*argument, self.tree),
                Some(ReferenceKind::Managed | ReferenceKind::Unique)
            ) =>
            {
                (
                    *destination,
                    self.places.get(*argument).clone(),
                    Some(*argument),
                    AddressKind::Borrow,
                )
            }
            _ => return None,
        };
        // a place step borrows nothing
        if kind == AddressKind::Projection {
            return None;
        }
        let ty = self.function.expect_value_type(representation);
        let value_type = self.tree.get(ty);
        if !value_type.is_borrowed_reference() {
            return None;
        }
        let Some(access) = value_type.reference_access() else {
            unreachable!("borrowed reference has no access");
        };
        let loan = Loan::new(
            place,
            source,
            access,
            representation,
            [],
            instruction_id.into_any(),
        );

        Some((representation, loan))
    }

    /// Bind each loan's parent loans from the solved flow.
    fn bind_parents(&mut self, flow: &Dataflow<OriginState>) {
        let cx = self.context();
        let mut parents = Vec::new();
        for &block_id in self.function.blocks() {
            let Some(mut state) = flow.entry(block_id).cloned() else {
                continue;
            };
            let block = self.tree.get(block_id);

            // replay the block, reading each issued loan's origins at its issue
            for &instruction_id in &block.instructions {
                let instruction = self.tree.get(instruction_id);
                if let Some(destination) = instruction.destination() {
                    for loan in cx.loans.roots_of(destination) {
                        if let Some(parent) = cx.loans.get(loan).source() {
                            parents.push((loan, state.value(parent).loans().to_vec()));
                        }
                    }
                }
                state.advance(&cx, instruction_id);
            }
        }

        for (loan, parents) in parents {
            self.loans.set_parents(loan, parents);
        }
    }
}
