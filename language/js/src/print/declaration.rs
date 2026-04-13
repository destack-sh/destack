use super::printer::Printer;
use crate::{
    Asynchrony, BindingModifier, Declaration, DeclarationAbstraction, DeclarationDescriptor,
    Declarator, FunctionAbstraction, FunctionCardinality, FunctionSignature, JsPrintResult, Key,
    Keyword, LocalNodeId, Member, Property,
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
            Declaration::Global {
                descriptor,
                statements,
            } => {
                self.print_statement_descriptor(descriptor);
                self.write_keyword(Keyword::Global);
                self.write_punct("{");
                self.print_statement_list(statements)?;
                self.write_punct("}");
            }
            Declaration::Namespace {
                descriptor,
                statements,
            } => {
                self.print_statement_descriptor(descriptor);
                self.write_keyword(Keyword::Namespace);

                if let Some(name) = descriptor.name {
                    self.write_name(name);
                }

                self.write_punct("{");
                self.print_statement_list(statements)?;
                self.write_punct("}");
            }
            Declaration::Type {
                descriptor,
                static_parameters,
                value,
            } => {
                self.print_type_declaration_prefix(descriptor);

                if let Some(static_parameters) = static_parameters {
                    self.write_punct("<");
                    self.print_type_parameter_list(static_parameters)?;
                    self.write_punct(">");
                }

                self.write_punct("=");
                self.print_type_id(*value)?;
            }
            Declaration::Class {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                self.print_class_like_prefix(Keyword::Class, descriptor);

                if self.include_types
                    && let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    self.write_punct("<");
                    self.print_type_parameter_list(static_parameters)?;
                    self.write_punct(">");
                }

                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    self.write_keyword(Keyword::Extends);
                    self.print_type_list(extends_types)?;
                }

                if self.include_types
                    && let Some(implements_types) = heritage.implements_types.as_ref()
                    && !implements_types.is_empty()
                {
                    self.write_keyword(Keyword::Implements);
                    self.print_type_list(implements_types)?;
                }

                self.write_punct("{");
                self.print_member_list(members)?;
                self.write_punct("}");
            }
            Declaration::Interface {
                descriptor,
                generics,
                heritage,
                members,
            } => {
                self.print_class_like_prefix(Keyword::Interface, descriptor);

                if let Some(static_parameters) = generics.static_parameters.as_ref()
                    && !static_parameters.is_empty()
                {
                    self.write_punct("<");
                    self.print_type_parameter_list(static_parameters)?;
                    self.write_punct(">");
                }

                if let Some(extends_types) = heritage.extends_types.as_ref()
                    && !extends_types.is_empty()
                {
                    self.write_keyword(Keyword::Extends);
                    self.print_type_list(extends_types)?;
                }

                self.write_punct("{");
                self.print_member_list(members)?;
                self.write_punct("}");
            }
            Declaration::Enum { descriptor, fields } => {
                self.print_class_like_prefix(Keyword::Enum, descriptor);
                self.write_punct("{");
                self.print_enum_field_list(fields)?;
                self.write_punct("}");
            }
            Declaration::Function {
                descriptor,
                signature,
                body,
            } => {
                self.print_statement_descriptor(descriptor);

                if descriptor.abstraction == DeclarationAbstraction::Abstract {
                    self.write_keyword(Keyword::Abstract);
                }

                if signature.asynchrony == Asynchrony::Async {
                    self.write_keyword(Keyword::Async);
                }

                self.write_keyword(Keyword::Function);

                if signature.cardinality == FunctionCardinality::Generator {
                    self.write_punct("*");
                }

                if let Some(name) = descriptor.name {
                    self.write_name(name);
                }

                self.print_function_signature(signature)?;

                if let Some(body) = body {
                    self.print_block_id(*body)?;
                } else if declaration_id.id != 0 {
                    self.write_punct(";");
                }
            }
        }

        Ok(())
    }

    /// Print one type declaration prefix.
    pub(crate) fn print_type_declaration_prefix(&mut self, descriptor: &DeclarationDescriptor) {
        if let Some(export) = descriptor.export {
            self.write_dependency_mode(export);
        }

        self.write_keyword(Keyword::Type);

        if let Some(name) = descriptor.name {
            self.write_name(name);
        }
    }

    /// Print one class-like declaration prefix.
    pub(crate) fn print_class_like_prefix(
        &mut self,
        keyword: Keyword,
        descriptor: &DeclarationDescriptor,
    ) {
        self.print_statement_descriptor(descriptor);

        if descriptor.abstraction == DeclarationAbstraction::Abstract {
            self.write_keyword(Keyword::Abstract);
        }

        self.write_keyword(keyword);

        if let Some(name) = descriptor.name {
            self.write_name(name);
        }
    }

    /// Print one function signature.
    pub(crate) fn print_function_signature(
        &mut self,
        signature: &FunctionSignature,
    ) -> JsPrintResult<()> {
        if self.include_types
            && let Some(static_parameters) = signature
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

            if !signature.dynamic_parameters.is_empty() {
                self.write_punct(",");
            }
        }

        self.print_parameter_list(&signature.dynamic_parameters)?;

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
                default,
            } => {
                self.print_binding_modifiers_prefix(*modifiers);
                self.print_optional_key(*key)?;
                self.print_binding_modifiers_postfix(*modifiers);

                if let Some(value) = value {
                    self.write_punct(":");
                    self.print_expression_id(*value)?;
                }

                if let Some(default) = default {
                    self.write_punct("=");
                    self.print_expression_id(*default)?;
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
                self.print_optional_key(*key)?;
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

        match signature.abstraction {
            FunctionAbstraction::Abstract => {
                self.write_keyword(Keyword::Abstract);
            }
            FunctionAbstraction::AbstractOverride => {
                self.write_keyword(Keyword::Abstract);
                self.write_keyword(Keyword::Override);
            }
            FunctionAbstraction::ConcreteOverride => {
                self.write_keyword(Keyword::Override);
            }
            FunctionAbstraction::Concrete => {}
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
        self.print_parameter_list(&signature.dynamic_parameters)?;
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
