use destack_dir as dir;
use destack_js as js;
use destack_source::NodeSpanType;

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Emit one object property from DIR into JavaScript.
    pub(crate) fn emit_property(
        &mut self,
        property_id: dir::LocalNodeId<dir::Property>,
    ) -> Result<js::LocalNodeId<js::Property>, EmitError> {
        let property = self.tree.get(property_id);
        let property = match property {
            // emit a shorthand field
            dir::Property::Field {
                name,
                value,
                is_shorthand: true,
            } => {
                let dir::Expression::Identifier { name: identifier } = self.tree.get(*value) else {
                    return Err(self.unhandled(
                        property_id.into_global_any(self.module),
                        Some("JavaScript shorthand properties require identifiers".to_string()),
                    ));
                };

                // require matching field and identifier names
                if dir::Name::Identifier(*identifier) != *name {
                    return Err(self.unhandled(
                        property_id.into_global_any(self.module),
                        Some(
                            "JavaScript shorthand property name differs from its value".to_string(),
                        ),
                    ));
                }
                let value = self.emit_identifier_reference(
                    *identifier,
                    value.into_any(),
                    NodeSpanType::Main,
                )?;

                js::Property::Shorthand { value }
            }
            // emit a keyed field
            dir::Property::Field {
                name,
                value,
                is_shorthand: false,
            } => {
                let key = self.property_name(*name, property_id);
                let value = self.emit_expression(*value)?;

                js::Property::Field { key, value }
            }
            // emit a method
            dir::Property::Method {
                name,
                signature,
                body,
            } => self.emit_property_method(property_id, *name, signature, *body)?,
            // emit a spread property
            dir::Property::Spread { value } => js::Property::Spread {
                value: self.emit_expression(*value)?,
            },
            // reject parser error slots
            dir::Property::Error => {
                return Err(self.unhandled(
                    property_id.into_global_any(self.module),
                    Some("property error slots cannot reach JavaScript emission".to_string()),
                ));
            }
        };

        Ok(self.insert_from_source(property, property_id))
    }

    /// Emit one JavaScript object method.
    fn emit_property_method(
        &mut self,
        property: dir::LocalNodeId<dir::Property>,
        name: Option<dir::Name>,
        signature: &dir::FunctionSignature,
        body: Option<dir::LocalNodeId<dir::Expression>>,
    ) -> Result<js::Property, EmitError> {
        let Some(body) = body else {
            return Err(self.unhandled(
                property.into_global_any(self.module),
                Some("JavaScript object methods require executable bodies".to_string()),
            ));
        };

        // require a method name
        let Some(name) = name else {
            return Err(self.unhandled(
                property.into_global_any(self.module),
                Some("JavaScript object methods require names".to_string()),
            ));
        };

        // reject invalid accessor modifiers
        let role = signature.role;
        let is_accessor = matches!(
            role,
            Some(dir::FunctionRole::Getter | dir::FunctionRole::Setter)
        );
        if is_accessor && (signature.asynchrony != dir::Asynchrony::Sync || signature.is_generator)
        {
            return Err(self.unhandled(
                property.into_global_any(self.module),
                Some("JavaScript object accessors cannot be async or generators".to_string()),
            ));
        }

        // emit the signature and body
        let key = self.property_name(name, property);
        let signature = self.emit_function_signature(signature)?;
        let body = self.emit_function_body(body)?;

        // select the method form
        match role {
            // emit a getter
            Some(dir::FunctionRole::Getter) => {
                if !signature.parameters.is_empty() || signature.rest.is_some() {
                    return Err(self.unhandled(
                        property.into_global_any(self.module),
                        Some("JavaScript getters cannot have parameters".to_string()),
                    ));
                }

                Ok(js::Property::Getter { key, body })
            }
            // emit a setter
            Some(dir::FunctionRole::Setter) => {
                let [parameter] = signature.parameters.as_slice() else {
                    return Err(self.unhandled(
                        property.into_global_any(self.module),
                        Some("JavaScript setters require one parameter".to_string()),
                    ));
                };

                // reject a rest parameter
                if signature.rest.is_some() {
                    return Err(self.unhandled(
                        property.into_global_any(self.module),
                        Some("JavaScript setters cannot have rest parameters".to_string()),
                    ));
                }

                Ok(js::Property::Setter {
                    key,
                    parameter: *parameter,
                    body,
                })
            }
            // emit an ordinary method
            None => Ok(js::Property::Method {
                key,
                signature,
                body,
            }),
            // reject non-method roles
            Some(_) => Err(self.unhandled(
                property.into_global_any(self.module),
                Some("invalid JavaScript object method role".to_string()),
            )),
        }
    }
}
