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
        self.metadata_name_for_type(instance_type_id, mir_type, anchor)?;

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
                self.symbol_form(symbol),
                Some(dir::SymbolForm::Struct | dir::SymbolForm::Class)
            )
        {
            self.lower_nominal_layout(symbol)?;
        }

        // walk nested type references
        match self.types.get_type(type_id) {
            dir::Type::Reference(reference) => {
                if matches!(
                    self.symbol_form(reference.symbol),
                    Some(dir::SymbolForm::Struct | dir::SymbolForm::Class)
                ) {
                    self.lower_nominal_layout(reference.symbol)?;
                }
            }
            dir::Type::Value(value) => {
                self.declare_nominal_layouts_for_type(value.value, visited)?;
            }
            dir::Type::Conditional(conditional) => {
                self.declare_nominal_layouts_for_type(conditional.left, visited)?;
                self.declare_nominal_layouts_for_type(conditional.right, visited)?;
                self.declare_nominal_layouts_for_type(conditional.then_type, visited)?;
                self.declare_nominal_layouts_for_type(conditional.else_type, visited)?;
            }
            dir::Type::Mapped(mapped) => {
                self.declare_nominal_layouts_for_type(mapped.parameter.constraint, visited)?;
                if let Some(key_remap) = mapped.parameter.key_remap {
                    self.declare_nominal_layouts_for_type(key_remap, visited)?;
                }
                self.declare_nominal_layouts_for_type(mapped.value, visited)?;
            }
            dir::Type::Index(index) => {
                self.declare_nominal_layouts_for_type(index.left, visited)?;
                self.declare_nominal_layouts_for_type(index.index, visited)?;
            }
            dir::Type::TemplateLiteral(template) => {
                for span in &template.spans {
                    self.declare_nominal_layouts_for_type(*span, visited)?;
                }
            }
            dir::Type::Import(_) => {}
            dir::Type::Infer(infer) => {
                if let Some(constraint) = infer.constraint {
                    self.declare_nominal_layouts_for_type(constraint, visited)?;
                }
            }
            dir::Type::Predicate(predicate) => {
                if let Some(target) = predicate.target {
                    self.declare_nominal_layouts_for_type(target, visited)?;
                }
            }
            dir::Type::KeyOf(unary)
            | dir::Type::Must(unary)
            | dir::Type::AsComptime(unary)
            | dir::Type::Not(unary) => {
                self.declare_nominal_layouts_for_type(unary.target_type, visited)?;
            }
            dir::Type::Form(form) => {
                self.declare_nominal_layouts_for_type(form.base, visited)?;
                self.declare_nominal_layouts_for_type(form.ownership, visited)?;
                self.declare_nominal_layouts_for_type(form.place, visited)?;
                self.declare_nominal_layouts_for_type(form.lifetime, visited)?;
                self.declare_nominal_layouts_for_type(form.access, visited)?;
            }
            dir::Type::In(binary) | dir::Type::Extends(binary) | dir::Type::Implements(binary) => {
                self.declare_nominal_layouts_for_type(binary.left, visited)?;
                self.declare_nominal_layouts_for_type(binary.right, visited)?;
            }
            dir::Type::FixedArray(array) => {
                self.declare_nominal_layouts_for_type(array.element, visited)?;
            }
            dir::Type::Slice(slice) => {
                if let Some(element) = slice.element {
                    self.declare_nominal_layouts_for_type(element, visited)?;
                }
            }
            dir::Type::Tuple(tuple) => {
                for element in &tuple.elements {
                    self.declare_nominal_layouts_for_type(element.ty, visited)?;
                }
            }
            dir::Type::Object(object) => {
                for field in &object.fields {
                    self.declare_nominal_layouts_for_type(field.ty, visited)?;
                }
                for signature in &object.call_signatures {
                    self.declare_nominal_layouts_for_type(*signature, visited)?;
                }
                for signature in &object.construct_signatures {
                    self.declare_nominal_layouts_for_type(*signature, visited)?;
                }
                for signature in &object.index_signatures {
                    self.declare_nominal_layouts_for_type(signature.key_type, visited)?;
                    self.declare_nominal_layouts_for_type(signature.value_type, visited)?;
                }
            }
            dir::Type::Function(function) => {
                for parameter in &function.generic_parameters {
                    self.declare_nominal_layouts_for_type(*parameter, visited)?;
                }
                if let Some(this_parameter) = function.this_parameter {
                    self.declare_nominal_layouts_for_type(this_parameter, visited)?;
                }
                for parameter in &function.parameters {
                    self.declare_nominal_layouts_for_type(*parameter, visited)?;
                }
                if let Some(return_type) = function.return_type {
                    self.declare_nominal_layouts_for_type(return_type, visited)?;
                }
            }
            dir::Type::Union(union) => {
                for element in &union.elements {
                    self.declare_nominal_layouts_for_type(*element, visited)?;
                }
            }
            dir::Type::Intersection(intersection) => {
                for element in &intersection.elements {
                    self.declare_nominal_layouts_for_type(*element, visited)?;
                }
            }
            _ => {}
        }

        Ok(())
    }
}
