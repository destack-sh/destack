use std::collections::HashSet;

use destack_dir as dir;
use destack_source::ModuleId;

use crate::{LowerError, LowerResult};

use super::TypeLowerer;

impl TypeLowerer<'_> {
    /// Select the primary type for an intersection layout.
    pub(crate) fn select_intersection_primary_type(
        &self,
        types: &dir::TypeTable<'_>,
        elements: &[dir::LocalTypeId],
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<dir::LocalTypeId> {
        // collect intersection elements with flattening
        let mut collected = Vec::new();
        let mut visited = HashSet::new();
        for element_id in elements {
            self.collect_intersection_element(*element_id, types, &mut visited, &mut collected);
        }

        // track candidate primary types
        let mut primary_nominal = None;
        let mut primary_object = None;

        // scan for nominal and object candidates
        for element_id in collected {
            let dir_type = types.get_type(element_id);
            match dir_type {
                dir::Type::Named(reference) => {
                    if matches!(
                        self.symbol_form(reference.symbol),
                        Some(
                            dir::SymbolForm::Struct
                                | dir::SymbolForm::Class
                                | dir::SymbolForm::Enum
                                | dir::SymbolForm::Newtype
                        )
                    ) {
                        if let Some(existing) = primary_nominal {
                            if existing != element_id {
                                return Err(LowerError::UnsupportedType {
                                    anchor: self.diagnostic_anchor(node),
                                    ty: element_id.into_global(module_id),
                                    message: "intersection has multiple nominal primaries"
                                        .to_string(),
                                }
                                .into());
                            }
                        } else {
                            primary_nominal = Some(element_id);
                        }
                    }
                }
                dir::Type::Shape(_) => {
                    if primary_object.is_none() {
                        primary_object = Some(element_id);
                    }
                }
                _ => {}
            }
        }

        // prefer nominal primary types
        if let Some(primary) = primary_nominal {
            return Ok(primary);
        }

        // fall back to object types
        if let Some(primary) = primary_object {
            return Ok(primary);
        }

        // report missing primary layouts
        let Some(primary) = elements.first().copied() else {
            return Err(LowerError::UnsupportedConstruct {
                anchor: self.diagnostic_anchor(node),
                message: "intersection missing primary layout type".to_string(),
            }
            .into());
        };
        Err(LowerError::UnsupportedType {
            anchor: self.diagnostic_anchor(node),
            ty: primary.into_global(module_id),
            message: "intersection missing primary layout type".to_string(),
        }
        .into())
    }

    /// Collect intersection elements with flattening.
    fn collect_intersection_element(
        &self,
        type_id: dir::LocalTypeId,
        types: &dir::TypeTable<'_>,
        visited: &mut HashSet<dir::LocalTypeId>,
        collected: &mut Vec<dir::LocalTypeId>,
    ) {
        // skip already visited types
        if !visited.insert(type_id) {
            return;
        }

        // flatten nested intersections
        match types.get_type(type_id) {
            dir::Type::Intersection(intersection) => {
                for element_id in &intersection.elements {
                    self.collect_intersection_element(*element_id, types, visited, collected);
                }
            }
            _ => collected.push(type_id),
        }
    }
}
