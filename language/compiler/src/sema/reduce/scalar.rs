use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{CheckState, Origin};
use crate::{CompilerError, CompilerResult};

/// Scalar interpretation requested from one type.
#[derive(Clone, Copy)]
enum ScalarUse {
    /// Runtime values belonging to scalar families.
    Value,
    /// Compiler-defined scalar operator behavior.
    Builtin,
}

impl CheckState<'_> {
    /// Return the scalar families one type can hold, or `None` when it is not scalar.
    pub(in crate::sema) fn scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        let mut parameters = SmallVec::new();

        self.type_scalar_families(origin, ty, ScalarUse::Value, &mut parameters)
    }

    /// Return the scalar families with compiler-defined closed operations.
    pub(in crate::sema) fn builtin_scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        let mut parameters = SmallVec::new();

        self.type_scalar_families(origin, ty, ScalarUse::Builtin, &mut parameters)
    }

    /// Return whether one literal is accepted by a builtin scalar operand.
    pub(in crate::sema) fn builtin_scalar_accepts_literal(
        &mut self,
        origin: Origin,
        source: dir::GlobalTypeId,
        target: dir::GlobalTypeId,
    ) -> CompilerResult<bool> {
        let dir::Type::Literal(literal) = self.ty(source)? else {
            return Err(CompilerError::Internal {
                message: format!("builtin scalar operand {source:?} is not a literal"),
            });
        };
        let Some(families) = self.builtin_scalar_families(origin, target)? else {
            return Ok(false);
        };
        let mut formats = SmallVec::new();
        let mut parameters = SmallVec::new();
        self.collect_builtin_scalar_formats(
            origin,
            target,
            &families,
            &mut parameters,
            &mut formats,
        )?;

        // require representability in every exact format admitted by each family
        for family in families.iter().copied() {
            let dir::ScalarFamily::Domain(domain) = family else {
                return Ok(false);
            };
            let mut family_formats = formats
                .iter()
                .copied()
                .filter(|format| format.scalar_domain() == domain)
                .peekable();
            if family_formats.peek().is_some() {
                if !family_formats.all(|format| literal.widens_to_primitive(format)) {
                    return Ok(false);
                }
            } else if !literal.widens_to_domain(domain) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return the scalar families admitted for one requested use.
    fn type_scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        use_: ScalarUse,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        let root = self.normalize(origin, ty)?;
        let families = match self.ty(root)? {
            // union alternatives contribute every possible family
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(root.module_id, union.elements)?.into();

                self.union_scalar_families(origin, &elements, use_, parameters)?
            }

            // intersection conjuncts retain only shared scalar families
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(root.module_id, intersection.elements)?.into();

                self.intersect_scalar_families(origin, &elements, use_, parameters)?
            }

            // parameter bounds narrow their possible families conjunctively
            dir::Type::Parameter(parameter) => {
                if parameters.contains(&parameter) {
                    return Ok(None);
                }
                let bounds = self.parameter_bounds(origin, parameter)?;
                parameters.push(parameter);
                let families = self.intersect_scalar_families(origin, &bounds, use_, parameters);
                parameters.pop();

                families?
            }

            // classify the remaining leaf for the requested behavior
            ty => self.scalar_leaf_families(origin, root, &ty, use_, parameters)?,
        };
        let families = families.filter(|families| !families.is_empty());

        Ok(families)
    }

    /// Union the families contributed by every scalar alternative.
    fn union_scalar_families(
        &mut self,
        origin: Origin,
        elements: &[dir::GlobalTypeId],
        use_: ScalarUse,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        let mut families = dir::ScalarFamilySet::new();
        for element in elements {
            let Some(element) = self.type_scalar_families(origin, *element, use_, parameters)?
            else {
                return Ok(None);
            };
            families.extend(element);
        }

        Ok(Some(families))
    }

    /// Intersect the families contributed by scalar conjuncts.
    fn intersect_scalar_families(
        &mut self,
        origin: Origin,
        elements: &[dir::GlobalTypeId],
        use_: ScalarUse,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        let mut intersection: Option<dir::ScalarFamilySet> = None;
        for element in elements {
            let Some(families) = self.type_scalar_families(origin, *element, use_, parameters)?
            else {
                continue;
            };
            intersection = Some(match intersection {
                Some(mut intersection) => {
                    intersection.intersect(&families);

                    intersection
                }
                None => families,
            });
        }

        Ok(intersection)
    }

    /// Return the scalar families contributed by one non-composite type.
    fn scalar_leaf_families(
        &mut self,
        origin: Origin,
        root: dir::GlobalTypeId,
        ty: &dir::Type,
        use_: ScalarUse,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        // runtime values inherit the physical family of transparent newtypes
        if matches!(use_, ScalarUse::Value)
            && let Some(instance) = self.newtype_payload(origin, root)?
        {
            return self.type_scalar_families(origin, instance.backing, use_, parameters);
        }

        // scalar interfaces classify values or builtin representations
        if let dir::Type::Application(instance) = ty {
            let item = self.language_item(instance.symbol)?;
            let domain = match use_ {
                ScalarUse::Value => item.and_then(|item| item.scalar_domain()),
                ScalarUse::Builtin => item.and_then(|item| item.scalar_representation()),
            };
            if let Some(domain) = domain {
                return Ok(Some(dir::ScalarFamily::Domain(domain).into()));
            }
        }

        // builtin use recognizes only compiler-defined scalar leaves
        if matches!(use_, ScalarUse::Builtin) {
            let families = match ty {
                dir::Type::Literal(literal) => literal.scalar_domain(),
                dir::Type::Range(range) => range.scalar_domain(),
                dir::Type::Primitive(primitive) => Some(primitive.scalar_domain()),
                _ => None,
            };
            let families = families.map(|domain| dir::ScalarFamily::Domain(domain).into());

            return Ok(families);
        }

        // enum values retain their declared family
        if let dir::Type::Variant(variant) = ty {
            let owner = self.normalize(origin, variant.owner)?;
            let symbol = match self.ty(owner)? {
                dir::Type::Application(instance)
                    if matches!(
                        self.definition(instance.symbol)?,
                        Some(dir::Definition::Enum(_))
                    ) =>
                {
                    Some(instance.symbol)
                }
                _ => None,
            };

            return Ok(symbol.map(|symbol| dir::ScalarFamily::Enum(symbol).into()));
        }
        if let dir::Type::Application(instance) = ty
            && matches!(
                self.definition(instance.symbol)?,
                Some(dir::Definition::Enum(_))
            )
        {
            let family = dir::ScalarFamily::Enum(instance.symbol).into();

            return Ok(Some(family));
        }

        // static type operations preserve their operand families
        if let dir::Type::Operation(operation) = ty {
            let operation = self.type_operation(root.module_id, *operation)?;
            match operation {
                dir::TypeOperation::TemplateLiteral(_) => {
                    let family = dir::ScalarFamily::Domain(dir::ScalarDomain::String).into();

                    return Ok(Some(family));
                }
                dir::TypeOperation::StaticBinary(binary) if binary.operator.yields_boolean() => {
                    let family = dir::ScalarFamily::Domain(dir::ScalarDomain::Boolean).into();

                    return Ok(Some(family));
                }
                dir::TypeOperation::StaticBinary(binary) => {
                    return self.union_scalar_families(
                        origin,
                        &[binary.left, binary.right],
                        use_,
                        parameters,
                    );
                }
                dir::TypeOperation::StaticUnary(unary) if unary.operator.yields_boolean() => {
                    let family = dir::ScalarFamily::Domain(dir::ScalarDomain::Boolean).into();

                    return Ok(Some(family));
                }
                dir::TypeOperation::StaticUnary(unary) => {
                    return self.type_scalar_families(origin, unary.target, use_, parameters);
                }
                _ => {}
            }
        }

        let families = ty
            .scalar_domain()
            .map(|domain| dir::ScalarFamily::Domain(domain).into());

        Ok(families)
    }

    /// Collect the exact scalar formats admitted by one builtin constraint.
    fn collect_builtin_scalar_formats(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        families: &dir::ScalarFamilySet,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
        formats: &mut SmallVec<[dir::PrimitiveType; 4]>,
    ) -> CompilerResult<()> {
        let target = self.normalize(origin, target)?;
        match self.ty(target)? {
            // retain exact formats from active scalar families
            dir::Type::Primitive(format)
                if families.contains(dir::ScalarFamily::Domain(format.scalar_domain())) =>
            {
                if !formats.contains(&format) {
                    formats.push(format);
                }
            }

            // recurse through every possible union arm
            dir::Type::Union(union) => {
                let elements: SmallVec<[_; 8]> =
                    self.type_ids(target.module_id, union.elements)?.into();
                for element in elements {
                    self.collect_builtin_scalar_formats(
                        origin, element, families, parameters, formats,
                    )?;
                }
            }

            // recurse through every conjunctive bound
            dir::Type::Intersection(intersection) => {
                let elements: SmallVec<[_; 8]> = self
                    .type_ids(target.module_id, intersection.elements)?
                    .into();
                for element in elements {
                    self.collect_builtin_scalar_formats(
                        origin, element, families, parameters, formats,
                    )?;
                }
            }

            // collect formats from every declared and assumed parameter bound
            dir::Type::Parameter(parameter) if !parameters.contains(&parameter) => {
                let bounds = self.parameter_bounds(origin, parameter)?;
                parameters.push(parameter);
                for bound in bounds {
                    // restore the active parameter stack before propagating a failure
                    let collected = self.collect_builtin_scalar_formats(
                        origin, bound, families, parameters, formats,
                    );
                    if let Err(error) = collected {
                        parameters.pop();

                        return Err(error);
                    }
                }
                parameters.pop();
            }

            // broad markers and inactive or nonformat constraints add no exact format
            _ => {}
        }

        Ok(())
    }

    /// Return the scalar result type of one static operation.
    pub(in crate::sema) fn static_operation_type(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut parameters = SmallVec::new();

        self.evaluate_static_operation(origin, ty, &mut parameters)
    }

    /// Evaluate one static operation while tracking recursive parameter bounds.
    fn evaluate_static_operation(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let root = self.normalize(origin, ty)?;
        match self.ty(root)? {
            // binary operators join their operand types
            dir::Type::Operation(operation)
                if let dir::TypeOperation::StaticBinary(binary) =
                    self.type_operation(root.module_id, operation)? =>
            {
                if binary.operator.yields_boolean() {
                    let boolean =
                        self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                    return Ok(Some(boolean));
                }
                let left = self.evaluate_scalar_operand(origin, binary.left, parameters)?;
                let right = self.evaluate_scalar_operand(origin, binary.right, parameters)?;

                // literals adopt their partner operand's type
                let joined = match (left, right) {
                    (Some(left), Some(right)) if left == right => Some(left),
                    (Some(left), None) => Some(left),
                    (None, Some(right)) => Some(right),
                    _ => None,
                };

                Ok(joined)
            }

            // unary operators keep their operand type
            dir::Type::Operation(operation)
                if let dir::TypeOperation::StaticUnary(unary) =
                    self.type_operation(root.module_id, operation)? =>
            {
                if unary.operator.yields_boolean() {
                    let boolean =
                        self.intern_type(dir::Type::Primitive(dir::PrimitiveType::Boolean))?;

                    return Ok(Some(boolean));
                }

                self.evaluate_scalar_operand(origin, unary.target, parameters)
            }

            _ => Ok(None),
        }
    }

    /// Return one operand's concrete scalar type.
    fn evaluate_scalar_operand(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let root = self.normalize(origin, ty)?;
        match self.ty(root)? {
            // concrete scalars type themselves
            dir::Type::Primitive(_) => Ok(Some(root)),

            // literals and ranges adopt their partner operand
            dir::Type::Literal(_) | dir::Type::Range(_) => Ok(None),

            // parameters type through every agreeing concrete scalar bound
            dir::Type::Parameter(parameter) => {
                if parameters.contains(&parameter) {
                    return Ok(None);
                }
                parameters.push(parameter);
                let selected = self.parameter_scalar_type(origin, parameter, parameters);
                parameters.pop();

                selected
            }

            // nested operations type through their own result
            dir::Type::Operation(operation)
                if matches!(
                    self.type_operation(root.module_id, operation)?,
                    dir::TypeOperation::StaticBinary(_) | dir::TypeOperation::StaticUnary(_)
                ) =>
            {
                self.evaluate_static_operation(origin, root, parameters)
            }

            _ => Ok(None),
        }
    }

    /// Return the common concrete scalar type admitted by one parameter's bounds.
    fn parameter_scalar_type(
        &mut self,
        origin: Origin,
        parameter: dir::GlobalGenericParameterId,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::GlobalTypeId>> {
        let mut selected = None;
        for bound in self.parameter_bounds(origin, parameter)? {
            let root = self.normalize(origin, bound)?;
            if matches!(self.ty(root)?, dir::Type::Parameter(_))
                || self
                    .type_scalar_families(origin, bound, ScalarUse::Value, parameters)?
                    .is_none()
            {
                continue;
            }
            let Some(candidate) = self.evaluate_scalar_operand(origin, bound, parameters)? else {
                continue;
            };
            if selected.is_some_and(|selected| selected != candidate) {
                return Ok(None);
            }
            selected = Some(candidate);
        }

        Ok(selected)
    }
}
