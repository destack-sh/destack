use destack_dir as dir;
use destack_js as js;

use crate::EmitError;
use crate::emit::js::ScriptEmitter;

impl ScriptEmitter<'_> {
    /// Emit the runtime members of one class declaration.
    pub(super) fn emit_members(
        &mut self,
        members: &[dir::LocalNodeId<dir::Member>],
    ) -> Result<Vec<js::LocalNodeId<js::Member>>, EmitError> {
        let mut emitted = Vec::with_capacity(members.len());

        // emit members in source order
        for member in members.iter().copied() {
            if let Some(member) = self.emit_member(member)? {
                emitted.push(member);
            }
        }

        Ok(emitted)
    }

    /// Emit one runtime class member.
    fn emit_member(
        &mut self,
        source: dir::LocalNodeId<dir::Member>,
    ) -> Result<Option<js::LocalNodeId<js::Member>>, EmitError> {
        let member = self.tree.get(source);
        let member = match member {
            // erase type-level members
            dir::Member::AssociatedType { .. }
            | dir::Member::AssociatedConst { .. }
            | dir::Member::ConstBlock { .. } => return Ok(None),
            // emit a field
            dir::Member::Field {
                name,
                default,
                is_static,
                is_accessor: false,
                ..
            } => {
                let key = js::ClassElementName::Public(self.property_name(*name, source));
                let default = default
                    .map(|default| self.emit_expression(default))
                    .transpose()?;

                js::Member::Field {
                    key,
                    default,
                    is_static: *is_static,
                }
            }
            // reject auto-accessor fields
            dir::Member::Field {
                is_accessor: true, ..
            } => {
                return Err(self.unhandled(
                    source.into_global_any(self.module),
                    Some("JavaScript auto-accessor fields are not represented".to_string()),
                ));
            }
            // emit a method
            dir::Member::Method {
                name,
                signature,
                body,
                is_static,
                is_accessor,
                ..
            } => {
                // erase methods without runtime bodies
                let Some(body) = *body else {
                    return Ok(None);
                };

                let member = self.emit_class_method(
                    source,
                    *name,
                    signature,
                    body,
                    *is_static,
                    *is_accessor,
                )?;

                member
            }
            // emit a static block
            dir::Member::StaticBlock { body } => js::Member::StaticBlock {
                body: self.emit_body(*body)?,
            },
            // reject parser error slots
            dir::Member::Error => {
                return Err(self.unhandled(
                    source.into_global_any(self.module),
                    Some("member error slots cannot reach JavaScript emission".to_string()),
                ));
            }
        };

        let member = self.insert_from_source(member, source);

        Ok(Some(member))
    }

    /// Emit one JavaScript class method.
    fn emit_class_method(
        &mut self,
        source: dir::LocalNodeId<dir::Member>,
        name: Option<dir::Name>,
        signature: &dir::FunctionSignature,
        body: dir::LocalNodeId<dir::Expression>,
        is_static: bool,
        is_accessor: bool,
    ) -> Result<js::Member, EmitError> {
        // reject auto-accessor methods
        if is_accessor {
            return Err(self.unhandled(
                source.into_global_any(self.module),
                Some("JavaScript auto-accessor methods are not represented".to_string()),
            ));
        }

        // reject invalid constructor and accessor modifiers
        let role = signature.role;
        let is_constructor_or_accessor = matches!(
            role,
            Some(
                dir::FunctionRole::Constructor
                    | dir::FunctionRole::Getter
                    | dir::FunctionRole::Setter
            )
        );
        if is_constructor_or_accessor
            && (signature.asynchrony != dir::Asynchrony::Sync || signature.is_generator)
        {
            return Err(self.unhandled(
                source.into_global_any(self.module),
                Some(
                    "JavaScript constructors and accessors cannot be async or generators"
                        .to_string(),
                ),
            ));
        }

        // emit the signature and body
        let signature = self.emit_function_signature(signature)?;
        let body = self.emit_function_body(body)?;

        // select the class method form
        let member = match role {
            // emit a constructor
            Some(dir::FunctionRole::Constructor) => {
                if is_static {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript constructors cannot be static".to_string()),
                    ));
                }

                js::Member::Constructor {
                    parameters: signature.parameters,
                    rest: signature.rest,
                    body,
                }
            }
            // emit a getter
            Some(dir::FunctionRole::Getter) => {
                let Some(name) = name else {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript getters require names".to_string()),
                    ));
                };

                // require an empty parameter list
                if !signature.parameters.is_empty() || signature.rest.is_some() {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript getters cannot have parameters".to_string()),
                    ));
                }

                let key = js::ClassElementName::Public(self.property_name(name, source));

                js::Member::Getter {
                    key,
                    body,
                    is_static,
                }
            }
            // emit a setter
            Some(dir::FunctionRole::Setter) => {
                let Some(name) = name else {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript setters require names".to_string()),
                    ));
                };

                // require one ordinary parameter
                let [parameter] = signature.parameters.as_slice() else {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript setters require one parameter".to_string()),
                    ));
                };
                if signature.rest.is_some() {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript setters cannot have rest parameters".to_string()),
                    ));
                }

                let key = js::ClassElementName::Public(self.property_name(name, source));

                js::Member::Setter {
                    key,
                    parameter: *parameter,
                    body,
                    is_static,
                }
            }
            // emit an ordinary method
            None => {
                let Some(name) = name else {
                    return Err(self.unhandled(
                        source.into_global_any(self.module),
                        Some("JavaScript methods require names".to_string()),
                    ));
                };

                let key = js::ClassElementName::Public(self.property_name(name, source));

                js::Member::Method {
                    key,
                    signature,
                    body,
                    is_static,
                }
            }
            // reject call and new signatures
            Some(dir::FunctionRole::New | dir::FunctionRole::Call) => {
                return Err(self.unhandled(
                    source.into_global_any(self.module),
                    Some("invalid JavaScript class method role".to_string()),
                ));
            }
        };

        Ok(member)
    }
}
