use super::printer::Printer;
use crate::{
    Asynchrony, BindingModifier, Declaration, Declarator, DependencyMode, FunctionCardinality,
    FunctionSignature, JsPrintResult, Key, Keyword, LocalNodeId, Member, Name, Precedence,
    Property,
};

impl<'a> Printer<'a> {
    /// Print one declaration.
    pub(crate) fn print_declaration(
        &mut self,
        declaration: &Declaration,
        declaration_id: LocalNodeId<Declaration>,
    ) -> JsPrintResult<()> {
        if self.declaration_is_elided(declaration) {
            return Ok(());
        }

        match declaration {
            Declaration::Global(global) => {
                let statements = &global.statements;

                self.print_statement_prefix(None, global.is_ambient);
                self.write_keyword(Keyword::Global);
                self.write_punct("{");
                self.print_statement_list(statements)?;
                self.write_punct("}");
            }
            Declaration::Namespace(namespace) => {
                let statements = &namespace.statements;

                self.print_statement_prefix(namespace.export, namespace.is_ambient);
                self.write_keyword(Keyword::Namespace);

                if let Some(name) = namespace.name {
                    self.write_name(name);
                }

                self.write_punct("{");
                self.print_statement_list(statements)?;
                self.write_punct("}");
            }
            Declaration::Type(ty) => {
                let generic_parameters = &ty.generic_parameters;
                let value = ty.value;

                self.print_type_declaration_prefix(ty.export, ty.name);

                if !generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(generic_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("=");
                self.print_type_id(value)?;
            }
            Declaration::Class(class) => {
                let generic_parameters = &class.generic_parameters;
                let extends_expression = class.extends_expression;
                let extends_generic_arguments = &class.extends_generic_arguments;
                let implements_types = &class.implements_types;
                let members = &class.members;

                self.print_class_like_prefix(
                    Keyword::Class,
                    class.export,
                    class.is_ambient,
                    class.is_abstract,
                    class.name,
                );

                if self.include_types && !generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(generic_parameters)?;
                    self.write_punct(">");
                }

                if let Some(extends_expression) = extends_expression {
                    self.write_keyword(Keyword::Extends);
                    let extends_expression_node = self.tree.get(extends_expression);
                    self.print_expression(
                        extends_expression,
                        extends_expression_node,
                        Precedence::Lowest,
                    )?;

                    if self.include_types && !extends_generic_arguments.is_empty() {
                        self.print_type_arguments(extends_generic_arguments)?;
                    }
                }

                if self.include_types && !implements_types.is_empty() {
                    self.write_keyword(Keyword::Implements);
                    self.print_type_list(implements_types)?;
                }

                self.write_punct("{");
                self.print_member_list(members)?;
                self.write_punct("}");
            }
            Declaration::Interface(interface) => {
                let generic_parameters = &interface.generic_parameters;
                let extends = &interface.extends;
                let members = &interface.members;

                self.print_class_like_prefix(
                    Keyword::Interface,
                    interface.export,
                    interface.is_ambient,
                    false,
                    interface.name,
                );

                if !generic_parameters.is_empty() {
                    self.write_punct("<");
                    self.print_type_parameter_list(generic_parameters)?;
                    self.write_punct(">");
                }

                if !extends.is_empty() {
                    self.write_keyword(Keyword::Extends);

                    for (index, heritage) in extends.iter().enumerate() {
                        if index > 0 {
                            self.write_punct(",");
                        }

                        let expression = self.tree.get(heritage.expression);
                        self.print_expression(heritage.expression, expression, Precedence::Lowest)?;

                        if !heritage.type_arguments.is_empty() {
                            self.print_type_arguments(&heritage.type_arguments)?;
                        }
                    }
                }

                self.write_punct("{");
                self.print_type_member_list(members)?;
                self.write_punct("}");
            }
            Declaration::Enum(enum_declaration) => {
                let fields = &enum_declaration.fields;

                self.print_class_like_prefix(
                    Keyword::Enum,
                    enum_declaration.export,
                    enum_declaration.is_ambient,
                    false,
                    enum_declaration.name,
                );
                self.write_punct("{");
                self.print_enum_field_list(fields)?;
                self.write_punct("}");
            }
            Declaration::Function(function) => {
                let signature = &function.signature;
                let body = function.body;

                self.print_statement_prefix(function.export, function.is_ambient);

                if function.is_abstract {
                    self.write_keyword(Keyword::Abstract);
                }

                if signature.asynchrony == Asynchrony::Async {
                    self.write_keyword(Keyword::Async);
                }

                self.write_keyword(Keyword::Function);

                if signature.cardinality == FunctionCardinality::Generator {
                    self.write_punct("*");
                }

                if let Some(name) = function.name {
                    self.write_name(name);
                }

                self.print_function_signature(signature)?;

                if let Some(body) = body {
                    self.print_block_id(body)?;
                } else if declaration_id.id != 0 {
                    self.write_punct(";");
                }
            }
        }

        Ok(())
    }

    /// Print one type declaration prefix.
    pub(crate) fn print_type_declaration_prefix(
        &mut self,
        export: Option<DependencyMode>,
        name: Option<Name>,
    ) {
        if let Some(export) = export {
            self.write_dependency_mode(export);
        }

        self.write_keyword(Keyword::Type);

        if let Some(name) = name {
            self.write_name(name);
        }
    }

