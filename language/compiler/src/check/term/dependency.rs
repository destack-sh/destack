use smallvec::SmallVec;

use crate::check::{
    CallArgument, CallCallee, CallTerm, CheckState, Dependency, FormTerm, GenericArgument,
    IdentityTerm, IndexSetTerm, IndexTerm, KeyMembershipTerm, LayoutTerm, MemberCallTerm,
    MemberReceiver, MemberTerm, OperatorTerm, RangeTerm, ReceiverTerm, ShapeMember, ShapeTerm,
    Solution, StaticOperand, StaticTerm, SuperTerm, TermId, TreeTerm, TryFailureTerm, TryTerm,
    TypeOperand, TypeOperationTerm, TypeTerm, TypeValueTerm, YieldTerm,
};

/// Collect solver dependencies from check operands and terms.
pub(in crate::check) struct DependencyCollector<'a> {
    /// The checked component state.
    state: &'a CheckState<'a>,
    /// The collected dependency set.
    dependencies: SmallVec<[Dependency; 2]>,
    /// Type terms already visited.
    type_terms: SmallVec<[TermId<TypeTerm>; 8]>,
    /// Static terms already visited.
    static_terms: SmallVec<[TermId<StaticTerm>; 8]>,
}

impl<'a> DependencyCollector<'a> {
    /// Return dependencies referenced by one type operand.
    pub(in crate::check) fn from_type_operand(
        state: &'a CheckState<'a>,
        operand: TypeOperand,
    ) -> SmallVec<[Dependency; 2]> {
        let mut collector = Self::new(state);
        collector.collect_type_operand(operand);

        collector.finish()
    }

    /// Return dependencies referenced by one static operand.
    pub(in crate::check) fn from_static_operand(
        state: &'a CheckState<'a>,
        operand: StaticOperand,
    ) -> SmallVec<[Dependency; 2]> {
        let mut collector = Self::new(state);
        collector.collect_static_operand(operand);

        collector.finish()
    }

    /// Return dependencies referenced by one static term.
    pub(in crate::check) fn from_static_term(
        state: &'a CheckState<'a>,
        term: &StaticTerm,
    ) -> SmallVec<[Dependency; 2]> {
        let mut collector = Self::new(state);
        collector.collect_static_term(term);

        collector.finish()
    }

    /// Create an empty dependency collector.
    fn new(state: &'a CheckState<'a>) -> Self {
        Self {
            state,
            dependencies: SmallVec::new(),
            type_terms: SmallVec::new(),
            static_terms: SmallVec::new(),
        }
    }

    /// Return collected dependencies in stable encounter order.
    fn finish(self) -> SmallVec<[Dependency; 2]> {
        self.dependencies
    }

    /// Collect one dependency.
    fn collect_dependency(&mut self, dependency: Dependency) {
        if !self.dependencies.contains(&dependency) {
            self.dependencies.push(dependency);
        }
    }

    /// Collect dependencies from one type operand.
    fn collect_type_operand(&mut self, operand: TypeOperand) {
        match operand {
            TypeOperand::Variable(variable) => {
                self.collect_dependency(Dependency::Variable(variable))
            }
            TypeOperand::Term(term) => self.collect_type_term_id(term),
            TypeOperand::Type(_) => {}
        }
    }

    /// Collect dependencies from one static operand.
    fn collect_static_operand(&mut self, operand: StaticOperand) {
        match operand {
            StaticOperand::Variable(variable) => {
                self.collect_dependency(Dependency::Variable(variable))
            }
            StaticOperand::Term(term) => self.collect_static_term_id(term),
            StaticOperand::Static(_) => {}
        }
    }

    /// Collect dependencies from one stored type term.
    fn collect_type_term_id(&mut self, term: TermId<TypeTerm>) {
        if self.type_terms.contains(&term) {
            return;
        }
        self.type_terms.push(term);

        let term = self.state.inference.term(term);
        self.collect_type_term(term);
    }

    /// Collect dependencies from one stored static term.
    fn collect_static_term_id(&mut self, term: TermId<StaticTerm>) {
        if self.static_terms.contains(&term) {
            return;
        }
        self.static_terms.push(term);

        let term = self.state.inference.term(term);
        self.collect_static_term(term);
    }

