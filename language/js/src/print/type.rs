use super::printer::Printer;
use crate::{
    Argument, EnumField, FunctionMode, JsPrintResult, Keyword, Mutability, Parameter, Pattern,
    PatternField, TupleElement, Type, TypeField,
};

impl<'a> Printer<'a> {
    /// Print one type.
    pub(crate) fn print_type(&mut self, ty: &Type) -> JsPrintResult<()> {
        match ty {
            Type::Scalar(scalar) => self.print_type_literal(scalar),
            Type::Path {
                path,
                static_arguments,
            } => {
                self.print_path(path);

                if let Some(static_arguments) = static_arguments {
                    self.print_type_arguments(static_arguments)?;
                }
            }
            Type::Expression(expression) => {
                self.print_expression_id(*expression)?;
            }
            Type::Unary { operator, right } => {
                self.write_type_unary_operator(*operator);
                self.write_punct(" ");
                self.print_type_id(*right)?;
            }
            Type::Binary {
                left,
                operator,
                right,
            } => {
                self.print_type_id(*left)?;
                self.write_punct(" ");
                self.write_type_binary_operator(*operator);
                self.write_punct(" ");
                self.print_type_id(*right)?;
            }
            Type::Array { element } => {
                if let Some(element) = element {
                    self.print_type_id(*element)?;
                    self.write_punct("[]");
                } else {
                    self.write_punct("Array<any>");
                }
            }
            Type::Tuple { elements } => {
                self.write_punct("[");
                self.print_tuple_element_list(elements)?;
                self.write_punct("]");
            }
            Type::Object { properties } => {
                self.write_punct("{");
                self.print_type_field_list(properties)?;
                self.write_punct("}");
            }
            Type::Union { elements } => {
                for (index, type_id) in elements.iter().enumerate() {
                    if index > 0 {
                        self.write_punct("|");
                    }

                    self.print_type_id(*type_id)?;
                }
            }
            Type::Intersection { elements } => {
                for (index, type_id) in elements.iter().enumerate() {
                    if index > 0 {
                        self.write_punct("&");
                    }

                    self.print_type_id(*type_id)?;
                }
            }
            Type::Function { signature } => {
                if let Some(mode) = signature.mode
                    && mode == FunctionMode::New
                {
                    self.write_keyword(Keyword::New);
                }

                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    self.write_punct("<");
                    self.print_parameter_list(static_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_parameter_list(&signature.dynamic_parameters)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct(":");
                    self.print_type_id(return_type)?;
                }
            }
            Type::Error => self.write_punct("/* ERROR */"),
        }

        Ok(())
    }

    /// Print one tuple element.
    pub(crate) fn print_tuple_element(
        &mut self,
        tuple_element: &TupleElement,
    ) -> JsPrintResult<()> {
        if tuple_element.is_readonly {
            self.write_keyword(Keyword::Readonly);
        }

        if tuple_element.is_rest {
            self.write_punct("...");
        }

        if let Some(label) = tuple_element.label {
            self.write_string_id(label);

            if tuple_element.is_optional {
                self.write_punct("?");
            }

            self.write_punct(":");
        }

        self.print_type_id(tuple_element.ty)?;

        if tuple_element.label.is_none() && tuple_element.is_optional {
            self.write_punct("?");
        }

        Ok(())
    }

    /// Print one tuple element list.
    pub(crate) fn print_tuple_element_list(
        &mut self,
        elements: &[crate::LocalNodeId<TupleElement>],
    ) -> JsPrintResult<()> {
        for (index, element_id) in elements.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            self.print_tuple_element_id(*element_id)?;
        }

