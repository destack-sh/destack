use destack_dir as dir;

use crate::check::{
    CheckEvent, FlowPath, NameLookup, ShapeMember, TypeLiteralTerm, TypeOperand, TypeOperationTerm,
    TypeTerm, WalkState,
};

/// Runtime flow predicate used to narrow one stable path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum NarrowPredicate {
    /// Keep values assignable to the target type.
    Is(TypeOperand),
    /// Keep values not assignable to the target type.
    IsNot(TypeOperand),
    /// Keep objects with a known member key.
    HasKey(dir::StaticKey),
}

impl WalkState<'_, '_> {
    /// Return the stable flow path for one expression.
    pub(in crate::check) fn flow_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<FlowPath> {
        match self.tree.get(id) {
            // value
            dir::Expression::Identifier { name } => {
                let guard = self.active_static_guard();

                // resolve root binding
                let lookup = self
                    .check.lookup_name_by_name(
                        self.module,
                        id.into_any(),
                        *name,
                        dir::SymbolSpace::Value,
                    )
                    .available_under(&guard);
                let symbol = match lookup {
                    NameLookup::Found(candidate) => candidate.symbol()?,
                    NameLookup::Missing => return None,
                    NameLookup::Ambiguous(_) => return None,
                };

                Some(FlowPath::symbol(symbol))
            }
            // namespace
            dir::Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                let guard = self.active_static_guard();

                // resolve root binding
                let name = path.segments[0];
                let lookup = self
                    .check.lookup_name_by_name(
                        self.module,
                        id.into_any(),
                        name,
                        dir::SymbolSpace::Value,
                    )
                    .available_under(&guard);
                let symbol = match lookup {
                    NameLookup::Found(candidate) => candidate.symbol()?,
                    NameLookup::Missing => return None,
                    NameLookup::Ambiguous(_) => return None,
                };

                Some(FlowPath::symbol(symbol))
            }
            // this
            dir::Expression::This if self.flow().current_receiver().is_some() => {
                Some(FlowPath::receiver(dir::ReceiverKind::This))
            }
            // value.member
            dir::Expression::Member {
                left,
                name: Some(name),
            }
            // value.#member
            | dir::Expression::PrivateMember {
                left,
                name: Some(name),
            } => {
                // extend root path with selected member
                let mut path = self.flow_path(*left)?;
                path.push_segment(dir::StaticKey::Name(*name));

                Some(path)
            }
            // value[index]
            dir::Expression::Index {
                left,
                index: Some(index),
                ..
            } => {
                // extend root path with static index key
                let key = self.tree.get(*index).static_key()?;
                let mut path = self.flow_path(*left)?;

                path.push_segment(key);

                Some(path)
            }
            // (value)
            dir::Expression::Parenthesized { expression } => self.flow_path(*expression),
            // not a stable flow path
            _ => None,
        }
    }

    /// Return the current narrowing for one flow path.
    pub(in crate::check) fn flow_path_narrowing(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<TypeOperand> {
        // resolve path before reading narrowing table
        let path = self.flow_path(id)?;

        self.flow().narrowing(&path)
    }

    /// Narrow one flow path to an exact type operand.
    pub(in crate::check) fn narrow_flow_path(
        &mut self,
        path: FlowPath,
        ty: impl Into<TypeOperand>,
    ) {
        let ty = ty.into();

        self.check.record_event(CheckEvent::FlowNarrow {
            path: path.clone(),
            value: ty,
        });
        self.flow_mut().narrow(path, ty);
    }

    /// Narrow one flow path with a runtime predicate.
    pub(in crate::check) fn narrow_flow_path_by(
        &mut self,
        path: FlowPath,
        source: TypeOperand,
        predicate: NarrowPredicate,
    ) {
        match predicate {
            // keep matching values
            NarrowPredicate::Is(target) => {
                let operation = self
                    .check
                    .inference
                    .push_term(TypeOperationTerm::Extract { source, target });
                let narrowed = self
                    .check
                    .inference
                    .push_term(TypeTerm::Operation(operation));

                self.narrow_flow_path(path, narrowed);
            }

            // keep non-matching values
            NarrowPredicate::IsNot(target) => {
                let operation = self
                    .check
                    .inference
                    .push_term(TypeOperationTerm::Exclude { source, target });
                let narrowed = self
                    .check
                    .inference
                    .push_term(TypeTerm::Operation(operation));

                self.narrow_flow_path(path, narrowed);
            }

            // keep objects with the requested key
            NarrowPredicate::HasKey(key) => {
                let unknown = self
                    .check
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Unknown))
                    .into();
                let target = self.member_shape_type(key, unknown);
                let predicate = NarrowPredicate::Is(target);

                self.narrow_flow_path_by(path, source, predicate);
            }
        }
    }

    /// Narrow one base flow path from a member predicate.
    pub(in crate::check) fn narrow_base_flow_path_by_member(
        &mut self,
        path: FlowPath,
        source: TypeOperand,
        key: dir::StaticKey,
        predicate: NarrowPredicate,
    ) {
        let predicate = match predicate {
            // keep parent values with a matching member type
            NarrowPredicate::Is(ty) => {
                let target = self.member_shape_type(key, ty);

                NarrowPredicate::Is(target)
            }

            // keep parent values without a matching member type
            NarrowPredicate::IsNot(ty) => {
                let target = self.member_shape_type(key, ty);

                NarrowPredicate::IsNot(target)
            }

            // keep parent values with a member that has the nested key
            NarrowPredicate::HasKey(member_key) => {
                let unknown = self
                    .check
                    .inference
                    .push_term(TypeTerm::Literal(TypeLiteralTerm::Unknown))
                    .into();
                let member = self.member_shape_type(member_key, unknown);
                let target = self.member_shape_type(key, member);

                NarrowPredicate::Is(target)
            }
        };

        self.narrow_flow_path_by(path, source, predicate);
    }

    /// Return one structural member shape type.
    fn member_shape_type(&mut self, key: dir::StaticKey, ty: TypeOperand) -> TypeOperand {
        let member = ShapeMember::Field {
            key,
            ty,
            is_optional: false,
            is_readonly: false,
        };
        let target = self.check.push_shape_type(vec![member].into());

        self.check.inference.push_term(target).into()
    }

    /// Clear flow narrowings invalidated by mutating an expression.
    pub(in crate::check) fn clear_mutated_expression_narrowings(
        &mut self,
        id: dir::LocalNodeId<dir::Expression>,
    ) {
        // ignore expressions without stable flow paths
        let Some(path) = self.flow_path(id) else {
            return;
        };

        // clear all dependent narrowings
        self.flow_mut().clear_narrowings_under(&path);
    }
}
