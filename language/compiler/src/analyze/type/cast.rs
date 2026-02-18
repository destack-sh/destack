use super::*;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    fn enum_backing_type_for_type(
        &self,
        module: &Module,
        profile: ProfileId,
        ty: &Type,
        types: &mut TypeTable,
    ) -> Option<EnumBackingType> {
        match ty {
            // read backing types directly from enum references
            Type::Reference { symbol, .. } => {
                self.enum_backing_type_for_symbol_best_effort(module, profile, *symbol, types)
            }
            // unwrap value containers to reach enum references
            Type::Value { value } => {
                let inner_ty = types.get_type(*value).clone();
                self.enum_backing_type_for_type(module, profile, &inner_ty, types)
            }
            // scan intersections for a matching enum reference
            Type::Intersection { elements } => elements.iter().find_map(|element_id| {
                let element_ty = types.get_type(*element_id).clone();
                self.enum_backing_type_for_type(module, profile, &element_ty, types)
            }),
            _ => None,
        }
    }

    /// Return true when the target type matches the enum backing type.
    fn enum_backing_type_matches(&self, backing: EnumBackingType, target: &Type) -> bool {
        // match integer or string targets
        match backing {
            EnumBackingType::Int(_) => matches!(
                target,
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::Int(_))
                } | Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::Integer(_))
                }
            ),
            EnumBackingType::String => matches!(
                target,
                Type::TypeLiteral {
                    value: TypeLiteral::Primitive(PrimitiveType::String)
                } | Type::TypeLiteral {
                    value: TypeLiteral::ScalarLiteral(ScalarLiteral::String(_))
                }
            ),
        }
    }

    /// Return true when an enum cast targets its backing type.
    pub(crate) fn is_enum_backing_cast(
        &self,
        module: &Module,
        profile: ProfileId,
        left_ty: &Type,
        right_ty: &Type,
        types: &mut TypeTable,
    ) -> bool {
        // read backing types
        let left_backing = self.enum_backing_type_for_type(module, profile, left_ty, types);
        let right_backing = self.enum_backing_type_for_type(module, profile, right_ty, types);

        // compare backing types against the other side
        match (left_backing, right_backing) {
            (Some(backing), None) => self.enum_backing_type_matches(backing, right_ty),
            (None, Some(backing)) => self.enum_backing_type_matches(backing, left_ty),
            _ => false,
        }
    }

    /// Return true when an explicit cast targets a record like map alias.
    pub(crate) fn allow_record_like_cast(
        &self,
        profile: ProfileId,
        left_ty_id: LocalTypeId,
        right_ty_id: LocalTypeId,
        types: &TypeTable,
    ) -> bool {
        // require an object source type
        let left_ty_id = self.unwrap_type_value(left_ty_id, types);
        if !matches!(types.get_type(left_ty_id), Type::Object { .. }) {
            return false;
        }

        // locate the map and record symbols
        let map_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Map);
        let record_symbol = self.get_well_known_type_symbol(profile, WellKnownSymbol::Record);
        let Some(map_symbol) = map_symbol else {
            return false;
        };

        // walk aliases to find map and record references
        let mut current = self.unwrap_type_value(right_ty_id, types);
        let mut visited = HashSet::new();
        loop {
            if !visited.insert(current) {
                return false;
            }

            match types.get_type(current) {
                // accept direct map and record references
                Type::Reference { symbol, .. } => {
                    if *symbol == map_symbol
                        || record_symbol.is_some_and(|record_symbol| record_symbol == *symbol)
                    {
                        return true;
                    }

                    // follow alias targets when available
                    if let Some(alias_target) = types.get_alias_target_type_id(*symbol) {
                        current = alias_target;
                        continue;
                    }

                    return false;
                }
                // unwrap value wrapper types
                Type::Value { value } => {
                    current = *value;
                }
                _ => return false,
            }
        }
    }
}
