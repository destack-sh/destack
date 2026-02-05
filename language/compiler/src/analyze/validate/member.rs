use crate::{AnalyzeError, AnalyzeOptions, Compiler};
use destack_dir::{
    AbstractionModifier, Asynchrony, BindingAnchor, BindingKind, Declaration,
    DeclarationAbstraction, DeclarationKind, DynamicKey, FunctionAbstraction, FunctionCardinality,
    FunctionMode, FunctionSignature, LocalNodeId, LocalNodeIdAny, Member, Mutability, NodeTree,
    NodeType, Parameter,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Validate a member.
    pub(super) fn validate_member(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        options: AnalyzeOptions,
        id: LocalNodeId<Member>,
        member: &Member,
    ) {
        // resolve the parent declaration descriptor
        let parent_declaration_id = self.member_parent_declaration(tree, id);
        let mut class_descriptor = None;
        let mut interface_descriptor = None;
        if let Some(parent_declaration_id) = parent_declaration_id {
            let declaration = tree.get(parent_declaration_id);
            match declaration {
                Declaration::Class { descriptor, .. } => {
                    class_descriptor = Some(descriptor);
                }
                Declaration::Interface { descriptor, .. } => {
                    interface_descriptor = Some(descriptor);
                }
                _ => {}
            }
        }

        // detect the enclosing declaration kind
        let is_class = class_descriptor.is_some();
        let is_interface = interface_descriptor.is_some();
        let is_declare_namespace = self.is_in_declare_namespace(tree, id.into_any());
        let is_javascript = module.language_type.is_javascript();

        // validate by member kind
        match member {
            // method validation
            Member::Method {
                modifiers,
                key,
                signature,
                body,
                ..
            } => {
                // normalize modifiers and flags
                let modifiers = modifiers.as_ref();
                let is_constructor = signature.mode == Some(FunctionMode::Constructor);
                let has_static_parameters = signature
                    .generics
                    .as_ref()
                    .is_some_and(|generics| generics.static_parameters.is_some());
                let abstraction = signature.abstraction;
                let is_abstract = matches!(
                    abstraction,
                    FunctionAbstraction::Abstract | FunctionAbstraction::AbstractOverride
                );
                let is_override = matches!(
                    abstraction,
                    FunctionAbstraction::AbstractOverride | FunctionAbstraction::ConcreteOverride
                );
                let is_accessor = matches!(
                    signature.mode,
                    Some(FunctionMode::Getter | FunctionMode::Setter)
                );
                let is_private_key = matches!(key, Some(DynamicKey::Private(_)));
                let is_generator_signature_without_body =
                    signature.cardinality == FunctionCardinality::Generator && body.is_none();

                // reject typescript-only method syntax in javascript modules
                if is_javascript {
                    let has_typescript_modifiers = modifiers.is_some_and(|modifiers| {
                        modifiers.visibility.is_some()
                            || modifiers.declaration.is_some()
                            || modifiers.abstraction.is_some()
                            || modifiers.mutability.is_some()
                            || modifiers.kind.is_some()
                            || modifiers.operator.is_some()
                            || modifiers.variance.is_some()
                            || modifiers.timing.is_some()
                    });
                    let has_typescript_signature = signature.generics.is_some()
                        || signature.return_type.is_some()
                        || signature.this_parameter.is_some();
                    if has_typescript_modifiers || has_typescript_signature {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
                    }
                }

                // variance modifiers are not valid on members
                if modifiers.is_some_and(|modifiers| modifiers.variance.is_some()) {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidMemberModifier { node });
                }

                // abstract constructors are invalid
                if is_constructor && is_abstract {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }

                // constructor cannot have static parameters
                if is_constructor && has_static_parameters {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }

                // constructor cannot declare a this parameter
                if is_constructor && signature.this_parameter.is_some() {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }

                // abstract methods cannot have bodies
                if is_abstract && body.is_some() {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidMethod { node, abstraction });
                }

                // abstract methods require abstract classes
                if is_abstract && !self.is_in_abstract_class(tree, id) {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidMethod { node, abstraction });
                }

                // readonly does not apply to methods
                if modifiers
                    .is_some_and(|modifiers| modifiers.mutability == Some(Mutability::Immutable))
                {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidMemberModifier { node });
                }

                // override is invalid on constructors
                if is_constructor && is_override {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }

                // validate accessor signature rules
                if is_accessor {
                    self.validate_accessor_signature(
                        module,
                        profile,
                        tree,
                        id.into_any(),
                        signature,
                    );
                }

                // class method constraints
                if is_class {
                    // collect class modifier flags
                    let is_static = modifiers
                        .is_some_and(|modifiers| modifiers.anchor == Some(BindingAnchor::Static));
                    let is_declare_class = class_descriptor
                        .is_some_and(|descriptor| descriptor.kind == DeclarationKind::Declaration)
                        || is_declare_namespace;
                    let is_declare_member =
                        modifiers.is_some_and(|modifiers| modifiers.declaration.is_some());

                    // static abstract methods are invalid
                    if is_static && is_abstract {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // abstract async methods are invalid
                    if is_abstract && signature.asynchrony == Asynchrony::Async {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // abstract private methods are invalid
                    if is_abstract && is_private_key {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // ambient method signatures cannot use async
                    if (is_declare_class || is_declare_member)
                        && signature.asynchrony == Asynchrony::Async
                    {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare members cannot use private keys
                    if is_declare_member && is_private_key {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare members cannot have bodies
                    if (is_declare_class || is_declare_member) && body.is_some() {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // method signatures cannot be generators
                    if is_generator_signature_without_body {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // accessors must have bodies unless abstract or declare
                    if is_accessor
                        && body.is_none()
                        && !is_abstract
                        && !is_declare_class
                        && !is_declare_member
                    {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare accessors are invalid
                    if is_declare_member && is_accessor {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare cannot combine with override
                    if is_declare_member && is_override {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }
                }

                // interface method constraints
                if is_interface {
                    // interface methods cannot have bodies
                    if body.is_some() {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // interface method signatures cannot be generators
                    if is_generator_signature_without_body {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // validate interface specific rules
                    self.validate_interface_method(module, profile, tree, id, modifiers, signature);
                }
            }

            // field validation
            Member::Field {
                modifiers,
                key,
                value,
                default,
                ..
            } => {
                // normalize modifiers and flags
                let modifiers = modifiers.as_ref();
                let has_default = default.is_some();
                let has_definite_assignment =
                    modifiers.is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Must));
                let has_abstraction =
                    modifiers.is_some_and(|modifiers| modifiers.abstraction.is_some());
                let is_private_key = matches!(key, Some(DynamicKey::Private(_)));
                let is_index_signature = Self::is_index_signature(key.as_ref());
                let is_abstract_field = modifiers.is_some_and(|modifiers| {
                    matches!(
                        modifiers.abstraction,
                        Some(AbstractionModifier::Abstract | AbstractionModifier::AbstractOverride)
                    )
                });

                // reject typescript-only field syntax in javascript modules
                if is_javascript {
                    let has_typescript_modifiers = modifiers.is_some_and(|modifiers| {
                        modifiers.visibility.is_some()
                            || modifiers.declaration.is_some()
                            || modifiers.abstraction.is_some()
                            || modifiers.mutability.is_some()
                            || modifiers.kind.is_some()
                            || modifiers.operator.is_some()
                            || modifiers.variance.is_some()
                            || modifiers.timing.is_some()
                    });
                    let has_typescript_syntax =
                        has_typescript_modifiers || value.is_some() || is_index_signature;
                    if has_typescript_syntax {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
                    }
                }

                // variance modifiers are not valid on members
                if modifiers.is_some_and(|modifiers| modifiers.variance.is_some()) {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::InvalidMemberModifier { node });
                }

                // enforce definite assignment assertion policy
                if has_definite_assignment && options.no_definite_assignment_assertions {
                    let node = id.into_global_any(module.id).into_anchored(Some(profile));
                    self.error(AnalyzeError::DefiniteAssignmentAssertionDisabled { node });
                }

                // class field constraints
                if is_class {
                    // collect class field flags
                    let is_declare_class = class_descriptor
                        .is_some_and(|descriptor| descriptor.kind == DeclarationKind::Declaration)
                        || is_declare_namespace;
                    let is_declare_member =
                        modifiers.is_some_and(|modifiers| modifiers.declaration.is_some());
                    let is_abstract_class = class_descriptor.is_some_and(|descriptor| {
                        descriptor.abstraction == DeclarationAbstraction::Abstract
                    });
                    let has_accessor =
                        modifiers.is_some_and(|modifiers| modifiers.accessor.is_some());
                    let has_kind = modifiers.is_some_and(|modifiers| modifiers.kind.is_some());
                    let is_readonly = modifiers.is_some_and(|modifiers| {
                        modifiers.mutability == Some(Mutability::Immutable)
                    });

                    // abstract fields cannot use definite assignment assertions
                    if is_abstract_field && has_definite_assignment {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // abstract fields require abstract classes
                    if is_abstract_field && !is_abstract_class {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // abstract fields cannot have initializers
                    if is_abstract_field && has_default {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // private fields cannot be abstract
                    if is_private_key && has_abstraction {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // auto accessor constraints
                    if has_accessor && (has_kind || is_readonly || has_abstraction) {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare members cannot use private keys
                    if is_declare_member && is_private_key {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare fields cannot have initializers
                    if (is_declare_class || is_declare_member) && has_default {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare fields cannot use definite assignment assertions
                    if (is_declare_class || is_declare_member) && has_definite_assignment {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // declare fields cannot be abstract
                    if is_declare_member && has_abstraction {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // index signatures cannot use modifiers
                    if is_index_signature
                        && modifiers.is_some_and(|modifiers| {
                            modifiers.visibility.is_some()
                                || modifiers.abstraction.is_some()
                                || modifiers.declaration.is_some()
                                || modifiers.accessor.is_some()
                                || modifiers.anchor == Some(BindingAnchor::Static)
                        })
                    {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }
                }

                // interface field constraints
                if is_interface {
                    // detect invalid interface field modifiers
                    let invalid_modifiers = modifiers.is_some_and(|modifiers| {
                        modifiers.visibility.is_some()
                            || modifiers.anchor.is_some()
                            || modifiers.abstraction.is_some()
                            || modifiers.operator.is_some()
                            || modifiers.accessor.is_some()
                            || modifiers.timing.is_some()
                            || modifiers.declaration.is_some()
                    });

                    // interface fields cannot use modifiers
                    if invalid_modifiers {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    // interface fields cannot have initializers
                    if has_default {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }
                }
            }

            // static block validation
            Member::StaticBlock { modifiers, .. } => {
                // static blocks do not allow modifiers
                if let Some(modifiers) = modifiers {
                    // detect invalid modifiers
                    let has_invalid_modifier = modifiers.visibility.is_some()
                        || modifiers.abstraction.is_some()
                        || modifiers.mutability.is_some()
                        || modifiers.kind.is_some()
                        || modifiers.operator.is_some()
                        || modifiers.accessor.is_some()
                        || modifiers.declaration.is_some();

                    // report invalid modifiers on static blocks
                    if has_invalid_modifier {
                        let node = id.into_global_any(module.id).into_anchored(Some(profile));
                        self.error(AnalyzeError::InvalidStaticBlockModifier { node });
                    }
                }
            }

            _ => {}
        }
    }

    /// Check whether a member belongs to an abstract class.
    fn is_in_abstract_class(&self, tree: &NodeTree, member_id: LocalNodeId<Member>) -> bool {
        // locate the parent declaration
        let Some(parent_declaration_id) = self.member_parent_declaration(tree, member_id) else {
            return false;
        };

        // confirm the declaration is an abstract class
        let declaration = tree.get(parent_declaration_id);
        matches!(
            declaration,
            Declaration::Class {
                descriptor,
                ..
            } if descriptor.abstraction == DeclarationAbstraction::Abstract
        )
    }

    /// Resolve the declaration that owns a member.
    fn member_parent_declaration(
        &self,
        tree: &NodeTree,
        member_id: LocalNodeId<Member>,
    ) -> Option<LocalNodeId<Declaration>> {
        // read the parent node
        let parent = tree.get_parent(member_id.id)?;

        // ensure the parent is a declaration
        if parent.ty != NodeType::Declaration {
            return None;
        }

        // return the declaration id
        Some(LocalNodeId::<Declaration>::new(parent.id))
    }

    /// Check whether a key is an index signature marker.
    fn is_index_signature(key: Option<&DynamicKey>) -> bool {
        matches!(key, Some(DynamicKey::NamedExpression { .. }))
    }

    /// Validate interface specific method rules.
    fn validate_interface_method(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        id: LocalNodeId<Member>,
        modifiers: Option<&destack_dir::BindingModifier>,
        signature: &FunctionSignature,
    ) {
        // reject invalid interface modifiers
        let invalid_modifiers = modifiers.is_some_and(|modifiers| {
            modifiers.visibility.is_some()
                || modifiers.anchor.is_some()
                || modifiers.abstraction.is_some()
                || modifiers.mutability.is_some()
                || modifiers.operator.is_some()
                || modifiers.accessor.is_some()
                || modifiers.timing.is_some()
                || modifiers.declaration.is_some()
        });
        if invalid_modifiers {
            let node = id.into_global_any(module.id).into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }

        // validate accessor constraints
        self.validate_accessor_signature(module, profile, tree, id.into_any(), signature);

        // reject async signatures in interfaces
        if signature.asynchrony == Asynchrony::Async {
            let node = id.into_global_any(module.id).into_anchored(Some(profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }
    }

    /// Validate getter and setter signatures.
    pub(super) fn validate_accessor_signature(
        &self,
        module: &Module,
        profile: ProfileId,
        tree: &NodeTree,
        node_id: LocalNodeIdAny,
        signature: &FunctionSignature,
    ) {
        // detect accessor signatures
        let is_getter = signature.mode == Some(FunctionMode::Getter);
        let is_setter = signature.mode == Some(FunctionMode::Setter);
        if !is_getter && !is_setter {
            return;
        }

        // accessors cannot be generic or declare a this parameter
        if signature.generics.is_some() || signature.this_parameter.is_some() {
            let node = node_id.into_anchored(module.id, Some(profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }

        // getters cannot take parameters
        if is_getter && !signature.dynamic_parameters.is_empty() {
            let node = node_id.into_anchored(module.id, Some(profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }

        // setters require exactly one non optional parameter
        if is_setter {
            let mut invalid_setter = signature.dynamic_parameters.len() != 1;
            if let Some(param_id) = signature.dynamic_parameters.first() {
                let param = tree.get(*param_id);
                invalid_setter = invalid_setter
                    || matches!(
                        param,
                        Parameter::Pattern { .. } | Parameter::Variadic { .. }
                    )
                    || param.has_default()
                    || param
                        .modifiers()
                        .is_some_and(|modifiers| modifiers.kind == Some(BindingKind::Maybe));
            }
            if invalid_setter {
                let node = node_id.into_anchored(module.id, Some(profile));
                self.error(AnalyzeError::InvalidMemberModifier { node });
            }

            // setters cannot declare return types
            if signature.return_type.is_some() {
                let node = node_id.into_anchored(module.id, Some(profile));
                self.error(AnalyzeError::InvalidMemberModifier { node });
            }
        }
    }
}
