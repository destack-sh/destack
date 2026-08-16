use smallvec::SmallVec;

use crate::{
    Block, BlockTarget, CallSite, ControlTable, Edge, Function, Instruction, Lattice, Lifetime,
    LifetimeParameter, LifetimeSlot, LifetimeTerm, LocalNodeId, Path, Place, PlaceOrigin,
    PlaceTable, Projection, ReferenceKind, SignatureParameter, Storage, Successor, Terminator,
    Tree, Type, TypeId, Value,
};

use super::{
    Analysis, Dataflow, ForwardTransfer, FunctionCache, Loan, LoanId, LoanTable, Mutation,
    ResolutionTable,
};

/// Borrow provenance across one MIR function.
#[derive(Debug)]
pub struct ProvenanceTable {
    /// Provenance at reachable block entries and exits.
    flow: Dataflow<ProvenanceState>,
    /// Loans issued by borrowed references.
    loans: LoanTable,
}

/// Provenance at one MIR program point.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProvenanceState {
    /// Provenance carried by SSA values.
    bindings: Vec<ValueBinding>,
    /// Stored place bindings.
    places: Vec<PlaceBinding>,
    /// Loans escaped through aliasable storage.
    escaped_loans: Vec<LoanId>,
}

/// Provenance that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Provenance {
    /// The lifetime regions.
    regions: SmallVec<[Region; 2]>,
    /// Loans kept active by this value.
    loans: SmallVec<[LoanId; 2]>,
}

/// Provenance for one value path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueBinding {
    /// SSA value carrying the borrowed path.
    pub value: Value,
    /// Path inside the value.
    pub path: Path,
    /// Provenance that keeps the path live.
    pub provenance: Provenance,
}

/// Provenance stored in one place path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceBinding {
    /// The exact place carrying the borrowed value.
    pub place: Place,
    /// Provenance that keeps the path live.
    pub provenance: Provenance,
}

/// Lifetime region that keeps a borrowed value valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Region {
    /// Global or static storage.
    Static,
    /// Explicit lifetime slot.
    Lifetime(LifetimeSlot),
    /// Storage in the current function frame.
    Frame,
}

impl ProvenanceTable {
    /// Build borrow provenance for one function.
    pub fn build(
        function: &Function,
        tree: &Tree,
        control: &ControlTable,
        places: &PlaceTable,
        resolution: &ResolutionTable,
    ) -> Self {
        let mut loans = Self::declare_loans(function, tree, places);
        let entry = Self::initial_state(function, tree, &loans);
        let flow = Dataflow::forward(function, tree, control, entry, |transfer, mut state, _| {
            match transfer {
                // transfer each instruction in the block
                ForwardTransfer::Block(block) => {
                    Self::solve_block(
                        block, &mut state, function, tree, places, resolution, &loans,
                    );
                }
                // bind provenance carried through the edge
                ForwardTransfer::Edge { edge, target } => {
                    Self::bind_edge(edge, target, &mut state, function, tree, resolution);
                }
            }

            state
        });
        Self::bind_loan_parents(&mut loans, function, tree, &flow, places, resolution);

        Self { flow, loans }
    }

    /// Return provenance at one reachable block entry.
    pub fn entry(&self, block: LocalNodeId<Block>) -> Option<&ProvenanceState> {
        self.flow.entry(block)
    }

    /// Return loans issued in the function.
    pub fn loans(&self) -> &LoanTable {
        &self.loans
    }

    /// Return initial provenance for function parameters.
    fn initial_state(function: &Function, tree: &Tree, loans: &LoanTable) -> ProvenanceState {
        let mut state = ProvenanceState::new();

        // seed parameter provenance from declared types
        for parameter in &function.parameters {
            for (path, mut provenance) in Self::parameter_bindings(parameter.ty, tree) {
                if let Some(loan) = loans.carried(parameter.value, &path) {
                    provenance = provenance.with_loan(loan);
                }
                state.insert_at(parameter.value, path, provenance);
            }
        }

        state
    }

    /// Return provenance implied by one parameter type.
    fn parameter_bindings(ty: TypeId, tree: &Tree) -> Vec<(Path, Provenance)> {
        let value_type = tree.get(ty);

        match value_type.reference_kind() {
            Some(ReferenceKind::Managed) => {
                vec![(Path::root(), Provenance::from_managed(value_type))]
            }
            Some(ReferenceKind::Borrowed) => {
                let Some(lifetime) = value_type.reference_lifetime() else {
                    return Vec::new();
                };

                vec![(Path::root(), Provenance::from_lifetime(lifetime))]
            }
            _ => {
                let borrowed_paths = tree.type_provenance_paths(ty);
                if borrowed_paths.is_empty() {
                    return Vec::new();
                }

                let mut bindings = Vec::new();

                // derive each borrowed aggregate path from its declared lifetime
                for borrowed_path in borrowed_paths {
                    let provenance = Provenance::from_lifetime(&borrowed_path.lifetime);
                    bindings.push((borrowed_path.path, provenance));
                }

                bindings
            }
        }
    }

