use super::printer::Printer;
use crate::{
    Argument, EnumField, FunctionMode, JsPrintResult, Keyword, LocalNodeId, Mutability, Parameter,
    Pattern, PatternField, TupleElement, Type, TypeField, TypeModifier, TypePredicateSubject,
};

impl<'a> Printer<'a> {
    /// Print one type.
    pub(crate) fn print_type(&mut self, ty: &Type) -> JsPrintResult<()> {
        match ty {
            Type::Scalar(scalar) => self.print_type_literal(scalar),
            Type::This => self.write_keyword(Keyword::This),
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
            Type::Conditional {
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
            Type::Mapped {
                parameter,
                modifiers,
                value,
            } => {
                self.write_punct("{");

                match modifiers.readonly {
                    TypeModifier::Present => {
                        self.write_keyword(Keyword::Readonly);
                        self.write_punct(" ");
                    }
                    TypeModifier::Add => {
                        self.write_punct("+");
                        self.write_keyword(Keyword::Readonly);
                        self.write_punct(" ");
                    }
                    TypeModifier::Remove => {
                        self.write_punct("-");
                        self.write_keyword(Keyword::Readonly);
                        self.write_punct(" ");
                    }
                    TypeModifier::None => {}
                }

                self.write_punct("[");
                self.write_string_id(parameter.name);
                self.write_punct(" ");
                self.write_keyword(Keyword::In);
                self.write_punct(" ");
                self.print_type_id(parameter.constraint)?;

                if let Some(key_remap) = parameter.key_remap {
                    self.write_punct(" ");
                    self.write_keyword(Keyword::As);
                    self.write_punct(" ");
                    self.print_type_id(key_remap)?;
                }

                self.write_punct("]");

                match modifiers.optional {
                    TypeModifier::Present => self.write_punct("?"),
                    TypeModifier::Add => self.write_punct("+?"),
                    TypeModifier::Remove => self.write_punct("-?"),
                    TypeModifier::None => {}
                }

                self.write_punct(":");
                self.print_type_id(*value)?;
                self.write_punct("}");
            }
            Type::Index { left, index } => {
                self.print_type_id(*left)?;
                self.write_punct("[");
                self.print_type_id(*index)?;
                self.write_punct("]");
            }
            Type::TemplateLiteral(template) => {
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
            Type::Import {
                target,
                qualifier,
                static_arguments,
            } => {
                self.write_keyword(Keyword::Import);
                self.write_punct("(");
                self.write_string_literal(*target);
                self.write_punct(")");

                if let Some(qualifier) = qualifier {
                    self.write_punct(".");
                    self.print_path(qualifier);
                }

                if let Some(static_arguments) = static_arguments {
                    self.print_type_arguments(static_arguments)?;
                }
            }
            Type::Infer { name, constraint } => {
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
            Type::Predicate {
                asserts,
                subject,
                target,
            } => {
                if *asserts {
                    self.write_keyword(Keyword::Asserts);
                    self.write_punct(" ");
                }

                match subject {
                    TypePredicateSubject::Name(name) => self.write_string_id(*name),
                    TypePredicateSubject::This => self.write_keyword(Keyword::This),
                }

                if let Some(target) = target {
                    self.write_punct(" ");
                    self.write_keyword(Keyword::Is);
                    self.write_punct(" ");
                    self.print_type_id(*target)?;
                }
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
                    self.print_type_parameter_list(static_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("(");
                self.print_function_signature_parameters(signature)?;
                self.write_punct(")");

                if let Some(return_type) = signature.return_type {
                    self.write_punct("=>");
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
                    self.print_type_parameter_list(static_parameters)?;
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
            TypeField::IndexSignature {
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

    /// Print one type parameter.
    pub(crate) fn print_type_parameter(&mut self, parameter: &Parameter) -> JsPrintResult<()> {
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
                    self.write_punct(" ");
                    self.write_keyword(Keyword::Extends);
                    self.write_punct(" ");
                    self.print_type_id(*ty)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            _ => {
                self.print_parameter(parameter)?;
            }
        }

        Ok(())
    }

    /// Print one comma-separated type parameter list.
    pub(crate) fn print_type_parameter_list(
        &mut self,
        parameters: &[LocalNodeId<Parameter>],
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

#[cfg(test)]
mod tests {
    use destack_core::StringPool;
    use destack_dir as dir;
    use destack_fir::format::{Document, FormatState, Formatter, VecBuffer};
    use destack_fir::print::Printer as FirPrinter;
    use destack_source::{File, FileId, FileType, ModuleId, Uri};

    use crate::{
        Asynchrony, Expression, FunctionAbstraction, FunctionCardinality, FunctionKind,
        FunctionSignature, Generics, JsFormatContext, JsFormatOptions, LocalNodeId, LocalNodeIdAny,
        NOOP_JS_SOURCE_MAP, NodeTree, Parameter, Path, PrimitiveType, ScalarLiteral, Type,
        TypeLiteral, TypeMappedModifiers, TypeMappedParameter, TypeModifier, TypePredicateSubject,
        TypeTemplateLiteral, TypeUnaryOperator, format_roots, print_roots_minified,
    };

    fn dummy_source_id() -> dir::LocalNodeIdAny {
        dir::LocalNodeIdAny::new(0, dir::NodeType::Expression)
    }

    fn insert_type(tree: &mut NodeTree, ty: Type) -> LocalNodeId<Type> {
        tree.insert_from_source_any(ty, ModuleId::EPHEMERAL, dummy_source_id())
    }

    fn insert_expression(tree: &mut NodeTree, expression: Expression) -> LocalNodeId<Expression> {
        tree.insert_from_source_any(expression, ModuleId::EPHEMERAL, dummy_source_id())
    }

    fn insert_parameter(tree: &mut NodeTree, parameter: Parameter) -> LocalNodeId<Parameter> {
        tree.insert_from_source_any(parameter, ModuleId::EPHEMERAL, dummy_source_id())
    }

    fn build_path(strings: &StringPool, segments: &[&str]) -> Path {
        let segments = segments
            .iter()
            .map(|segment| strings.intern(segment))
            .collect();

        Path { segments }
    }

    fn build_modern_type_roots(tree: &mut NodeTree, strings: &StringPool) -> Vec<LocalNodeIdAny> {
        let type_parameter_t = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["T"]),
                static_arguments: None,
            },
        );
        let type_parameter_u = strings.intern("U");
        let type_parameter_k = strings.intern("K");

        let infer_u = insert_type(
            tree,
            Type::Infer {
                name: type_parameter_u,
                constraint: None,
            },
        );
        let boxed_u_span = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["U"]),
                static_arguments: None,
            },
        );
        let boxed_u = insert_type(
            tree,
            Type::TemplateLiteral(TypeTemplateLiteral {
                strings: vec![strings.intern("box:"), strings.intern("")],
                spans: vec![boxed_u_span],
            }),
        );
        let never = insert_type(tree, Type::Scalar(TypeLiteral::Never));
        let conditional = insert_type(
            tree,
            Type::Conditional {
                left: type_parameter_t,
                right: infer_u,
                then_type: boxed_u,
                else_type: never,
            },
        );

        let string_type = insert_type(
            tree,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::String)),
        );
        let imported_box = insert_type(
            tree,
            Type::Import {
                target: strings.intern("./shared"),
                qualifier: Some(build_path(strings, &["Box"])),
                static_arguments: Some(vec![string_type]),
            },
        );
        let value_key = insert_type(
            tree,
            Type::Scalar(TypeLiteral::ScalarLiteral(ScalarLiteral::String(
                strings.intern("value"),
            ))),
        );
        let import_index = insert_type(
            tree,
            Type::Index {
                left: imported_box,
                index: value_key,
            },
        );

        let keyof_target = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["T"]),
                static_arguments: None,
            },
        );
        let keyof_t = insert_type(
            tree,
            Type::Unary {
                operator: TypeUnaryOperator::Keyof,
                right: keyof_target,
            },
        );
        let key_remap_span = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["K"]),
                static_arguments: None,
            },
        );
        let key_remap = insert_type(
            tree,
            Type::TemplateLiteral(TypeTemplateLiteral {
                strings: vec![strings.intern("box:"), strings.intern("")],
                spans: vec![key_remap_span],
            }),
        );
        let mapped_value_left = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["T"]),
                static_arguments: None,
            },
        );
        let mapped_value_index = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["K"]),
                static_arguments: None,
            },
        );
        let mapped_value = insert_type(
            tree,
            Type::Index {
                left: mapped_value_left,
                index: mapped_value_index,
            },
        );
        let mapped = insert_type(
            tree,
            Type::Mapped {
                parameter: TypeMappedParameter {
                    name: type_parameter_k,
                    constraint: keyof_t,
                    key_remap: Some(key_remap),
                },
                modifiers: TypeMappedModifiers {
                    readonly: TypeModifier::Add,
                    optional: TypeModifier::Add,
                },
                value: mapped_value,
            },
        );

        let predicate_argument = insert_type(
            tree,
            Type::Scalar(TypeLiteral::ScalarLiteral(ScalarLiteral::String(
                strings.intern("alpha"),
            ))),
        );
        let predicate_target = insert_type(
            tree,
            Type::Import {
                target: strings.intern("./shared"),
                qualifier: Some(build_path(strings, &["Box"])),
                static_arguments: Some(vec![predicate_argument]),
            },
        );
        let predicate = insert_type(
            tree,
            Type::Predicate {
                asserts: true,
                subject: TypePredicateSubject::Name(strings.intern("value")),
                target: Some(predicate_target),
            },
        );

        let generic_constraint = insert_type(
            tree,
            Type::Scalar(TypeLiteral::Primitive(PrimitiveType::String)),
        );
        let generic_default = insert_expression(
            tree,
            Expression::ScalarLiteral {
                value: ScalarLiteral::String(strings.intern("alpha")),
            },
        );
        let static_parameter = insert_parameter(
            tree,
            Parameter::Named {
                modifiers: None,
                name: strings.intern("T"),
                ty: Some(generic_constraint),
                default: Some(generic_default),
            },
        );
        let dynamic_parameter_type = insert_type(
            tree,
            Type::Path {
                path: build_path(strings, &["T"]),
                static_arguments: None,
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
            Type::Path {
                path: build_path(strings, &["T"]),
                static_arguments: None,
            },
        );
        let function_type = insert_type(
            tree,
            Type::Function {
                signature: FunctionSignature {
                    abstraction: FunctionAbstraction::Concrete,
                    asynchrony: Asynchrony::Sync,
                    cardinality: FunctionCardinality::Scalar,
                    mode: None,
                    kind: FunctionKind::Lambda,
                    generics: Some(Generics {
                        static_parameters: Some(vec![static_parameter]),
                    }),
                    this_parameter: None,
                    dynamic_parameters: vec![dynamic_parameter],
                    return_type: Some(function_return_type),
                },
            },
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
        tree: &NodeTree,
        roots: &[LocalNodeIdAny],
        strings: &StringPool,
    ) -> String {
        let strings = strings.clone().into_immutable();
        let printed = print_roots_minified(FileType::TypeScript, tree, roots, &strings).unwrap();

        printed.code
    }

    fn format_typescript_roots_pretty(
        tree: &NodeTree,
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
        let strings = strings.clone().into_immutable();
        let context = JsFormatContext {
            options: JsFormatOptions::pretty().with_file_type(FileType::TypeScript),
            file: &file,
            tree,
            roots,
            strings: &strings,
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
        let mut tree = NodeTree::new();
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
        let mut tree = NodeTree::new();
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
