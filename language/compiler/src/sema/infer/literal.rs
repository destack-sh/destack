use dir::NodeVisitor as _;
use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{BoundSide, CheckState, Origin, Value, VariableKind};

/// The context one expression is inferred in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::sema) enum InferMode {
    /// Aggregate slots widen the fresh literals they store.
    Regular,
    /// A literal context keeps the literals it stores.
    Literal,
    /// A const context keeps literals and makes its aggregates readonly.
    Const,
}

/// The node form one node produces a value through.
pub(in crate::sema) enum NodeForm {
    /// A literal or template constant.
    Literal,
    /// An array, tuple, or object literal.
    Composite,
    /// A block, conditional, or match producing the values of its branches.
    Branching(SmallVec<[dir::GlobalNodeIdAny; 4]>),
    /// An expression producing another expression's value without a separate context.
    Forward(SmallVec<[dir::GlobalNodeIdAny; 4]>),
    /// A function value.
    FunctionValue,
    /// A name read.
    Name,
    /// Every other expression.
    Other,
}

/// The values the breaks of one loop pass out.
struct BreakValues {
    /// The loop's label.
    label: Option<dir::StringId>,
    /// How many unlabeled loops enclose the visited node inside the loop.
    nesting: u32,
    /// The collected break values.
    values: SmallVec<[dir::LocalNodeId<dir::Expression>; 4]>,
}

impl dir::NodeVisitor for BreakValues {
    /// Collect the value of every break leaving the loop, skipping nested functions.
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        match expression {
            dir::Expression::Break { label, value } => {
                let leaves = match label {
                    Some(label) => Some(*label) == self.label,
                    None => self.nesting == 0,
                };
                if leaves && let Some(value) = value {
                    self.values.push(*value);
                }
            }
            dir::Expression::Loop { .. }
            | dir::Expression::While { .. }
            | dir::Expression::For { .. }
            | dir::Expression::ForEach { .. } => {
                self.nesting += 1;
                dir::walk_expression(self, tree, id, expression);
                self.nesting -= 1;
            }
            dir::Expression::Declaration(_) => {}
            _ => dir::walk_expression(self, tree, id, expression),
        }
    }
}

impl InferMode {
    /// Return whether this mode keeps literals.
    pub(in crate::sema) fn keeps_literals(self) -> bool {
        self != Self::Regular
    }

    /// Return whether aggregates inferred in this mode are readonly.
    pub(in crate::sema) fn is_readonly(self) -> bool {
        self == Self::Const
    }
}

