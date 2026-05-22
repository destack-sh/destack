use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{CheckComponentState, TypeRelation, VariableId};

/// Result of checking one type relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum RelationResult {
    /// The relation holds.
    Holds,
    /// The relation cannot be decided yet.
    Waits,
    /// The relation does not hold.
    Fails,
}

/// A type id with its owning module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct TypeRef {
    /// The module that owns the type id.
    module: ModuleId,
    /// The local type id.
    id: dir::LocalTypeId,
}

impl TypeRef {
    /// Create one type reference.
    pub(in crate::check) fn new(module: ModuleId, id: dir::LocalTypeId) -> Self {
        Self { module, id }
    }
}

impl CheckComponentState<'_> {
    /// Check one solved type relation.
    pub(in crate::check) fn check_type_relation(
        &self,
        relation: TypeRelation,
        left: VariableId,
        right: VariableId,
    ) -> CompilerResult<RelationResult> {
        let Some(left) = self.type_ref(left)? else {
            return Ok(RelationResult::Waits);
        };
        let Some(right) = self.type_ref(right)? else {
            return Ok(RelationResult::Waits);
        };

        let result = match relation {
            TypeRelation::Equal => self.check_type_equal(left, right)?,
            TypeRelation::Assignable | TypeRelation::Satisfies | TypeRelation::Extends => {
                self.check_type_assignable(left, right)?
            }
            TypeRelation::Implements => self.check_type_assignable(left, right)?,
        };

        Ok(result)
    }

    /// Return one solved type reference.
    pub(in crate::check) fn type_ref(
        &self,
        variable: VariableId,
    ) -> CompilerResult<Option<TypeRef>> {
        let Some(type_id) = self.type_value(variable)? else {
            return Ok(None);
        };

        Ok(Some(TypeRef::new(variable.module, type_id)))
    }

    /// Check exact type equality.
    fn check_type_equal(&self, left: TypeRef, right: TypeRef) -> CompilerResult<RelationResult> {
        if left == right {
            return Ok(RelationResult::Holds);
        }

        let left_type = self.get_type(left)?;
        let right_type = self.get_type(right)?;
        let result = if left_type == right_type {
            RelationResult::Holds
        } else {
            RelationResult::Fails
        };

        Ok(result)
    }

    /// Check assignability from left to right.
    fn check_type_assignable(
        &self,
        source: TypeRef,
        target: TypeRef,
    ) -> CompilerResult<RelationResult> {
        if source == target {
            return Ok(RelationResult::Holds);
        }

        let source_type = self.get_type(source)?;
        let target_type = self.get_type(target)?;

        // handle top and bottom types
        match (&source_type, &target_type) {
            (dir::Type::Error, _) | (_, dir::Type::Error) => return Ok(RelationResult::Holds),
            (dir::Type::Any, _) | (_, dir::Type::Any) => return Ok(RelationResult::Holds),
            (dir::Type::Never, _) => return Ok(RelationResult::Holds),
            (_, dir::Type::Unknown) => return Ok(RelationResult::Holds),
            (dir::Type::Unknown, _) => return Ok(RelationResult::Fails),
            _ => {}
        }

        // compare structural type forms
        let result = match (&source_type, &target_type) {
            (left, right) if left == right => RelationResult::Holds,
            (dir::Type::Literal(literal), target) => self.check_literal_assignable(literal, target),
            (dir::Type::Union(union), _) => {
                self.check_all_assignable(source.module, &union.elements, target)?
            }
            (_, dir::Type::Union(union)) => {
                self.check_any_target_assignable(source, target.module, &union.elements)?
            }
            (_, dir::Type::Intersection(intersection)) => {
                self.check_all_targets_assignable(source, target.module, &intersection.elements)?
            }
            (dir::Type::Tuple(source_tuple), dir::Type::Tuple(target_tuple)) => self
                .check_tuple_assignable(source.module, source_tuple, target.module, target_tuple)?,
            (dir::Type::Shape(source_shape), dir::Type::Shape(target_shape)) => self
                .check_shape_assignable(source.module, source_shape, target.module, target_shape)?,
            (dir::Type::Form(source_form), dir::Type::Form(target_form)) => self
                .check_type_assignable(
                    TypeRef::new(source.module, source_form.value),
                    TypeRef::new(target.module, target_form.value),
                )?,
            _ => RelationResult::Fails,
        };

        Ok(result)
    }

    /// Return one type by reference.
    fn get_type(&self, ty: TypeRef) -> CompilerResult<dir::Type> {
        let ty = self.module(ty.module)?.get_type(ty.id);

        Ok(ty)
    }

    /// Check literal widening assignability.
    fn check_literal_assignable(
        &self,
        literal: &dir::ScalarLiteral,
        target: &dir::Type,
    ) -> RelationResult {
        match (literal, target) {
            (dir::ScalarLiteral::Null, dir::Type::Null) => RelationResult::Holds,
            (dir::ScalarLiteral::Boolean(_), dir::Type::Primitive(dir::PrimitiveType::Boolean)) => {
                RelationResult::Holds
            }
            (
                dir::ScalarLiteral::Character(_),
                dir::Type::Primitive(dir::PrimitiveType::Character),
            ) => RelationResult::Holds,
            (dir::ScalarLiteral::String(_), dir::Type::Primitive(dir::PrimitiveType::String)) => {
                RelationResult::Holds
            }
            (dir::ScalarLiteral::Bigint(_), dir::Type::Primitive(dir::PrimitiveType::Bigint)) => {
                RelationResult::Holds
            }
            (
                dir::ScalarLiteral::Integer(_),
                dir::Type::Primitive(dir::PrimitiveType::Integer(_)),
            ) => RelationResult::Holds,
            (dir::ScalarLiteral::Float(_), dir::Type::Primitive(dir::PrimitiveType::Float(_))) => {
                RelationResult::Holds
            }
            (dir::ScalarLiteral::RegexString { .. }, dir::Type::Object) => RelationResult::Holds,
            _ => RelationResult::Fails,
        }
    }

    /// Check whether all source union elements assign to one target.
    fn check_all_assignable(
        &self,
        source_module: ModuleId,
        sources: &[dir::LocalTypeId],
        target: TypeRef,
    ) -> CompilerResult<RelationResult> {
        for source in sources {
            let source = TypeRef::new(source_module, *source);
            let result = self.check_type_assignable(source, target)?;
            if result != RelationResult::Holds {
                return Ok(result);
            }
        }

        Ok(RelationResult::Holds)
    }

    /// Check whether one source assigns to any target union element.
    fn check_any_target_assignable(
        &self,
        source: TypeRef,
        target_module: ModuleId,
        targets: &[dir::LocalTypeId],
    ) -> CompilerResult<RelationResult> {
        for target in targets {
            let target = TypeRef::new(target_module, *target);
            if self.check_type_assignable(source, target)? == RelationResult::Holds {
                return Ok(RelationResult::Holds);
            }
        }

        Ok(RelationResult::Fails)
    }

    /// Check whether one source assigns to all target intersection elements.
    fn check_all_targets_assignable(
        &self,
        source: TypeRef,
        target_module: ModuleId,
        targets: &[dir::LocalTypeId],
    ) -> CompilerResult<RelationResult> {
        for target in targets {
            let target = TypeRef::new(target_module, *target);
            let result = self.check_type_assignable(source, target)?;
            if result != RelationResult::Holds {
                return Ok(result);
            }
        }

        Ok(RelationResult::Holds)
    }

    /// Check tuple assignability.
    fn check_tuple_assignable(
        &self,
        source_module: ModuleId,
        source: &dir::TupleType,
        target_module: ModuleId,
        target: &dir::TupleType,
    ) -> CompilerResult<RelationResult> {
        if source.elements.len() != target.elements.len() {
            return Ok(RelationResult::Fails);
        }

        // compare each element type positionally
        for (source, target) in source.elements.iter().zip(&target.elements) {
            let source = TypeRef::new(source_module, source.ty);
            let target = TypeRef::new(target_module, target.ty);
            let result = self.check_type_assignable(source, target)?;
            if result != RelationResult::Holds {
                return Ok(result);
            }
        }

        Ok(RelationResult::Holds)
    }

    /// Check structural object assignability.
    fn check_shape_assignable(
        &self,
        source_module: ModuleId,
        source: &dir::ShapeType,
        target_module: ModuleId,
        target: &dir::ShapeType,
    ) -> CompilerResult<RelationResult> {
        for target_field in &target.fields {
            let Some(source_field) = source
                .fields
                .iter()
                .find(|field| field.key == target_field.key)
            else {
                if target_field.is_optional {
                    continue;
                }

                return Ok(RelationResult::Fails);
            };
            let source = TypeRef::new(source_module, source_field.ty);
            let target = TypeRef::new(target_module, target_field.ty);
            let result = self.check_type_assignable(source, target)?;
            if result != RelationResult::Holds {
                return Ok(result);
            }
        }

        Ok(RelationResult::Holds)
    }
}
