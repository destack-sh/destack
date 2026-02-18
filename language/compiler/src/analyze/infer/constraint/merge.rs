use crate::{AnalyzeResult, Compiler, InferContext};
use destack_dir::{
    InferTable, LocalNodeId, LocalNodeIdAny, LocalTypeId, NodeTree, Property, SymbolTable, Type,
    TypeLiteral, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use super::expression::ObjectLiteralField;
use crate::analyze::common::{NormalizationMode, ObjectShape, RelationMode};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Infer literal fields and spread shapes for an object literal.
    pub(crate) fn infer_object_literal_shapes(
        &self,
        module: &Module,
        properties: &[LocalNodeId<Property>],
        expected_object_ty_id: Option<LocalTypeId>,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: &mut InferTable,
        ctx: &mut InferContext,
    ) -> AnalyzeResult<(
        Vec<ObjectLiteralField>,
        Vec<ObjectShape>,
        Option<LocalTypeId>,
    )> {
        let mut literal_fields = Vec::new();
        let mut shapes = vec![ObjectShape::default()];
        let mut spread_override = None;
        let mut has_any_spread = false;

        // collect literal fields and spread shapes
        for property_id in properties {
            let property = tree.get(*property_id);
            match property {
                Property::Spread { value, .. } => {
                    let mut spread_ctx = ctx.fork().with_expected_type(None);
                    let spread_type = self.infer_expression(
                        module,
                        *value,
                        tree,
                        symbols,
                        types,
                        infer,
                        &mut spread_ctx,
                    )?;
                    // normalize spreads before extracting shapes
                    let spread_type = self.normalize_type_with_relation(
                        module,
                        ctx.profile,
                        spread_type,
                        symbols,
                        types,
                        NormalizationMode::Assign,
                        RelationMode::TYPE_OPS,
                    );

                    // short circuit on any or unknown spreads
                    match types.get_type(spread_type) {
                        Type::TypeLiteral {
                            value: TypeLiteral::Any,
                        } => {
                            spread_override = Some(spread_type);
                            has_any_spread = true;
                            continue;
                        }
                        Type::TypeLiteral {
                            value: TypeLiteral::Unknown,
                        } => {
                            if !has_any_spread {
                                spread_override = Some(spread_type);
                            }
                            continue;
                        }
                        Type::TypeLiteral {
                            value: TypeLiteral::Never,
                        } => continue,
                        _ => {}
                    }

                    // expand spread types into object shapes
                    let spread_shapes = self.collect_object_spread_shapes(
                        module,
                        ctx.profile,
                        (*property_id).into_any(),
                        spread_type,
                        symbols,
                        types,
                    )?;
                    shapes = self.merge_object_spread_shape_sets(shapes, spread_shapes);
                }
                _ => {
                    if let Some(field) = self.infer_property(
                        module,
                        *property_id,
                        expected_object_ty_id,
                        tree,
                        symbols,
                        types,
                        infer,
                        ctx,
                    )? {
                        literal_fields.push(field.clone());
                        for shape in shapes.iter_mut() {
                            shape.apply_field(field.field().clone());
                        }
                    }
                }
            }
        }

        Ok((literal_fields, shapes, spread_override))
    }

    /// Merge shape sets with spread override semantics.
    fn merge_object_spread_shape_sets(
        &self,
        base_shapes: Vec<ObjectShape>,
        spread_shapes: Vec<ObjectShape>,
    ) -> Vec<ObjectShape> {
        let mut merged = Vec::new();

        // apply each spread shape onto each base shape
        for base_shape in base_shapes {
            for spread_shape in &spread_shapes {
                let mut combined = base_shape.clone();
                combined.apply_spread(spread_shape);
                merged.push(combined);
            }
        }

        merged
    }

    /// Merge shape sets with intersection semantics.
    fn merge_object_spread_shape_sets_for_intersection(
        &self,
        base_shapes: Vec<ObjectShape>,
        intersection_shapes: Vec<ObjectShape>,
        types: &mut TypeTable,
    ) -> Vec<ObjectShape> {
        let mut merged = Vec::new();

        // intersect each incoming shape with each base shape
        for base_shape in base_shapes {
            for intersection_shape in &intersection_shapes {
                let mut combined = base_shape.clone();
                combined.apply_intersection(intersection_shape, types);
                merged.push(combined);
            }
        }

        merged
    }

    /// Collect object shapes from a spread type.
    fn collect_object_spread_shapes(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
    ) -> AnalyzeResult<Vec<ObjectShape>> {
        let mut visited = Vec::new();
        self.collect_object_spread_shapes_inner(
            module,
            profile,
            node_id,
            type_id,
            symbols,
            types,
            &mut visited,
        )
    }

    /// Collect object shapes with recursion protection.
    fn collect_object_spread_shapes_inner(
        &self,
        module: &Module,
        profile: ProfileId,
        node_id: LocalNodeIdAny,
        type_id: LocalTypeId,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        visited: &mut Vec<LocalTypeId>,
    ) -> AnalyzeResult<Vec<ObjectShape>> {
        // guard against recursive types
        if visited.contains(&type_id) {
            return Ok(vec![ObjectShape::default()]);
        }
        visited.push(type_id);

        // normalize before inspecting the spread shape
        let normalized_type = self.normalize_type_with_relation(
            module,
            profile,
            type_id,
            symbols,
            types,
            NormalizationMode::Assign,
            RelationMode::TYPE_OPS,
        );

        // derive shapes based on the normalized type
        let normalized_type_value = types.get_type(normalized_type).clone();
        let shapes = match normalized_type_value {
            Type::Object { .. } => {
                // direct object shapes map to a single inferred shape
                let mut shape = ObjectShape::default();
                shape.extend_from_object(types.get_type(normalized_type));
                vec![shape]
            }
            Type::Reference { symbol, .. } => {
                // use apparent types for spreads to match shape behavior
                let instance_id =
                    self.apparent_instance_type(module, profile, node_id, symbol, symbols, types);

                if let Some(instance_id) = instance_id {
                    self.collect_object_spread_shapes_inner(
                        module,
                        profile,
                        node_id,
                        instance_id,
                        symbols,
                        types,
                        visited,
                    )?
                } else {
                    // default to an empty shape when no instance type is available
                    vec![ObjectShape::default()]
                }
            }
            Type::Union { elements } => {
                // union spreads fan out into distinct shapes
                let mut merged = Vec::new();
                for element_id in elements {
                    let mut element_shapes = self.collect_object_spread_shapes_inner(
                        module, profile, node_id, element_id, symbols, types, visited,
                    )?;
                    merged.append(&mut element_shapes);
                }
                merged
            }
            Type::Intersection { elements } => {
                // intersection spreads merge shapes together
                let mut merged = vec![ObjectShape::default()];
                for element_id in elements {
                    let element_shapes = self.collect_object_spread_shapes_inner(
                        module, profile, node_id, element_id, symbols, types, visited,
                    )?;
                    merged = self.merge_object_spread_shape_sets_for_intersection(
                        merged,
                        element_shapes,
                        types,
                    );
                }
                merged
            }
            _ => {
                // non object spreads contribute no fields
                vec![ObjectShape::default()]
            }
        };

        visited.pop();
        Ok(shapes)
    }
}
