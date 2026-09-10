use destack_core::FxIndexSet;
use smallvec::SmallVec;

use crate::{
    Access, AddressKind, Analysis, Block, BlockTarget, BorrowedPath, Call, ControlTable,
    DataflowTable, Edge, ForwardTransfer, Function, Instruction, Intrinsic, Lattice, Lifetime,
    LifetimeParameter, LifetimeSlot, LifetimeTerm, Loan, LoanId, LoanTable, LocalId, LocalNodeId,
    Mutation, Path, Place, PlaceOrigin, PlaceTable, Projection, ReferenceKind, Storage, Successor,
    Terminator, Tree, Type, TypeId, Value,
};

/// Borrow origin across one MIR function.
#[derive(Debug)]
pub struct OriginTable {
    /// Origin at reachable block entries and exits.
    flow: DataflowTable<OriginState>,
    /// Loans issued by borrowed references.
    loans: LoanTable,
}

/// Origin that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Origin {
    /// The lifetime regions.
    regions: SmallVec<[Region; 2]>,
    /// Loans kept active by this value.
    loans: SmallVec<[LoanId; 2]>,
}

/// Origin at one MIR program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OriginState {
    /// Origin carried by SSA values.
    bindings: Vec<ValueBinding>,
    /// Stored place bindings.
    places: Vec<PlaceBinding>,
    /// Loans escaped through aliasable storage.
    escaped_loans: Vec<LoanId>,
}

/// The per-function analyses one origin transfer reads.
#[derive(Debug, Clone, Copy)]
pub struct OriginContext<'a> {
    /// The function being transferred.
    pub function: &'a Function,
    /// The MIR tree.
    pub tree: &'a Tree,
    /// Places derived by address values.
    pub places: &'a PlaceTable,
    /// The loans of the function.
    pub loans: &'a LoanTable,
}

/// Origin for one value path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueBinding {
    /// SSA value carrying the borrowed path.
    pub value: Value,
    /// Path inside the value.
    pub path: Path,
    /// Origin that keeps the path live.
    pub origin: Origin,
}

/// Origin stored in one place path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceBinding {
    /// The exact place carrying the borrowed value.
    pub place: Place,
    /// Origin that keeps the path live.
    pub origin: Origin,
}

/// Lifetime region that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Region {
    /// Global or static storage.
    Static,
    /// Explicit lifetime slot.
    Lifetime(LifetimeSlot),
    /// Managed storage, alive while reachable.
    Managed,
    /// Storage in the current function frame.
    Frame,
}

/// The per-function analyses one origin table is built from.
struct OriginBuilder<'a> {
    /// The function being analyzed.
    function: &'a Function,
    /// The MIR tree.
    tree: &'a Tree,
    /// Places derived by address values.
    places: &'a PlaceTable,
    /// The loans declared so far.
    loans: LoanTable,
}

/// A borrowed value that a call may write into one addressed place.
struct CallWrite {
    /// The destination storage when its address is known.
    place: Option<Place>,
    /// The possible incoming borrow origins.
    origin: Origin,
}

impl<'a> OriginContext<'a> {
    /// Create the context for one function.
    pub fn new(
        function: &'a Function,
        tree: &'a Tree,
        places: &'a PlaceTable,
        loans: &'a LoanTable,
    ) -> Self {
        Self {
            function,
            tree,
            places,
            loans,
        }
    }