impl CheckState<'_> {
    /// Return the type of one literal expression.
    pub(in crate::sema) fn literal_type(
        &mut self,
        value: dir::Literal,
    ) -> CompilerResult<dir::GlobalTypeId> {
        match value {
            // read a regex literal as a RegExp object
            dir::Literal::RegexString { .. } => self.language_type(dir::LanguageItem::RegExp, &[]),
            dir::Literal::Null => self.intern_type(dir::Type::Null),
            dir::Literal::Undefined => self.intern_type(dir::Type::Undefined),
            value => self.intern_type(dir::Type::Literal(value)),
        }
    }

    /// Return the numeric variable kind one literal widens into, none for every other literal.
    pub(in crate::sema) fn numeric_literal_kind(
        &self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<VariableKind>> {
        Ok(match self.ty(ty)? {
            dir::Type::Literal(dir::Literal::Integer(_)) => Some(VariableKind::Integer),
            dir::Type::Literal(dir::Literal::Float(_)) => Some(VariableKind::Float),
            _ => None,
        })
    }

    /// Widen one fresh type onto the base its leaves name.
    pub(in crate::sema) fn widen_fresh(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<dir::GlobalTypeId> {
        // widen by the head of the type
        let ty = self.shallow_resolve(ty)?;
        match self.ty(ty)? {
            // widen a numeric literal into a variable its uses decide
            dir::Type::Literal(dir::Literal::Integer(_) | dir::Literal::Float(_)) => {
                let kind = self.numeric_literal_kind(ty)?.expect("numeric literal");
                let variable = self.open_variable_of(origin, kind);

                self.variable_type(variable)
            }
            // widen a variant literal to its enum
            dir::Type::Variant(variant) => Ok(variant.owner),
            // widen a template expression to the string it prints, an object in constant space
            dir::Type::Operation(operation)
                if matches!(
                    self.type_operation(ty.module_id, operation)?,
                    dir::TypeOperation::TemplateLiteral(_)
                ) =>
            {
                self.intern_type(dir::Type::Primitive(dir::PrimitiveType::String))
            }
            // widen a union member-wise
            dir::Type::Union(union) => {
                let arms =
                    SmallVec::<[_; 4]>::from_slice(self.type_ids(ty.module_id, union.elements)?);
                let mut widened = SmallVec::<[dir::GlobalTypeId; 4]>::new();
                for arm in arms {
                    widened.push(self.widen_fresh(origin, arm)?);
                }

                self.normalized_union_type(widened)
            }
            // widen every other leaf onto its base
            _ => self.widen_type(ty),
        }
    }

    /// Return the candidate one value contributes to the open slot it stores into.
    pub(in crate::sema) fn store_candidate(
        &mut self,
        origin: Origin,
        value: Value,
        slot: dir::GlobalTypeId,
        keeps_literals: bool,
    ) -> CompilerResult<Value> {
        // widen fresh literal values alone
        if !value.is_fresh || keeps_literals || !self.is_literal_shape(value.ty)? {
            return Ok(value);
        }

        // keep the literals the slot's parameter, written type, or upper bounds name
        let keeps = match self.root_variable(slot)? {
            Some(variable) => match self.infer.variable(variable)?.parameter {
                Some(parameter) => {
                    self.parameter_keeps_literals(origin, parameter, value.ty, false)?
                }
                None => {
                    let uppers = self
                        .infer
                        .variables
                        .side_bounds(variable, BoundSide::Upper)?
                        .map(|bound| bound.ty)
                        .collect::<SmallVec<[_; 2]>>();
                    let mut keeps = false;
                    for upper in uppers {
                        keeps |= self.type_keeps_literal(origin, value.ty, upper, false)?;
                    }

                    keeps
                }
            },
            None => self.type_keeps_literal(origin, value.ty, slot, false)?,
        };
        if keeps {
            return Ok(value);
        }

        self.widen_candidate(origin, value, false)
    }

    /// Widen one fresh literal-shaped value unless its destination keeps literals.
    pub(in crate::sema) fn widen_candidate(
        &mut self,
        origin: Origin,
        value: Value,
        keeps_literals: bool,
    ) -> CompilerResult<Value> {
        // widen fresh literal values alone
        if !value.is_fresh || keeps_literals || !self.is_literal_shape(value.ty)? {
            return Ok(value);
        }

        // widen the value onto its base
        let ty = self.widen_fresh(origin, value.ty)?;

        Ok(Value { ty, ..value })
    }

    /// Return the form one node produces its value through.
    pub(in crate::sema) fn node_form(&self, node: dir::GlobalNodeIdAny) -> NodeForm {
        // read the expression standing at the node
        let module = node.module_id;
        let view = self.module(module).view();
        match node.local_id.ty {
            // a block produces its tail expression
            dir::NodeType::Block => {
                let block = view.get(node.local_id.into_typed::<dir::Block>());
                let mut values = SmallVec::new();
                values.extend(
                    block
                        .tail_expression
                        .map(|tail| tail.into_global_any(module)),
                );

                NodeForm::Branching(values)
            }
            // produce an expression's value by its form
            dir::NodeType::Expression => {
                let expression = node.local_id.into_typed::<dir::Expression>();
                match view.get(expression) {
                    dir::Expression::Literal(_) | dir::Expression::TemplateExpression { .. } => {
                        NodeForm::Literal
                    }
                    dir::Expression::ArrayExpression { .. }
                    | dir::Expression::FixedArrayExpression { .. }
                    | dir::Expression::TupleExpression { .. }
                    | dir::Expression::ObjectExpression { .. } => NodeForm::Composite,
                    dir::Expression::Declaration(declaration) => {
                        match self.is_function_value_declaration(node, *declaration) {
                            Ok(true) => NodeForm::FunctionValue,
                            _ => NodeForm::Other,
                        }
                    }
                    dir::Expression::Identifier { .. } => NodeForm::Name,
                    // produce the values a loop's breaks pass out
                    dir::Expression::Loop { label, body } => {
                        let mut breaks = BreakValues {
                            label: *label,
                            nesting: 0,
                            values: SmallVec::new(),
                        };
                        let tree = view.tree();
                        breaks.visit_block(tree, *body, tree.get(*body));

                        NodeForm::Forward(
                            breaks
                                .values
                                .into_iter()
                                .map(|value| value.into_global_any(module))
                                .collect(),
                        )
                    }
                    // produce the value an assignment assigns
                    dir::Expression::Assign { right, .. } => {
                        let mut values = SmallVec::new();
                        values.push(right.into_global_any(module));

                        NodeForm::Forward(values)
                    }
                    // produce a block's tail expression
                    dir::Expression::Block(block) => {
                        let block = view.get(*block);
                        let mut values = SmallVec::new();
                        values.extend(
                            block
                                .tail_expression
                                .map(|tail| tail.into_global_any(module)),
                        );

                        NodeForm::Branching(values)
                    }
                    // produce the values of both branches
                    dir::Expression::If {
                        then_expression,
                        else_expression,
                        ..
                    } => {
                        let mut values = SmallVec::new();
                        values.push(then_expression.into_global_any(module));
                        values.extend(else_expression.map(|node| node.into_global_any(module)));

                        NodeForm::Branching(values)
                    }
                    // produce the value of every match arm
                    dir::Expression::Match { arms, .. } => {
                        let mut values = SmallVec::new();
                        for arm in arms {
                            values.push(match view.get(*arm) {
                                dir::MatchArm::Expression { body, .. } => {
                                    body.into_global_any(module)
                                }
                                dir::MatchArm::Block { body, .. } => body.into_global_any(module),
                            });
                        }

                        NodeForm::Branching(values)
                    }
                    // produce either coalesce operand
                    dir::Expression::Binary {
                        operator: dir::BinaryOperator::Coalesce,
                        left,
                        right,
                    } => NodeForm::Branching(SmallVec::from_slice(&[
                        left.into_global_any(module),
                        right.into_global_any(module),
                    ])),
                    _ => NodeForm::Other,
                }
            }
            _ => NodeForm::Other,
        }
    }

    /// Return the values one branching node produces through its branches.
    pub(in crate::sema) fn branching_value_nodes(
        &self,
        node: dir::GlobalNodeIdAny,
    ) -> Option<SmallVec<[dir::GlobalNodeIdAny; 4]>> {
        match self.node_form(node) {
            NodeForm::Branching(values) => Some(values),
            _ => None,
        }
    }

    /// Return whether one node produces a fresh value, open to widening at its destination.
    pub(in crate::sema) fn is_fresh_node(
        &mut self,
        node: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        Ok(match self.node_form(node) {
            NodeForm::Literal | NodeForm::Composite | NodeForm::FunctionValue => true,
            NodeForm::Name => self.is_fresh_const_read(node)?,
            // keep a branching node fresh while every branch value is fresh
            NodeForm::Branching(values) | NodeForm::Forward(values) => {
                let mut is_fresh = true;
                for value in values {
                    is_fresh &= self.is_fresh_node(value)?;
                }

                is_fresh
            }
            // read a variant access as the literal form of its variant
            NodeForm::Other => match self.own_node_type(node) {
                Some(ty) => matches!(self.ty_raw(ty)?, dir::Type::Variant(_)),
                None => false,
            },
        })
    }

    /// Return whether one node checks under its destination as context.
    pub(in crate::sema) fn is_composite_node(&self, node: dir::GlobalNodeIdAny) -> bool {
        matches!(
            self.node_form(node),
            NodeForm::Composite | NodeForm::Branching(_)
        )
    }

    /// Return whether one composite or function value node takes its type from its expectation.
    pub(in crate::sema) fn is_contextually_typed(&self, node: dir::GlobalNodeIdAny) -> bool {
        self.is_composite_node(node) || matches!(self.node_form(node), NodeForm::FunctionValue)
    }

    /// Return whether one name reads an unannotated const binding initialized by a fresh value.
    fn is_fresh_const_read(&mut self, node: dir::GlobalNodeIdAny) -> CompilerResult<bool> {
        // require a unique const binding behind the name
        let module = node.module_id;
        let symbols = match self.name_decision(node) {
            Some(resolution) => resolution.symbols().to_vec(),
            None => match self.module(module).resolved.references.get(node) {
                Some(dir::Reference::Bound(symbols)) => self.present_symbols(symbols).to_vec(),
                _ => return Ok(false),
            },
        };
        let [symbol] = symbols.as_slice() else {
            return Ok(false);
        };
        let symbol = *symbol;
        let binding = self
            .binding_table(symbol.module_id)?
            .get_symbol(symbol.local_id)
            .clone();
        if binding.binding_mutability != Some(dir::Mutability::Immutable) {
            return Ok(false);
        }

        // read a foreign const's committed type, a local one's recorded freshness
        let Some(declaration) = binding.declaration else {
            return Ok(false);
        };
        if self.module_maybe(declaration.module_id).is_none() {
            return Ok(self
                .symbol_type_maybe(symbol)?
                .map(|ty| self.is_literal_shape(ty))
                .transpose()?
                .unwrap_or(false));
        }

        Ok(self.fresh_consts.contains(&symbol))
    }
}
