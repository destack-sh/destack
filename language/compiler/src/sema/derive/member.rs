use destack_core::StringId;
use destack_dir as dir;
use destack_source::{ModuleId, Span};

use crate::sema::{CheckState, Origin, ProtocolCall, TypeSubstitution, Verdict};
use crate::{CompilerError, CompilerResult};

/// One component a derived body runs over and the protocol call it runs on it.
pub(in crate::sema) struct Component {
    /// How the body reads the component beneath its value.
    pub(in crate::sema) read: ComponentProjection,
    /// The type the component stores.
    pub(in crate::sema) ty: dir::GlobalTypeId,
    /// The protocol call the component runs, absent on a valueless unit member.
    pub(in crate::sema) call: Option<ProtocolCall>,
}

/// The projection one derived body reads a component through.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum ComponentProjection {
    /// A field beneath the value, by key and by symbol on nominal receivers.
    Field {
        /// The key naming the field.
        key: dir::StaticKey,
        /// The field symbol, on a nominal receiver.
        symbol: Option<dir::GlobalSymbolId>,
    },
    /// The value itself, a newtype's backing.
    Backing,
    /// The value at the base its heritage extends.
    Base,
    /// The value narrowed to one union member.
    Member,
}

/// The composite shape one derived body assembles its receiver as.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Composite {
    /// A nominal struct constructed field by field.
    Struct(dir::GlobalSymbolId),
    /// A tuple constructed element by element.
    Tuple,
    /// A newtype wrapping its backing value.
    Newtype(dir::GlobalSymbolId),
    /// A union dispatched member by member.
    Union,
    /// A class rendered field by field.
    Class(dir::GlobalSymbolId),
}

/// One derivation: the synthesized member and the frame its body reads its parameters through.
pub(in crate::sema) struct Derivation {
    /// The member symbol.
    pub(in crate::sema) symbol: dir::GlobalSymbolId,
    /// The scope every synthesized node sits in.
    pub(in crate::sema) scope: dir::LocalScope,
    /// The body node, whose scope the body's instances close under.
    pub(in crate::sema) body: Option<dir::GlobalNodeIdAny>,
    /// The member's callable type at its receiver.
    pub(in crate::sema) callable: dir::GlobalTypeId,
    /// The module the body lives in.
    pub(in crate::sema) module: ModuleId,
    /// The span every synthesized node takes.
    pub(in crate::sema) span: Span,
    /// The receiver type the body runs over.
    pub(in crate::sema) receiver: dir::GlobalTypeId,
    /// The type `this` reads as, absent on static bodies.
    pub(in crate::sema) this: Option<dir::GlobalTypeId>,
    /// The parameter symbols and types after the receiver, in declaration order.
    pub(in crate::sema) parameters: Vec<(dir::LocalSymbolId, StringId, dir::GlobalTypeId)>,
    /// The key naming the member every component runs.
    pub(in crate::sema) member: dir::StaticKey,
    /// Every call the body records, their instances closing beside the member's.
    pub(in crate::sema) calls: Vec<dir::Call>,
}

impl Derivation {
    /// Iterate the parameter symbols with their types.
    pub(in crate::sema) fn parameters(
        &self,
    ) -> impl Iterator<Item = (dir::GlobalSymbolId, dir::GlobalTypeId)> + '_ {
        let module = self.module;

        self.parameters
            .iter()
            .map(move |(symbol, _, ty)| (symbol.into_global(module), *ty))
    }
}