    /// Return initial origin for the function parameters.
    fn initial_state(&self) -> OriginState {
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
    fn parameter_bindings(&self, ty: TypeId) -> Vec<(Path, Origin)> {
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
    fn solve_block(&self, block_id: LocalNodeId<Block>, state: &mut OriginState) {
        let block = self.tree.get(block_id);
        for &instruction_id in &block.instructions {
            state.advance(self, instruction_id);
        }
    }

    /// Bind predecessor values to successor parameters.
    fn bind_edge(&self, edge: Edge, target: &BlockTarget, state: &mut OriginState) {
        let predecessor = self.tree.get(edge.source);
        let terminator = self.tree.get(predecessor.terminator);
        let arguments = target.arguments(self.tree);
        let Some(parameters) = terminator.target_parameters(self.tree, edge.successor, target)
        else {
            return;
        };

        // retain possible call writes on both normal and unwind edges
        if let Terminator::Invoke { call, .. } = terminator {
            state.transfer_call(self, call);
        }

        // read argument and result origins before replacing any successor parameter
        let bindings = parameters
            .iter()
            .zip(arguments)
            .map(|(parameter, &argument)| (parameter.value, state.value_bindings(self, argument)))
            .collect::<Vec<_>>();
        let result = self.invoke_result(edge.successor, terminator, state);

        // replace all successor parameters from the original edge state
        for (parameter, bindings) in bindings {
            state.insert_bindings(parameter, bindings);
        }
        if let Some(bindings) = result {
            let parameter = self.tree.get(target.block).parameters[0].value;
            state.insert_bindings(parameter, bindings);
        }

        // replace index values simultaneously inside each dynamic path
        for binding in &mut state.bindings {
            binding.path.map_values(|value| {
                arguments
                    .iter()
                    .position(|&argument| argument == value)
                    .map_or(value, |index| parameters[index].value)
            });
        }
    }

    /// Return origin carried by one normal invoke result.
    fn invoke_result(
        &self,
        successor: Successor,
        terminator: &Terminator,
        state: &OriginState,
    ) -> Option<Vec<(Path, Origin)>> {
        let Terminator::Invoke { call, .. } = terminator else {
            return None;
        };
        if successor != Successor::InvokeNormal
            || terminator.target_result_count(self.tree, successor) == 0
        {
            return None;
        }

        let arguments = self.tree.get_values(call.arguments);

        Some(self.call_result(state, call.signature, arguments))
    }

    /// Bind origin carried by one instruction call result.
    fn define_call_result(&self, state: &mut OriginState, instruction: &Instruction) {
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

        let arguments = self.tree.get_values(call.arguments);
        let bindings = self.call_result(state, call.signature, arguments);
        state.insert_bindings(destination, bindings);
    }

    /// Return the borrowed arguments one call result reborrows, with the access each takes.
    fn call_reborrows(&self, signature: TypeId, arguments: &[Value]) -> Vec<(Value, Access)> {
        let Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        } = self.tree.get(signature)
        else {
            unreachable!("call has no function signature")
        };

        // read the parameter types recorded on the call
        let result = *result;
        let parameter_types = parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();

        // collect the regions the result names
        let mut result_lifetimes = self
            .tree
            .type_borrowed_paths(result)
            .into_iter()
            .map(|borrowed| borrowed.lifetime)
            .collect::<Vec<_>>();
        result_lifetimes.extend(self.tree.type_lifetime(result));
        result_lifetimes.retain(|lifetime| !lifetime.is_empty());

        // skip results without lifetime regions
        if result_lifetimes.is_empty() {
            return Vec::new();
        }

        // keep each borrowed argument whose region the result covers
        let mut reborrows = Vec::new();
        for (parameter, &argument) in parameter_types.iter().zip(arguments) {
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
        signature: TypeId,
        arguments: &[Value],
    ) -> Vec<(Path, Origin)> {
        let Type::FunctionSignature {
            lifetimes,
            parameters,
            result,
        } = self.tree.get(signature)
        else {
            unreachable!("call has no function signature")
        };

        // read the parameter types recorded on the call
        let result = *result;
        let parameter_types = parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();
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
        for (parameter, argument) in parameters.iter().zip(arguments) {
            for (path, callee) in self.parameter_bindings(*parameter) {
                if !callee.is_covered_by(lifetime, lifetimes) {
                    continue;
                }

                let argument = state.value_path(*argument, &path);
                origin = origin.merge(&argument);
            }

            // map stored borrow paths through the actual argument's pointee type
            let parameter = self.tree.storage_type(*parameter);
            let Type::Reference { pointee, .. } = self.tree.get(parameter) else {
                continue;
            };
            let actual = self
                .tree
                .storage_type(self.function.expect_value_type(*argument));
            let Type::Reference {
                pointee: actual, ..
            } = self.tree.get(actual)
            else {
                unreachable!("reference parameter has a nonreference argument")
            };
            let actual_paths = self.tree.type_origin_paths(*actual);
            for borrowed in self.tree.type_origin_paths(*pointee) {
                let callee = Origin::from_path(&borrowed);
                if !callee.is_covered_by(lifetime, lifetimes) {
                    continue;
                }
                let place = self.places.get(*argument).clone().with_path(&borrowed.path);
                let stored = match state.get_place(&place) {
                    Some(origin) => origin.clone(),
                    None => {
                        let actual = actual_paths
                            .iter()
                            .find(|actual| actual.path == borrowed.path)
                            .unwrap_or_else(|| {
                                unreachable!("call argument has no corresponding borrow path")
                            });

                        Origin::from_path(actual)
                    }
                };
                origin = origin.merge(&stored);
            }
        }

        origin
    }
}

impl OriginTable {
    /// Analyse borrow origins for one function.
    pub fn analyse(
        function: &Function,
        control: &ControlTable,
        places: &PlaceTable,
        tree: &Tree,
    ) -> Self {
        let mut builder = OriginBuilder {
            function,
            tree,
            places,
            loans: LoanTable::new(),
        };

        builder.declare_loans();
        let flow = builder.solve(control);
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
        .union(Mutation::LAYOUT);
}

impl OriginBuilder<'_> {
    /// Return the transfer context over the loans declared so far.
    fn context(&self) -> OriginContext<'_> {
        OriginContext::new(self.function, self.tree, self.places, &self.loans)
    }

    /// Solve origin flow to its fixed point.
    fn solve(&self, control: &ControlTable) -> DataflowTable<OriginState> {
        let cx = self.context();
        let entry = cx.initial_state();
        DataflowTable::forward(
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

    /// Return one result reborrow for each argument covered by the result's region.
    fn call_reborrow_loans(&self, instruction_id: LocalNodeId<Instruction>) -> Vec<(Value, Loan)> {
        let instruction = self.tree.get(instruction_id);
        let Instruction::Call { call, .. } = instruction else {
            return Vec::new();
        };
        let Some(destination) = instruction.destination() else {
            return Vec::new();
        };

        // reborrow through the call signature
        let arguments = self.tree.get_values(call.arguments);
        let cx = self.context();

        cx.call_reborrows(call.signature, arguments)
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

    /// Return the loan issued by a borrow address or a cast from an owning reference.
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
    fn bind_parents(&mut self, flow: &DataflowTable<OriginState>) {
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

impl Region {
    /// Return this region as a MIR lifetime.
    fn lifetime(&self) -> Lifetime {
        match self {
            Self::Static => Lifetime::static_storage(),
            Self::Lifetime(slot) => Lifetime::slot(slot.0),
            Self::Managed => Lifetime::managed(),
            Self::Frame => Lifetime::frame(),
        }
    }

    /// Return whether this region is local to the current function.
    fn is_local(&self) -> bool {
        match self {
            Self::Frame => true,
            Self::Static | Self::Lifetime(_) | Self::Managed => false,
        }
    }

    /// Return whether this region is covered by a required lifetime.
    fn is_covered_by(&self, required: &Lifetime, parameters: &[LifetimeParameter]) -> bool {
        required.accepts(&self.lifetime(), parameters)
    }
}

impl Origin {
    /// Create empty origin.
    fn none() -> Self {
        Self::default()
    }

    /// Create origin from one origin.
    fn one(origin: Region) -> Self {
        Self::new([origin])
    }

    /// Create origin from many regions.
    fn new(regions: impl IntoIterator<Item = Region>) -> Self {
        let mut unique = SmallVec::new();

        // retain insertion order while removing duplicates
        for region in regions {
            if !unique.contains(&region) {
                unique.push(region);
            }
        }

        Self {
            regions: unique,
            loans: SmallVec::new(),
        }
    }

    /// Create origin from one MIR lifetime.
    fn from_lifetime(lifetime: &Lifetime) -> Self {
        Self::new(lifetime.terms.iter().map(|term| match term {
            LifetimeTerm::Static => Region::Static,
            LifetimeTerm::Frame => Region::Frame,
            LifetimeTerm::Managed => Region::Managed,
            LifetimeTerm::Slot(slot) => Region::Lifetime(*slot),
            LifetimeTerm::Parameter(index) => {
                unreachable!("region parameter {index} of a type declaration reached a body")
            }
        }))
    }

    /// Derive a reference path's origin from its lifetime or implicit managed storage.
    fn from_path(borrowed: &BorrowedPath) -> Self {
        match borrowed.kind {
            ReferenceKind::Managed if borrowed.lifetime.is_empty() => Self::one(Region::Managed),
            _ => Self::from_lifetime(&borrowed.lifetime),
        }
    }

    /// Create origin for one managed reference type.
    fn from_managed(ty: &Type) -> Self {
        if ty
            .reference_storage()
            .and_then(Storage::heap_space)
            .is_none()
        {
            unreachable!("managed reference has non-heap storage")
        }
        ty.reference_lifetime()
            .filter(|lifetime| !lifetime.is_empty())
            .map(Self::from_lifetime)
            .unwrap_or_else(|| Self::one(Region::Managed))
    }

    /// Create origin for one reference-like type.
    fn from_reference(ty: &Type) -> Self {
        match ty.reference_kind() {
            Some(ReferenceKind::Managed) => Self::from_managed(ty),
            Some(ReferenceKind::Borrowed) => ty
                .reference_lifetime()
                .filter(|lifetime| !lifetime.is_empty())
                .map(Self::from_lifetime)
                .unwrap_or_default(),
            Some(ReferenceKind::Unique) => Self::one(Region::Frame),
            None => Self::none(),
        }
    }

    /// Return this origin carrying additional loans.
    fn with_loans(mut self, loans: &[LoanId]) -> Self {
        for &loan in loans {
            if !self.loans.contains(&loan) {
                self.loans.push(loan);
            }
        }
        self
    }

    /// Return whether this origin is empty.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && self.loans.is_empty()
    }

    /// Return whether any region may escape the function.
    pub fn has_escaping_region(&self) -> bool {
        self.regions.iter().any(|region| !region.is_local())
    }

    /// Return whether any region is local to this function.
    pub fn has_local_region(&self) -> bool {
        self.regions.iter().any(Region::is_local)
    }

    /// Return whether this origin has a proven lifetime region.
    pub fn has_region(&self) -> bool {
        !self.regions.is_empty()
    }

    /// Return whether this origin satisfies a required MIR lifetime.
    pub fn is_covered_by(&self, required: &Lifetime, parameters: &[LifetimeParameter]) -> bool {
        self.regions
            .iter()
            .all(|region| region.is_covered_by(required, parameters))
    }

    /// Return whether this origin lives at least as long as `other`.
    pub fn outlives(&self, other: &Origin, parameters: &[LifetimeParameter]) -> bool {
        other.regions.is_empty()
            || (!self.regions.is_empty()
                && self.regions.iter().all(|region| {
                    other
                        .regions
                        .iter()
                        .all(|shorter| shorter.lifetime().accepts(&region.lifetime(), parameters))
                }))
    }

    /// Merge two origin sets.
    fn merge(&self, other: &Self) -> Self {
        Self::new(self.regions.iter().chain(&other.regions).cloned())
            .with_loans(&self.loans)
            .with_loans(&other.loans)
    }

    /// Add one issuing loan.
    fn with_loan(mut self, loan: LoanId) -> Self {
        if !self.loans.contains(&loan) {
            self.loans.push(loan);
        }

        self
    }

    /// Return loans retained by this origin.
    pub fn loans(&self) -> &[LoanId] {
        &self.loans
    }
}

impl OriginState {
    /// Create empty origin flow.
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterate all value bindings.
    pub fn bindings(&self) -> impl Iterator<Item = &ValueBinding> {
        self.bindings.iter()
    }

    /// Iterate all place bindings.
    pub fn places(&self) -> impl Iterator<Item = &PlaceBinding> {
        self.places.iter()
    }

    /// Bind one value to borrow origin.
    fn insert(&mut self, value: Value, origin: Origin) {
        self.insert_at(value, Path::root(), origin);
    }

    /// Bind one value path to borrow origin.
    fn insert_at(&mut self, value: Value, path: Path, origin: Origin) {
        if origin.is_empty() {
            return;
        }

        // replace existing bindings instead of growing duplicate entries
        if let Some(current) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.value == value && binding.path == path)
        {
            current.origin = origin;
            return;
        }

        self.bindings.push(ValueBinding {
            value,
            path,
            origin,
        });
    }

    /// Bind all origin paths for one value.
    fn insert_bindings(&mut self, value: Value, bindings: Vec<(Path, Origin)>) {
        self.bindings.retain(|binding| binding.value != value);

        // insert each structural path once
        for (path, origin) in bindings {
            self.insert_at(value, path, origin);
        }
    }

    /// Replace borrow origin below one value path.
    fn replace_paths(&mut self, value: Value, base: Path, bindings: Vec<(Path, Origin)>) {
        self.bindings
            .retain(|binding| binding.value != value || !base.contains(&binding.path));

        // insert each origin below the replacement path
        for (path, origin) in bindings {
            let path = base.clone().with_path(&path);
            self.insert_at(value, path, origin);
        }
    }

    /// Merge borrow origin below one value path.
    fn merge_paths(&mut self, value: Value, base: Path, bindings: Vec<(Path, Origin)>) {
        // merge each origin below the shared path
        for (path, origin) in bindings {
            let path = base.clone().with_path(&path);
            self.merge_origins_at(value, &path, &origin);
        }
    }

    /// Return borrow origin for one value path.
    fn get_at(&self, value: Value, path: &Path) -> Option<&Origin> {
        self.bindings.iter().find_map(|binding| {
            (binding.value == value && &binding.path == path).then_some(&binding.origin)
        })
    }

    /// Merge origin at one value path.
    fn merge_origins_at(&mut self, value: Value, path: &Path, origin: &Origin) {
        // accumulate origin from repeated paths
        let origin = self
            .get_at(value, path)
            .map(|current| current.merge(origin))
            .unwrap_or_else(|| origin.clone());

        self.insert_at(value, path.clone(), origin);
    }

    /// Bind one place path to borrow origin.
    fn insert_place(&mut self, place: Place, origin: Origin) {
        if origin.is_empty() {
            return;
        }

        // replace the previous value stored in this exact place path
        if let Some(current) = self
            .places
            .iter_mut()
            .find(|binding| binding.place == place)
        {
            current.origin = origin;
            return;
        }

        self.places.push(PlaceBinding { place, origin });
    }

    /// Return borrow origin stored in one exact place.
    fn get_place(&self, place: &Place) -> Option<&Origin> {
        self.places
            .iter()
            .find_map(|binding| (&binding.place == place).then_some(&binding.origin))
    }

    /// Replace every borrowed path stored in one place.
    fn insert_place_bindings(&mut self, place: Place, bindings: Vec<(Path, Origin)>) {
        self.places
            .retain(|binding| !place.contains(&binding.place));

        // retain each exact stored path
        for (path, origin) in bindings {
            self.insert_place(place.clone().with_path(&path), origin);
        }
    }

    /// Mark borrow loans carried into aliasable storage as escaped.
    fn escape_loans(&mut self, bindings: &[(Path, Origin)]) {
        for (_, origin) in bindings {
            Self::merge_loans(&mut self.escaped_loans, origin.loans());
        }
    }

    /// Release the loans of the storage one assignment overwrites, as rustc's `loan_killed_at`.
    fn kill(&mut self, cx: &OriginContext<'_>, written: &Place) {
        let killed = cx
            .loans
            .iter()
            .filter(|(_, loan)| {
                loan.place()
                    .is_some_and(|place| place.contains(written) || written.contains(place))
            })
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        if killed.is_empty() {
            return;
        }
        for origin in self.origins_mut() {
            origin.loans.retain(|loan| !killed.contains(loan));
        }
    }

    /// Iterate every origin carried by a value or stored in a place.
    fn origins_mut(&mut self) -> impl Iterator<Item = &mut Origin> {
        let bindings = self.bindings.iter_mut().map(|binding| &mut binding.origin);
        let places = self.places.iter_mut().map(|binding| &mut binding.origin);

        bindings.chain(places)
    }

    /// Return active loan identities retained by live values and places.
    pub fn active_loans(
        &self,
        mut is_value_live: impl FnMut(Value) -> bool,
        mut is_place_live: impl FnMut(&Place) -> bool,
    ) -> Vec<LoanId> {
        let mut loans = self.escaped_loans.clone();

        // collect loans carried by live SSA values
        for binding in self.bindings() {
            if is_value_live(binding.value) {
                Self::merge_loans(&mut loans, binding.origin.loans());
            }
        }

        // collect loans carried by live memory places
        for binding in self.places() {
            if is_place_live(&binding.place) {
                Self::merge_loans(&mut loans, binding.origin.loans());
            }
        }

        loans
    }

    /// Return loans retained through aliasable storage.
    pub fn escaped_loans(&self) -> &[LoanId] {
        &self.escaped_loans
    }

    /// Return origin carried by one value.
    pub fn value(&self, value: Value) -> Origin {
        self.value_path(value, &Path::root())
    }

    /// Return origin carried by one value path.
    pub fn value_path(&self, value: Value, path: &Path) -> Origin {
        if let Some(origin) = self.get_at(value, path) {
            return origin.clone();
        }
        if !path.is_root() {
            return Origin::none();
        }

        let mut origin = Origin::none();

        // merge sparse child paths for whole-value checks
        for binding in &self.bindings {
            if binding.value == value {
                origin = origin.merge(&binding.origin);
            }
        }

        origin
    }

    /// Return origin stored in one place.
    pub fn place(&self, cx: &OriginContext<'_>, place: &Place) -> Origin {
        let mut origin = self
            .get_place(place)
            .cloned()
            .unwrap_or_else(|| self.storage_origin(cx, place));

        // merge sparse child paths for whole-place checks
        for binding in &self.places {
            if &binding.place != place && place.contains(&binding.place) {
                origin = origin.merge(&binding.origin);
            }
        }

        origin
    }

    /// Return origin of one place's storage itself.
    fn storage_origin(&self, cx: &OriginContext<'_>, place: &Place) -> Origin {
        match (place.origin, place.path.first()) {
            (PlaceOrigin::Local(local), Some(Projection::Deref)) => self
                .get_place(&Place::local(local))
                .cloned()
                .unwrap_or_else(|| Self::local_reference_origin(cx, local)),
            (PlaceOrigin::Local(_), _) => Origin::one(Region::Frame),
            (PlaceOrigin::Global(_), _) => Origin::one(Region::Static),
            (PlaceOrigin::Value(value), _) => self.storage(cx, value, &place.path),
        }
    }

    /// Return the origin one reference local's type gives its storage before any store.
    fn local_reference_origin(cx: &OriginContext<'_>, local: LocalId) -> Origin {
        let ty = cx.tree.get(local).ty;
        let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));
        match ty.reference_kind() {
            Some(ReferenceKind::Managed | ReferenceKind::Borrowed) => Origin::from_reference(ty),
            Some(ReferenceKind::Unique) => Origin::one(Region::Frame),
            None => Origin::none(),
        }
    }

    /// Return origin paths carried by one value.
    pub fn value_bindings(&self, cx: &OriginContext<'_>, value: Value) -> Vec<(Path, Origin)> {
        self.value_paths(cx, value)
            .into_iter()
            .map(|(path, origin, _)| (path, origin))
            .collect()
    }

    /// Return the origin at each borrowed path of a value.
    pub fn value_borrows(&self, cx: &OriginContext<'_>, value: Value) -> Vec<(Path, Origin)> {
        self.value_paths(cx, value)
            .into_iter()
            .filter(|(_, _, kind)| *kind == ReferenceKind::Borrowed)
            .map(|(path, origin, _)| (path, origin))
            .collect()
    }

    /// Return origins and reference kinds, binding values without reference paths at the root.
    fn value_paths(
        &self,
        cx: &OriginContext<'_>,
        value: Value,
    ) -> Vec<(Path, Origin, ReferenceKind)> {
        let ty = cx.function.expect_value_type(value);
        let paths = cx.tree.type_origin_paths(TypeId::from(ty));
        if paths.is_empty() {
            let origin = self.value(value);
            if origin.is_empty() {
                return Vec::new();
            }

            return vec![(Path::root(), origin, ReferenceKind::Borrowed)];
        }

        paths
            .into_iter()
            .map(|borrowed| {
                let origin = self.value_path(value, &borrowed.path);

                (borrowed.path, origin, borrowed.kind)
            })
            // skip handle paths holding no tracked handle
            .filter(|(_, origin, kind)| *kind == ReferenceKind::Borrowed || !origin.is_empty())
            .collect()
    }

    /// Merge unique loans into one destination.
    fn merge_loans(loans: &mut Vec<LoanId>, added: &[LoanId]) {
        for &loan in added {
            if !loans.contains(&loan) {
                loans.push(loan);
            }
        }
    }

    /// Advance through one MIR instruction.
    pub fn advance(&mut self, cx: &OriginContext<'_>, instruction_id: LocalNodeId<Instruction>) {
        let instruction = cx.tree.get(instruction_id);

        // bind a borrowed call result before transferring stored origin
        cx.define_call_result(self, instruction);

        self.transfer_instruction(cx, instruction);

        // bind loans onto the value this instruction defines
        let Some(destination) = instruction.destination() else {
            return;
        };

        // merge the origin of every borrowed place this instruction loans
        let mut issued = Origin::none();
        for loan_id in cx.loans.roots_of(destination) {
            let loan = cx.loans.get(loan_id);
            let place = loan
                .place()
                .unwrap_or_else(|| unreachable!("instruction loan has no concrete place"));
            issued = issued.merge(&self.place(cx, place).with_loan(loan_id));
        }

        if issued.is_empty() {
            return;
        }

        // keep the regions a call signature mapped beside the reborrows
        match instruction {
            Instruction::Call { .. } => self.merge_origins_at(destination, &Path::root(), &issued),
            _ => self.insert(destination, issued),
        }
    }

    /// Copy origin from one value into another.
    fn copy(&mut self, cx: &OriginContext<'_>, source: Value, destination: Value) {
        let place = cx.places.get(source);
        self.copy_place(cx, place, destination);

        // carry the source value's loans into the copy
        if let Some(loans) = self.carried_loans(cx, source, destination) {
            self.merge_origins_at(destination, &Path::root(), &loans);
        }
    }

    /// Return the loans one copied reference-like value carries into its copy.
    fn carried_loans(
        &self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
    ) -> Option<Origin> {
        // keep the same borrow through a reference-like copy under another type
        let ty = cx
            .tree
            .storage_type(TypeId::from(cx.function.expect_value_type(destination)));
        cx.tree.get(ty).reference_kind()?;
        let carried = self.value(source);
        if carried.loans().is_empty() {
            return None;
        }
        Some(Origin::new([]).with_loans(carried.loans()))
    }

    /// Copy origin from one place into one value.
    fn copy_place(&mut self, cx: &OriginContext<'_>, place: &Place, destination: Value) {
        if !Self::carries(cx, destination) {
            return;
        }

        let root = self.place(cx, place);
        self.insert(destination, root);

        // preserve nested borrowed paths
        let ty = cx.function.expect_value_type(destination);
        for borrowed in cx.tree.type_origin_paths(TypeId::from(ty)) {
            let source = place.clone().with_path(&borrowed.path);
            let origin = self.place(cx, &source);
            self.insert_at(destination, borrowed.path, origin);
        }
    }

    /// Load stored origin into one value.
    fn load(&mut self, cx: &OriginContext<'_>, place: &Place, destination: Value) {
        if !Self::carries(cx, destination) {
            return;
        }

        let ty = cx.function.expect_value_type(destination);
        let paths = cx.tree.type_origin_paths(TypeId::from(ty));
        let bindings = if paths.is_empty() {
            let origin = self
                .get_place(place)
                .cloned()
                .unwrap_or_else(|| Self::type_origin(cx, destination));

            vec![(Path::root(), origin)]
        } else {
            paths
                .into_iter()
                .map(|borrowed| {
                    let stored = place.clone().with_path(&borrowed.path);
                    let mut origin = self
                        .get_place(&stored)
                        .cloned()
                        .unwrap_or_else(|| Origin::from_path(&borrowed));
                    if let Some(loan) = cx.loans.carried(destination, &borrowed.path) {
                        origin = origin.with_loan(loan);
                    }

                    (borrowed.path, origin)
                })
                .collect()
        };

        self.insert_bindings(destination, bindings);
    }

    /// Merge origin from alternative values.
    fn merge_values(
        &mut self,
        cx: &OriginContext<'_>,
        values: impl IntoIterator<Item = Value>,
        destination: Value,
    ) {
        if !Self::carries(cx, destination) {
            return;
        }

        let mut bindings: Vec<(Path, Origin)> = Vec::new();

        // merge each structural path across every source
        for value in values {
            for (path, origin) in self.value_bindings(cx, value) {
                if let Some((_, current)) =
                    bindings.iter_mut().find(|(current, _)| *current == path)
                {
                    *current = current.merge(&origin);
                } else {
                    bindings.push((path, origin));
                }
            }
        }

        self.insert_bindings(destination, bindings);
    }

    /// Return the origin path one projection selects inside a value.
    fn value_projection(cx: &OriginContext<'_>, value: Value, projection: Projection) -> Path {
        // read through applications, dropping the projection at a newtype layer
        let mut ty = TypeId::from(cx.function.expect_value_type(value));
        loop {
            ty = match cx.tree.get(ty) {
                Type::Newtype { .. } => return Path::root(),
                Type::Application { base, .. } => *base,
                _ => return Path::root().with_projection(projection),
            };
        }
    }

    /// Replace one destination projection from a value.
    fn replace(
        &mut self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
        projection: Projection,
    ) {
        let base = Self::value_projection(cx, destination, projection);
        let bindings = self.value_bindings(cx, source);
        self.replace_paths(destination, base, bindings);
    }

    /// Merge one value into a destination projection.
    fn merge_projection(
        &mut self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
        projection: Projection,
    ) {
        let base = Self::value_projection(cx, destination, projection);
        let bindings = self.value_bindings(cx, source);
        self.merge_paths(destination, base, bindings);
    }

    /// Project one structural value path into a destination value.
    fn project(
        &mut self,
        cx: &OriginContext<'_>,
        source: Value,
        destination: Value,
        projection: Projection,
    ) {
        let prefix = Self::value_projection(cx, source, projection);
        let bindings = self
            .value_bindings(cx, source)
            .into_iter()
            .filter_map(|(path, origin)| path.strip_prefix(&prefix).map(|path| (path, origin)))
            .collect();

        self.insert_bindings(destination, bindings);
    }

    /// Transfer one MIR instruction.
    fn transfer_instruction(&mut self, cx: &OriginContext<'_>, instruction: &Instruction) {
        match instruction {
            Instruction::Call { call, .. } => self.transfer_call(cx, call),
            Instruction::LocalGet { destination, local } => {
                self.load(cx, &Place::local(*local), *destination);
            }
            Instruction::LocalSet { local, value } => {
                let place = Place::local(*local);
                let bindings = self.value_bindings(cx, *value);
                self.kill(cx, &place);
                self.insert_place_bindings(place, bindings);
            }
            Instruction::Store { pointer, value } => {
                let place = cx.places.get(*pointer).clone();
                let mut bindings = self.value_bindings(cx, *value);
                if Self::storage_outlives_reference(cx, &place) {
                    self.escape_loans(&bindings);
                }
                self.root_handles(cx, &place, &mut bindings, *value);
                self.kill(cx, &place);
                self.insert_place_bindings(place, bindings);
            }
            Instruction::Aggregate {
                destination,
                values,
            } => {
                self.aggregate(cx, *destination, cx.tree.get_values(*values));
            }
            Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } => {
                self.merge_values(cx, [*then_value, *else_value], *destination);
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => {
                self.project(
                    cx,
                    *aggregate,
                    *destination,
                    Projection::Field { index: *field },
                );
            }
            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => {
                self.project(
                    cx,
                    *aggregate,
                    *destination,
                    Projection::Element { index: *index },
                );
            }
            Instruction::FieldSet {
                destination,
                aggregate,
                value,
                field,
            } => {
                self.copy(cx, *aggregate, *destination);
                self.replace(
                    cx,
                    *value,
                    *destination,
                    Projection::Field { index: *field },
                );
            }
            Instruction::ElementSet {
                destination,
                aggregate,
                value,
                index,
            } => {
                self.copy(cx, *aggregate, *destination);
                self.replace(
                    cx,
                    *value,
                    *destination,
                    Projection::Element { index: *index },
                );
            }
            Instruction::Load {
                destination,
                pointer,
                ..
            } => {
                self.load(cx, cx.places.get(*pointer), *destination);
            }
            // a projected place carries the origin of the place it projects from
            Instruction::FieldAddr {
                destination,
                aggregate: base,
                kind: AddressKind::Projection,
                ..
            }
            | Instruction::ElementAddr {
                destination,
                base,
                kind: AddressKind::Projection,
                ..
            }
            | Instruction::VariantPayloadAddr {
                destination,
                variant: base,
                kind: AddressKind::Projection,
                ..
            } => {
                let origin = self.value(*base);
                if !origin.is_empty() {
                    self.insert(*destination, origin);
                }
            }
            Instruction::Cast {
                destination,
                argument,
                ..
            }
            | Instruction::FunctionEnvironment {
                destination,
                function: argument,
            }
            | Instruction::FunctionBind {
                destination,
                environment: argument,
                ..
            }
            | Instruction::DynamicBind {
                destination,
                payload: argument,
                ..
            }
            | Instruction::DynamicPayload {
                destination,
                dynamic: argument,
                ..
            } => {
                self.copy(cx, *argument, *destination);
            }
            // keep the origin the reinterpret reads
            Instruction::Intrinsic {
                destination: Some(destination),
                intrinsic: Intrinsic::Transmute,
                arguments,
            } => {
                if let [argument] = cx.tree.get_values(*arguments) {
                    let argument_type = cx.function.expect_value_type(*argument);
                    let storage = cx.tree.storage_type(TypeId::from(argument_type));
                    match cx.tree.get(storage).reference_kind() {
                        Some(ReferenceKind::Unique) => {
                            self.insert(*destination, Origin::one(Region::Static));
                        }
                        _ => self.copy(cx, *argument, *destination),
                    }
                }
            }
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. } => {
                let origin = Self::destination(cx, *destination);
                self.insert(*destination, origin);
            }
            Instruction::NewComplete {
                destination, value, ..
            } => {
                let origin = Self::destination(cx, *destination);
                self.insert(*destination, origin);

                let place = cx.places.get(*destination).clone();
                let mut bindings = self.value_bindings(cx, *value);
                if Self::storage_outlives_reference(cx, &place) {
                    self.escape_loans(&bindings);
                }
                self.root_handles(cx, &place, &mut bindings, *value);
                self.insert_place_bindings(place, bindings);
            }
            // null references and constants outlive every frame
            Instruction::Const { destination, .. } => {
                let ty = cx.function.expect_value_type(*destination);
                let bindings = cx
                    .tree
                    .type_origin_paths(TypeId::from(ty))
                    .into_iter()
                    .map(|borrowed| (borrowed.path, Origin::one(Region::Static)))
                    .collect();
                self.insert_bindings(*destination, bindings);
            }
            Instruction::VariantPayload {
                destination,
                variant,
                case,
            } => {
                self.project(
                    cx,
                    *variant,
                    *destination,
                    Projection::Variant { case: *case },
                );
            }
            // bind the absent cases to static and the built case to its payload
            Instruction::VariantNew {
                destination,
                case,
                payload,
                ..
            } => {
                let ty = cx.function.expect_value_type(*destination);
                let built = Path::root().with_projection(Projection::Variant { case: *case });
                let absent = cx
                    .tree
                    .type_origin_paths(TypeId::from(ty))
                    .into_iter()
                    .filter(|borrowed| borrowed.path.strip_prefix(&built).is_none())
                    .map(|borrowed| (borrowed.path, Origin::one(Region::Static)))
                    .collect();
                self.insert_bindings(*destination, absent);
                if let Some(payload) = payload {
                    self.replace(
                        cx,
                        *payload,
                        *destination,
                        Projection::Variant { case: *case },
                    );
                }
            }
            _ => {}
        }
    }

    /// Populate origin for one aggregate construction.
    fn aggregate(&mut self, cx: &OriginContext<'_>, destination: Value, values: &[Value]) {
        let ty = cx.function.expect_value_type(destination);

        // bind each logical slot to its destination path
        for (index, value) in values.iter().copied().enumerate() {
            if let Some(projection) = Self::aggregate_projection(cx, ty, index) {
                self.merge_projection(cx, value, destination, projection);
            } else {
                let source = self.value(value);
                let current = self.value(destination);
                self.insert(destination, current.merge(&source));
            }
        }
    }

    /// Return whether one value type carries origin.
    fn carries(cx: &OriginContext<'_>, value: Value) -> bool {
        let ty = TypeId::from(cx.function.expect_value_type(value));

        cx.tree.type_lifetime(ty).is_some() || !cx.tree.type_origin_paths(ty).is_empty()
    }

    /// Root stored handles in the written storage when it outlives their regions.
    fn root_handles(
        &self,
        cx: &OriginContext<'_>,
        place: &Place,
        bindings: &mut [(Path, Origin)],
        value: Value,
    ) {
        let ty = cx.function.expect_value_type(value);
        let paths = cx.tree.type_origin_paths(TypeId::from(ty));
        for (path, origin) in bindings.iter_mut() {
            let is_handle = paths
                .iter()
                .any(|borrowed| &borrowed.path == path && borrowed.kind == ReferenceKind::Managed);
            if !is_handle {
                continue;
            }
            let slot = self.storage_origin(cx, &place.clone().with_path(path));
            if slot.outlives(origin, &cx.function.lifetimes) {
                *origin = slot;
            }
        }
    }

    /// Return the path for one logical aggregate slot.
    fn aggregate_projection(
        cx: &OriginContext<'_>,
        ty: LocalNodeId<Type>,
        index: usize,
    ) -> Option<Projection> {
        match cx.tree.get(ty) {
            Type::Struct { .. } | Type::Tuple { .. } => Some(Projection::Field {
                index: index as u32,
            }),
            Type::FixedArray { .. } => Some(Projection::Element {
                index: index as u32,
            }),
            Type::Newtype { .. } => None,
            Type::Application { base, .. } => Self::aggregate_projection(cx, *base, index),
            _ => unreachable!("aggregate destination is not an aggregate type"),
        }
    }

    /// Return origin implied by one value type.
    fn type_origin(cx: &OriginContext<'_>, value: Value) -> Origin {
        let ty = cx.function.expect_value_type(value);

        cx.tree
            .type_lifetime(TypeId::from(ty))
            .map(|lifetime| Origin::from_lifetime(&lifetime))
            .unwrap_or_default()
    }

    /// Return origin for one storage-producing destination.
    fn destination(cx: &OriginContext<'_>, destination: Value) -> Origin {
        let ty = cx.function.expect_value_type(destination);
        let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));