    /// Transfer one block from its entry state.
    fn solve_block(
        block_id: LocalNodeId<Block>,
        state: &mut ProvenanceState,
        function: &Function,
        tree: &Tree,
        places: &PlaceTable,
        resolution: &ResolutionTable,
        loans: &LoanTable,
    ) {
        let block = tree.get(block_id);
        for &instruction_id in &block.instructions {
            state.advance(instruction_id, function, tree, places, resolution, loans);
        }
    }

    /// Declare every loan identity before provenance flow.
    fn declare_loans(function: &Function, tree: &Tree, places: &PlaceTable) -> LoanTable {
        let mut loans = LoanTable::new();
        let entry = function
            .entry()
            .unwrap_or_else(|| unreachable!("defined function has no entry block"));

        // declare borrowed paths entering through function parameters
        for parameter in &function.parameters {
            for borrowed in tree.type_provenance_paths(parameter.ty) {
                let loan = Loan::parameter(
                    parameter.value,
                    borrowed.path.clone(),
                    borrowed.access,
                    entry.into_any(),
                );
                loans.insert(parameter.value, borrowed.path, loan);
            }
        }

        // declare every instruction-issued loan
        for &block_id in function.blocks() {
            let block = tree.get(block_id);
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some((carrier, path, loan)) =
                    Self::loan(instruction_id, instruction, function, tree, places)
                {
                    loans.insert(carrier, path, loan);
                }
            }
        }

        loans.sort();