    /// Print one class-like declaration prefix.
    pub(crate) fn print_class_like_prefix(
        &mut self,
        keyword: Keyword,
        export: Option<DependencyMode>,
        is_ambient: bool,
        is_abstract: bool,
        name: Option<Name>,
    ) {
        self.print_statement_prefix(export, is_ambient);

        if is_abstract {
            self.write_keyword(Keyword::Abstract);
        }

        self.write_keyword(keyword);

        if let Some(name) = name {
            self.write_name(name);
        }
    }

    /// Print one function signature.
    pub(crate) fn print_function_signature(
        &mut self,
        signature: &FunctionSignature,
    ) -> JsPrintResult<()> {
        if self.include_types && !signature.generic_parameters.is_empty() {
            self.write_punct("<");
            self.print_type_parameter_list(&signature.generic_parameters)?;
            self.write_punct(">");
        }

        self.write_punct("(");
        self.print_function_signature_parameters(signature)?;
        self.write_punct(")");

        if self.include_types
            && let Some(return_type) = signature.return_type
        {
            self.write_punct(":");
            self.print_type_id(return_type)?;
        }

        Ok(())
    }

    /// Print one function signature parameter sequence.
    pub(crate) fn print_function_signature_parameters(
        &mut self,
        signature: &FunctionSignature,
    ) -> JsPrintResult<()> {
        if self.include_types
            && let Some(this_parameter) = signature.this_parameter
        {
            self.print_parameter_id(this_parameter)?;

            if !signature.parameters.is_empty() {
                self.write_punct(",");
            }
        }

        self.print_parameter_list(&signature.parameters)?;

        Ok(())
    }

    /// Print one declarator.
    pub(crate) fn print_declarator(&mut self, declarator: &Declarator) -> JsPrintResult<()> {
        self.print_pattern_id(declarator.pattern)?;

        if self.include_types
            && let Some(ty) = declarator.ty
        {
            self.write_punct(":");
            self.print_type_id(ty)?;
        }

        if let Some(value) = declarator.value {
            self.write_punct("=");
            self.print_expression_id(value)?;
        }

        Ok(())
    }

    /// Print one property.
    pub(crate) fn print_property(&mut self, property: &Property) -> JsPrintResult<()> {
        match property {
            Property::Field {
                modifiers,
                key,
                value,
                is_shorthand,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);

                if !is_shorthand {
                    self.write_punct(":");
                    self.print_expression_id(*value)?;
                }
            }
            Property::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                self.print_method_like_prefix(*modifiers, *key, signature)?;

                if let Some(body) = body {
                    self.print_block_id(*body)?;
                }
            }
            Property::Spread { modifiers, value } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.write_punct("...");
                self.print_expression_id(*value)?;
            }
        }

        Ok(())
    }

    /// Print one member.
    pub(crate) fn print_member(&mut self, member: &Member) -> JsPrintResult<()> {
        match member {
            Member::Field {
                modifiers,
                key,
                value,
                default,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);

                if let Some(value) = value {
                    self.write_punct(":");
                    self.print_type_id(*value)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
                }
            }
            Member::Method {
                modifiers,
                key,
                signature,
                body,
            } => {
                self.print_method_like_prefix(*modifiers, *key, signature)?;

                if let Some(body) = body {
                    self.print_block_id(*body)?;
                }
            }
            Member::StaticBlock { body } => {
                self.write_keyword(Keyword::Static);
                self.print_block_id(*body)?;
            }
        }

        Ok(())
    }

    /// Print one method-like prefix.
    pub(crate) fn print_method_like_prefix(
        &mut self,
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: &FunctionSignature,
    ) -> JsPrintResult<()> {
        self.print_binding_modifiers_prefix(modifiers);

        if signature.is_abstract {
            self.write_keyword(Keyword::Abstract);
        }

        if signature.is_override {
            self.write_keyword(Keyword::Override);
        }

        if signature.asynchrony == Asynchrony::Async {
            self.write_keyword(Keyword::Async);
        }

        if let Some(mode) = signature.mode
            && let Some(keyword) = mode.to_keyword()
        {
            self.write_keyword(keyword);
        }

        if signature.cardinality == FunctionCardinality::Generator {
            self.write_punct("*");
        }

        self.print_optional_key(key)?;

        if !signature.generic_parameters.is_empty() {
            self.write_punct("<");
            self.print_type_parameter_list(&signature.generic_parameters)?;
            self.write_punct(">");
        }

        self.write_punct("(");
        self.print_parameter_list(&signature.parameters)?;
        self.write_punct(")");

        self.print_binding_modifiers_postfix(modifiers);

        if self.include_types
            && let Some(return_type) = signature.return_type
        {
            self.write_punct(":");
            self.print_type_id(return_type)?;
        }

        Ok(())
    }
    /// Print one optional key.
    pub(crate) fn print_optional_key(&mut self, key: Option<Key>) -> JsPrintResult<()> {
        if let Some(key) = key {
            self.print_key(key)?;
        }

        Ok(())
    }

    /// Print one key.
    pub(crate) fn print_key(&mut self, key: Key) -> JsPrintResult<()> {
        match key {
            Key::Name(name) => self.write_name(name),
            Key::Private(name) => {
                self.write_punct("#");
                self.write_string_id(name);
            }
            Key::Expression(expression) => {
                self.write_punct("[");
                self.print_expression_id(expression)?;
                self.write_punct("]");
            }
            Key::NamedExpression { name, key } => {
                self.write_punct("[");
                self.write_name(name);
                self.write_punct(":");
                self.print_type_id(key)?;
                self.write_punct("]");
            }
        }

        Ok(())
    }
}
