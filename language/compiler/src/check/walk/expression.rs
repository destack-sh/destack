use destack_dir as dir;

use crate::check::{
    CheckModuleState, Constraint, Obligation, ObligationContext, ObligationSource, TypeInferId,
    TypeTerm,
};

impl CheckModuleState {
    /// Walk one expression and collect check work.
    pub(in crate::check) fn walk_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        let node = id.into_global_any(self.module());
        let infer = self.infer_node(node);

        match expression {
            dir::Expression::Identifier { name } => self.walk_identifier(id, *name, infer),
            dir::Expression::ScalarLiteral(value) => self.walk_scalar_literal(value, infer),
            dir::Expression::Member { left, name } => {
                self.walk_member_expression(id, *left, *name, infer)
            }
            dir::Expression::Call {
                left, arguments, ..
            } => self.walk_call_expression(id, *left, arguments, infer),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } => self.walk_binary_expression(id, *left, *operator, *right, infer),
            dir::Expression::Assign { left, right, .. } => self.walk_assignment(id, *left, *right),
            _ => {}
        }

        dir::walk_expression(self, tree, id, expression);
    }

    /// Walk one declarator and collect initializer work.
    pub(in crate::check) fn walk_declarator(
        &mut self,
        tree: &dir::Tree,
        _id: dir::LocalNodeId<dir::Declarator>,
        declarator: &dir::Declarator,
    ) {
        let symbol = self.pattern_symbol(declarator.pattern);
        let annotation = declarator.ty.map(|ty| self.source_type_infer(ty));
        let value = declarator
            .value
            .map(|value| self.infer_node(value.into_global_any(self.module())));

        if let Some(symbol) = symbol {
            self.walk_declarator_symbol(symbol, annotation, value);
        }

        if let (Some(source), Some(target), Some(symbol), Some(value)) =
            (value, annotation, symbol, declarator.value)
        {
            self.push_assignable(
                source,
                target,
                ObligationSource::VariableInitializer { symbol },
                value.into_any(),
            );
        }

        dir::walk_declarator(self, tree, _id, declarator);
    }

    /// Walk one parameter and collect declared type work.
    pub(in crate::check) fn walk_parameter(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Parameter>,
        parameter: &dir::Parameter,
    ) {
        if let (Some(symbol), Some(annotation)) = (
            self.symbol_for_declaration(id.into_any()),
            parameter.declared_type(),
        ) {
            let symbol_infer = self.infer_symbol(symbol);
            let annotation = self.source_type_infer(annotation);

            self.push_constraint(Constraint::Equals {
                left: symbol_infer,
                right: annotation,
            });
        }

        dir::walk_parameter(self, tree, id, parameter);
    }

    /// Return the inference variable for a source type expression.
    pub(in crate::check) fn source_type_infer(
        &mut self,
        type_id: dir::LocalNodeId<dir::TypeExpression>,
    ) -> TypeInferId {
        let node = type_id.into_global_any(self.module());
        let infer = self.infer_node(node);

        self.push_constraint(Constraint::Bind {
            result: infer,
            term: TypeTerm::TypeExpression(type_id.into_global(self.module())),
        });

        infer
    }

    /// Walk an identifier read.
    fn walk_identifier(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        name: dir::StringId,
        infer: TypeInferId,
    ) {
        let key = dir::StaticKey::Name(name);
        let symbols = self.resolve_name_symbols(id.into_any(), key, dir::SymbolSpace::Value);
        let resolution = dir::NameResolution::from_symbols(symbols.clone());
        let node = id.into_global_any(self.module());

        self.resolutions_mut().set_name_resolution(node, resolution);

        if let Some(symbol) = symbols.first().copied() {
            let symbol = self.infer_symbol(symbol);

            self.push_constraint(Constraint::Equals {
                left: infer,
                right: symbol,
            });
        }
    }

    /// Walk a scalar literal expression.
    fn walk_scalar_literal(&mut self, value: &dir::ScalarLiteral, infer: TypeInferId) {
        self.push_constraint(Constraint::Bind {
            result: infer,
            term: TypeTerm::Type(dir::Type::from(value)),
        });
    }

    /// Walk a member expression.
    fn walk_member_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        name: Option<dir::StringId>,
        result: TypeInferId,
    ) {
        let Some(name) = name else {
            return;
        };
        let receiver = self.infer_node(left.into_global_any(self.module()));
        let key = dir::StaticKey::Name(name);
        let node = id.into_global_any(self.module());

        self.push_constraint(Constraint::SelectMember {
            node,
            receiver,
            key,
            result,
        });
    }

    /// Walk a call expression.
    fn walk_call_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        arguments: &[dir::LocalNodeId<dir::Argument>],
        result: TypeInferId,
    ) {
        let callee = self.infer_node(left.into_global_any(self.module()));
        let arguments: Vec<TypeInferId> = arguments
            .iter()
            .filter_map(|argument| self.argument_infer(*argument))
            .collect();
        let node = id.into_global_any(self.module());

        self.push_constraint(Constraint::SelectCall {
            node,
            callee,
            arguments: arguments.clone(),
            result,
        });

        self.push_callable(callee, arguments, node, id.into_any());
    }

    /// Walk a binary expression.
    fn walk_binary_expression(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        right: dir::LocalNodeId<dir::Expression>,
        result: TypeInferId,
    ) {
        let left = self.infer_node(left.into_global_any(self.module()));
        let right = self.infer_node(right.into_global_any(self.module()));
        let node = id.into_global_any(self.module());

        self.push_constraint(Constraint::SelectBinaryOperator {
            node,
            operator,
            left,
            right,
            result,
        });
    }

    /// Walk a simple assignment expression.
    fn walk_assignment(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
        left: dir::LocalNodeId<dir::AssignPattern>,
        right: dir::LocalNodeId<dir::Expression>,
    ) {
        let Some(target) = self.assignment_target_infer(left) else {
            return;
        };
        let result = self.infer_node(id.into_global_any(self.module()));
        let source = self.infer_node(right.into_global_any(self.module()));

        self.push_constraint(Constraint::Equals {
            left: result,
            right: target,
        });

        self.push_assignable(
            source,
            target,
            ObligationSource::DeclarationType {
                declaration: id.into_global_any(self.module()),
            },
            right.into_any(),
        );
    }

    /// Walk the symbol introduced by one declarator.
    fn walk_declarator_symbol(
        &mut self,
        symbol: dir::GlobalSymbolId,
        annotation: Option<TypeInferId>,
        value: Option<TypeInferId>,
    ) {
        let symbol_infer = self.infer_symbol(symbol);
        let source = annotation.or(value);

        if let Some(source) = source {
            self.push_constraint(Constraint::Equals {
                left: symbol_infer,
                right: source,
            });
        }
    }

    /// Push one assignability obligation.
    fn push_assignable(
        &mut self,
        source: TypeInferId,
        target: TypeInferId,
        source_context: ObligationSource,
        anchor_node: dir::LocalNodeIdAny,
    ) {
        let context = ObligationContext {
            anchor: self.anchor_node(anchor_node),
            source: source_context,
        };

        self.push_obligation(Obligation::Assignable {
            source,
            target,
            context,
        });
    }

    /// Push one call selection obligation.
    fn push_callable(
        &mut self,
        callee: TypeInferId,
        arguments: Vec<TypeInferId>,
        call: dir::GlobalNodeIdAny,
        anchor_node: dir::LocalNodeIdAny,
    ) {
        let context = ObligationContext {
            anchor: self.anchor_node(anchor_node),
            source: ObligationSource::Call { call },
        };

        self.push_obligation(Obligation::Callable {
            call,
            callee,
            arguments,
            context,
        });
    }
}