        loans
    }

    /// Bind parent loans after provenance reaches a fixed point.
    fn bind_loan_parents(
        loans: &mut LoanTable,
        function: &Function,
        tree: &Tree,
        flow: &Dataflow<ProvenanceState>,
        places: &PlaceTable,
        resolution: &ResolutionTable,
    ) {
        for &block_id in function.blocks() {
            let Some(mut state) = flow.entry(block_id).cloned() else {
                continue;
            };
            let block = tree.get(block_id);

            // replay instruction-issued loans from solved block entry state
            for &instruction_id in &block.instructions {
                let instruction = tree.get(instruction_id);
                if let Some(destination) = instruction.destination()
                    && let Some(loan) = loans.root(destination)
                    && let Some(parent) = loans.get(loan).source()
                {
                    let provenance = state.value(parent);
                    loans.set_parents(loan, provenance.loans().iter().copied());
                }
                state.advance(instruction_id, function, tree, places, resolution, loans);
            }
        }
    }

    /// Return the loan issued by one address instruction.
    fn loan(
        instruction_id: LocalNodeId<Instruction>,
        instruction: &Instruction,
        function: &Function,
        tree: &Tree,
        places: &PlaceTable,
    ) -> Option<(Value, Path, Loan)> {
        let (carrier, place, source) = match instruction {
            Instruction::FieldAddr {
                destination,
                aggregate,
                ..
            } => (
                *destination,
                places.get(*destination).clone(),
                Some(*aggregate),
            ),
            Instruction::ElementAddr {
                destination, base, ..
            } => (*destination, places.get(*destination).clone(), Some(*base)),
            Instruction::VariantPayloadAddr {
                destination,
                variant,
                ..
            } => (
                *destination,
                places.get(*destination).clone(),
                Some(*variant),
            ),
            Instruction::SliceView {
                destination,
                source,
                ..
            } => (
                *destination,
                places.get(*destination).clone(),
                Some(*source),
            ),
            Instruction::TensorView {
                destination, view, ..
            } => (*destination, places.get(*destination).clone(), Some(*view)),
            Instruction::LocalAddr {
                destination, local, ..
            } => (*destination, Place::local(*local), None),
            Instruction::GlobalAddr {
                destination,
                global,
                ..
            } => (*destination, Place::global(*global), None),
            _ => return None,
        };
        let ty = function.expect_value_type(carrier);
        let value_type = tree.get(ty);
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
            carrier,
            [],
            instruction_id.into_any(),
        );

        Some((carrier, Path::root(), loan))
    }

    /// Bind predecessor values to successor parameters.
    fn bind_edge(
        edge: Edge,
        target: &BlockTarget,
        state: &mut ProvenanceState,
        function: &Function,
        tree: &Tree,
        resolution: &ResolutionTable,
    ) {
        let predecessor = tree.get(edge.source);
        let terminator = tree.get(predecessor.terminator);
        let arguments = target.arguments(tree);
        let Some(parameters) = terminator.target_parameters(tree, edge.successor, target) else {
            return;
        };

        // bind edge arguments to successor parameters
        for (parameter, argument) in parameters.iter().zip(arguments.iter().copied()) {
            state.bind(argument, parameter.value, function, tree);
        }

        // bind one normal invoke result to its successor parameter
        let Some(bindings) = Self::invoke_result(
            edge.source,
            edge.successor,
            terminator,
            state,
            tree,
            resolution,
        ) else {
            return;
        };
        let Some(parameter) = tree
            .get(target.block)
            .parameters
            .first()
            .map(|parameter| parameter.value)
        else {
            return;
        };

        state.insert_bindings(parameter, bindings);
    }

    /// Return provenance carried by one normal invoke result.
    fn invoke_result(
        block: LocalNodeId<Block>,
        successor: Successor,
        terminator: &Terminator,
        state: &ProvenanceState,
        tree: &Tree,
        resolution: &ResolutionTable,
    ) -> Option<Vec<(Path, Provenance)>> {
        let Terminator::Invoke { call, .. } = terminator else {
            return None;
        };
        if successor != Successor::InvokeNormal {
            return None;
        }

        let callsite = CallSite::Terminator(block);
        let target = terminator
            .call_direct_target()
            .or_else(|| resolution.target(callsite));
        let arguments = tree.get_values(call.arguments);

        Some(Self::call_result(
            target,
            &call.signature,
            arguments,
            state,
            tree,
        ))
    }

    /// Bind provenance carried by one instruction call result.
    fn define_call_result(
        destination: Option<Value>,
        target: Option<LocalNodeId<Function>>,
        signature: &TypeId,
        arguments: &[Value],
        state: &mut ProvenanceState,
        function: &Function,
        tree: &Tree,
    ) {
        let Some(destination) = destination else {
            return;
        };
        let ty = function.expect_value_type(destination);
        if tree.type_lifetime(ty).is_none() && !tree.type_contains_borrowed_refs(ty) {
            return;
        }

        let bindings = Self::call_result(target, signature, arguments, state, tree);
        state.insert_bindings(destination, bindings);
    }

    /// Return provenance carried by one call result.
    fn call_result(
        target: Option<LocalNodeId<Function>>,
        signature: &TypeId,
        arguments: &[Value],
        state: &ProvenanceState,
        tree: &Tree,
    ) -> Vec<(Path, Provenance)> {
        let Type::FunctionSignature {
            lifetimes: signature_lifetimes,
            parameters: signature_parameters,
            result: signature_result,
        } = tree.get(*signature)
        else {
            unreachable!("call has no function signature")
        };

        // use the resolved declaration as the direct call lifetime environment
        let target = target.map(|target| tree.get(target));
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
        let paths = tree.type_borrowed_paths(result);
        if !paths.is_empty() {
            return paths
                .into_iter()
                .map(|borrowed_path| {
                    let provenance = Self::map_lifetime(
                        &borrowed_path.lifetime,
                        lifetimes,
                        &parameter_types,
                        arguments,
                        state,
                        tree,
                    );

                    (borrowed_path.path, provenance)
                })
                .collect();
        }

        let lifetime = tree
            .type_lifetime(result)
            .filter(|lifetime| !lifetime.is_empty());
        let Some(lifetime) = lifetime else {
            return Vec::new();
        };
        let provenance = Self::map_lifetime(
            &lifetime,
            lifetimes,
            &parameter_types,
            arguments,
            state,
            tree,
        );

        vec![(Path::root(), provenance)]
    }

    /// Map callee lifetime regions to caller provenance.
    fn map_lifetime(
        lifetime: &Lifetime,
        lifetimes: &[LifetimeParameter],
        parameters: &[TypeId],
        arguments: &[Value],
        state: &ProvenanceState,
        tree: &Tree,
    ) -> Provenance {
        let mut provenance = Provenance::none();

        // map static regions directly
        for term in &lifetime.terms {
            if matches!(term, LifetimeTerm::Static) {
                provenance = provenance.merge(&Provenance::one(Region::Static));
            }
        }

        // map lifetime slots through actual argument paths
        for (index, parameter) in parameters.iter().enumerate() {
            let Some(argument) = arguments.get(index) else {
                continue;
            };
            for (path, callee) in Self::parameter_bindings(*parameter, tree) {
                if !callee.is_covered_by(lifetime, lifetimes) {
                    continue;
                }

                let argument = state.value_path(*argument, &path);
                provenance = provenance.merge(&argument);
            }
        }

        provenance
    }
}

impl Analysis for ProvenanceTable {
    const INVALIDATED_BY: Mutation = Mutation::VALUE
        .union(Mutation::CONTROL)
        .union(Mutation::LAYOUT);
}

impl ProvenanceTable {
    /// Compute borrow provenance for one function.
    pub(crate) fn compute(
        function: &Function,
        tree: &Tree,
        analyses: &mut FunctionCache,
        resolution: &ResolutionTable,
    ) -> Self {
        let control = analyses.control(function, tree);
        let places = analyses.place(function, tree);

        Self::build(function, tree, &control, &places, resolution)
    }
}

impl Region {
    /// Return this region as a MIR lifetime.
    fn lifetime(&self) -> Lifetime {
        match self {
            Self::Static => Lifetime::static_storage(),
            Self::Lifetime(slot) => Lifetime::slot(slot.0),
            Self::Frame => Lifetime::frame(),
        }
    }

