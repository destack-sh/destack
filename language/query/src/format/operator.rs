use destack_dir as dir;

use crate::{QueryError, QueryResult};

use super::r#type::TypeOperand;
use super::{Formatter, formatted};

impl Formatter<'_, '_, '_> {
    /// Format one type operation.
    pub(super) fn operation(&self, operation: &dir::TypeOperation) -> QueryResult<Option<String>> {
        match operation {
            dir::TypeOperation::StringMapping { mapping, target } => {
                let target = formatted!(self.global_type(*target));

                Ok(Some(format!("{}<{target}>", mapping.text())))
            }
            dir::TypeOperation::Conditional(conditional) => self.conditional(*conditional),
            dir::TypeOperation::Narrow(narrow) => Err(QueryError::invalid(format!(
                "narrow operation formatting: {narrow:?}"
            ))),
            dir::TypeOperation::Mapped(mapped) => self.mapped(*mapped),
            dir::TypeOperation::Index(index) => {
                let left = formatted!(self.type_operand(index.left, TypeOperand::Postfix));
                let index = formatted!(self.global_type(index.index));

                Ok(Some(format!("{left}[{index}]")))
            }
            dir::TypeOperation::Infer(infer) => self.infer(*infer),
            dir::TypeOperation::TypeOf(query) => {
                let value = formatted!(self.type_query(query.value));

                Ok(Some(format!("typeof {value}")))
            }
            dir::TypeOperation::KeyOf(target) => {
                let target = formatted!(self.type_operand(target.target, TypeOperand::Prefix));

                Ok(Some(format!("keyof {target}")))
            }
            dir::TypeOperation::NoInfer(target) => {
                let target = formatted!(self.global_type(target.target));

                Ok(Some(format!("NoInfer<{target}>")))
            }
            dir::TypeOperation::Awaited(target) => {
                let target = formatted!(self.global_type(target.target));

                Ok(Some(format!("Awaited<{target}>")))
            }
            dir::TypeOperation::TryOutput { value } => Err(QueryError::invalid(format!(
                "try output formatting: {value:?}"
            ))),
            dir::TypeOperation::TryResidual { value } => Err(QueryError::invalid(format!(
                "try residual formatting: {value:?}"
            ))),
            dir::TypeOperation::StaticBinary(binary) => self.static_binary(*binary),
            dir::TypeOperation::StaticUnary(unary) => self.static_unary(*unary),
            dir::TypeOperation::TemplateLiteral(template) => self.template_literal(*template),
        }
    }

    /// Format one conditional type.
    fn conditional(&self, conditional: dir::ConditionalType) -> QueryResult<Option<String>> {
        let left = formatted!(self.type_operand(conditional.left, TypeOperand::Relation));
        let right = formatted!(self.type_operand(conditional.right, TypeOperand::Relation));
        let then_type = formatted!(self.global_type(conditional.then_type));
        let else_type = formatted!(self.global_type(conditional.else_type));

        Ok(Some(format!(
            "{left} extends {right} ? {then_type} : {else_type}"
        )))
    }

    /// Format one mapped type.
    fn mapped(&self, mapped: dir::MappedType) -> QueryResult<Option<String>> {
        let parameter = self.module.strings().get(mapped.parameter.name);
        let constraint = formatted!(self.global_type(mapped.parameter.constraint));
        let value = formatted!(self.global_type(mapped.value));
        let remap = match mapped.parameter.key_remap {
            Some(key_remap) => {
                let key_remap = formatted!(self.global_type(key_remap));

                format!(" as {key_remap}")
            }
            None => String::new(),
        };
        let readonly = mapped_modifier(mapped.modifiers.readonly, "readonly ");
        let optional = mapped_modifier(mapped.modifiers.optional, "?");

        Ok(Some(format!(
            "{{ {readonly}[{parameter} in {constraint}{remap}]{optional}: {value} }}"
        )))
    }

    /// Format one inferred type.
    fn infer(&self, infer: dir::InferType) -> QueryResult<Option<String>> {
        let name = match (infer.name, infer.symbol) {
            (Some(name), _) => self.module.strings().get(name).to_string(),
            (None, Some(symbol)) => formatted!(self.symbol(symbol)),
            (None, None) => return Err(QueryError::invalid("inferred type name")),
        };
        let constraint = match infer.constraint {
            Some(constraint) => {
                let constraint = formatted!(self.global_type(constraint));

                format!(" extends {constraint}")
            }
            None => String::new(),
        };

        Ok(Some(format!("infer {name}{constraint}")))
    }

    /// Format one static binary type.
    fn static_binary(&self, binary: dir::StaticBinaryType) -> QueryResult<Option<String>> {
        let left = formatted!(self.type_operand(binary.left, TypeOperand::StaticBinary));
        let right = formatted!(self.type_operand(binary.right, TypeOperand::StaticBinary));
        let operator = binary.operator.text();

        Ok(Some(format!("{left} {operator} {right}")))
    }

    /// Format one static unary type.
    fn static_unary(&self, unary: dir::StaticUnaryType) -> QueryResult<Option<String>> {
        let target = formatted!(self.type_operand(unary.target, TypeOperand::Prefix));
        let operator = unary.operator.text();

        Ok(Some(format!("{operator}{target}")))
    }

    /// Format one template literal type.
    fn template_literal(&self, template: dir::TemplateLiteralType) -> QueryResult<Option<String>> {
        let strings = self.module.types().type_ids(template.strings);
        let spans = self.module.types().type_ids(template.spans);
        if strings.len() != spans.len() + 1 {
            return Err(QueryError::invalid("template literal"));
        }

        let mut text = String::from("`");
        for (index, string_type) in strings.iter().copied().enumerate() {
            let segment = formatted!(self.template_segment(string_type));
            text.push_str(&segment);

            if let Some(span_type) = spans.get(index) {
                text.push_str("${");
                let span = formatted!(self.global_type(*span_type));
                text.push_str(&span);
                text.push('}');
            }
        }
        text.push('`');

        Ok(Some(text))
    }

    /// Format one literal string segment from a template type.
    fn template_segment(&self, type_id: dir::GlobalTypeId) -> QueryResult<Option<String>> {
        self.program
            .read_type(type_id, |type_value, module| match type_value {
                dir::Type::Literal(dir::ScalarLiteral::String(string_id)) => {
                    Ok(Some(module.strings().get(*string_id).to_string()))
                }
                dir::Type::Error => Ok(None),
                _ => Err(QueryError::invalid(format!(
                    "template segment: {type_id:?}"
                ))),
            })
    }

    /// Format one type query operand.
    fn type_query(&self, value: dir::GlobalNodeIdAny) -> QueryResult<Option<String>> {
        if value.local_id.ty != dir::NodeType::Expression
            || value.module_id != self.module.module_id()
        {
            return Err(QueryError::invalid(format!("type query: {value:?}")));
        }

        let expression = value.into_typed::<dir::Expression>().local_id;
        let path = self
            .module
            .view()
            .reference_path(expression)
            .ok_or(QueryError::invalid(format!("type query: {value:?}")))?;
        let text = path
            .segments
            .iter()
            .map(|segment| self.module.strings().get(*segment))
            .collect::<Vec<_>>()
            .join(".");

        Ok(Some(text))
    }
}

/// Format one mapped type modifier.
fn mapped_modifier(modifier: dir::MappedTypeModifier, token: &str) -> String {
    if modifier.is_present() {
        format!("{}{token}", modifier.sign())
    } else {
        String::new()
    }
}
