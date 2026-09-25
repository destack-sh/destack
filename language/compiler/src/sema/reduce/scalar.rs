use smallvec::SmallVec;
use tspp_dir as dir;

use crate::CompilerResult;
use crate::sema::{CheckState, Origin, VariableKind};

/// Scalar interpretation requested from one type.
#[derive(Clone, Copy)]
enum ScalarUse {
    /// Runtime values belonging to scalar families.
    Value,
    /// Compiler-defined scalar operator behavior.
    Builtin,
}

impl CheckState<'_> {
    /// Return the scalar families one type can hold, or `None` for every other scalar.
    pub(in crate::sema) fn scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        // reuse the families a closed type already decided
        let ty = self.shallow_resolve(ty)?;
        let flags = self.type_flags(ty)?;
        let is_closed = !flags.has_variable() && !flags.has_parameter() && !flags.has_this();
        if is_closed && let Some(families) = self.scalar_families.get(&ty) {
            return Ok(families.clone());
        }

        // walk the type for its families and record them
        let mut parameters = SmallVec::new();
        let families = self.type_scalar_families(origin, ty, ScalarUse::Value, &mut parameters)?;
        if is_closed {
            self.scalar_families.insert(ty, families.clone());
        }

        Ok(families)
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

    /// Return the scalar families admitted for one requested use.
    fn type_scalar_families(
        &mut self,
        origin: Origin,
        ty: dir::GlobalTypeId,
        use_: ScalarUse,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
    ) -> CompilerResult<Option<dir::ScalarFamilySet>> {
        // read the families by the head of the type
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

            // read an alias through the value it names
            dir::Type::Application(instance)
                if let Some(value) = self.type_alias_body(origin, root.module_id, &instance)? =>
            {
                self.type_scalar_families(origin, value, use_, parameters)?
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
        // an open integer variable is an integer until its uses decide the width
        if let dir::Type::Variable(variable) = ty
            && let root = self.infer.alias_root(*variable)?
            && self.infer.variable(root)?.kind == VariableKind::Integer
        {
            return Ok(Some(
                dir::ScalarFamily::Domain(dir::ScalarDomain::Integer).into(),
            ));
        }

        // runtime values inherit the physical family of transparent newtypes
        if matches!(use_, ScalarUse::Value)
            && let Some(instance) = self.decompose_newtype(origin, root)?
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
                        self.definition(instance.symbol)?.as_deref(),
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
                self.definition(instance.symbol)?.as_deref(),
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

        // read the family a scalar leaf belongs to
        let families = ty
            .scalar_domain()
            .map(|domain| dir::ScalarFamily::Domain(domain).into());

        Ok(families)
    }

    /// Collect the exact scalar formats admitted by one builtin constraint.
    pub(in crate::sema) fn collect_builtin_scalar_formats(
        &mut self,
        origin: Origin,
        target: dir::GlobalTypeId,
        families: &dir::ScalarFamilySet,
        parameters: &mut SmallVec<[dir::GlobalGenericParameterId; 4]>,
        formats: &mut SmallVec<[dir::PrimitiveType; 4]>,
    ) -> CompilerResult<()> {
        // collect the formats the target names
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
}
