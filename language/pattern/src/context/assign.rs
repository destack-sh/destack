use tspp_dir as dir;

use crate::{ContextError, ModuleContext, ProgramContext};

/// The checked and parsed operand order of one active type relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeDirection {
    /// The checked type is assignable to the parsed type.
    CheckedToParsed,
    /// The parsed type is assignable to the checked type.
    ParsedToChecked,
}

/// One active recursive type relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TypeRelation {
    /// The checked type operand.
    checked: dir::GlobalTypeId,
    /// The parsed type operand.
    parsed: dir::LocalNodeId<dir::TypeExpression>,
    /// The operand order.
    direction: TypeDirection,
}

#[allow(clippy::too_many_arguments)]
impl ModuleContext {
    /// Return whether a checked source type is assignable to predicate type syntax.
    pub fn is_assignable(
        &self,
        candidate: dir::LocalNodeIdAny,
        source: dir::GlobalTypeId,
        tree: &dir::Tree,
        target: dir::LocalNodeId<dir::TypeExpression>,
        program: &ProgramContext,
    ) -> Result<bool, ContextError> {
        let mut active = Vec::new();

        self.is_checked_assignable_to_parsed(candidate, source, tree, target, &mut active, program)
    }

    /// Decide whether one checked type is assignable to one parsed type.
    fn is_checked_assignable_to_parsed(
        &self,
        candidate: dir::LocalNodeIdAny,
        source: dir::GlobalTypeId,
        tree: &dir::Tree,
        target: dir::LocalNodeId<dir::TypeExpression>,
        active: &mut Vec<TypeRelation>,
        program: &ProgramContext,
    ) -> Result<bool, ContextError> {
        let relation = TypeRelation {
            checked: source,
            parsed: target,
            direction: TypeDirection::CheckedToParsed,
        };
        if active.contains(&relation) {
            return Ok(true);
        }
        active.push(relation);

        let source_type = program.type_by_id(source)?;

        // accept the bottom source type for every supported target
        if matches!(source_type, dir::Type::Never) {
            active.pop();

            return Ok(true);
        }

        // dynamic storage relates as its constraint
        if let dir::Type::Dynamic(dynamic) = source_type {
            let is_assignable = self.is_checked_assignable_to_parsed(
                candidate,
                dynamic.constraint,
                tree,
                target,
                active,
                program,
            )?;
            active.pop();

            return Ok(is_assignable);
        }

        // require every union source member to fit the same target
        if let dir::Type::Union(union) = source_type {
            let elements = program.type_ids(source.module_id, union.elements)?;
            for element in elements {
                if !self.is_checked_assignable_to_parsed(
                    candidate, *element, tree, target, active, program,
                )? {
                    active.pop();

                    return Ok(false);
                }
            }
            active.pop();

            return Ok(true);
        }

        // any intersection source constituent may prove the target
        if let dir::Type::Intersection(intersection) = source_type {
            let elements = program.type_ids(source.module_id, intersection.elements)?;
            for element in elements {
                if self.is_checked_assignable_to_parsed(
                    candidate, *element, tree, target, active, program,
                )? {
                    active.pop();

                    return Ok(true);
                }
            }
            active.pop();

            return Ok(false);
        }

        let is_assignable = match tree.get(target) {
            dir::TypeExpression::Keyword { value } => {
                Self::is_leaf_assignable(source_type, dir::Type::from(value.clone()))
            }
            dir::TypeExpression::Literal { value } => {
                Self::is_leaf_assignable(source_type, Self::scalar_type(*value))
            }
            dir::TypeExpression::Union { elements } => {
                let mut is_assignable = false;
                for element in elements {
                    if self.is_checked_assignable_to_parsed(
                        candidate, source, tree, *element, active, program,
                    )? {
                        is_assignable = true;
                        break;
                    }
                }

                is_assignable
            }
            dir::TypeExpression::Intersection { elements } => {
                let mut is_assignable = true;
                for element in elements {
                    if !self.is_checked_assignable_to_parsed(
                        candidate, source, tree, *element, active, program,
                    )? {
                        is_assignable = false;
                        break;
                    }
                }

                is_assignable
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => self.is_checked_assignable_to_reference(
                candidate,
                source,
                source_type,
                tree,
                path,
                generic_arguments,
                active,
                program,
            )?,
            _ => return Err(ContextError::InvalidPredicateType),
        };
        active.pop();

        Ok(is_assignable)
    }

    /// Decide whether one parsed type is assignable to one checked type.
    fn is_parsed_assignable_to_checked(
        &self,
        candidate: dir::LocalNodeIdAny,
        tree: &dir::Tree,
        source: dir::LocalNodeId<dir::TypeExpression>,
        target: dir::GlobalTypeId,
        active: &mut Vec<TypeRelation>,
        program: &ProgramContext,
    ) -> Result<bool, ContextError> {
        let relation = TypeRelation {
            checked: target,
            parsed: source,
            direction: TypeDirection::ParsedToChecked,
        };
        if active.contains(&relation) {
            return Ok(true);
        }
        active.push(relation);

        let target_type = program.type_by_id(target)?;

        // accept every supported source for the top target type
        if matches!(target_type, dir::Type::Unknown) {
            active.pop();

            return Ok(true);
        }

        // require the source to fit at least one target union member
        if let dir::Type::Union(union) = target_type {
            let elements = program.type_ids(target.module_id, union.elements)?;
            for element in elements {
                if self.is_parsed_assignable_to_checked(
                    candidate, tree, source, *element, active, program,
                )? {
                    active.pop();

                    return Ok(true);
                }
            }
            active.pop();

            return Ok(false);
        }

        // require the source to fit every target intersection member
        if let dir::Type::Intersection(intersection) = target_type {
            let elements = program.type_ids(target.module_id, intersection.elements)?;
            for element in elements {
                if !self.is_parsed_assignable_to_checked(
                    candidate, tree, source, *element, active, program,
                )? {
                    active.pop();

                    return Ok(false);
                }
            }
            active.pop();

            return Ok(true);
        }

        let is_assignable = match tree.get(source) {
            dir::TypeExpression::Keyword { value } => {
                Self::is_leaf_assignable(dir::Type::from(value.clone()), target_type)
            }
            dir::TypeExpression::Literal { value } => {
                Self::is_leaf_assignable(Self::scalar_type(*value), target_type)
            }
            dir::TypeExpression::Union { elements } => {
                let mut is_assignable = true;
                for element in elements {
                    if !self.is_parsed_assignable_to_checked(
                        candidate, tree, *element, target, active, program,
                    )? {
                        is_assignable = false;
                        break;
                    }
                }

                is_assignable
            }
            dir::TypeExpression::Intersection { elements } => {
                let mut is_assignable = false;
                for element in elements {
                    if self.is_parsed_assignable_to_checked(
                        candidate, tree, *element, target, active, program,
                    )? {
                        is_assignable = true;
                        break;
                    }
                }

                is_assignable
            }
            dir::TypeExpression::Reference {
                path,
                generic_arguments,
            } => self.is_reference_assignable_to_checked(
                candidate,
                tree,
                path,
                generic_arguments,
                target,
                target_type,
                active,
                program,
            )?,
            _ => return Err(ContextError::InvalidPredicateType),
        };
        active.pop();

        Ok(is_assignable)
    }

    /// Relate one checked nominal application to parsed reference syntax.
    fn is_checked_assignable_to_reference(
        &self,
        candidate: dir::LocalNodeIdAny,
        source_id: dir::GlobalTypeId,
        source: dir::Type,
        tree: &dir::Tree,
        target_path: &dir::Path,
        target_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        active: &mut Vec<TypeRelation>,
        program: &ProgramContext,
    ) -> Result<bool, ContextError> {
        let Some(source) = Self::type_application(source) else {
            return Ok(false);
        };
        let target_symbols = self.resolve_path(candidate, target_path, program)?;
        let source_symbols = program.symbol_targets(&[source.symbol])?;
        if source_symbols != target_symbols {
            return Ok(false);
        }
        let [symbol] = source_symbols.as_slice() else {
            return Ok(false);
        };
        let checked_arguments = program.type_ids(source_id.module_id, source.arguments)?;
        let parsed_arguments = Self::predicate_type_arguments(tree, target_arguments)?;

        self.relate_application_arguments(
            candidate,
            *symbol,
            checked_arguments,
            tree,
            &parsed_arguments,
            TypeDirection::CheckedToParsed,
            active,
            program,
        )
    }

    /// Relate parsed reference syntax to one checked nominal application.
    fn is_reference_assignable_to_checked(
        &self,
        candidate: dir::LocalNodeIdAny,
        tree: &dir::Tree,
        source_path: &dir::Path,
        source_arguments: &[dir::LocalNodeId<dir::GenericArgument>],
        target_id: dir::GlobalTypeId,
        target: dir::Type,
        active: &mut Vec<TypeRelation>,
        program: &ProgramContext,
    ) -> Result<bool, ContextError> {
        let Some(target) = Self::type_application(target) else {
            return Ok(false);
        };
        let source_symbols = self.resolve_path(candidate, source_path, program)?;
        let target_symbols = program.symbol_targets(&[target.symbol])?;
        if source_symbols != target_symbols {
            return Ok(false);
        }
        let [symbol] = target_symbols.as_slice() else {
            return Ok(false);
        };
        let checked_arguments = program.type_ids(target_id.module_id, target.arguments)?;
        let parsed_arguments = Self::predicate_type_arguments(tree, source_arguments)?;

        self.relate_application_arguments(
            candidate,
            *symbol,
            checked_arguments,
            tree,
            &parsed_arguments,
            TypeDirection::ParsedToChecked,
            active,
            program,
        )
    }

    /// Relate checked and parsed application arguments by committed variance.
    fn relate_application_arguments(
        &self,
        candidate: dir::LocalNodeIdAny,
        symbol: dir::GlobalSymbolId,
        checked: &[dir::GlobalTypeId],
        tree: &dir::Tree,
        parsed: &[dir::LocalNodeId<dir::TypeExpression>],
        direction: TypeDirection,
        active: &mut Vec<TypeRelation>,
        program: &ProgramContext,
    ) -> Result<bool, ContextError> {
        if checked.is_empty() && parsed.is_empty() {
            return Ok(true);
        }
        let module = program.module(symbol.module_id)?;
        let Some(template) = module.generics().template_by_symbol(symbol) else {
            return Err(ContextError::MissingGenericTemplate(symbol));
        };
        let template = module.generics().get_template(template);
        let written = template
            .parameters
            .iter()
            .filter(|parameter| {
                let binding = module.generics().get_parameter(**parameter);

                matches!(binding.key, dir::GenericParameterKey::Symbol(_))
            })
            .count();
        if checked.len() != parsed.len() || checked.len() != written {
            return Err(ContextError::MismatchedGenericArguments(symbol));
        }

        // relate every parameter independently in declaration order
        for ((checked, parsed), parameter) in checked.iter().zip(parsed).zip(&template.parameters) {
            let variance = module.generics().parameter_variance(*parameter);
            let is_assignable = match (direction, variance) {
                (_, None) => true,
                (TypeDirection::CheckedToParsed, Some(dir::VarianceModifier::Out))
                | (TypeDirection::ParsedToChecked, Some(dir::VarianceModifier::In)) => self
                    .is_checked_assignable_to_parsed(
                        candidate, *checked, tree, *parsed, active, program,
                    )?,
                (TypeDirection::CheckedToParsed, Some(dir::VarianceModifier::In))
                | (TypeDirection::ParsedToChecked, Some(dir::VarianceModifier::Out)) => self
                    .is_parsed_assignable_to_checked(
                        candidate, tree, *parsed, *checked, active, program,
                    )?,
                (_, Some(dir::VarianceModifier::InOut)) => {
                    self.is_checked_assignable_to_parsed(
                        candidate, *checked, tree, *parsed, active, program,
                    )? && self.is_parsed_assignable_to_checked(
                        candidate, tree, *parsed, *checked, active, program,
                    )?
                }
            };
            if !is_assignable {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Return one checked reference as a complete nominal application.
    fn type_application(ty: dir::Type) -> Option<dir::GenericApplication> {
        match ty {
            dir::Type::Reference(reference) => Some(dir::GenericApplication {
                symbol: reference.symbol,
                arguments: dir::TypeListId::EMPTY,
            }),
            dir::Type::Application(application) => Some(application),
            _ => None,
        }
    }

    /// Return plain type arguments from one compiled predicate reference.
    fn predicate_type_arguments(
        tree: &dir::Tree,
        arguments: &[dir::LocalNodeId<dir::GenericArgument>],
    ) -> Result<Vec<dir::LocalNodeId<dir::TypeExpression>>, ContextError> {
        let mut types = Vec::with_capacity(arguments.len());

        // preserve the compiler's plain type argument invariant
        for argument in arguments {
            let dir::GenericArgument::Type { value } = tree.get(*argument) else {
                return Err(ContextError::InvalidPredicateType);
            };
            types.push(*value);
        }

        Ok(types)
    }

    /// Return the exact checked type denoted by one scalar type literal.
    fn scalar_type(value: dir::Literal) -> dir::Type {
        match value {
            dir::Literal::Null => dir::Type::Null,
            dir::Literal::Undefined => dir::Type::Undefined,
            _ => dir::Type::Literal(value),
        }
    }

    /// Decide one closed checked leaf relation.
    fn is_leaf_assignable(source: dir::Type, target: dir::Type) -> bool {
        if source == target {
            return true;
        }

        match (source, target) {
            (dir::Type::Never, _) => true,
            (_, dir::Type::Unknown) => true,
            (dir::Type::Literal(source), dir::Type::Primitive(target)) => {
                source.widens_to_primitive(target)
            }
            _ => false,
        }
    }
}