        Ok(())
    }
    /// Print one type field.
    pub(crate) fn print_type_field(&mut self, type_field: &TypeField) -> JsPrintResult<()> {
        match type_field {
            TypeField::Field { modifiers, key, ty } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_optional_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);
                self.write_punct(":");
                self.print_type_id(*ty)?;
            }
            TypeField::Method {
                modifiers,
                key,
                signature,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_optional_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);

                if let Some(mode) = signature.mode
                    && mode == FunctionMode::New
                {
                    self.write_keyword(Keyword::New);
                }

                if let Some(static_parameters) = signature
                    .generics
                    .as_ref()
                    .and_then(|generics| generics.static_parameters.as_ref())
                    && !static_parameters.is_empty()
                {
                    self.write_punct("<");
                    self.print_parameter_list(static_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_parameter_list(&signature.dynamic_parameters)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct(":");
                    self.print_type_id(return_type)?;
                }
            }
        }

        Ok(())
    }

    /// Print one enum field.
    pub(crate) fn print_enum_field(&mut self, field: &EnumField) -> JsPrintResult<()> {
        self.write_string_id(field.name);

        if let Some(value) = field.value {
            self.write_punct("=");
            self.print_expression_id(value)?;
        }

        Ok(())
    }

    /// Print one pattern.
    pub(crate) fn print_pattern(&mut self, pattern: &Pattern) -> JsPrintResult<()> {
        match pattern {
            Pattern::Binding { mutability, name } => {
                if mutability == &Some(Mutability::Immutable) {
                    self.write_keyword(Keyword::Const);
                }

                self.write_string_id(*name);
            }
            Pattern::Array { fields } => {
                self.write_punct("[");
                self.print_pattern_array_field_list(fields)?;
                self.write_punct("]");
            }
            Pattern::Object { fields } => {
                self.write_punct("{");
                self.print_pattern_field_list(fields)?;
                self.write_punct("}");
            }
            Pattern::Hole => {
                self.write_punct(",");
            }
        }

        Ok(())
    }

    /// Print one pattern field.
    pub(crate) fn print_pattern_field(&mut self, field: &PatternField) -> JsPrintResult<()> {
        match field {
            PatternField::Named {
                mutability,
                name,
                pattern,
                default,
            } => {
                self.print_pattern_field_mutability(*mutability);
                self.write_string_id(*name);

                if let Some(pattern) = pattern {
                    self.write_punct(":");
                    self.print_pattern_id(*pattern)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            PatternField::Computed {
                mutability,
                key,
                pattern,
                default,
            } => {
                self.print_pattern_field_mutability(*mutability);
                self.write_punct("[");
                self.print_expression_id(*key)?;
                self.write_punct("]");

                if let Some(pattern) = pattern {
                    self.write_punct(":");
                    self.print_pattern_id(*pattern)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            PatternField::Alias {
                mutability,
                name,
                alias,
                default,
            } => {
                self.print_pattern_field_mutability(*mutability);
                self.write_string_id(*name);
                self.write_punct(":");
                self.write_string_id(*alias);

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            PatternField::Positional { pattern, default } => {
                self.print_pattern_id(*pattern)?;

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            PatternField::Spread { pattern, .. } => {
                self.write_punct("...");

                if let Some(pattern) = pattern {
                    self.print_pattern_id(*pattern)?;
                }
            }
            PatternField::Elision => {}
        }

        Ok(())
    }

    /// Print one pattern field mutability prefix.
    pub(crate) fn print_pattern_field_mutability(&mut self, mutability: Option<Mutability>) {
        if mutability == Some(Mutability::Immutable) {
            self.write_keyword(Keyword::Const);
        }
    }

    /// Print one parameter.
    pub(crate) fn print_parameter(&mut self, parameter: &Parameter) -> JsPrintResult<()> {
        match parameter {
            Parameter::Named {
                modifiers,
                name,
                ty,
                default,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.write_string_id(*name);
                self.print_binding_modifiers_postfix(*modifiers);

                if self.include_types
                    && let Some(ty) = ty
                {
                    self.write_punct(":");
                    self.print_type_id(*ty)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            Parameter::Pattern {
                modifiers,
                pattern,
                ty,
                default,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_pattern_id(*pattern)?;
                self.print_binding_modifiers_postfix(*modifiers);

                if self.include_types
                    && let Some(ty) = ty
                {
                    self.write_punct(":");
                    self.print_type_id(*ty)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            Parameter::VariadicNamed {
                modifiers,
                name,
                ty,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.write_punct("...");
                self.write_string_id(*name);

                if self.include_types
                    && let Some(ty) = ty
                {
                    self.write_punct(":");
                    self.print_type_id(*ty)?;
                }
            }
            Parameter::VariadicPattern {
                modifiers,
                pattern,
                ty,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.write_punct("...");
                self.print_pattern_id(*pattern)?;

                if self.include_types
                    && let Some(ty) = ty
                {
                    self.write_punct(":");
                    self.print_type_id(*ty)?;
                }
            }
        }

        Ok(())
    }

    /// Print one argument.
    pub(crate) fn print_argument(&mut self, argument: &Argument) -> JsPrintResult<()> {
        match argument {
            Argument::Positional { value } => self.print_expression_id(*value)?,
            Argument::Spread { value } => {
                self.write_punct("...");
                self.print_expression_id(*value)?;
            }
            Argument::Dynamic { key, value } => {
                self.write_punct("[");
                self.print_expression_id(*key)?;
                self.write_punct("]:");
                self.print_expression_id(*value)?;
            }
        }

        Ok(())
    }
}
