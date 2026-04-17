use destack_source::ModuleId;
use {destack_dir as dir, destack_mir as mir};

use super::lower::TypeLowerer;
use crate::analyze::StaticArgumentResolver;
use crate::{LowerError, LowerResult};

impl TypeLowerer {
    /// Check if a symbol should be treated as the Vector intrinsic.
    pub(super) fn is_vector_symbol(&self, symbol: dir::GlobalSymbolId) -> bool {
        self.vector_symbol
            .is_some_and(|vector_symbol| symbol == vector_symbol)
    }

    /// Lower a Vector<T, N> reference type to a MIR vector type.
    pub(super) fn lower_vector_reference_type(
        &mut self,
        types: &dir::TypeTable,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        generic_arguments: Option<&[dir::StaticArgument]>,
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // require static arguments for Vector<T, N>
        let generic_arguments = generic_arguments.ok_or_else(|| LowerError::UnsupportedType {
            node,
            ty: type_id.into_global(module_id),
            message: "Vector<T, N> requires static arguments".to_string(),
        })?;

        // resolve element and lane arguments
        let (element_expression, lane_expression) =
            self.vector_static_argument_pair(type_id, module_id, node, generic_arguments, builder)?;

        // parse lane count
        let lanes = self.vector_lane_count(type_id, module_id, node, &lane_expression)?;

        // resolve element type
        let element_type_id =
            self.vector_element_type_id(type_id, module_id, node, &element_expression)?;

        // lower element type
        let element_type = self.lower_type(types, element_type_id, module_id, node, builder)?;
        let element_copyability = builder.tree().get(element_type).copyability();

        Ok(builder.type_vector(element_type, lanes, element_copyability))
    }

    /// Resolve evaluated static arguments for Vector<T, N>.
    fn vector_static_argument_pair(
        &self,
        _type_id: dir::LocalTypeId,
        _module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        generic_arguments: &[dir::StaticArgument],
        builder: &mut mir::ModuleBuilder,
    ) -> LowerResult<(dir::StaticExpression, dir::StaticExpression)> {
        let element_name = builder.strings().intern("T");
        let lane_name = builder.strings().intern("N");

        let resolver =
            StaticArgumentResolver::new(generic_arguments, "Vector<T, N>").map_err(|error| {
                LowerError::InvalidStaticArgument {
                    node,
                    message: error.to_string(),
                }
            })?;

        resolver
            .ensure_exact_arity(2)
            .map_err(|error| LowerError::InvalidStaticArgument {
                node,
                message: error.to_string(),
            })?;
        resolver
            .ensure_only_names(&[(element_name, "T"), (lane_name, "N")])
            .map_err(|error| LowerError::InvalidStaticArgument {
                node,
                message: error.to_string(),
            })?;

        let element_expression = resolver
            .argument_by_name_or_index((element_name, "T"), 0)
            .map_err(|error| LowerError::InvalidStaticArgument {
                node,
                message: error.to_string(),
            })?;
        let lane_expression = resolver
            .argument_by_name_or_index((lane_name, "N"), 1)
            .map_err(|error| LowerError::InvalidStaticArgument {
                node,
                message: error.to_string(),
            })?;

        Ok((element_expression.clone(), lane_expression.clone()))
    }

    /// Parse and validate the Vector<T, N> lane count.
    fn vector_lane_count(
        &self,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        lane_expression: &dir::StaticExpression,
    ) -> LowerResult<u32> {
        // parse lane count literal
        let lane_count = match lane_expression {
            dir::StaticExpression::ScalarLiteral { value } => match value {
                dir::ScalarLiteral::Integer(value) | dir::ScalarLiteral::Bigint(value) => *value,
                _ => {
                    return Err(LowerError::UnsupportedType {
                        node,
                        ty: type_id.into_global(module_id),
                        message: "Vector<T, N> lane count must be an integer".to_string(),
                    });
                }
            },
            _ => {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "Vector<T, N> lane count must be a scalar literal".to_string(),
                });
            }
        };

        // validate lane count
        let lanes = u32::try_from(lane_count).map_err(|_| LowerError::UnsupportedType {
            node,
            ty: type_id.into_global(module_id),
            message: "Vector<T, N> lane count must fit in a u32".to_string(),
        })?;
        if lanes == 0 {
            return Err(LowerError::UnsupportedType {
                node,
                ty: type_id.into_global(module_id),
                message: "Vector<T, N> lane count must be greater than zero".to_string(),
            });
        }

        Ok(lanes)
    }

    /// Resolve the Vector<T, N> element type id.
    fn vector_element_type_id(
        &self,
        type_id: dir::LocalTypeId,
        module_id: ModuleId,
        node: dir::AnchoredGlobalNodeId,
        element_expression: &dir::StaticExpression,
    ) -> LowerResult<dir::LocalTypeId> {
        // resolve element type argument
        let element_type_id = match element_expression {
            dir::StaticExpression::Type { ty } => *ty,
            _ => {
                return Err(LowerError::UnsupportedType {
                    node,
                    ty: type_id.into_global(module_id),
                    message: "Vector<T, N> element type must be a type argument".to_string(),
                });
            }
        };

        Ok(element_type_id)
    }
}