        match ty.reference_kind() {
            Some(ReferenceKind::Managed) => Origin::from_managed(ty),
            Some(ReferenceKind::Unique) => Origin::one(Region::Frame),
            Some(ReferenceKind::Borrowed) | None => Origin::none(),
        }
    }

    /// Return origin implied by one value path used as storage.
    fn storage(&self, cx: &OriginContext<'_>, value: Value, path: &Path) -> Origin {
        if let Some(origin) = self.get_at(value, path) {
            return origin.clone();
        }
        if !path.is_root()
            && let Some(origin) = self.get_at(value, &Path::root())
        {
            return origin.clone();
        }

        let ty = cx.function.expect_value_type(value);
        let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));
        match ty.reference_kind() {
            Some(ReferenceKind::Managed | ReferenceKind::Borrowed) => self
                .get_at(value, &Path::root())
                .cloned()
                .unwrap_or_else(|| Origin::from_reference(ty)),
            Some(ReferenceKind::Unique) => Origin::one(Region::Frame),
            None => Origin::none(),
        }
    }

    /// Return whether addressed storage may outlive its reference value.
    fn storage_outlives_reference(cx: &OriginContext<'_>, place: &Place) -> bool {
        match (place.origin, place.path.first()) {
            (PlaceOrigin::Local(local), Some(Projection::Deref)) => {
                let ty = cx.tree.get(local).ty;
                let ty = cx.tree.get(cx.tree.storage_type(TypeId::from(ty)));

                matches!(
                    ty.reference_kind(),
                    Some(ReferenceKind::Managed | ReferenceKind::Borrowed)
                )
            }
            (PlaceOrigin::Local(_), _) => false,
            (PlaceOrigin::Global(_), _) => true,
            (PlaceOrigin::Value(value), _) => matches!(
                cx.function.reference_kind(value, cx.tree),
                Some(ReferenceKind::Managed | ReferenceKind::Borrowed)
            ),
        }
    }

    /// Merge the borrow writes permitted by one call signature.
    fn transfer_call(&mut self, cx: &OriginContext<'_>, call: &Call) {
        // read the declared lifetime relationships and arguments
        let signature = cx.tree.get(call.signature);
        let Some((lifetimes, parameters, _)) = signature.function_signature_parts() else {
            unreachable!("call has no function signature")
        };
        let arguments = cx.tree.get_values(call.arguments);
        let parameter_types = parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();
        let mut writes = Vec::new();

        // read every source before recording any writes performed by the call
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            let mut pending = vec![(parameter.ty, true)];
            let mut visited = FxIndexSet::default();
            while let Some((ty, is_root)) = pending.pop() {
                let ty = cx.tree.storage_type(ty);
                if !visited.insert((ty, is_root)) {
                    continue;
                }

                // classify the writable storage exposed by this type
                let (storage, access, has_address) = match cx.tree.get(ty) {
                    Type::Reference {
                        pointee, access, ..
                    } => {
                        pending.push((*pointee, false));

                        (*pointee, *access, is_root)
                    }
                    Type::Slice {
                        element, access, ..
                    } => {
                        pending.push((*element, false));

                        (*element, *access, false)
                    }
                    Type::Dynamic { access, .. } | Type::Function { access, .. } => {
                        (ty, *access, false)
                    }
                    Type::FunctionSignature { .. } | Type::FunctionPointer { .. } => continue,
                    _ => {
                        pending
                            .extend(cx.tree.child_type_ids(ty).into_iter().map(|ty| (ty, false)));
                        continue;
                    }
                };

                // require writable storage before collecting possible borrow writes
                if !access.can_write() {
                    continue;
                }

                // connect each stored lifetime to the arguments that can supply it
                for borrowed in cx.tree.type_borrowed_paths(storage) {
                    let origin = cx.map_lifetime(
                        self,
                        &borrowed.lifetime,
                        lifetimes,
                        &parameter_types,
                        arguments,
                    );
                    if origin.is_empty() {
                        continue;
                    }
                    let place = has_address
                        .then(|| cx.places.get(argument).clone().with_path(&borrowed.path));
                    writes.push(CallWrite { place, origin });
                }
            }
        }

        // preserve previous contents because a signature permits writes without requiring them
        for write in writes {
            if write
                .place
                .as_ref()
                .is_none_or(|place| Self::storage_outlives_reference(cx, place))
            {
                self.escape_loans(&[(Path::root(), write.origin.clone())]);
            }
            if let Some(place) = write.place {
                let origin = match self.get_place(&place) {
                    Some(previous) => previous.merge(&write.origin),
                    None => write.origin,
                };
                self.insert_place(place, origin);
            }
        }
    }
}

