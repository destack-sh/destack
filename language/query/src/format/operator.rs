use destack_dir as dir;

use crate::{QueryError, QueryResult};

use super::Formatter;
use super::r#type::TypeOperand;

impl Formatter<'_, '_, '_> {
    /// Format one type operation.
    pub(super) fn operation(&self, operation: &dir::TypeOperation) -> QueryResult<String> {
        match operation {
            dir::TypeOperation::StringMapping { mapping, target } => {
                let target = self.global_type(*target)?;

                Ok(format!("{}<{target}>", mapping.text()))
            }
            dir::TypeOperation::Conditional(conditional) => self.conditional(*conditional),
            dir::TypeOperation::Narrow(narrow) => {
                let source = self.global_type(narrow.source)?;
                let target = self.global_type(narrow.target)?;
                let operator = if narrow.is_positive { "is" } else { "is not" };

                Ok(format!("{source} {operator} {target}"))
            }
            dir::TypeOperation::Mapped(mapped) => self.mapped(*mapped),
            dir::TypeOperation::Index(index) => {
                let left = self.type_operand(index.left, TypeOperand::Postfix)?;
                let index = self.global_type(index.index)?;

                Ok(format!("{left}[{index}]"))
            }
            dir::TypeOperation::Infer(infer) => self.infer(*infer),
            dir::TypeOperation::TypeOf(query) => {
                let value = self.type_query(query.value)?;

                Ok(format!("typeof {value}"))
            }
            dir::TypeOperation::KeyOf(target) => {
                let target = self.type_operand(target.target, TypeOperand::Prefix)?;

                Ok(format!("keyof {target}"))
            }
            dir::TypeOperation::NoInfer(target) => {
                let target = self.global_type(target.target)?;

                Ok(format!("NoInfer<{target}>"))
            }
            dir::TypeOperation::Awaited(target) => {
                let target = self.global_type(target.target)?;

                Ok(format!("Awaited<{target}>"))
            }
            dir::TypeOperation::TryOutput { value } => {
                let value = self.global_type(*value)?;

                Ok(format!("TryOutput<{value}>"))
            }
            dir::TypeOperation::TryResidual { value } => {
                let value = self.global_type(*value)?;

                Ok(format!("TryResidual<{value}>"))
            }
            dir::TypeOperation::StaticBinary(binary) => self.static_binary(*binary),
            dir::TypeOperation::StaticUnary(unary) => self.static_unary(*unary),
            dir::TypeOperation::TemplateLiteral(template) => self.template_literal(*template),
        }
    }

    /// Format one conditional type.
    fn conditional(&self, conditional: dir::ConditionalType) -> QueryResult<String> {
        let left = self.type_operand(conditional.left, TypeOperand::Relation)?;
        let right = self.type_operand(conditional.right, TypeOperand::Relation)?;
        let then_type = self.global_type(conditional.then_type)?;
        let else_type = self.global_type(conditional.else_type)?;

        Ok(format!(
            "{left} extends {right} ? {then_type} : {else_type}"
        ))
    }

    /// Format one mapped type.
    fn mapped(&self, mapped: dir::MappedType) -> QueryResult<String> {
        let parameter = self.module.strings().get(mapped.parameter.name);
        let constraint = self.global_type(mapped.parameter.constraint)?;
        let value = self.global_type(mapped.value)?;
        let remap = match mapped.parameter.key_remap {
            Some(key_remap) => {
                let key_remap = self.global_type(key_remap)?;

                format!(" as {key_remap}")
            }
            None => String::new(),
        };
        let readonly = mapped_modifier(mapped.modifiers.readonly, "readonly ");
        let optional = mapped_modifier(mapped.modifiers.optional, "?");

        Ok(format!(
            "{{ {readonly}[{parameter} in {constraint}{remap}]{optional}: {value} }}"
        ))
    }

    /// Format one inferred type.
    fn infer(&self, infer: dir::InferType) -> QueryResult<String> {
        let name = match (infer.name, infer.symbol) {
            (Some(name), _) => self.module.strings().get(name).to_string(),
            (None, Some(symbol)) => self.symbol(symbol)?,
            (None, None) => return Err(QueryError::invalid("inferred type name")),
        };
        let constraint = match infer.constraint {
            Some(constraint) => {
                let constraint = self.global_type(constraint)?;

                format!(" extends {constraint}")
            }
            None => String::new(),
        };

        Ok(format!("infer {name}{constraint}"))
    }

    /// Format one static binary type.
    fn static_binary(&self, binary: dir::StaticBinaryType) -> QueryResult<String> {
        let left = self.type_operand(binary.left, TypeOperand::StaticBinary)?;
        let right = self.type_operand(binary.right, TypeOperand::StaticBinary)?;
        let operator = binary.operator.text();

        Ok(format!("{left} {operator} {right}"))
    }

    /// Format one static unary type.
    fn static_unary(&self, unary: dir::StaticUnaryType) -> QueryResult<String> {
        let target = self.type_operand(unary.target, TypeOperand::Prefix)?;
        let operator = unary.operator.text();

        Ok(format!("{operator}{target}"))
    }

    /// Format one template literal type.
    fn template_literal(&self, template: dir::TemplateLiteralType) -> QueryResult<String> {
        let strings = self.module.types().type_ids(template.strings);
        let spans = self.module.types().type_ids(template.spans);
        if strings.len() != spans.len() + 1 {
            return Err(QueryError::invalid("template literal"));
        }

        let mut text = String::from("`");
        for (index, string_type) in strings.iter().copied().enumerate() {
            let segment = self.template_segment(string_type)?;
            text.push_str(&segment);

            if let Some(span_type) = spans.get(index) {
                text.push_str("${");
                let span = self.global_type(*span_type)?;
                text.push_str(&span);
                text.push('}');
            }
        }
        text.push('`');

        Ok(text)
    }

    /// Format one literal string segment from a template type.
    fn template_segment(&self, type_id: dir::GlobalTypeId) -> QueryResult<String> {
        self.query
            .read_type(type_id, |type_value, module| match type_value {
                dir::Type::Literal(dir::ScalarLiteral::String(string_id)) => {
                    Ok(module.strings().get(*string_id).to_string())
                }
                dir::Type::Error => Err(QueryError::missing(format!(
                    "template segment type: {type_id:?}"
                ))),
                _ => Err(QueryError::invalid(format!(
                    "template segment: {type_id:?}"
                ))),
            })
    }

    /// Format one type query operand.
    fn type_query(&self, value: dir::GlobalNodeIdAny) -> QueryResult<String> {
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

        Ok(text)
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
