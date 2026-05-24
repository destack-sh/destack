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
            dir::Type::Named(reference) => {
                if matches!(
                    self.symbol_form(reference.symbol),
                    Some(dir::SymbolForm::Struct | dir::SymbolForm::Class)
                ) {
                    self.lower_nominal_layout(reference.symbol)?;
                }
            }
            dir::Type::Form(value) => {
                self.declare_nominal_layouts_for_type(value.value, visited)?;
            }
            dir::Type::Operation(operation) => {
                self.declare_nominal_layouts_for_type_operation(operation, visited)?;
            }
            dir::Type::Predicate(predicate) => {
                if let Some(target) = predicate.target {
                    self.declare_nominal_layouts_for_type(target, visited)?;
                }
            }
            dir::Type::Dynamic(erased) => {
                self.declare_nominal_layouts_for_type(erased.constraint, visited)?;
            }
            dir::Type::FixedArray(array) => {
                self.declare_nominal_layouts_for_type(array.element, visited)?;
            }
            dir::Type::Slice(slice) => {
                self.declare_nominal_layouts_for_type(slice.element, visited)?;
            }
            dir::Type::Tuple(tuple) => {
                for element in &tuple.elements {
                    self.declare_nominal_layouts_for_type(element.ty, visited)?;
                }
            }
            dir::Type::Shape(object) => {
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

    /// Predeclare nominal layouts referenced by one type operation.
    fn declare_nominal_layouts_for_type_operation(
        &mut self,
        operation: &dir::TypeOperation,
        visited: &mut HashSet<dir::LocalTypeId>,
    ) -> LowerResult<()> {
        match operation {
            dir::TypeOperation::BuiltinTypeFunction(_) => {}
            dir::TypeOperation::Conditional(conditional) => {
                self.declare_nominal_layouts_for_type(conditional.left, visited)?;
                self.declare_nominal_layouts_for_type(conditional.right, visited)?;
                self.declare_nominal_layouts_for_type(conditional.then_type, visited)?;
                self.declare_nominal_layouts_for_type(conditional.else_type, visited)?;
            }
            dir::TypeOperation::Mapped(mapped) => {
                self.declare_nominal_layouts_for_type(mapped.parameter.constraint, visited)?;
                if let Some(key_remap) = mapped.parameter.key_remap {
                    self.declare_nominal_layouts_for_type(key_remap, visited)?;
                }
                self.declare_nominal_layouts_for_type(mapped.value, visited)?;
            }
            dir::TypeOperation::Index(index) => {
                self.declare_nominal_layouts_for_type(index.left, visited)?;
                self.declare_nominal_layouts_for_type(index.index, visited)?;
            }
            dir::TypeOperation::TemplateLiteral(template) => {
                for span in &template.spans {
                    self.declare_nominal_layouts_for_type(*span, visited)?;
                }
            }
            dir::TypeOperation::Infer(infer) => {
                if let Some(constraint) = infer.constraint {
                    self.declare_nominal_layouts_for_type(constraint, visited)?;
                }
            }
            dir::TypeOperation::KeyOf(unary) => {
                self.declare_nominal_layouts_for_type(unary.target, visited)?;
            }
        }

        Ok(())
    }
}
