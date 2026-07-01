use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{Answer, answer};

use super::query::LayoutQuery;
use super::scalar::align_to;

impl LayoutQuery<'_, '_> {
    /// Compute the inline layout of `Vector<T, N>`.
    pub(super) fn vector_layout(
        &mut self,
        owner: ModuleId,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::Layout>>> {
        let [element, count] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };

        let source = self
            .check
            .origin_source_node(self.origin)?
            .into_global(self.origin.module());
        let Some(layout_id) = answer!(self.slot_layout(owner, *element, source)?) else {
            return Ok(Answer::Ready(None));
        };
        let (element_size, element_alignment, element_niche) = {
            let layout = self.layout(owner, layout_id);
            (layout.size, layout.alignment, layout.niche)
        };
        let origin = self.origin;
        let count = answer!(self.check.reduce_type_head(origin, *count)?);
        let lanes = match self.check.ty(count)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => u32::try_from(*value).ok(),
            _ => None,
        };
        let Some(lanes) = lanes else {
            return Ok(Answer::Ready(None));
        };

        let stride = align_to(element_size, element_alignment);

        Ok(Answer::Ready(Some(dir::Layout {
            shape: dir::LayoutShape::Vector(dir::ElementLayout {
                element: *element,
                stride,
                count: lanes,
            }),
            size: stride.saturating_mul(lanes),
            alignment: element_alignment,
            niche: (lanes > 0).then_some(element_niche).flatten(),
        })))
    }

    /// Return normalized layout input for one `Tensor<T, Rank, F, P>` instance.
    pub(super) fn tensor_layout_input(
        &mut self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<TensorLayoutInput>>> {
        let [element, rank, format, placement] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let Some(rank) = answer!(self.static_u32(*rank)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(format) = answer!(self.tensor_format(*format)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(placement) = answer!(self.tensor_placement(*placement)?) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(TensorLayoutInput {
            element: *element,
            rank,
            format,
            placement,
        })))
    }

    /// Return normalized layout input for one `TensorView<T, Rank, F, P, A>` instance.
    pub(super) fn tensor_view_layout_input(
        &mut self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<TensorViewLayoutInput>>> {
        let [element, rank, format, placement, ..] = instance.arguments.as_slice() else {
            return Ok(Answer::Ready(None));
        };
        let Some(rank) = answer!(self.static_u32(*rank)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(format) = answer!(self.tensor_view_format(*format)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(placement) = answer!(self.tensor_placement(*placement)?) else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(TensorViewLayoutInput {
            element: *element,
            rank,
            format,
            placement,
        })))
    }

    /// Return the normalized owning tensor format.
    pub(super) fn tensor_format(
        &mut self,
        format: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::TensorFormat>>> {
        let Some(format) = answer!(self.tensor_view_format(format)?) else {
            return Ok(Answer::Ready(None));
        };

        // owning tensors currently require contiguous storage
        let format = match format {
            dir::TensorViewFormat::Dense { order } => Some(dir::TensorFormat::Dense { order }),
            dir::TensorViewFormat::Strided => None,
        };

        Ok(Answer::Ready(format))
    }

    /// Return the normalized tensor view format.
    pub(super) fn tensor_view_format(
        &mut self,
        format: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::TensorViewFormat>>> {
        let Some(instance) = answer!(self.language_item_instance(format)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(item) = self.check.language_item(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };
        let format = match item {
            dir::LanguageItem::TensorDense => {
                let Some(order) = answer!(self.tensor_dimension_order(&instance)?) else {
                    return Ok(Answer::Ready(None));
                };

                dir::TensorViewFormat::Dense { order }
            }
            dir::LanguageItem::TensorStrided => dir::TensorViewFormat::Strided,
            _ => return Ok(Answer::Ready(None)),
        };

        Ok(Answer::Ready(Some(format)))
    }

    /// Return the normalized tensor placement.
    pub(super) fn tensor_placement(
        &mut self,
        placement: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::TensorSharding>>> {
        let Some(instance) = answer!(self.language_item_instance(placement)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(item) = self.check.language_item(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };
        let placement = match item {
            dir::LanguageItem::TensorUnsharded => dir::TensorSharding::Unsharded,
            dir::LanguageItem::TensorShardingAxes => {
                let Some(axes) = answer!(self.tensor_placement_axes(&instance)?) else {
                    return Ok(Answer::Ready(None));
                };

                dir::TensorSharding::Sharding { axes }
            }
            _ => return Ok(Answer::Ready(None)),
        };

        Ok(Answer::Ready(Some(placement)))
    }

    /// Return normalized sharding axes from one `Sharding<...Axes>` instance.
    pub(super) fn tensor_placement_axes(
        &mut self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<Vec<dir::TensorShardingAxis>>>> {
        let mut axes = Vec::with_capacity(instance.arguments.len());

        for axis in instance.arguments.iter().copied() {
            let Some(axis) = answer!(self.tensor_placement_axis(axis)?) else {
                return Ok(Answer::Ready(None));
            };

            axes.push(axis);
        }

        Ok(Answer::Ready(Some(axes)))
    }

    /// Return one normalized sharding axis descriptor.
    pub(super) fn tensor_placement_axis(
        &mut self,
        axis: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::TensorShardingAxis>>> {
        let Some(instance) = answer!(self.language_item_instance(axis)?) else {
            return Ok(Answer::Ready(None));
        };
        let Some(item) = self.check.language_item(instance.symbol)? else {
            return Ok(Answer::Ready(None));
        };
        let axis = match item {
            dir::LanguageItem::TensorShard => {
                let Some(axis) = instance.arguments.first().copied() else {
                    return Ok(Answer::Ready(None));
                };
                let Some(axis) = answer!(self.static_i32(axis)?) else {
                    return Ok(Answer::Ready(None));
                };

                dir::TensorShardingAxis::Shard { axis }
            }
            dir::LanguageItem::TensorReplicate => dir::TensorShardingAxis::Replicate,
            dir::LanguageItem::TensorPartial => {
                let Some(reduction) = answer!(self.tensor_reduction(&instance)?) else {
                    return Ok(Answer::Ready(None));
                };

                dir::TensorShardingAxis::Partial { reduction }
            }
            _ => return Ok(Answer::Ready(None)),
        };

        Ok(Answer::Ready(Some(axis)))
    }

    /// Return the normalized reduction carried by one `Partial<R>` instance.
    pub(super) fn tensor_reduction(
        &mut self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::TensorReduction>>> {
        let Some(reduction) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };
        let origin = self.origin;
        let reduction = answer!(self.check.reduce_type_head(origin, reduction)?);
        let reduction = self.layout_type(reduction)?;
        let reduction = match self.check.ty(reduction)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                dir::TensorReduction::from_discriminant(*value)
            }
            dir::Type::Static(value) => self.tensor_reduction_from_static(*value),
            _ => None,
        };

        Ok(Answer::Ready(reduction))
    }

    /// Return the normalized dimension order for one `Dense<Order>` instance.
    pub(super) fn tensor_dimension_order(
        &mut self,
        instance: &dir::GenericInstance,
    ) -> CompilerResult<Answer<Option<dir::TensorDimensionOrder>>> {
        let Some(order) = instance.arguments.first().copied() else {
            return Ok(Answer::Ready(None));
        };
        let origin = self.origin;
        let order = answer!(self.check.reduce_type_head(origin, order)?);
        let order = self.layout_type(order)?;
        let order = match self.check.ty(order)? {
            dir::Type::Literal(dir::ScalarLiteral::Integer(value)) => {
                dir::TensorDimensionOrder::from_discriminant(*value)
            }
            dir::Type::Static(value) => self.tensor_dimension_order_from_static(*value),
            _ => None,
        };

        Ok(Answer::Ready(order))
    }

    /// Return the normalized dimension order carried by one static value.
    pub(super) fn tensor_dimension_order_from_static(
        &self,
        value: dir::GlobalStaticId,
    ) -> Option<dir::TensorDimensionOrder> {
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        } = self.check.r#static(value)
        else {
            return None;
        };

        dir::TensorDimensionOrder::from_discriminant(*value)
    }

    /// Return the normalized reduction carried by one static value.
    pub(super) fn tensor_reduction_from_static(
        &self,
        value: dir::GlobalStaticId,
    ) -> Option<dir::TensorReduction> {
        let dir::StaticTerm::ScalarLiteral {
            value: dir::ScalarLiteral::Integer(value),
        } = self.check.r#static(value)
        else {
            return None;
        };

        dir::TensorReduction::from_discriminant(*value)
    }

    /// Return one normalized language item instance.
    pub(super) fn language_item_instance(
        &mut self,
        ty: dir::GlobalTypeId,
    ) -> CompilerResult<Answer<Option<dir::GenericInstance>>> {
        let origin = self.origin;
        let ty = answer!(self.check.reduce_type_head(origin, ty)?);
        let ty = self.layout_type(ty)?;
        let dir::Type::Instance(instance) = self.check.ty(ty)? else {
            return Ok(Answer::Ready(None));
        };

        Ok(Answer::Ready(Some(instance.clone())))
    }
}

/// Normalized tensor layout input.
pub(super) struct TensorLayoutInput {
    /// The tensor element type.
    element: dir::GlobalTypeId,
    /// The tensor rank.
    rank: u32,
    /// The tensor storage format.
    format: dir::TensorFormat,
    /// The tensor placement.
    placement: dir::TensorSharding,
}

impl TensorLayoutInput {
    /// Return the concrete tensor handle layout.
    pub(super) fn layout(self, pointer_bytes: u32) -> dir::Layout {
        dir::Layout {
            shape: dir::LayoutShape::Tensor(dir::TensorLayout {
                element: self.element,
                format: self.format,
                sharding: self.placement,
                rank: self.rank,
            }),
            size: pointer_bytes,
            alignment: pointer_bytes,
            niche: Some(dir::Niche::non_null_pointer(pointer_bytes)),
        }
    }
}

/// Normalized tensor view layout input.
pub(super) struct TensorViewLayoutInput {
    /// The viewed element type.
    element: dir::GlobalTypeId,
    /// The tensor view rank.
    rank: u32,
    /// The tensor view format.
    format: dir::TensorViewFormat,
    /// The tensor view placement.
    placement: dir::TensorSharding,
}

impl TensorViewLayoutInput {
    /// Return the concrete tensor view descriptor layout.
    pub(super) fn layout(self, pointer_bytes: u32) -> dir::Layout {
        let field_count = self.format.descriptor_slots(self.rank);

        dir::Layout {
            shape: dir::LayoutShape::TensorView(dir::TensorViewLayout {
                element: self.element,
                format: self.format,
                sharding: self.placement,
                rank: self.rank,
            }),
            size: pointer_bytes.saturating_mul(field_count),
            alignment: pointer_bytes,
            niche: Some(dir::Niche::non_null_pointer(pointer_bytes)),
        }
    }
}