    /// Return whether this region is local to the current function.
    fn is_local(&self) -> bool {
        match self {
            Self::Frame => true,
            Self::Static | Self::Lifetime(_) => false,
        }
    }

    /// Return whether this region is covered by a required lifetime.
    fn is_covered_by(&self, required: &Lifetime, parameters: &[LifetimeParameter]) -> bool {
        required.accepts(&self.lifetime(), parameters)
    }
}

impl Provenance {
    /// Create empty provenance.
    fn none() -> Self {
        Self::default()
    }

    /// Create provenance from one origin.
    fn one(origin: Region) -> Self {
        let mut regions = SmallVec::new();
        regions.push(origin);

        Self {
            regions,
            loans: SmallVec::new(),
        }
    }

    /// Create provenance from many regions.
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

    /// Create provenance from one MIR lifetime.
    fn from_lifetime(lifetime: &Lifetime) -> Self {
        Self::new(lifetime.terms.iter().map(|term| match term {
            LifetimeTerm::Static => Region::Static,
            LifetimeTerm::Frame => Region::Frame,
            LifetimeTerm::Slot(slot) => Region::Lifetime(*slot),
        }))
    }

    /// Create provenance for one managed reference type.
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
            .unwrap_or_else(|| Self::one(Region::Frame))
    }

    /// Create provenance for one reference-like type.
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

    /// Return whether this provenance is empty.
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

    /// Return whether this provenance has a proven lifetime region.
    pub fn has_region(&self) -> bool {
        !self.regions.is_empty()
    }

    /// Return whether this provenance satisfies a required MIR lifetime.
    pub fn is_covered_by(&self, required: &Lifetime, parameters: &[LifetimeParameter]) -> bool {
        self.regions
            .iter()
            .all(|region| region.is_covered_by(required, parameters))
    }

    /// Return whether this provenance lives at least as long as `other`.
    pub fn outlives(&self, other: &Provenance, parameters: &[LifetimeParameter]) -> bool {
        other.regions.is_empty()
            || (!self.regions.is_empty()
                && self.regions.iter().all(|region| {
                    other
                        .regions
                        .iter()
                        .all(|shorter| shorter.lifetime().accepts(&region.lifetime(), parameters))
                }))
    }

    /// Merge two provenance sets.
    fn merge(&self, other: &Self) -> Self {
        let mut merged = Self::new(self.regions.iter().chain(&other.regions).cloned());

        // retain each loan once
        for &loan in self.loans.iter().chain(&other.loans) {
            if !merged.loans.contains(&loan) {
                merged.loans.push(loan);
            }
        }

        merged
    }

    /// Add one issuing loan.
    fn with_loan(mut self, loan: LoanId) -> Self {
        if !self.loans.contains(&loan) {
            self.loans.push(loan);
        }

        self
    }

    /// Return loans retained by this provenance.
    pub fn loans(&self) -> &[LoanId] {
        &self.loans
    }
}

impl ProvenanceState {
    /// Create empty provenance flow.
    pub fn new() -> Self {
        Self::default()
    }

    /// Advance through one MIR instruction.
    pub fn advance(
        &mut self,
        instruction_id: LocalNodeId<Instruction>,
        function: &Function,
        tree: &Tree,
        places: &PlaceTable,
        resolution: &ResolutionTable,
        loans: &LoanTable,
    ) {
        let instruction = tree.get(instruction_id);

        // bind an issued loan before transferring stored provenance
        if let Some(destination) = instruction.destination()
            && let Some(loan_id) = loans.root(destination)
        {
            let loan = loans.get(loan_id);
            let place = loan
                .place()
                .unwrap_or_else(|| unreachable!("instruction loan has no concrete place"));
            let provenance = self.place(place, function, tree);
            self.insert(loan.carrier, provenance.with_loan(loan_id));
        }

        // bind a borrowed call result before transferring stored provenance
        if let Instruction::Call { call, .. } = instruction {
            let callsite = CallSite::Instruction(instruction_id);
            let target = instruction
                .call_direct_target()
                .or_else(|| resolution.target(callsite));
            let arguments = tree.get_values(call.arguments);
            ProvenanceTable::define_call_result(
                instruction.destination(),
                target,
                &call.signature,
                arguments,
                self,
                function,
                tree,
            );
        }

        self.transfer_instruction(instruction, function, tree, places);
    }

    /// Return provenance carried by one call result.
    pub fn call_result(
        &self,
        target: Option<LocalNodeId<Function>>,
        signature: TypeId,
        arguments: &[Value],
        tree: &Tree,
    ) -> Vec<(Path, Provenance)> {
        ProvenanceTable::call_result(target, &signature, arguments, self, tree)
    }