    /// Collect dependencies from one type term.
    fn collect_type_term(&mut self, term: &TypeTerm) {
        match term {
            TypeTerm::Reference {
                origin: _,
                symbol: _,
                arguments,
            } => self.collect_arguments(arguments),
            TypeTerm::Member(member) => {
                let member = self.state.inference.term(*member);
                self.collect_member_term(member);
            }
            TypeTerm::Form { form, payload } => {
                let form = self.state.inference.term(*form);
                self.collect_form_term(form);
                self.collect_type_operand(*payload);
            }
            TypeTerm::Dynamic { constraint } => self.collect_type_operand(*constraint),
            TypeTerm::Operation(operation) => {
                let operation = self.state.inference.term(*operation);
                self.collect_type_operation_term(operation);
            }
            TypeTerm::Array { element } | TypeTerm::Slice { element } => {
                self.collect_type_operand(*element);
            }
            TypeTerm::FixedArray { element, length } => {
                self.collect_type_operand(*element);
                self.collect_static_operand(*length);
            }
            TypeTerm::Tuple { form: _, elements } => {
                for element in elements {
                    self.collect_type_operand(element.ty);
                }
            }
            TypeTerm::Shape(shape) => {
                let shape = self.state.inference.term(*shape);
                self.collect_shape_term(shape);
            }
            TypeTerm::Function(function) => {
                let function = self.state.inference.term(*function);
                if let Some(parameter) = function.this_parameter {
                    self.collect_type_operand(parameter);
                }
                for parameter in &function.parameters {
                    self.collect_type_operand(parameter.ty);
                }
                if let Some(return_type) = function.return_type {
                    self.collect_type_operand(return_type);
                }
            }
            TypeTerm::Closure {
                function,
                environment,
            } => {
                self.collect_type_operand(*function);
                self.collect_type_operand(*environment);
            }
            TypeTerm::Union { elements } | TypeTerm::Intersection { elements } => {
                self.collect_type_operands(elements);
            }
            TypeTerm::StaticValue { value } => self.collect_static_operand(*value),
            TypeTerm::Call(call) => {
                let call = self.state.inference.term(*call);
                self.collect_call_term(call);
            }
            TypeTerm::Construct(construct) => {
                let construct = self.state.inference.term(*construct);
                self.collect_type_operand(construct.callee);
                self.collect_arguments(&construct.generic_arguments);
                self.collect_call_arguments(&construct.arguments);
            }
            TypeTerm::RangeValue(range) => {
                let range = self.state.inference.term(*range);
                self.collect_range_value_term(range);
            }
            TypeTerm::Tree(tree) => {
                let tree = self.state.inference.term(*tree);
                self.collect_tree_term(tree);
            }
            TypeTerm::TypeValue(value) => {
                let value = self.state.inference.term(*value);
                self.collect_type_value_term(value);
            }
            TypeTerm::Receiver(receiver) => {
                let receiver = self.state.inference.term(*receiver);
                self.collect_receiver_term(receiver);
            }
            TypeTerm::Super(term) => {
                let term = self.state.inference.term(*term);
                self.collect_super_term(term);
            }
            TypeTerm::Operator(operator) => {
                let operator = self.state.inference.term(*operator);
                self.collect_operator_term(operator);
            }
            TypeTerm::Index(index) => {
                let index = self.state.inference.term(*index);
                self.collect_index_term(index);
            }
            TypeTerm::IndexSet(set) => {
                let set = self.state.inference.term(*set);
                self.collect_index_set_term(set);
            }
            TypeTerm::KeyMembership(membership) => {
                let membership = self.state.inference.term(*membership);
                self.collect_key_membership_term(membership);
            }
            TypeTerm::InstanceCheck(instance) => {
                let instance = self.state.inference.term(*instance);
                self.collect_type_operand(instance.value);
                self.collect_type_operand(instance.target);
            }
            TypeTerm::Identity(identity) => {
                let identity = self.state.inference.term(*identity);
                self.collect_identity_term(identity);
            }
            TypeTerm::Await(awaited) => {
                let awaited = self.state.inference.term(*awaited);
                self.collect_type_operand(awaited.value);
            }
            TypeTerm::Try(tried) => {
                let tried = self.state.inference.term(*tried);
                self.collect_try_term(tried);
            }
            TypeTerm::Yield(yielded) => {
                let yielded = self.state.inference.term(*yielded);
                self.collect_yield_term(yielded);
            }
            TypeTerm::TryFailure(tried) => {
                let tried = self.state.inference.term(*tried);
                self.collect_try_failure_term(tried);
            }
            TypeTerm::Template(template) => {
                let template = self.state.inference.term(*template);
                self.collect_type_operands(&template.spans);
            }
            TypeTerm::TaggedTemplate(template) => {
                let template = self.state.inference.term(*template);
                self.collect_type_operand(template.tag);
                self.collect_arguments(&template.generic_arguments);
                self.collect_type_operands(&template.spans);
            }
            TypeTerm::Range { .. }
            | TypeTerm::ImportMeta(_)
            | TypeTerm::Literal(_)
            | TypeTerm::Intrinsic
            | TypeTerm::Type(_)
            | TypeTerm::Parameter(_)
            | TypeTerm::This => {}
        }
    }