impl Lattice for OriginState {
    /// Merge origin carried by two incoming edges.
    fn meet(&self, other: &Self) -> Self {
        let mut merged = Self::new();

        // merge value origin from both predecessors
        for predecessor in [self, other] {
            for binding in predecessor.bindings() {
                merged.merge_origins_at(binding.value, &binding.path, &binding.origin);
            }
        }

        // merge stored origin from both predecessors
        for predecessor in [self, other] {
            for binding in predecessor.places() {
                let origin = merged
                    .get_place(&binding.place)
                    .map(|current| current.merge(&binding.origin))
                    .unwrap_or_else(|| binding.origin.clone());
                merged.insert_place(binding.place.clone(), origin);
            }
        }

        // retain every loan escaped through aliasable storage
        Self::merge_loans(&mut merged.escaped_loans, &self.escaped_loans);
        Self::merge_loans(&mut merged.escaped_loans, &other.escaped_loans);

        merged
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;
    use crate::analyses::tests::TestModule;

    /// A call preserves loan identities when it copies a borrow from addressed storage.
    #[test]
    fn test_retain_borrows_read_from_argument_storage() {
        let program = TestModule::new(
            r#"
external function save<'a, 'b, 'c>(ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, readonly, local>, ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'c, mutable, local>): void

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>): void {
    local l0: ref<int32, borrowed, 'a, readonly, local>

entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>):
    local.set l0, v0
    v2: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'frame, readonly, frame> = local.address l0
    call save(v2, v1): <'x, 'y, 'z>(ref<ref<int32, borrowed, 'x, readonly, local>, borrowed, 'y, readonly, local>, ref<ref<int32, borrowed, 'x, readonly, local>, borrowed, 'z, mutable, local>) => void
    jump done

done:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let done = function.blocks()[1];
        let retained = origins
            .entry(done)
            .unwrap()
            .escaped_loans()
            .iter()
            .map(|&loan| origins.loans().get(loan).representation)
            .collect::<Vec<_>>();

        assert_eq!(retained, vec![Value(0)]);
    }

