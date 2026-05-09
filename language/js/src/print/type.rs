use super::printer::Printer;
use crate::{
    Argument, AssignPattern, AssignPatternField, EnumField, GenericParameter, JsPrintResult,
    Keyword, LocalNodeId, MappedTypeModifier, Mutability, Parameter, Pattern, PatternField,
    TupleElement, TypeExpression, TypeMember, TypePredicateSubject,
};

impl<'a> Printer<'a> {
    /// Print one type.
    pub(crate) fn print_type(&mut self, ty: &TypeExpression) -> JsPrintResult<()> {
        match ty {
            TypeExpression::Scalar(scalar) => self.print_type_literal(scalar),
            TypeExpression::This => self.write_keyword(Keyword::This),
            TypeExpression::Path {
                path,
                generic_arguments,
            } => {
                self.print_path(path);

                if !generic_arguments.is_empty() {
                    self.print_type_arguments(generic_arguments)?;
                }
            }
            TypeExpression::Readonly { target_type } => {
                self.write_keyword(Keyword::Readonly);
                self.write_punct(" ");
                self.print_type_id(*target_type)?;
            }
            TypeExpression::KeyOf { target_type } => {
                self.write_keyword(Keyword::Keyof);
                self.write_punct(" ");
                self.print_type_id(*target_type)?;
            }
            TypeExpression::Must { target_type } => {
                self.print_type_id(*target_type)?;
                self.write_punct("!");
            }
            TypeExpression::AsComptime { target_type } => {
                self.print_type_id(*target_type)?;
                self.write_punct(" ");
                self.write_punct("as comptime");
            }
            TypeExpression::Not { target_type } => {
                self.write_punct("!");
                self.print_type_id(*target_type)?;
            }
            TypeExpression::In { left, right } => {
                self.print_type_id(*left)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::In);
                self.write_punct(" ");
                self.print_type_id(*right)?;
            }
            TypeExpression::Extends { left, right } => {
                self.print_type_id(*left)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::Extends);
                self.write_punct(" ");
                self.print_type_id(*right)?;
            }
            TypeExpression::Implements { left, right } => {
                self.print_type_id(*left)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::Implements);
                self.write_punct(" ");
                self.print_type_id(*right)?;
            }
            TypeExpression::Conditional {
                left,
                right,
                then_type,
                else_type,
            } => {
                self.print_type_id(*left)?;
                self.write_punct(" ");
                self.write_keyword(Keyword::Extends);
                self.write_punct(" ");
                self.print_type_id(*right)?;
                self.write_punct("?");
                self.print_type_id(*then_type)?;
                self.write_punct(":");
                self.print_type_id(*else_type)?;
            }
            TypeExpression::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                self.write_punct("{");

                match modifiers.readonly {
                    MappedTypeModifier::Present => {
                        self.write_keyword(Keyword::Readonly);
                        self.write_punct(" ");
                    }
                    MappedTypeModifier::Add => {
                        self.write_punct("+");
                        self.write_keyword(Keyword::Readonly);
                        self.write_punct(" ");
                    }
                    MappedTypeModifier::Remove => {
                        self.write_punct("-");
                        self.write_keyword(Keyword::Readonly);
                        self.write_punct(" ");
                    }
                    MappedTypeModifier::None => {}
                }

                self.write_punct("[");
                self.write_string_id(parameter.name);
                self.write_punct(" ");
                self.write_keyword(Keyword::In);
                self.write_punct(" ");
                self.print_type_id(parameter.source_type)?;

                if let Some(key_remap) = parameter.key_remap {
                    self.write_punct(" ");
                    self.write_keyword(Keyword::As);
                    self.write_punct(" ");
                    self.print_type_id(key_remap)?;
                }

                self.write_punct("]");

                match modifiers.optional {
                    MappedTypeModifier::Present => self.write_punct("?"),
                    MappedTypeModifier::Add => self.write_punct("+?"),
                    MappedTypeModifier::Remove => self.write_punct("-?"),
                    MappedTypeModifier::None => {}
                }

                self.write_punct(":");
                self.print_type_id(*value)?;
                self.write_punct("}");
            }
            TypeExpression::Index { left, index } => {
                self.print_type_id(*left)?;
                self.write_punct("[");
                self.print_type_id(*index)?;
                self.write_punct("]");
            }
            TypeExpression::TemplateLiteral(template) => {
                self.write_punct("`");

                for (index, string) in template.strings.iter().enumerate() {
                    self.write_string_id(*string);

                    if let Some(span) = template.spans.get(index) {
                        self.write_punct("${");
                        self.print_type_id(*span)?;
                        self.write_punct("}");
                    }
                }

                self.write_punct("`");
            }
            TypeExpression::Import {
                target,
                qualifier,
                generic_arguments,
            } => {
                self.write_keyword(Keyword::Import);
                self.write_punct("(");
                self.write_string_literal(*target);
                self.write_punct(")");

                if let Some(qualifier) = qualifier {
                    self.write_punct(".");
                    self.print_path(qualifier);
                }

                if !generic_arguments.is_empty() {
                    self.print_type_arguments(generic_arguments)?;
                }
            }
            TypeExpression::Infer { name, constraint } => {
                self.write_keyword(Keyword::Infer);
                self.write_punct(" ");
                self.write_string_id(*name);

                if let Some(constraint) = constraint {
                    self.write_punct(" ");
                    self.write_keyword(Keyword::Extends);
                    self.write_punct(" ");
                    self.print_type_id(*constraint)?;
                }
            }
            TypeExpression::Predicate {
                asserts,
                subject,
                target,
            } => {
                if *asserts {
                    self.write_keyword(Keyword::Asserts);
                    self.write_punct(" ");
                }

                match subject {
                    TypePredicateSubject::Identifier(name) => self.write_string_id(*name),
                    TypePredicateSubject::This => self.write_keyword(Keyword::This),
                }

                if let Some(target) = target {
                    self.write_punct(" ");
                    self.write_keyword(Keyword::Is);
                    self.write_punct(" ");
                    self.print_type_id(*target)?;
                }
            }
            TypeExpression::Array { element } => {
                self.print_type_id(*element)?;
                self.write_punct("[]");
            }
            TypeExpression::Tuple { elements } => {
                self.write_punct("[");
                self.print_tuple_element_list(elements)?;
                self.write_punct("]");
            }
            TypeExpression::Object { members } => {
                self.write_punct("{");
                self.print_type_member_list(members)?;
                self.write_punct("}");
            }
            TypeExpression::Union { elements } => {
                for (index, type_id) in elements.iter().enumerate() {
                    if index > 0 {
                        self.write_punct("|");
                    }

                    self.print_type_id(*type_id)?;
                }
            }
            TypeExpression::Intersection { elements } => {
                for (index, type_id) in elements.iter().enumerate() {
                    if index > 0 {
                        self.write_punct("&");
                    }

                    self.print_type_id(*type_id)?;
                }
            }
            TypeExpression::FunctionTypeDeclaration(signature) => {
                if !signature.generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(&signature.generic_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_parameter_list(&signature.parameters)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct("=>");
                    self.print_type_id(return_type)?;
                }
            }
            TypeExpression::ConstructorTypeDeclaration(signature) => {
                if signature.is_abstract {
                    self.write_keyword(Keyword::Abstract);
                    self.write_punct(" ");
                }

                self.write_keyword(Keyword::New);

                if !signature.generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(&signature.generic_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_parameter_list(&signature.parameters)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct("=>");
                    self.print_type_id(return_type)?;
                }
            }
            TypeExpression::Error => self.write_punct("/* ERROR */"),
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
    /// Print one type member.
    pub(crate) fn print_type_member(&mut self, type_member: &TypeMember) -> JsPrintResult<()> {
        match type_member {
            TypeMember::Field { modifiers, key, ty } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);
                self.write_punct(":");
                self.print_type_id(*ty)?;
            }
            TypeMember::Method {
                modifiers,
                key,
                signature,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);

                if !signature.generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(&signature.generic_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_function_signature_parameters(signature)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct(":");
                    self.print_type_id(return_type)?;
                }
            }
            TypeMember::CallSignature {
                modifiers,
                signature,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_binding_modifiers_postfix(*modifiers);

                if !signature.generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(&signature.generic_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_parameter_list(&signature.parameters)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct(":");
                    self.print_type_id(return_type)?;
                }
            }
            TypeMember::ConstructSignature {
                modifiers,
                signature,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_binding_modifiers_postfix(*modifiers);

                if signature.is_abstract {
                    self.write_keyword(Keyword::Abstract);
                    self.write_punct(" ");
                }

                self.write_keyword(Keyword::New);

                if !signature.generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(&signature.generic_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_parameter_list(&signature.parameters)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct(":");
                    self.print_type_id(return_type)?;
                }
            }
            TypeMember::IndexSignature {
                modifiers,
                name,
                key_type,
                value_type,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.write_punct("[");
                self.write_string_id(*name);
                self.write_punct(":");
                self.print_type_id(*key_type)?;
                self.write_punct("]");
                self.print_binding_modifiers_postfix(*modifiers);
                self.write_punct(":");
                self.print_type_id(*value_type)?;
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
            Pattern::Assign { pattern, value } => {
                self.print_pattern_id(*pattern)?;
                self.write_punct("=");
                self.print_expression_id(*value)?;
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
                is_shorthand,
                pattern,
            } => {
                self.print_pattern_field_mutability(*mutability);
                self.write_string_id(*name);

                if !is_shorthand {
                    let pattern = pattern.expect("expanded named js pattern field");
                    self.write_punct(":");
                    self.print_pattern_id(pattern)?;
                } else if let Some(pattern) = pattern {
                    let Pattern::Assign { value, .. } = self.tree.get(*pattern) else {
                        unreachable!("expected shorthand assignment pattern");
                    };
                    self.write_punct("=");
                    self.print_expression_id(*value)?;
                }
            }
            PatternField::Computed {
                mutability,
                key,
                pattern,
            } => {
                self.print_pattern_field_mutability(*mutability);
                self.write_punct("[");
                self.print_expression_id(*key)?;
                self.write_punct("]");
                self.write_punct(":");
                self.print_pattern_id(*pattern)?;
            }
            PatternField::Positional { pattern } => {
                self.print_pattern_id(*pattern)?;
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

    /// Print one assign pattern.
    pub(crate) fn print_assign_pattern(
        &mut self,
        assign_pattern: &AssignPattern,
    ) -> JsPrintResult<()> {
        match assign_pattern {
            AssignPattern::Expression { value } => {
                self.print_expression_id(*value)?;
            }
            AssignPattern::Assign { pattern, value } => {
                self.print_assign_pattern_id(*pattern)?;
                self.write_punct("=");
                self.print_expression_id(*value)?;
            }
            AssignPattern::Array { fields } => {
                self.write_punct("[");
                self.print_assign_pattern_field_list(fields)?;
                self.write_punct("]");
            }
            AssignPattern::Object { fields } => {
                self.write_punct("{");
                self.print_assign_pattern_field_list(fields)?;
                self.write_punct("}");
            }
        }

        Ok(())
    }

    /// Print one assign pattern field.
    pub(crate) fn print_assign_pattern_field(
        &mut self,
        field: &AssignPatternField,
    ) -> JsPrintResult<()> {
        match field {
            AssignPatternField::Named {
                name,
                is_shorthand,
                pattern,
            } => {
                self.write_name(*name);

                if !is_shorthand {
                    let pattern = pattern.expect("expanded named js assign pattern field");
                    self.write_punct(":");
                    self.print_assign_pattern_id(pattern)?;
                } else if let Some(pattern) = pattern {
                    let AssignPattern::Assign { value, .. } = self.tree.get(*pattern) else {
                        unreachable!("expected shorthand assignment target");
                    };
                    self.write_punct("=");
                    self.print_expression_id(*value)?;
                }
            }
            AssignPatternField::Computed { key, pattern } => {
                self.write_punct("[");
                self.print_expression_id(*key)?;
                self.write_punct("]");
                self.write_punct(":");
                self.print_assign_pattern_id(*pattern)?;
            }
            AssignPatternField::Positional { pattern } => {
                self.print_assign_pattern_id(*pattern)?;
            }
            AssignPatternField::Spread { pattern } => {
                self.write_punct("...");

                if let Some(pattern) = pattern {
                    self.print_assign_pattern_id(*pattern)?;
                }
            }
            AssignPatternField::Elision => {}
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

    /// Print one type parameter.
    pub(crate) fn print_type_parameter(
        &mut self,
        parameter: &GenericParameter,
    ) -> JsPrintResult<()> {
        match parameter {
            GenericParameter::Type {
                modifiers,
                name,
                constraint,
                default,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.write_string_id(*name);
                self.print_binding_modifiers_postfix(*modifiers);

                if self.include_types
                    && let Some(constraint) = constraint
                {
                    self.write_punct(" ");
                    self.write_keyword(Keyword::Extends);
                    self.write_punct(" ");
                    self.print_type_id(*constraint)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_type_id(*default)?;
                }
            }
        }

        Ok(())
    }

    /// Print one comma-separated type parameter list.
    pub(crate) fn print_type_parameter_list(
        &mut self,
        parameters: &[LocalNodeId<GenericParameter>],
    ) -> JsPrintResult<()> {
        for (index, parameter_id) in parameters.iter().enumerate() {
            if index > 0 {
                self.write_punct(",");
            }

            let parameter = self.tree.get(*parameter_id);
            self.print_type_parameter(parameter)?;
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
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use destack_core::StringPool;
    use destack_fir::format::{Document, FormatState, Formatter, VecBuffer};
    use destack_fir::print::Printer as FirPrinter;
    use destack_source::{File, FileId, FileType, Uri};

    use crate::{
        FunctionTypeDeclaration, GenericParameter, JsFormatContext, JsFormatOptions, LocalNodeId,
        LocalNodeIdAny, MappedTypeModifier, NOOP_JS_SOURCE_MAP, Parameter, Path, PrimitiveType,
        ScalarLiteral, Tree, TypeExpression, TypeLiteral, TypeMappedModifiers, TypeMappedParameter,
        TypePredicateSubject, TypeTemplateLiteral, format_roots, print_roots_minified,
    };

    fn insert_type(tree: &mut Tree, ty: TypeExpression) -> LocalNodeId<TypeExpression> {
        tree.insert_generated(ty)
    }

    fn insert_parameter(tree: &mut Tree, parameter: Parameter) -> LocalNodeId<Parameter> {
        tree.insert_generated(parameter)
    }

    fn insert_generic_parameter(
        tree: &mut Tree,
        parameter: GenericParameter,
    ) -> LocalNodeId<GenericParameter> {
        tree.insert_generated(parameter)
    }

    fn build_path(strings: &StringPool, segments: &[&str]) -> Path {
        let segments = segments
            .iter()
            .map(|segment| strings.intern(segment))
            .collect();

        Path { segments }
    }

    fn build_modern_type_roots(tree: &mut Tree, strings: &StringPool) -> Vec<LocalNodeIdAny> {
        let type_parameter_t = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["T"]),
                generic_arguments: vec![],
            },
        );
        let type_parameter_u = strings.intern("U");
        let type_parameter_k = strings.intern("K");

        let infer_u = insert_type(
            tree,
            TypeExpression::Infer {
                name: type_parameter_u,
                constraint: None,
            },
        );
        let boxed_u_span = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["U"]),
                generic_arguments: vec![],
            },
        );
        let boxed_u = insert_type(
            tree,
            TypeExpression::TemplateLiteral(TypeTemplateLiteral {
                strings: vec![strings.intern("box:"), strings.intern("")],
                spans: vec![boxed_u_span],
            }),
        );
        let never = insert_type(tree, TypeExpression::Scalar(TypeLiteral::Never));
        let conditional = insert_type(
            tree,
            TypeExpression::Conditional {
                left: type_parameter_t,
                right: infer_u,
                then_type: boxed_u,
                else_type: never,
            },
        );

        let string_type = insert_type(
            tree,
            TypeExpression::Scalar(TypeLiteral::Primitive(PrimitiveType::String)),
        );
        let imported_box = insert_type(
            tree,
            TypeExpression::Import {
                target: strings.intern("./shared"),
                qualifier: Some(build_path(strings, &["Box"])),
                generic_arguments: vec![string_type],
            },
        );
        let value_key = insert_type(
            tree,
            TypeExpression::Scalar(TypeLiteral::ScalarLiteral(ScalarLiteral::String(
                strings.intern("value"),
            ))),
        );
        let import_index = insert_type(
            tree,
            TypeExpression::Index {
                left: imported_box,
                index: value_key,
            },
        );

        let keyof_target = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["T"]),
                generic_arguments: vec![],
            },
        );
        let keyof_t = insert_type(
            tree,
            TypeExpression::KeyOf {
                target_type: keyof_target,
            },
        );
        let key_remap_span = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["K"]),
                generic_arguments: vec![],
            },
        );
        let key_remap = insert_type(
            tree,
            TypeExpression::TemplateLiteral(TypeTemplateLiteral {
                strings: vec![strings.intern("box:"), strings.intern("")],
                spans: vec![key_remap_span],
            }),
        );
        let mapped_value_left = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["T"]),
                generic_arguments: vec![],
            },
        );
        let mapped_value_index = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["K"]),
                generic_arguments: vec![],
            },
        );
        let mapped_value = insert_type(
            tree,
            TypeExpression::Index {
                left: mapped_value_left,
                index: mapped_value_index,
            },
        );
        let mapped = insert_type(
            tree,
            TypeExpression::Mapped {
                parameter: TypeMappedParameter {
                    name: type_parameter_k,
                    source_type: keyof_t,
                    key_remap: Some(key_remap),
                },
                modifiers: TypeMappedModifiers {
                    readonly: MappedTypeModifier::Present,
                    optional: MappedTypeModifier::Present,
                },
                value: mapped_value,
            },
        );

        let predicate_argument = insert_type(
            tree,
            TypeExpression::Scalar(TypeLiteral::ScalarLiteral(ScalarLiteral::String(
                strings.intern("alpha"),
            ))),
        );
        let predicate_target = insert_type(
            tree,
            TypeExpression::Import {
                target: strings.intern("./shared"),
                qualifier: Some(build_path(strings, &["Box"])),
                generic_arguments: vec![predicate_argument],
            },
        );
        let predicate = insert_type(
            tree,
            TypeExpression::Predicate {
                asserts: true,
                subject: TypePredicateSubject::Identifier(strings.intern("value")),
                target: Some(predicate_target),
            },
        );

        let generic_constraint = insert_type(
            tree,
            TypeExpression::Scalar(TypeLiteral::Primitive(PrimitiveType::String)),
        );
        let generic_default = insert_type(
            tree,
            TypeExpression::Scalar(TypeLiteral::ScalarLiteral(ScalarLiteral::String(
                strings.intern("alpha"),
            ))),
        );
        let generic_parameter = insert_generic_parameter(
            tree,
            GenericParameter::Type {
                modifiers: None,
                name: strings.intern("T"),
                constraint: Some(generic_constraint),
                default: Some(generic_default),
            },
        );
        let dynamic_parameter_type = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["T"]),
                generic_arguments: vec![],
            },
        );
        let dynamic_parameter = insert_parameter(
            tree,
            Parameter::Named {
                modifiers: None,
                name: strings.intern("value"),
                ty: Some(dynamic_parameter_type),
                default: None,
            },
        );
        let function_return_type = insert_type(
            tree,
            TypeExpression::Path {
                path: build_path(strings, &["T"]),
                generic_arguments: vec![],
            },
        );
        let function_type = insert_type(
            tree,
            TypeExpression::FunctionTypeDeclaration(FunctionTypeDeclaration {
                generic_parameters: vec![generic_parameter],
                this_parameter: None,
                parameters: vec![dynamic_parameter],
                return_type: Some(function_return_type),
            }),
        );

        vec![
            conditional.into_any(),
            import_index.into_any(),
            mapped.into_any(),
            predicate.into_any(),
            function_type.into_any(),
        ]
    }

    fn print_typescript_roots_minified(
        tree: &Tree,
        roots: &[LocalNodeIdAny],
        strings: &StringPool,
    ) -> String {
        let printed = print_roots_minified(FileType::TypeScript, tree, roots, strings).unwrap();

        printed.code
    }

    fn format_typescript_roots_pretty(
        tree: &Tree,
        roots: &[LocalNodeIdAny],
        strings: &StringPool,
    ) -> String {
        let file = File::from_text(
            FileId(1),
            "test.ts".to_string(),
            Uri::from_string("file:///test.ts"),
            None,
            FileType::TypeScript,
            String::new(),
        );
        let context = JsFormatContext {
            options: JsFormatOptions::pretty().with_file_type(FileType::TypeScript),
            file: &file,
            tree,
            roots,
            strings,
            source_map: &NOOP_JS_SOURCE_MAP,
        };
        let mut state = FormatState::new(context);
        let mut buffer = VecBuffer::new(&mut state);

        {
            let mut formatter = Formatter::new(&mut buffer);
            format_roots(&mut formatter, roots).unwrap();
        }

        let document = Document::from(buffer.into_vec());
        let printed = FirPrinter::new(&file, state.context().options.as_print_options())
            .print(&document)
            .unwrap();

        printed.as_str().to_string()
    }

    /// Print modern typescript type nodes through the direct minified printer.
    #[test]
    fn test_prints_modern_typescript_type_nodes_minified() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        // one representative modern type root list
        let roots = build_modern_type_roots(&mut tree, &strings);

        // exact minified output
        let printed = print_typescript_roots_minified(&tree, &roots, &strings);

        assert_eq!(
            printed,
            "T extends infer U?`box:${U}`:never;import(\"./shared\").Box<string>[\"value\"];{readonly [K in keyof T as `box:${K}`]?:T[K]};asserts value is import(\"./shared\").Box<\"alpha\">;<T extends string=\"alpha\">(value:T)=>T"
        );
    }

    /// Format modern typescript type nodes through the pretty formatter.
    #[test]
    fn test_formats_modern_typescript_type_nodes_pretty() {
        let mut tree = Tree::new();
        let strings = StringPool::new();

        // one representative modern type root list
        let roots = build_modern_type_roots(&mut tree, &strings);

        // exact pretty output
        let printed = format_typescript_roots_pretty(&tree, &roots, &strings);

        assert_eq!(
            printed,
            "T extends infer U ? `box:${U}` : never\nimport(\"./shared\").Box<string>[\"value\"]\n{readonly [K in keyof T as `box:${K}`]?: T[K]}\nasserts value is import(\"./shared\").Box<\"alpha\">\n<T extends string = \"alpha\">(value: T) => T\n"
        );
    }
}