impl CheckState<'_> {
    /// Build the member implementing one derivable requirement at a composite receiver.
    pub(in crate::sema) fn build_derived_member(
        &mut self,
        requirement: dir::GlobalSymbolId,
        receiver: dir::GlobalTypeId,
        arguments: &[dir::GenericArgumentBinding],
        source: dir::GlobalNodeIdAny,
    ) -> CompilerResult<Option<Derivation>> {
        // only a derivable requirement synthesizes a member
        let Some((interface, item)) = self.derived_requirement(requirement)? else {
            return Ok(None);
        };
        let origin = Origin::Node(source, None);

        // clone a bitwise copy by read and a class handle through its object
        let is_class = match self.nominal_application_maybe(receiver)? {
            Some((_, instance)) => matches!(
                self.definition(instance.symbol)?.as_deref(),
                Some(dir::Definition::Class(_))
            ),
            None => false,
        };
        if interface == dir::AutoInterface::Clone && !is_class {
            let (verdict, _) = self.decide(|state| {
                state.decide_auto_interface(origin, receiver, dir::AutoInterface::Copy)
            })?;
            if verdict == Verdict::Holds {
                return Ok(None);
            }
        }

        // instantiate the requirement's signature at the receiver
        let module = self.module_id;
        let requirement_type = self.symbol_type(requirement)?;
        let mut substitution = TypeSubstitution {
            bindings: arguments.iter().copied().collect(),
            receiver: Some(receiver),
        };
        let callable = self.substitute_type(requirement_type, &substitution)?;
        let Some((_, signature)) = self.callable_signature_type(origin, callable)? else {
            return Err(CompilerError::Internal {
                message: "a derived requirement without a signature".to_owned(),
            });
        };

        // read the shape the receiver derives over before declaring anything
        let Some((shape, components)) = self.derived_family(origin, receiver)? else {
            return Ok(None);
        };

        // derive a member for these interfaces alone, a class by its fields
        let is_class = matches!(shape, Composite::Class(_));
        let is_derivable = (!is_class || interface.derives_over_class())
            && matches!(
                interface,
                dir::AutoInterface::Clone
                    | dir::AutoInterface::Default
                    | dir::AutoInterface::Equal
                    | dir::AutoInterface::PartialEqual
                    | dir::AutoInterface::Hash
                    | dir::AutoInterface::Debug
                    | dir::AutoInterface::Display
            );
        if !is_derivable {
            return Ok(None);
        }

        // anchor the member and its body at the node demanding the derivation
        let Some(span) = self
            .module(source.module_id)
            .view()
            .get_span_by_id(source.local_id.id)
        else {
            return Err(CompilerError::Internal {
                message: format!(
                    "a derivation requested by a node without a source span: {source:?}"
                ),
            });
        };

        // declare the member node ahead of its body
        let key = match item.key {
            dir::StaticKey::Name(key) => self.strings().get(key).to_string(),
            dir::StaticKey::Index(index) => index.to_string(),
        };
        let name = self
            .strings()
            .intern(&format!("{}.{key}", item.owner.export_name()));
        let module_scope = self.module(module).bindings.module_scope();
        let is_static = signature.this_parameter.is_none();
        let member = self.module_mut(module).bindings_tail.insert_symbol(
            dir::SymbolRole::Local,
            dir::SymbolKind::Function,
            Some(dir::StaticKey::Name(name)),
            module_scope,
            None,
            dir::SymbolVisibility::Hidden,
        );
        let node = self.build_node(
            module,
            span,
            dir::TypeMember::Method {
                name: dir::Name::Identifier(name),
                signature: dir::FunctionSignature {
                    asynchrony: dir::Asynchrony::Sync,
                    role: None,
                    form: dir::FunctionForm::Function,
                    phase: dir::FunctionPhase::Normal,
                    generic_parameters: Vec::new(),
                    where_clauses: Vec::new(),
                    this_form: (!is_static).then_some(dir::ThisForm::Explicit),
                    this_parameter: None,
                    parameters: Vec::new(),
                    return_type: None,
                    is_abstract: false,
                    is_generator: false,
                    is_override: false,
                },
                body: None,
                visibility: None,
                is_static,
                is_optional: false,
            },
        );

        // record the transform that created this declaration
        let derivation = self.strings().intern("sema.derive");
        self.module_mut(module)
            .patch
            .tree
            .set_origin(node.id, dir::Origin::synthetic(derivation));

        // open the member's own scope over its declaration
        let scope = {
            let bindings = &mut self.module_mut(module).bindings_tail;
            let scope = bindings.insert_scope(dir::ScopeKind::Function, None, Some(member));
            bindings.introduce_scope(node.into_any(), scope);
            bindings.declare_symbol(member, node);
            dir::LocalScope::new(scope, dir::LocalScopeMark::end())
        };
        let member = member.into_global(module);
        let source = node.into_global_any(module);

        // nest the member template under a receiver template this module declares
        let parent = match self.nominal_application_maybe(receiver)? {
            Some((_, instance)) => self
                .template_by_symbol(instance.symbol)?
                .filter(|parent| parent.module_id == module),
            None => None,
        };
        let template = self.open_generic_template(source, parent)?;
        let Some((_, requirement_signature)) =
            self.callable_signature_type(origin, requirement_type)?
        else {
            return Err(CompilerError::Internal {
                message: "a derived requirement without a signature".to_owned(),
            });
        };
        if let Some(inherited) = requirement_signature.template {
            for parameter in self.generic_template_parameters(inherited)? {
                let binding = self.require_generic_parameter(parameter)?.clone();
                let constraint = binding
                    .constraint
                    .map(|constraint| self.substitute_type(constraint, &substitution))
                    .transpose()?;
                let default = binding
                    .default
                    .map(|default| self.substitute_type(default, &substitution))
                    .transpose()?;

                let kind = binding.kind;
                let declared = self.push_generic_parameter(
                    template,
                    source,
                    None,
                    binding.key,
                    binding.variance,
                    constraint,
                    default,
                    binding.origin,
                    kind,
                    binding.is_variadic,
                    binding.is_const,
                )?;
                let argument = self.intern_type(dir::Type::Parameter(declared))?;
                substitution.bindings.push(dir::GenericArgumentBinding {
                    parameter,
                    argument,
                });
            }
        }

        // read the signature the substituted requirement type declares
        let callable = self.substitute_type(requirement_type, &substitution)?;
        let (function, signature_type) = match self.ty(callable)? {
            dir::Type::FunctionSignature(_) => (None, callable),
            dir::Type::Function(function) => (Some(function), function.signature),
            _ => {
                return Err(CompilerError::Internal {
                    message: "a derived requirement without a callable type".to_owned(),
                });
            }
        };
        let dir::Type::FunctionSignature(signature) = self.ty(signature_type)? else {
            return Err(CompilerError::Internal {
                message: "a derived requirement without a signature".to_owned(),
            });
        };

        // re-intern that signature over the member's own template
        let signature = dir::FunctionSignatureType {
            template: Some(template),
            ..self.type_signature(signature_type.module_id, signature)?
        };
        let parameters =
            self.signature_parameters(signature_type.module_id, signature.parameters)?;
        let return_type = match signature.return_type {
            Some(ty) => ty,
            None => self.intern_type(dir::Type::Void)?,
        };
        let signature_type = self.intern_signature(signature)?;
        let callable = match function {
            Some(function) => self.intern_type(dir::Type::Function(dir::FunctionType {
                signature: signature_type,
                ..function
            }))?,
            None => signature_type,
        };
        self.commit_symbol_type(member, callable)?;
        self.module_mut(module)
            .types_tail
            .set_symbol_type(member, callable);

        // declare the parameters in the member's scope
        let mut declared = Vec::with_capacity(parameters.len());
        let mut parameter_nodes = Vec::with_capacity(parameters.len());
        for (index, parameter) in parameters.iter().enumerate() {
            let text = match parameter.name {
                Some(name) => name,
                None => self.strings().intern(&format!("argument{index}")),
            };
            let symbol = self.module_mut(module).bindings_tail.insert_symbol(
                dir::SymbolRole::Local,
                dir::SymbolKind::Parameter,
                Some(dir::StaticKey::Name(text)),
                scope,
                None,
                dir::SymbolVisibility::Hidden,
            );
            self.commit_symbol_type(symbol.into_global(module), parameter.ty)?;
            self.module_mut(module)
                .types_tail
                .set_symbol_type(symbol.into_global(module), parameter.ty);
            let parameter_node = self.build_node(
                module,
                span,
                dir::Parameter::Named {
                    name: text,
                    declared_type: None,
                    default: None,
                    is_optional: false,
                },
            );
            self.module_mut(module)
                .bindings_tail
                .declare_symbol(symbol, parameter_node);
            declared.push((symbol, text, parameter.ty));
            parameter_nodes.push(parameter_node);
        }

        // open the frame the body synthesizes its nodes in
        let mut frame = Derivation {
            symbol: member,
            scope,
            body: None,
            callable,
            module,
            span,
            receiver,
            this: signature.this_parameter,
            parameters: declared,
            member: item.key,
            calls: Vec::new(),
        };

        // clone a class field by field along its heritage, every other body reading the base whole
        let components = match (shape, interface) {
            (Composite::Class(_), dir::AutoInterface::Clone) => {
                self.class_field_chain(origin, components)?
            }
            _ => components,
        };

        // check the generated body under its declaration's template and inference scope
        let origin = self.anchored_origin(source)?;
        let body = self.with_body_scope(|state| {
            // select each component's protocol call under the member's own template
            let components =
                state.derived_components(&mut frame, origin, interface, item, components)?;

            // build the body the interface's combinator folds the components with
            let body = match (shape, interface) {
                (Composite::Union, _) => state.build_union_body(
                    &mut frame,
                    origin,
                    &components,
                    interface,
                    return_type,
                )?,
                (_, dir::AutoInterface::Clone | dir::AutoInterface::Default) => {
                    state.build_construct_body(&mut frame, shape, &components, interface)?
                }
                (_, dir::AutoInterface::Equal | dir::AutoInterface::PartialEqual) => {
                    state.build_equal_body(&mut frame, &components)?
                }
                (_, dir::AutoInterface::Hash) => state.build_hash_body(&mut frame, &components)?,
                (_, dir::AutoInterface::Debug | dir::AutoInterface::Display) => {
                    state.build_text_body(&mut frame, shape, &components, origin, return_type)?
                }
                _ => {
                    return Err(CompilerError::Internal {
                        message: "a derived body for an underivable requirement".to_owned(),
                    });
                }
            };

            Ok(body)
        })?;

        // attach the parameters and the body to the member node, making them visible
        let tree = &mut self.module_mut(module).patch.tree;
        let dir::TypeMember::Method {
            signature: written,
            body: attached,
            ..
        } = tree.get_mut(node)
        else {
            return Err(CompilerError::Internal {
                message: "a derived member declared outside a method".to_owned(),
            });
        };
        written.parameters = parameter_nodes;
        *attached = Some(body);
        tree.index_parents(&[node]);
        frame.body = Some(body.into_global_any(module));

        Ok(Some(frame))
    }

    /// Return the derivable interface one requirement belongs to, with the item naming its call.
    fn derived_requirement(
        &mut self,
        requirement: dir::GlobalSymbolId,
    ) -> CompilerResult<Option<(dir::AutoInterface, dir::LanguageMember)>> {
        // read the interface owning the requirement and the auto interface it stands for
        let Some(owner) = self.interface_member_owner(requirement)? else {
            return Ok(None);
        };
        let Some(item) = self.language_item(owner)? else {
            return Ok(None);
        };
        let Some(interface) = dir::AutoInterface::all()
            .filter(|interface| {
                interface.is_auto_derivable() || *interface == dir::AutoInterface::Default
            })
            .find(|interface| dir::LanguageItem::from(*interface) == item)
        else {
            return Ok(None);
        };

        // name the member the derivation implements and confirm the interface declares it
        let Some(name) = interface.derived_member() else {
            return Ok(None);
        };
        let member = item.member(name);
        let declared = self.definition(owner)?;
        let Some(dir::Definition::Interface(definition)) = declared.as_deref() else {
            return Ok(None);
        };
        let declares = definition.members.iter().any(|declared| match declared {
            dir::DefinitionMember::Method(method) => {
                method.symbol == requirement && method.slot == dir::MemberSlot::Key(member.key)
            }
            _ => false,
        });

        Ok(declares.then_some((interface, member)))
    }
}
