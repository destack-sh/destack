use std::collections::HashSet;
use {destack_dir as dir, destack_mir as mir};

use crate::LowerResult;
use crate::lower::ModuleLowerer;

impl ModuleLowerer<'_> {
    /// Resolve a lowered mir type for a local type id.
    pub(crate) fn lower_type(
        &mut self,
        type_id: dir::LocalTypeId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<mir::LocalNodeId<mir::Type>> {
        // predeclare nominal layouts for nested references
        let mut visited = HashSet::new();
        self.declare_nominal_layouts_for_type(type_id, &mut visited)?;

        // lower the type through the shared cache
        self.type_lowerer.lower_type(
            self.types,
            type_id,
            self.module_id,
            anchor,
            &mut self.builder,
        )
    }

    /// Resolve a lowered mir type for a nominal symbol.
    pub(crate) fn lower_instance_type(
        &mut self,
        symbol: dir::GlobalSymbolId,
        anchor: dir::AnchoredGlobalNodeId,
    ) -> LowerResult<Option<mir::LocalNodeId<mir::Type>>> {
        // resolve the instance type id
        let Some(instance_type_id) = self.types.get_instance_type_id(symbol) else {
            return Ok(None);
        };

        // lower the instance type
        let mir_type = self.lower_type(instance_type_id, anchor)?;
        Ok(Some(mir_type))
    }

    /// Predeclare nominal layouts referenced by a type.
    fn declare_nominal_layouts_for_type(
        &mut self,
        type_id: dir::LocalTypeId,
        visited: &mut HashSet<dir::LocalTypeId>,
    ) -> LowerResult<()> {
        // avoid infinite recursion on cyclic types
        if !visited.insert(type_id) {
            return Ok(());
        }

        // predeclare nominal instance layouts
        if let Some(symbol) = self.types.symbol_for_instance_type(type_id)
            && matches!(
                symbol.ty(),
                dir::SymbolType::Struct | dir::SymbolType::Class
            )
        {
            self.lower_nominal_layout(symbol)?;
        }

        // walk nested type references
        match self.types.get_type(type_id) {
            dir::Type::Reference { symbol, .. } => {
                if matches!(
                    symbol.ty(),
                    dir::SymbolType::Struct | dir::SymbolType::Class
                ) {
                    self.lower_nominal_layout(*symbol)?;
                }
            }
            dir::Type::Value { value } => {
                self.declare_nominal_layouts_for_type(*value, visited)?;
            }
            dir::Type::Conditional {
                left,
                right,
                then_type,
                else_type,
                ..
            } => {
                self.declare_nominal_layouts_for_type(*left, visited)?;
                self.declare_nominal_layouts_for_type(*right, visited)?;
                self.declare_nominal_layouts_for_type(*then_type, visited)?;
                self.declare_nominal_layouts_for_type(*else_type, visited)?;
            }
            dir::Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                // modifiers do not affect nominal layout
                let _ = modifiers;
                self.declare_nominal_layouts_for_type(parameter.constraint, visited)?;
                if let Some(key_remap) = parameter.key_remap {
                    self.declare_nominal_layouts_for_type(key_remap, visited)?;
                }
                self.declare_nominal_layouts_for_type(*value, visited)?;
            }
            dir::Type::Index { left, index } => {
                self.declare_nominal_layouts_for_type(*left, visited)?;
                self.declare_nominal_layouts_for_type(*index, visited)?;
            }
            dir::Type::TemplateLiteral { spans, .. } => {
                for span in spans {
                    self.declare_nominal_layouts_for_type(*span, visited)?;
                }
            }
            dir::Type::Import { .. } => {}
            dir::Type::Infer { constraint, .. } => {
                if let Some(constraint) = constraint {
                    self.declare_nominal_layouts_for_type(*constraint, visited)?;
                }
            }
            dir::Type::Predicate { target, .. } => {
                if let Some(target) = target {
                    self.declare_nominal_layouts_for_type(*target, visited)?;
                }
            }
            dir::Type::Readonly { target_type: right }
            | dir::Type::KeyOf { target_type: right }
            | dir::Type::Must { target_type: right }
            | dir::Type::AsComptime { target_type: right }
            | dir::Type::Not { target_type: right }
            | dir::Type::ValueOf { right, .. }
            | dir::Type::ReferenceOf { right, .. }
            | dir::Type::PointerOf { right, .. } => {
                self.declare_nominal_layouts_for_type(*right, visited)?;
            }
            dir::Type::In { left, right }
            | dir::Type::Extends { left, right }
            | dir::Type::Implements { left, right } => {
                self.declare_nominal_layouts_for_type(*left, visited)?;
                self.declare_nominal_layouts_for_type(*right, visited)?;
            }
            dir::Type::ArraySized { element, .. } => {
                self.declare_nominal_layouts_for_type(*element, visited)?;
            }
            dir::Type::Array { element, .. } => {
                if let Some(element) = element {
                    self.declare_nominal_layouts_for_type(*element, visited)?;
                }
            }
            dir::Type::Tuple { elements, .. } => {
                for element in elements {
                    self.declare_nominal_layouts_for_type(element.ty, visited)?;
                }
            }
            dir::Type::Object {
                fields,
                call_signatures,
                construct_signatures,
                index_signatures,
            } => {
                for field in fields {
                    self.declare_nominal_layouts_for_type(field.ty, visited)?;
                }
                for signature in call_signatures {
                    self.declare_nominal_layouts_for_type(*signature, visited)?;
                }
                for signature in construct_signatures {
                    self.declare_nominal_layouts_for_type(*signature, visited)?;
                }
                for signature in index_signatures {
                    self.declare_nominal_layouts_for_type(signature.key_type, visited)?;
                    self.declare_nominal_layouts_for_type(signature.value_type, visited)?;
                }
            }
            dir::Type::Function {
                generic_parameters,
                this_parameter,
                parameters,
                return_type,
                ..
            } => {
                for parameter in generic_parameters {
                    self.declare_nominal_layouts_for_type(*parameter, visited)?;
                }
                if let Some(this_parameter) = this_parameter {
                    self.declare_nominal_layouts_for_type(*this_parameter, visited)?;
                }
                for parameter in parameters {
                    self.declare_nominal_layouts_for_type(*parameter, visited)?;
                }
                if let Some(return_type) = return_type {
                    self.declare_nominal_layouts_for_type(*return_type, visited)?;
                }
            }
            dir::Type::Union { elements } | dir::Type::Intersection { elements } => {
                for element in elements {
                    self.declare_nominal_layouts_for_type(*element, visited)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