    /// Map callee lifetime regions to caller provenance.
    pub fn map_lifetime(
        &self,
        lifetime: &Lifetime,
        lifetimes: &[LifetimeParameter],
        parameters: &[SignatureParameter],
        arguments: &[Value],
        tree: &Tree,
    ) -> Provenance {
        let parameter_types = parameters
            .iter()
            .map(|parameter| parameter.ty)
            .collect::<Vec<_>>();

        ProvenanceTable::map_lifetime(lifetime, lifetimes, &parameter_types, arguments, self, tree)
    }

    /// Iterate all value bindings.
    pub fn bindings(&self) -> impl Iterator<Item = &ValueBinding> {
        self.bindings.iter()
    }

    /// Iterate all place bindings.
    pub fn places(&self) -> impl Iterator<Item = &PlaceBinding> {
        self.places.iter()
    }

    /// Bind one value to borrow provenance.
    fn insert(&mut self, value: Value, provenance: Provenance) {
        self.insert_at(value, Path::root(), provenance);
    }

    /// Bind one value path to borrow provenance.
    fn insert_at(&mut self, value: Value, path: Path, provenance: Provenance) {
        if provenance.is_empty() {
            return;
        }

        // replace existing bindings instead of growing duplicate entries
        if let Some(current) = self
            .bindings
            .iter_mut()
            .find(|binding| binding.value == value && binding.path == path)
        {
            current.provenance = provenance;
            return;
        }

        self.bindings.push(ValueBinding {
            value,
            path,
            provenance,
        });
    }

    /// Bind all provenance paths for one value.
    fn insert_bindings(&mut self, value: Value, bindings: Vec<(Path, Provenance)>) {
        self.bindings.retain(|binding| binding.value != value);

        // insert each structural path once
        for (path, provenance) in bindings {
            self.insert_at(value, path, provenance);
        }
    }

    /// Replace borrow provenance below one value path.
    fn replace_paths(&mut self, value: Value, base: Path, bindings: Vec<(Path, Provenance)>) {
        self.bindings
            .retain(|binding| binding.value != value || !base.contains(&binding.path));

        // insert each provenance below the replacement path
        for (path, provenance) in bindings {
            let path = base.clone().with_path(&path);
            self.insert_at(value, path, provenance);
        }
    }

    /// Merge borrow provenance below one value path.
    fn merge_paths(&mut self, value: Value, base: Path, bindings: Vec<(Path, Provenance)>) {
        // merge each provenance below the shared path
        for (path, provenance) in bindings {
            let path = base.clone().with_path(&path);
            self.merge_provenance_at(value, &path, &provenance);
        }
    }

    /// Return borrow provenance for one value path.
    fn get_at(&self, value: Value, path: &Path) -> Option<&Provenance> {
        self.bindings.iter().find_map(|binding| {
            (binding.value == value && &binding.path == path).then_some(&binding.provenance)
        })
    }

    /// Merge provenance at one value path.
    fn merge_provenance_at(&mut self, value: Value, path: &Path, provenance: &Provenance) {
        // accumulate provenance from repeated paths
        let provenance = self
            .get_at(value, path)
            .map(|current| current.merge(provenance))
            .unwrap_or_else(|| provenance.clone());

        self.insert_at(value, path.clone(), provenance);
    }

    /// Bind one place path to borrow provenance.
    fn insert_place(&mut self, place: Place, provenance: Provenance) {
        if provenance.is_empty() {
            return;
        }

        // replace the previous value stored in this exact place path
        if let Some(current) = self
            .places
            .iter_mut()
            .find(|binding| binding.place == place)
        {
            current.provenance = provenance;
            return;
        }

        self.places.push(PlaceBinding { place, provenance });
    }

    /// Return borrow provenance stored in one exact place.
    fn get_place(&self, place: &Place) -> Option<&Provenance> {
        self.places
            .iter()
            .find_map(|binding| (&binding.place == place).then_some(&binding.provenance))
    }

    /// Replace every borrowed path stored in one place.
    fn insert_place_bindings(&mut self, place: Place, bindings: Vec<(Path, Provenance)>) {
        self.places
            .retain(|binding| !place.contains(&binding.place));

        // retain each exact stored path
        for (path, provenance) in bindings {
            self.insert_place(place.clone().with_path(&path), provenance);
        }
    }