    /// Collect dependencies from one static term.
    fn collect_static_term(&mut self, term: &StaticTerm) {
        match term {
            StaticTerm::Member {
                source: _,
                owner,
                key: _,
                arguments,
            } => {
                self.collect_type_operand(*owner);
                self.collect_arguments(arguments);
            }
            StaticTerm::Layout(layout) => self.collect_layout_term(layout),
            StaticTerm::Intrinsic { item: _, arguments } => self.collect_arguments(arguments),
            StaticTerm::Equal { left, right, .. } => {
                self.collect_static_operand(*left);
                self.collect_static_operand(*right);
            }
            StaticTerm::TypeRelation {
                relation: _,
                left,
                right,
            } => {
                self.collect_type_operand(*left);
                self.collect_type_operand(*right);
            }
            StaticTerm::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                self.collect_static_operand(*condition);
                self.collect_static_operand(*then_value);
                self.collect_static_operand(*else_value);
            }
            StaticTerm::Union { elements } => self.collect_static_operands(elements),
            StaticTerm::Static(_)
            | StaticTerm::Literal(_)
            | StaticTerm::Parameter(_)
            | StaticTerm::Expression(_) => {}
        }
    }

    /// Collect dependencies from generic arguments.
    fn collect_arguments<'b>(&mut self, arguments: impl IntoIterator<Item = &'b GenericArgument>) {
        for argument in arguments {
            self.collect_argument(argument);
        }
    }

    /// Collect dependencies from one generic argument.
    fn collect_argument(&mut self, argument: &GenericArgument) {
        match argument {
            GenericArgument::Type(operand)
            | GenericArgument::SpreadType(operand)
            | GenericArgument::AssociatedType { value: operand, .. } => {
                self.collect_type_operand(*operand);
            }
            GenericArgument::Static(operand)
            | GenericArgument::SpreadStatic(operand)
            | GenericArgument::AssociatedConst { value: operand, .. } => {
                self.collect_static_operand(*operand);
            }
            GenericArgument::TypeOrStatic { source }
            | GenericArgument::SpreadTypeOrStatic { source } => {
                self.collect_type_operand(source.ty);
                if let Some(value) = source.r#static {
                    self.collect_static_operand(value);
                }
            }
        }
    }

    /// Collect dependencies from call arguments.
    fn collect_call_arguments<'b>(
        &mut self,
        arguments: impl IntoIterator<Item = &'b CallArgument>,
    ) {
        for argument in arguments {
            self.collect_type_operand(argument.ty);
        }
    }

    /// Collect dependencies from type operands.
    fn collect_type_operands<'b>(&mut self, operands: impl IntoIterator<Item = &'b TypeOperand>) {
        for operand in operands {
            self.collect_type_operand(*operand);
        }
    }

    /// Collect dependencies from static operands.
    fn collect_static_operands<'b>(
        &mut self,
        operands: impl IntoIterator<Item = &'b StaticOperand>,
    ) {
        for operand in operands {
            self.collect_static_operand(*operand);
        }
    }

    /// Collect dependencies from one memory form.
    fn collect_form_term(&mut self, form: &FormTerm) {
        match form {
            FormTerm::Borrowed { lifetime, access } => {
                self.collect_static_operand(*lifetime);
                self.collect_static_operand(*access);
            }
            FormTerm::Placed { place } => self.collect_static_operand(*place),
            FormTerm::Managed | FormTerm::Owned | FormTerm::Raw | FormTerm::Readonly => {}
        }
    }

    /// Collect dependencies from one structural shape.
    fn collect_shape_term(&mut self, shape: &ShapeTerm) {
        for member in &shape.members {
            self.collect_shape_member(member);
        }
    }

    /// Collect dependencies from one structural shape member.
    fn collect_shape_member(&mut self, member: &ShapeMember) {
        match member {
            ShapeMember::Field { ty, .. }
            | ShapeMember::CallSignature { ty }
            | ShapeMember::ConstructSignature { ty } => self.collect_type_operand(*ty),
            ShapeMember::Spread { source, .. } => self.collect_type_operand(*source),
            ShapeMember::IndexSignature {
                key_type,
                value_type,
                ..
            } => {
                self.collect_type_operand(*key_type);
                self.collect_type_operand(*value_type);
            }
        }
    }

    /// Collect dependencies from one call expression.
    fn collect_call_term(&mut self, call: &CallTerm) {
        match call.callee {
            CallCallee::Expression(callee) => self.collect_type_operand(callee),
            CallCallee::Reference { value, symbol: _ } => self.collect_type_operand(value),
            CallCallee::Member(member) => {
                let member = self.state.inference.term(member);
                self.collect_member_call_term(member);
            }
        }
        self.collect_arguments(&call.generic_arguments);
        self.collect_call_arguments(&call.arguments);
    }

    /// Collect dependencies from one member call callee.
    fn collect_member_call_term(&mut self, member: &MemberCallTerm) {
        self.collect_member_receiver(&member.receiver);
        self.collect_arguments(&member.arguments);
    }

    /// Collect dependencies from one member receiver.
    fn collect_member_receiver(&mut self, receiver: &MemberReceiver) {
        match receiver {
            MemberReceiver::Value(operand) => {
                self.collect_type_operand(*operand);
                self.collect_parameter_constraint(*operand);
            }
            MemberReceiver::GenericParameter(parameter) => {
                let Some(parameter) = self.state.inference.generic_parameter(*parameter) else {
                    return;
                };
                if let Some(constraint) = parameter.type_constraint() {
                    self.collect_type_operand(constraint);
                }
            }
            MemberReceiver::Declaration {
                origin: _,
                symbol: _,
                arguments,
            } => self.collect_arguments(arguments),
        }
    }

    /// Collect dependencies from a constrained generic parameter receiver.
    fn collect_parameter_constraint(&mut self, operand: TypeOperand) {
        let TypeOperand::Term(term) = operand else {
            return;
        };
        let TypeTerm::Parameter(parameter) = self.state.inference.term(term) else {
            return;
        };

        let Some(parameter) = self.state.inference.generic_parameter(*parameter) else {
            return;
        };
        if let Some(constraint) = parameter.type_constraint() {
            self.collect_type_operand(constraint);
        }
    }

    /// Collect dependencies from one type member projection.
    fn collect_member_term(&mut self, member: &MemberTerm) {
        self.collect_member_receiver(&member.receiver);
        self.collect_arguments(&member.arguments);
    }

    /// Collect dependencies from one operator expression.
    fn collect_operator_term(&mut self, operator: &OperatorTerm) {
        self.collect_type_operand(operator.receiver);
        if let Some(argument) = operator.argument {
            self.collect_type_operand(argument);
        }
    }

    /// Collect dependencies from one identity expression.
    fn collect_identity_term(&mut self, identity: &IdentityTerm) {
        self.collect_type_operand(identity.left);
        self.collect_type_operand(identity.right);
    }

    /// Collect dependencies from one index expression.
    fn collect_index_term(&mut self, index: &IndexTerm) {
        self.collect_type_operand(index.receiver);
        self.collect_parameter_constraint(index.receiver);
        self.collect_type_operand(index.index);
    }

    /// Collect dependencies from one index set expression.
    fn collect_index_set_term(&mut self, set: &IndexSetTerm) {
        self.collect_type_operand(set.receiver);
        self.collect_parameter_constraint(set.receiver);
        self.collect_type_operand(set.index);
        self.collect_type_operand(set.value);
    }

    /// Collect dependencies from one key membership expression.
    fn collect_key_membership_term(&mut self, membership: &KeyMembershipTerm) {
        self.collect_type_operand(membership.key);
        self.collect_type_operand(membership.receiver);
    }

    /// Collect dependencies from one layout query.
    fn collect_layout_term(&mut self, layout: &LayoutTerm) {
        self.collect_type_operand(layout.target);
    }

    /// Collect dependencies from one runtime range.
    fn collect_range_value_term(&mut self, range: &RangeTerm) {
        if let Some(start) = range.start {
            self.collect_type_operand(start);
        }
        if let Some(end) = range.end {
            self.collect_type_operand(end);
        }
    }

    /// Collect dependencies from one tree expression.
    fn collect_tree_term(&mut self, tree: &TreeTerm) {
        if let Some(tag) = tree.tag {
            self.collect_type_operand(tag);
        }
        self.collect_arguments(&tree.generic_arguments);
        self.collect_type_operands(&tree.arguments);
        self.collect_type_operands(&tree.elements);
    }

    /// Collect dependencies from one reflected type value.
    fn collect_type_value_term(&mut self, value: &TypeValueTerm) {
        self.collect_type_operand(value.ty);
    }

    /// Collect dependencies from one contextual receiver.
    fn collect_receiver_term(&mut self, receiver: &ReceiverTerm) {
        self.collect_type_operand(receiver.ty);
    }

    /// Collect dependencies from one super receiver.
    fn collect_super_term(&mut self, term: &SuperTerm) {
        if let Some(receiver) = term.receiver {
            self.collect_type_operand(receiver);
        }
    }

    /// Collect dependencies from one try expression.
    fn collect_try_term(&mut self, term: &TryTerm) {
        self.collect_type_operand(term.value);
    }

    /// Collect dependencies from one try failure projection.
    fn collect_try_failure_term(&mut self, term: &TryFailureTerm) {
        self.collect_type_operand(term.value);
    }

    /// Collect dependencies from one yield expression.
    fn collect_yield_term(&mut self, term: &YieldTerm) {
        if let Some(value) = term.value {
            self.collect_type_operand(value);
        }
        if let Some(target) = term.yield_target {
            self.collect_type_operand(target);
        }
        if let Some(target) = term.resume_target {
            self.collect_type_operand(target);
        }
        if let Some(target) = term.delegate_return_target {
            self.collect_type_operand(target);
        }
    }

    /// Collect dependencies from one type operation.
    fn collect_type_operation_term(&mut self, operation: &TypeOperationTerm) {
        match operation {
            TypeOperationTerm::StringMapping {
                mapping: _,
                argument,
            } => self.collect_type_operand(*argument),
            TypeOperationTerm::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.collect_type_operand(*left);
                self.collect_type_operand(*right);
                self.collect_type_operand(*then_type);
                self.collect_type_operand(*else_type);
            }
            TypeOperationTerm::Mapped {
                parameter,
                modifiers: _,
                value,
            } => {
                self.collect_type_operand(parameter.constraint);
                if let Some(key_remap) = parameter.key_remap {
                    self.collect_type_operand(key_remap);
                }
                self.collect_type_operand(*value);
            }
            TypeOperationTerm::Index { left, index } => {
                self.collect_type_operand(*left);
                self.collect_type_operand(*index);
            }
            TypeOperationTerm::TemplateLiteral { strings: _, spans } => {
                self.collect_type_operands(spans);
            }
            TypeOperationTerm::Infer {
                name: _,
                constraint,
            } => {
                if let Some(constraint) = constraint {
                    self.collect_type_operand(*constraint);
                }
            }
            TypeOperationTerm::KeyOf { target } => self.collect_type_operand(*target),
            TypeOperationTerm::BestCommon { elements } => self.collect_type_operands(elements),
            TypeOperationTerm::Widen { source } => {
                self.collect_type_operand(*source);
                if let TypeOperand::Variable(variable) = source
                    && let Some(Solution::Type(solution)) =
                        self.state.inference.variable_solution(*variable)
                {
                    self.collect_type_operand(solution.into());
                }
            }
            TypeOperationTerm::Exclude { source, target }
            | TypeOperationTerm::Extract { source, target } => {
                self.collect_type_operand(*source);
                self.collect_type_operand(*target);
            }
            TypeOperationTerm::Intrinsic { item: _, arguments } => {
                self.collect_arguments(arguments)
            }
        }
    }
}