    /// Both invoke successors retain borrows that the callee may store through an argument.
    #[test]
    fn test_retain_invoke_writes_on_normal_and_unwind_edges() {
        let program = TestModule::new(
            r#"
external function save<'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>): void

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>): void {
entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>):
    invoke save(v0, v1): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>) => void => normal | unwind

normal:
    return

unwind:
    unwind.resume
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let retained = function.blocks()[1..]
            .iter()
            .map(|&block| {
                origins
                    .entry(block)
                    .unwrap()
                    .escaped_loans()
                    .iter()
                    .map(|&loan| origins.loans().get(loan).representation)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        assert_eq!(retained, vec![vec![Value(0)], vec![Value(0)]]);
    }

    /// An unresolved indirect call propagates stored borrows from its signature.
    #[test]
    fn test_retain_indirect_call_writes() {
        let program = TestModule::new(
            r#"
function test<'a, 'b>(v0: fn(ref<int32, borrowed, 'a, readonly, local>, ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>) => void, v1: ref<int32, borrowed, 'a, readonly, local>, v2: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>): void {
entry(v0: fn(ref<int32, borrowed, 'a, readonly, local>, ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>) => void, v1: ref<int32, borrowed, 'a, readonly, local>, v2: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>):
    call.indirect v0(v1, v2): <'x, 'y>(ref<int32, borrowed, 'x, readonly, local>, ref<ref<int32, borrowed, 'x, readonly, local>, borrowed, 'y, mutable, local>) => void
    jump done

done:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let done = function.blocks()[1];
        let retained = origins
            .entry(done)
            .unwrap()
            .escaped_loans()
            .iter()
            .map(|&loan| origins.loans().get(loan).representation)
            .collect::<Vec<_>>();

        assert_eq!(retained, vec![Value(1)]);
    }

    /// A call can retain a borrow in an element of a writable slice.
    #[test]
    fn test_retain_writes_through_borrowed_slice() {
        let program = TestModule::new(
            r#"
external function save<'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, slice<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>): void

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: slice<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>): void {
entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: slice<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>):
    call save(v0, v1): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, slice<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>) => void
    jump done

done:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let done = function.blocks()[1];
        let retained = origins
            .entry(done)
            .unwrap()
            .escaped_loans()
            .iter()
            .map(|&loan| origins.loans().get(loan).representation)
            .collect::<Vec<_>>();

        assert_eq!(retained, vec![Value(0)]);
    }

    /// A writable reference nested in an aggregate can retain a borrowed argument.
    #[test]
    fn test_retain_writes_through_nested_reference() {
        let program = TestModule::new(
            r#"
type Target<'a, 'b> {
    value: ref<ref<int32, borrowed, 'a, readonly, local>, borrowed, 'b, mutable, local>;
}

external function save<'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, Target<'a & local, 'b & local>): void

function test<'a, 'b>(v0: ref<int32, borrowed, 'a, readonly, local>, v1: Target<'a & local, 'b & local>): void {
entry(v0: ref<int32, borrowed, 'a, readonly, local>, v1: Target<'a & local, 'b & local>):
    call save(v0, v1): <'a, 'b>(ref<int32, borrowed, 'a, readonly, local>, Target<'a & local, 'b & local>) => void
    jump done

done:
    return
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let origins = analyses.origin(function, &program.tree);
        let done = function.blocks()[1];
        let retained = origins
            .entry(done)
            .unwrap()
            .escaped_loans()
            .iter()
            .map(|&loan| origins.loans().get(loan).representation)
            .collect::<Vec<_>>();

        assert_eq!(retained, vec![Value(0)]);
    }

    /// Merge the distinct borrow arguments of two edges entering the same block.
    #[test]
    fn test_merge_borrow_arguments() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: boolean, v1: ref<int32, borrowed, 'a, readonly, local>, v2: ref<int32, borrowed, 'a, readonly, local>): ref<int32, borrowed, 'a, readonly, local> {
entry(v0: boolean, v1: ref<int32, borrowed, 'a, readonly, local>, v2: ref<int32, borrowed, 'a, readonly, local>):
    branch v0 => join(v1) | join(v2)

join(v3: ref<int32, borrowed, 'a, readonly, local>):
    return v3
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.origin(function, &program.tree);
        let join = table.entry(function.block(1)).unwrap();
        let origins = [Value(1), Value(2), Value(3)].map(|value| {
            join.value(value)
                .loans()
                .iter()
                .map(|&loan| table.loans().get(loan).representation)
                .collect::<Vec<_>>()
        });

        assert_eq!(
            origins,
            [vec![Value(1)], vec![Value(2)], vec![Value(1), Value(2)]]
        );
        assert_eq!(join.escaped_loans(), &[]);
    }

    /// Merge backedge borrows with function parameters before forwarding them to the exit.
    #[test]
    fn test_merge_entry_backedge_borrows() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: boolean, v1: ref<int32, borrowed, 'a, readonly, local>, v2: ref<int32, borrowed, 'a, readonly, local>): ref<int32, borrowed, 'a, readonly, local> {
entry(v0: boolean, v1: ref<int32, borrowed, 'a, readonly, local>, v2: ref<int32, borrowed, 'a, readonly, local>):
    branch v0 => entry(v0, v2, v1) | done(v1)

done(v3: ref<int32, borrowed, 'a, readonly, local>):
    return v3
}
"#,
        );
        let function = program.tree.get(program.entry_function_id());
        let mut analyses = program.function_analyses();
        let table = analyses.origin(function, &program.tree);
        let actual = [
            (function.block(0), Value(1)),
            (function.block(0), Value(2)),
            (function.block(1), Value(3)),
        ]
        .map(|(block, value)| {
            let mut origins = table
                .entry(block)
                .unwrap()
                .value(value)
                .loans()
                .iter()
                .map(|&loan| table.loans().get(loan).representation)
                .collect::<Vec<_>>();
            origins.sort_unstable();

            origins
        });

        assert_eq!(
            actual,
            [
                vec![Value(1), Value(2)],
                vec![Value(1), Value(2)],
                vec![Value(1), Value(2)]
            ]
        );
    }
}