    /// Mark loans carried into aliasable storage as escaped.
    fn escape_loans(&mut self, bindings: &[(Path, Provenance)]) {
        for (_, provenance) in bindings {
            Self::merge_loans(&mut self.escaped_loans, provenance.loans());
        }
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
                Self::merge_loans(&mut loans, binding.provenance.loans());
            }
        }

        // collect loans carried by live memory places
        for binding in self.places() {
            if is_place_live(&binding.place) {
                Self::merge_loans(&mut loans, binding.provenance.loans());
            }
        }

        loans
    }

    /// Return loans retained through aliasable storage.
    pub fn escaped_loans(&self) -> &[LoanId] {
        &self.escaped_loans
    }

    /// Return provenance carried by one value.
    pub fn value(&self, value: Value) -> Provenance {
        self.value_path(value, &Path::root())
    }

    /// Return provenance carried by one value path.
    pub fn value_path(&self, value: Value, path: &Path) -> Provenance {
        if let Some(provenance) = self.get_at(value, path) {
            return provenance.clone();
        }
        if !path.is_root() {
            return Provenance::none();
        }

        let mut provenance = Provenance::none();

        // merge sparse child paths for whole-value checks
        for binding in &self.bindings {
            if binding.value == value {
                provenance = provenance.merge(&binding.provenance);
            }
        }

        provenance
    }

    /// Return provenance stored in one place.
    pub fn place(&self, place: &Place, function: &Function, tree: &Tree) -> Provenance {
        let mut provenance = self
            .get_place(place)
            .cloned()
            .unwrap_or_else(|| match place.origin {
                PlaceOrigin::Local(_) => Provenance::one(Region::Frame),
                PlaceOrigin::Global(_) => Provenance::one(Region::Static),
                PlaceOrigin::Value(value) => self.storage(value, &place.path, function, tree),
            });

        // merge sparse child paths for whole-place checks
        for binding in &self.places {
            if &binding.place != place && place.contains(&binding.place) {
                provenance = provenance.merge(&binding.provenance);
            }
        }

        provenance
    }

    /// Return provenance paths carried by one value.
    pub fn value_bindings(
        &self,
        value: Value,
        function: &Function,
        tree: &Tree,
    ) -> Vec<(Path, Provenance)> {
        let ty = function.expect_value_type(value);
        let paths = tree.type_provenance_paths(TypeId::from(ty));
        if paths.is_empty() {
            let provenance = self.value(value);
            if provenance.is_empty() {
                return Vec::new();
            }

            return vec![(Path::root(), provenance)];
        }

        paths
            .into_iter()
            .map(|borrowed| {
                let provenance = self.value_path(value, &borrowed.path);

                (borrowed.path, provenance)
            })
            .collect()
    }

    /// Copy provenance from one value into another.
    fn copy(
        &mut self,
        source: Value,
        destination: Value,
        function: &Function,
        tree: &Tree,
        places: &PlaceTable,
    ) {
        let place = places.get(source);
        self.copy_place(place, destination, function, tree);
    }

    /// Copy provenance from one place into one value.
    fn copy_place(&mut self, place: &Place, destination: Value, function: &Function, tree: &Tree) {
        if !Self::carries(destination, function, tree) {
            return;
        }

        let root = self.place(place, function, tree);
        self.insert(destination, root);

        // preserve nested borrowed paths
        let ty = function.expect_value_type(destination);
        for borrowed in tree.type_provenance_paths(TypeId::from(ty)) {
            let source = place.clone().with_path(&borrowed.path);
            let provenance = self.place(&source, function, tree);
            self.insert_at(destination, borrowed.path, provenance);
        }
    }

    /// Load stored provenance into one value.
    fn load(&mut self, place: &Place, destination: Value, function: &Function, tree: &Tree) {
        if !Self::carries(destination, function, tree) {
            return;
        }

        let ty = function.expect_value_type(destination);
        let paths = tree.type_provenance_paths(TypeId::from(ty));
        let bindings = if paths.is_empty() {
            let provenance = self
                .get_place(place)
                .cloned()
                .unwrap_or_else(|| Self::type_provenance(destination, function, tree));

            vec![(Path::root(), provenance)]
        } else {
            paths
                .into_iter()
                .map(|borrowed| {
                    let stored = place.clone().with_path(&borrowed.path);
                    let provenance = self
                        .get_place(&stored)
                        .cloned()
                        .unwrap_or_else(|| Provenance::from_lifetime(&borrowed.lifetime));

                    (borrowed.path, provenance)
                })
                .collect()
        };

        self.insert_bindings(destination, bindings);
    }

    /// Merge provenance from alternative values.
    fn merge_values(
        &mut self,
        values: impl IntoIterator<Item = Value>,
        destination: Value,
        function: &Function,
        tree: &Tree,
    ) {
        if !Self::carries(destination, function, tree) {
            return;
        }

        let mut bindings: Vec<(Path, Provenance)> = Vec::new();

        // merge each structural path across every source
        for value in values {
            for (path, provenance) in self.value_bindings(value, function, tree) {
                if let Some((_, current)) =
                    bindings.iter_mut().find(|(current, _)| *current == path)
                {
                    *current = current.merge(&provenance);
                } else {
                    bindings.push((path, provenance));
                }
            }
        }

        self.insert_bindings(destination, bindings);
    }

    /// Replace one destination projection from a value.
    fn replace(
        &mut self,
        source: Value,
        destination: Value,
        projection: Projection,
        function: &Function,
        tree: &Tree,
    ) {
        let base = Path::root().with_projection(projection);
        let bindings = self.value_bindings(source, function, tree);
        self.replace_paths(destination, base, bindings);
    }

    /// Merge one value into a destination projection.
    fn merge_projection(
        &mut self,
        source: Value,
        destination: Value,
        projection: Projection,
        function: &Function,
        tree: &Tree,
    ) {
        let base = Path::root().with_projection(projection);
        let bindings = self.value_bindings(source, function, tree);
        self.merge_paths(destination, base, bindings);
    }

    /// Project one structural value path into a destination value.
    fn project(
        &mut self,
        source: Value,
        destination: Value,
        projection: Projection,
        function: &Function,
        tree: &Tree,
    ) {
        let prefix = Path::root().with_projection(projection);
        let bindings = self
            .value_bindings(source, function, tree)
            .into_iter()
            .filter_map(|(path, provenance)| {
                path.strip_prefix(&prefix).map(|path| (path, provenance))
            })
            .collect();

        self.insert_bindings(destination, bindings);
    }

    /// Transfer one MIR instruction.
    fn transfer_instruction(
        &mut self,
        instruction: &Instruction,
        function: &Function,
        tree: &Tree,
        places: &PlaceTable,
    ) {
        match instruction {
            Instruction::LocalGet { destination, local } => {
                self.load(&Place::local(*local), *destination, function, tree);
            }
            Instruction::LocalSet { local, value } => {
                let bindings = self.value_bindings(*value, function, tree);
                self.insert_place_bindings(Place::local(*local), bindings);
            }
            Instruction::Store { pointer, value } => {
                let place = places.get(*pointer).clone();
                let bindings = self.value_bindings(*value, function, tree);
                if Self::storage_outlives_reference(&place, function, tree) {
                    self.escape_loans(&bindings);
                }
                self.insert_place_bindings(place, bindings);
            }
            Instruction::Aggregate {
                destination,
                values,
            } => {
                self.aggregate(*destination, tree.get_values(*values), function, tree);
            }
            Instruction::Select {
                destination,
                then_value,
                else_value,
                ..
            } => {
                self.merge_values([*then_value, *else_value], *destination, function, tree);
            }
            Instruction::FieldGet {
                destination,
                aggregate,
                field,
            } => {
                self.project(
                    *aggregate,
                    *destination,
                    Projection::Field { index: *field },
                    function,
                    tree,
                );
            }
            Instruction::ElementGet {
                destination,
                aggregate,
                index,
            } => {
                self.project(
                    *aggregate,
                    *destination,
                    Projection::Element { index: *index },
                    function,
                    tree,
                );
            }
            Instruction::FieldSet {
                destination,
                aggregate,
                value,
                field,
            } => {
                self.copy(*aggregate, *destination, function, tree, places);
                self.replace(
                    *value,
                    *destination,
                    Projection::Field { index: *field },
                    function,
                    tree,
                );
            }
            Instruction::ElementSet {
                destination,
                aggregate,
                value,
                index,
            } => {
                self.copy(*aggregate, *destination, function, tree, places);
                self.replace(
                    *value,
                    *destination,
                    Projection::Element { index: *index },
                    function,
                    tree,
                );
            }
            Instruction::Load {
                destination,
                pointer,
                ..
            } => {
                self.load(places.get(*pointer), *destination, function, tree);
            }
            Instruction::Cast {
                destination,
                argument,
                ..
            }
            | Instruction::Pin {
                destination,
                value: argument,
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
            }
            | Instruction::TensorCast {
                destination,
                tensor: argument,
            } => {
                self.copy(*argument, *destination, function, tree, places);
            }
            Instruction::NewZeroed { destination, .. }
            | Instruction::NewUninit { destination, .. }
            | Instruction::NewSliceZeroed { destination, .. }
            | Instruction::NewSliceUninit { destination, .. } => {
                let provenance = Self::destination(*destination, function, tree);
                self.insert(*destination, provenance);
            }
            Instruction::NewComplete {
                destination, value, ..
            } => {
                let provenance = Self::destination(*destination, function, tree);
                self.insert(*destination, provenance);

                let place = places.get(*destination).clone();
                let bindings = self.value_bindings(*value, function, tree);
                if Self::storage_outlives_reference(&place, function, tree) {
                    self.escape_loans(&bindings);
                }
                self.insert_place_bindings(place, bindings);
            }
            Instruction::VariantNew {
                destination,
                case,
                payload: Some(payload),
                ..
            } => {
                self.replace(
                    *payload,
                    *destination,
                    Projection::Variant { case: *case },
                    function,
                    tree,
                );
            }
            _ => {}
        }
    }

    /// Populate provenance for one aggregate construction.
    fn aggregate(
        &mut self,
        destination: Value,
        values: &[Value],
        function: &Function,
        tree: &Tree,
    ) {
        let ty = function.expect_value_type(destination);

        // bind each logical slot to its destination path
        for (index, value) in values.iter().copied().enumerate() {
            if let Some(projection) = Self::aggregate_projection(ty, index, tree) {
                self.merge_projection(value, destination, projection, function, tree);
            } else {
                let source = self.value(value);
                let current = self.value(destination);
                self.insert(destination, current.merge(&source));
            }
        }
    }

    /// Return whether one value type carries provenance.
    fn carries(value: Value, function: &Function, tree: &Tree) -> bool {
        let ty = TypeId::from(function.expect_value_type(value));

        tree.type_lifetime(ty).is_some() || tree.type_contains_borrowed_refs(ty)
    }

    /// Return the path for one logical aggregate slot.
    fn aggregate_projection(
        ty: LocalNodeId<Type>,
        index: usize,
        tree: &Tree,
    ) -> Option<Projection> {
        match tree.get(ty) {
            Type::Struct { .. } | Type::Tuple { .. } => Some(Projection::Field {
                index: index as u32,
            }),
            Type::FixedArray { .. } => Some(Projection::Element {
                index: index as u32,
            }),
            Type::Newtype { .. } => None,
            Type::Application { base, .. } => Self::aggregate_projection(*base, index, tree),
            _ => unreachable!("aggregate destination is not an aggregate type"),
        }
    }

    /// Return provenance implied by one value type.
    fn type_provenance(value: Value, function: &Function, tree: &Tree) -> Provenance {
        let ty = function.expect_value_type(value);

        tree.type_lifetime(TypeId::from(ty))
            .map(|lifetime| Provenance::from_lifetime(&lifetime))
            .unwrap_or_default()
    }

    /// Return provenance for one storage-producing destination.
    fn destination(destination: Value, function: &Function, tree: &Tree) -> Provenance {
        let ty = function.expect_value_type(destination);
        let ty = tree.get(tree.storage_type(TypeId::from(ty)));

        match ty.reference_kind() {
            Some(ReferenceKind::Managed) => Provenance::from_managed(ty),
            Some(ReferenceKind::Unique) => Provenance::one(Region::Frame),
            Some(ReferenceKind::Borrowed) | None => Provenance::none(),
        }
    }

    /// Return provenance implied by one value path used as storage.
    fn storage(&self, value: Value, path: &Path, function: &Function, tree: &Tree) -> Provenance {
        if let Some(provenance) = self.get_at(value, path) {
            return provenance.clone();
        }
        if !path.is_root()
            && let Some(provenance) = self.get_at(value, &Path::root())
        {
            return provenance.clone();
        }

        let ty = function.expect_value_type(value);
        let ty = tree.get(tree.storage_type(TypeId::from(ty)));
        match ty.reference_kind() {
            Some(ReferenceKind::Managed | ReferenceKind::Borrowed) => self
                .get_at(value, &Path::root())
                .cloned()
                .unwrap_or_else(|| Provenance::from_reference(ty)),
            Some(ReferenceKind::Unique) => Provenance::one(Region::Frame),
            None => Provenance::none(),
        }
    }

    /// Return whether addressed storage may outlive its reference value.
    fn storage_outlives_reference(place: &Place, function: &Function, tree: &Tree) -> bool {
        match place.origin {
            PlaceOrigin::Local(_) => false,
            PlaceOrigin::Global(_) => true,
            PlaceOrigin::Value(value) => matches!(
                function.reference_kind(value, tree),
                Some(ReferenceKind::Managed | ReferenceKind::Borrowed)
            ),
        }
    }

    /// Bind successor parameter provenance.
    fn bind(&mut self, argument: Value, parameter: Value, function: &Function, tree: &Tree) {
        if argument == parameter {
            return;
        }

        // replace the successor parameter with every provenance path from this edge
        let bindings = self.value_bindings(argument, function, tree);
        self.insert_bindings(parameter, bindings);

        // bind index values embedded in dynamic paths
        for binding in &mut self.bindings {
            binding.path.replace_value(argument, parameter);
        }
    }

    /// Merge unique loans into one destination.
    fn merge_loans(loans: &mut Vec<LoanId>, added: &[LoanId]) {
        for &loan in added {
            if !loans.contains(&loan) {
                loans.push(loan);
            }
        }
    }
}

impl Lattice for ProvenanceState {
    /// Merge provenance carried by two incoming edges.
    fn meet(&self, other: &Self) -> Self {
        let mut merged = Self::new();

        // merge value provenance from both predecessors
        for predecessor in [self, other] {
            for binding in predecessor.bindings() {
                merged.merge_provenance_at(binding.value, &binding.path, &binding.provenance);
            }
        }

        // merge stored provenance from both predecessors
        for predecessor in [self, other] {
            for binding in predecessor.places() {
                let provenance = merged
                    .get_place(&binding.place)
                    .map(|current| current.merge(&binding.provenance))
                    .unwrap_or_else(|| binding.provenance.clone());
                merged.insert_place(binding.place.clone(), provenance);
            }
        }

        // retain every loan escaped through aliasable storage
        Self::merge_loans(&mut merged.escaped_loans, &self.escaped_loans);
        Self::merge_loans(&mut merged.escaped_loans, &other.escaped_loans);

        merged
    }
}
