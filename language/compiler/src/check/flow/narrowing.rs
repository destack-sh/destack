use destack_dir as dir;

use crate::CompilerResult;
use crate::check::{FlowPath, WalkState};

/// Runtime flow predicate used to narrow one stable path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum NarrowPredicate {
    /// Keep values assignable to the target type.
    Is(dir::GlobalTypeId),
    /// Keep values not assignable to the target type.
    IsNot(dir::GlobalTypeId),
    /// Keep objects with a known member key.
    Has(dir::StaticKey),
}

impl WalkState<'_, '_> {
    /// Return the stable flow path for one expression.
    pub(in crate::check) fn flow_path(
        &self,
        id: dir::LocalNodeId<dir::Expression>,
    ) -> Option<FlowPath> {
        match self.tree.get(id) {
            // value
            dir::Expression::Identifier { .. } => {
                let reference = self
                    .check
                    .module(self.module)
                    .resolved
                    .references
                    .get(id.into_global_any(self.module))?;
                let symbol = match reference {
                    dir::Reference::Bound(symbols) => {
                        let symbols = self.check.available_symbols(symbols);
                        match symbols.as_slice() {
                            [symbol] => *symbol,
                            _ => return None,
                        }
                    }
                    dir::Reference::Missing
                    | dir::Reference::Namespace(_)
                    | dir::Reference::Projected { .. }
                    | dir::Reference::Ambiguous(_) => return None,
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
    ) -> Option<dir::GlobalTypeId> {
        // resolve path before reading narrowing table
        let path = self.flow_path(id)?;

        self.flow().narrowing(&path)
    }

    /// Narrow one flow path to an exact type.
    pub(in crate::check) fn narrow_flow_path(&mut self, path: FlowPath, ty: dir::GlobalTypeId) {
        self.flow_mut().narrow(path, ty);
    }

    /// Narrow one flow path with a runtime predicate.
    pub(in crate::check) fn narrow_flow_path_by(
        &mut self,
        path: FlowPath,
        source: dir::GlobalTypeId,
        source_node: dir::LocalNodeIdAny,
        predicate: NarrowPredicate,
    ) -> CompilerResult<()> {
        let narrowed = match predicate {
            // keep matching values
            NarrowPredicate::Is(target) => {
                let operation = dir::TypeOperation::Narrow(dir::NarrowType {
                    source,
                    target,
                    is_positive: true,
                });

                self.push_type(dir::Type::Operation(operation), source_node)?
            }

            // keep non-matching values
            NarrowPredicate::IsNot(target) => {
                let operation = dir::TypeOperation::Narrow(dir::NarrowType {
                    source,
                    target,
                    is_positive: false,
                });

                self.push_type(dir::Type::Operation(operation), source_node)?
            }

            // keep objects with the requested key
            NarrowPredicate::Has(key) => {
                let unknown = self.push_type(dir::Type::Unknown, source_node)?;
                let target = self.member_shape_type(key, unknown, source_node)?;
                let predicate = NarrowPredicate::Is(target);

                return self.narrow_flow_path_by(path, source, source_node, predicate);
            }
        };
        self.narrow_flow_path(path, narrowed);

        Ok(())
    }

    /// Narrow one base flow path from a member predicate.
    pub(in crate::check) fn narrow_base_flow_path_by_member(
        &mut self,
        path: FlowPath,
        source: dir::GlobalTypeId,
        source_node: dir::LocalNodeIdAny,
        key: dir::StaticKey,
        predicate: NarrowPredicate,
    ) -> CompilerResult<()> {
        let predicate = match predicate {
            // keep parent values with a matching member type
            NarrowPredicate::Is(ty) => {
                let target = self.member_shape_type(key, ty, source_node)?;

                NarrowPredicate::Is(target)
            }

            // keep parent values without a matching member type
            NarrowPredicate::IsNot(ty) => {
                let target = self.member_shape_type(key, ty, source_node)?;

                NarrowPredicate::IsNot(target)
            }

            // keep parent values with a member that has the nested key
            NarrowPredicate::Has(member_key) => {
                let unknown = self.push_type(dir::Type::Unknown, source_node)?;
                let member = self.member_shape_type(member_key, unknown, source_node)?;
                let target = self.member_shape_type(key, member, source_node)?;

                NarrowPredicate::Is(target)
            }
        };

        self.narrow_flow_path_by(path, source, source_node, predicate)
    }

    /// Return one single-field structural shape type.
    fn member_shape_type(
        &mut self,
        key: dir::StaticKey,
        ty: dir::GlobalTypeId,
        source_node: dir::LocalNodeIdAny,
    ) -> CompilerResult<dir::GlobalTypeId> {
        let field = dir::TypeField {
            key,
            ty,
            is_optional: false,
            is_readonly: false,
        };
        let shape = dir::ShapeType {
            fields: vec![field],
            call_signatures: Vec::new(),
            construct_signatures: Vec::new(),
            index_signatures: Vec::new(),
        };

        self.push_type(dir::Type::Shape(shape), source_node)
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
