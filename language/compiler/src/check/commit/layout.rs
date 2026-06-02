use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::{
    CheckState, Layout, LayoutDecision, LayoutField, LayoutResolution, LayoutShape, LayoutType,
    NewtypeRepresentation, VariantCaseLayout, VariantTagLayout,
};

use super::CheckModuleOutput;

impl CheckState<'_> {
    /// Commit solved layouts into one DIR layout segment.
    pub(super) fn commit_layout_table(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<dir::LayoutSegment> {
        let mut table = dir::LayoutSegment::new(module);
        let pointer_bytes = self.target_pointer_bytes()?;
        let newtypes = self
            .representations
            .iter_newtypes()
            .filter(|(symbol, _)| symbol.module_id == module)
            .map(|(_, representation)| *representation)
            .collect::<Vec<_>>();
        let layouts = self
            .inference
            .layouts_vec()
            .into_iter()
            .filter_map(|decision| match decision {
                LayoutDecision::Resolved(layout) => Some(layout),
                LayoutDecision::Rejected(_) => None,
            })
            .collect::<Vec<_>>();
        let nominals = output
            .nominals
            .iter_definitions()
            .map(|(symbol, _)| symbol)
            .collect::<Vec<_>>();

        // write nominal representation facts
        for representation in newtypes {
            self.commit_newtype_representation(
                module,
                output,
                environment,
                &mut table,
                representation,
            );
        }

        // write concrete nominal layouts
        for nominal in nominals {
            self.commit_nominal_layout(
                module,
                output,
                environment,
                &mut table,
                pointer_bytes,
                nominal,
            )?;
        }

        // write one layout binding per resolved query target
        for layout in layouts {
            self.commit_layout_resolution(module, output, environment, &mut table, &layout);
        }

        Ok(table)
    }

    /// Commit one newtype representation.
    fn commit_newtype_representation(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        representation: NewtypeRepresentation,
    ) {
        let source = self.symbol_source_node(representation.symbol);
        let Some(backing) =
            self.commit_type_operand(module, output, environment, representation.backing, source)
        else {
            return;
        };

        table.insert_newtype_representation(dir::NewtypeRepresentation {
            symbol: representation.symbol,
            backing,
        });
    }

    /// Commit one concrete nominal layout when fully known.
    fn commit_nominal_layout(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        pointer_bytes: u32,
        symbol: dir::GlobalSymbolId,
    ) -> CompilerResult<()> {
        let source = self.symbol_source_node(symbol);
        let Some(operand) = self.operands.symbol_types.get(&symbol).copied() else {
            panic!("check nominal symbol {symbol:?} has no checked type operand")
        };
        let Some(layout) = self.type_operand_layout(module, operand, pointer_bytes)? else {
            return Ok(());
        };
        let Some(type_id) = self.commit_type_operand(module, output, environment, operand, source)
        else {
            panic!("check nominal symbol {symbol:?} has unresolved checked type {operand:?}")
        };

        self.commit_type_layout(module, output, environment, table, type_id, &layout, source);

        Ok(())
    }

    /// Commit one solved layout query.
    fn commit_layout_resolution(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        resolution: &LayoutResolution,
    ) {
        let Some(type_id) = self.commit_type_operand(
            module,
            output,
            environment,
            resolution.target,
            resolution.source.local_id,
        ) else {
            return;
        };

        self.commit_type_layout(
            module,
            output,
            environment,
            table,
            type_id,
            &resolution.layout,
            resolution.source.local_id,
        );
    }

    /// Commit one concrete layout binding.
    fn commit_type_layout(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        type_id: dir::GlobalTypeId,
        layout: &Layout,
        source: dir::LocalNodeIdAny,
    ) {
        if table.layout_id_for_type(type_id).is_some() {
            return;
        }

        let Some(layout_id) =
            self.commit_layout(module, output, environment, table, layout, source)
        else {
            return;
        };

        table.set_type_layout(type_id, layout_id);
    }

    /// Commit one layout tree.
    fn commit_layout(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        layout: &Layout,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::LocalLayoutId> {
        let shape = match &layout.shape {
            LayoutShape::None => dir::LayoutShape::None,
            LayoutShape::Scalar => dir::LayoutShape::Scalar,
            LayoutShape::Struct(layout) => {
                let fields = self.commit_layout_fields(
                    module,
                    output,
                    environment,
                    table,
                    &layout.fields,
                    source,
                )?;

                dir::LayoutShape::Struct(dir::StructLayout { fields })
            }
            LayoutShape::Tuple(layout) => {
                let elements = self.commit_layout_fields(
                    module,
                    output,
                    environment,
                    table,
                    &layout.elements,
                    source,
                )?;

                dir::LayoutShape::Tuple(dir::TupleLayout { elements })
            }
            LayoutShape::Slice(layout) => {
                let element =
                    self.commit_layout_type(module, output, environment, layout.element, source)?;

                dir::LayoutShape::Slice(dir::SliceLayout { element })
            }
            LayoutShape::Array(layout) => {
                let element =
                    self.commit_layout_type(module, output, environment, layout.element, source)?;

                dir::LayoutShape::Array(dir::ArrayLayout {
                    element,
                    stride: layout.stride,
                    count: layout.count,
                })
            }
            LayoutShape::Variant(layout) => {
                let tag =
                    self.commit_variant_tag(module, output, environment, layout.tag, source)?;
                let variants = self.commit_variant_layouts(
                    module,
                    output,
                    environment,
                    table,
                    &layout.variants,
                    source,
                )?;

                dir::LayoutShape::Variant(dir::VariantLayout {
                    tag,
                    payload_offset: layout.payload_offset,
                    variants,
                })
            }
            LayoutShape::Object(layout) => {
                let fields = self.commit_layout_fields(
                    module,
                    output,
                    environment,
                    table,
                    &layout.fields,
                    source,
                )?;

                dir::LayoutShape::Object(dir::ObjectLayout { fields })
            }
            LayoutShape::Dynamic => dir::LayoutShape::Dynamic,
            LayoutShape::Closure => dir::LayoutShape::Closure,
            LayoutShape::Newtype(layout) => {
                let backing_type = self.commit_layout_type(
                    module,
                    output,
                    environment,
                    layout.backing_type,
                    source,
                )?;
                let backing_layout = self.commit_layout(
                    module,
                    output,
                    environment,
                    table,
                    &layout.backing_layout,
                    source,
                )?;

                dir::LayoutShape::Newtype(dir::NewtypeLayout {
                    backing_type,
                    backing_layout,
                })
            }
        };
        let layout = dir::Layout {
            shape,
            size: layout.size,
            alignment: layout.alignment,
        };

        Some(table.insert_layout(layout))
    }

    /// Commit aggregate layout fields.
    fn commit_layout_fields(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        fields: &[LayoutField],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::LayoutField>> {
        let mut committed = Vec::with_capacity(fields.len());

        // commit fields in source layout order
        for field in fields {
            let ty = self.commit_layout_type(module, output, environment, field.ty, source)?;
            let layout =
                self.commit_layout(module, output, environment, table, &field.layout, source)?;
            committed.push(dir::LayoutField {
                key: field.key,
                ty,
                layout,
                offset: field.offset,
                size: field.size,
                alignment: field.alignment,
            });
        }

        Some(committed)
    }

    /// Commit variant case layouts.
    fn commit_variant_layouts(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        table: &mut dir::LayoutSegment,
        variants: &[VariantCaseLayout],
        source: dir::LocalNodeIdAny,
    ) -> Option<Vec<dir::VariantCaseLayout>> {
        let mut committed = Vec::with_capacity(variants.len());

        // commit variants in source layout order
        for variant in variants {
            let ty = self.commit_layout_type(module, output, environment, variant.ty, source)?;
            let layout =
                self.commit_layout(module, output, environment, table, &variant.layout, source)?;
            committed.push(dir::VariantCaseLayout { ty, layout });
        }

        Some(committed)
    }

    /// Commit one variant tag layout.
    fn commit_variant_tag(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        tag: VariantTagLayout,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::VariantTagLayout> {
        let ty = match tag.ty {
            Some(ty) => Some(self.commit_layout_type(module, output, environment, ty, source)?),
            None => None,
        };

        Some(dir::VariantTagLayout {
            ty,
            size: tag.size,
            alignment: tag.alignment,
        })
    }

    /// Commit the type attached to one layout node.
    fn commit_layout_type(
        &mut self,
        module: ModuleId,
        output: &mut CheckModuleOutput,
        environment: &GlobalEnvironment,
        ty: LayoutType,
        source: dir::LocalNodeIdAny,
    ) -> Option<dir::GlobalTypeId> {
        match ty {
            LayoutType::Operand(operand) => {
                self.commit_type_operand(module, output, environment, operand, source)
            }
            LayoutType::TypeId(ty) => Some(ty),
        }
    }
}
