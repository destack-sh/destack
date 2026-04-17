use crate::analyze::common::TypeContext;
use crate::{AnalyzeError, AnalyzeOptions, CallableAbstraction, Compiler};
use destack_dir::{
    Ambientness, Asynchrony, Declaration, FunctionCardinality, FunctionMode, FunctionSignature,
    Key, LocalNodeId, LocalNodeIdAny, Member, Mutability, NodeTree, NodeType, Parameter, StringId,
};

impl Compiler {
    /// Validate a member.
    pub(super) fn validate_member(
        &self,
        ctx: &TypeContext<'_>,
        options: AnalyzeOptions,
        id: LocalNodeId<Member>,
        member: &Member,
    ) {
        // parent declaration context
        let parent_declaration = self
            .member_parent_declaration(ctx.tree, id)
            .map(|declaration_id| ctx.tree.get(declaration_id));
        let class_declaration = match parent_declaration {
            Some(Declaration::Class(declaration)) => Some(declaration),
            _ => None,
        };
        let interface_declaration = match parent_declaration {
            Some(Declaration::Interface(declaration)) => Some(declaration),
            _ => None,
        };

        // shared environment flags
        let is_class = class_declaration.is_some();
        let is_interface = interface_declaration.is_some();
        let is_declare_namespace = self.is_in_declare_namespace(ctx.tree, id.into_any());
        let is_javascript = ctx.module.language_type.is_javascript();

        match member {
            // associated comptime constants are owner scoped already
            Member::AssociatedConst { is_static, .. } => {
                if *is_static {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidMemberModifier { node });
                }
            }

            // methods
            Member::Method {
                key,
                signature,
                body,
                visibility,
                ambient,
                is_abstract,
                is_override,
                is_static,
                is_accessor,
                is_comptime,
                ..
            } => {
                // method flags
                let is_constructor = self.method_signature_is_constructor(signature, key.as_ref());
                let abstraction = self.callable_abstraction(*is_abstract, *is_override);
                let is_declare_class = class_declaration
                    .is_some_and(|declaration| declaration.ambient.is_ambient())
                    || is_declare_namespace;
                let is_declare_member = ambient.is_ambient();
                let is_private_key = matches!(key, Some(Key::Private(_)));
                let is_generator_signature_without_body =
                    signature.cardinality == FunctionCardinality::Generator && body.is_none();

                // javascript does not support typed or declaration only method syntax
                if is_javascript && (*visibility).is_some()
                    || (is_javascript && ambient.is_ambient())
                    || (is_javascript && *is_abstract)
                    || (is_javascript && *is_override)
                    || (is_javascript && *is_accessor)
                    || (is_javascript && *is_comptime)
                    || (is_javascript && !signature.generic_parameters.is_empty())
                    || (is_javascript && signature.return_type.is_some())
                    || (is_javascript && signature.this_parameter.is_some())
                {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
                }

                // constructor invariants
                if is_constructor && (*is_abstract || *is_override) {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }
                if is_constructor && !signature.generic_parameters.is_empty() {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }
                if is_constructor && signature.this_parameter.is_some() {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidConstructor { node });
                }

                // abstract methods
                if *is_abstract && body.is_some() {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidMethod { node, abstraction });
                }
                if *is_abstract && !self.is_in_abstract_class(ctx.tree, id) {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidMethod { node, abstraction });
                }

                // accessor signatures
                if matches!(
                    signature.mode,
                    Some(FunctionMode::Getter | FunctionMode::Setter)
                ) {
                    self.validate_accessor_signature(ctx, id.into_any(), signature);
                }

                // strict directive prologues require simple parameter lists in JS/TS modes
                if !ctx.module.language_type.is_destack()
                    && let Some(body) = body
                    && self.has_non_simple_dynamic_parameters(ctx.tree, &signature.parameters)
                    && self.body_declares_use_strict_directive(ctx.tree, *body)
                {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::InvalidFunction { node });
                }

                // class method constraints
                if is_class {
                    let key_name = self.member_key_name(key.as_ref());
                    let is_constructor_name = key_name
                        .is_some_and(|name| self.repository.strings.get(name) == "constructor");
                    let is_prototype_name = key_name
                        .is_some_and(|name| self.repository.strings.get(name) == "prototype");

                    if !*is_static && is_constructor_name && !is_constructor {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidConstructor { node });
                    }

                    if *is_static && is_prototype_name {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidConstructor { node });
                    }

                    if *is_static && *is_abstract {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if *is_abstract && signature.asynchrony == Asynchrony::Async {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if *is_abstract && is_private_key {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if (is_declare_class || is_declare_member)
                        && signature.asynchrony == Asynchrony::Async
                    {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_declare_member && is_private_key {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if (is_declare_class || is_declare_member) && body.is_some() {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_generator_signature_without_body {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if *is_accessor
                        && body.is_none()
                        && !*is_abstract
                        && !is_declare_class
                        && !is_declare_member
                    {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_declare_member && *is_accessor {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_declare_member && *is_override {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_javascript && body.is_none() && !is_declare_class && !is_declare_member {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidFunction { node });
                    }
                }

                // interface method constraints
                if is_interface {
                    if body.is_some() {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_generator_signature_without_body {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    self.validate_interface_method(
                        ctx,
                        id,
                        signature,
                        *visibility,
                        *ambient,
                        *is_abstract,
                        *is_override,
                        *is_static,
                        *is_accessor,
                        *is_comptime,
                    );
                }
            }

            // fields
            Member::Field {
                key,
                declared_type,
                default,
                is_optional,
                is_readonly,
                mutability,
                visibility,
                ambient,
                is_abstract,
                is_override,
                is_static,
                is_const_asserted,
                is_accessor,
                is_comptime,
                ..
            } => {
                // field flags
                let has_default = default.is_some();
                let is_private_key = matches!(key, Key::Private(_));
                let is_index_signature = Self::is_index_signature(key);
                let is_declare_class = class_declaration
                    .is_some_and(|declaration| declaration.ambient.is_ambient())
                    || is_declare_namespace;
                let is_declare_member = ambient.is_ambient();
                let is_abstract_class =
                    class_declaration.is_some_and(|declaration| declaration.is_abstract);
                let has_abstraction = *is_abstract || *is_override;
                let is_effectively_readonly =
                    *is_readonly || *mutability == Some(Mutability::Immutable);

                // javascript does not support typed field syntax beyond runtime fields
                if is_javascript
                    && ((*visibility).is_some()
                        || ambient.is_ambient()
                        || *is_abstract
                        || *is_override
                        || declared_type.is_some()
                        || *is_optional
                        || *is_readonly
                        || mutability.is_some()
                        || *is_const_asserted
                        || *is_accessor
                        || *is_comptime
                        || is_index_signature)
                {
                    let node = id
                        .into_global_any(ctx.module.id)
                        .into_anchored(Some(ctx.profile));
                    self.error(AnalyzeError::TypeScriptSyntaxInJavaScript { node });
                }

                // field specific policy
                let _ = options;

                if is_class {
                    if *is_abstract && !is_abstract_class {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if *is_abstract && has_default {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_private_key && has_abstraction {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if *is_accessor && (*is_optional || is_effectively_readonly || has_abstraction)
                    {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_declare_member && is_private_key {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if (is_declare_class || is_declare_member) && has_default {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_declare_member && *is_abstract {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if is_index_signature
                        && ((*visibility).is_some()
                            || *is_abstract
                            || is_declare_member
                            || *is_accessor
                            || *is_static)
                    {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }
                }

                if is_interface {
                    if (*visibility).is_some()
                        || *is_static
                        || *is_abstract
                        || *is_override
                        || *is_accessor
                        || ambient.is_ambient()
                        || *is_comptime
                    {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }

                    if has_default {
                        let node = id
                            .into_global_any(ctx.module.id)
                            .into_anchored(Some(ctx.profile));
                        self.error(AnalyzeError::InvalidMemberModifier { node });
                    }
                }
            }

            // static blocks are already canonical
            Member::StaticBlock { .. }
            | Member::AssociatedType { .. }
            | Member::Embed { .. }
            | Member::ComptimeBlock { .. }
            | Member::Error { .. } => {}
        }
    }

    /// Return the callable abstraction for one method.
    fn callable_abstraction(&self, is_abstract: bool, is_override: bool) -> CallableAbstraction {
        match (is_abstract, is_override) {
            (true, true) => CallableAbstraction::AbstractOverride,
            (true, false) => CallableAbstraction::Abstract,
            (false, true) => CallableAbstraction::Override,
            (false, false) => CallableAbstraction::Concrete,
        }
    }

    /// Check whether a method signature is a constructor.
    fn method_signature_is_constructor(
        &self,
        signature: &FunctionSignature,
        key: Option<&Key>,
    ) -> bool {
        if signature.mode == Some(FunctionMode::Constructor) {
            return true;
        }

        if !matches!(signature.mode, None | Some(FunctionMode::Call)) {
            return false;
        }

        if signature.asynchrony == Asynchrony::Async
            || signature.cardinality == FunctionCardinality::Generator
        {
            return false;
        }

        let key_name = self.member_key_name(key);
        key_name.is_some_and(|name| self.repository.strings.get(name) == "constructor")
    }

    /// Check whether a member belongs to an abstract class.
    fn is_in_abstract_class(&self, tree: &NodeTree, member_id: LocalNodeId<Member>) -> bool {
        let Some(parent_declaration_id) = self.member_parent_declaration(tree, member_id) else {
            return false;
        };

        matches!(
            tree.get(parent_declaration_id),
            Declaration::Class(declaration) if declaration.is_abstract
        )
    }

    /// Resolve the declaration that owns a member.
    fn member_parent_declaration(
        &self,
        tree: &NodeTree,
        member_id: LocalNodeId<Member>,
    ) -> Option<LocalNodeId<Declaration>> {
        let parent = tree.get_parent(member_id.id)?;
        if parent.ty != NodeType::Declaration {
            return None;
        }

        Some(LocalNodeId::<Declaration>::new(parent.id))
    }

    /// Check whether a key is an index signature marker.
    fn is_index_signature(key: &Key) -> bool {
        matches!(key, Key::Expression(_))
    }

    /// Return the static key name when a member key is non computed.
    fn member_key_name(&self, key: Option<&Key>) -> Option<StringId> {
        match key {
            Some(Key::Name(name)) => Some(name.string()),
            _ => None,
        }
    }

    /// Validate interface specific method rules.
    fn validate_interface_method(
        &self,
        ctx: &TypeContext<'_>,
        id: LocalNodeId<Member>,
        signature: &FunctionSignature,
        visibility: Option<destack_dir::Visibility>,
        ambient: Ambientness,
        is_abstract: bool,
        is_override: bool,
        is_static: bool,
        is_accessor: bool,
        is_comptime: bool,
    ) {
        if visibility.is_some()
            || ambient.is_ambient()
            || is_abstract
            || is_override
            || is_static
            || is_accessor
            || is_comptime
        {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }

        self.validate_accessor_signature(ctx, id.into_any(), signature);

        if signature.asynchrony == Asynchrony::Async {
            let node = id
                .into_global_any(ctx.module.id)
                .into_anchored(Some(ctx.profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }
    }

    /// Validate getter and setter signatures.
    pub(super) fn validate_accessor_signature(
        &self,
        ctx: &TypeContext<'_>,
        node_id: LocalNodeIdAny,
        signature: &FunctionSignature,
    ) {
        // accessor mode
        let is_getter = signature.mode == Some(FunctionMode::Getter);
        let is_setter = signature.mode == Some(FunctionMode::Setter);
        if !is_getter && !is_setter {
            return;
        }

        // shared accessor restrictions
        if !signature.generic_parameters.is_empty() || signature.this_parameter.is_some() {
            let node = node_id.into_anchored(ctx.module.id, Some(ctx.profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }

        // getters do not take parameters
        if is_getter && !signature.parameters.is_empty() {
            let node = node_id.into_anchored(ctx.module.id, Some(ctx.profile));
            self.error(AnalyzeError::InvalidMemberModifier { node });
        }

        // setters take exactly one required named parameter
        if is_setter {
            let mut invalid_setter = signature.parameters.len() != 1;

            if let Some(parameter_id) = signature.parameters.first() {
                let parameter = ctx.tree.get(*parameter_id);
                invalid_setter = invalid_setter
                    || matches!(
                        parameter,
                        Parameter::Pattern { .. }
                            | Parameter::VariadicNamed { .. }
                            | Parameter::VariadicPattern { .. }
                    )
                    || self.accessor_parameter_has_default(parameter)
                    || self.accessor_parameter_is_optional(parameter);
            }

            if invalid_setter {
                let node = node_id.into_anchored(ctx.module.id, Some(ctx.profile));
                self.error(AnalyzeError::InvalidMemberModifier { node });
            }

            if signature.return_type.is_some() {
                let node = node_id.into_anchored(ctx.module.id, Some(ctx.profile));
                self.error(AnalyzeError::InvalidMemberModifier { node });
            }
        }
    }

    /// Return true when a parameter has a default.
    fn accessor_parameter_has_default(&self, parameter: &Parameter) -> bool {
        match parameter {
            Parameter::Named { default, .. } | Parameter::Pattern { default, .. } => {
                default.is_some()
            }
            Parameter::VariadicNamed { .. }
            | Parameter::VariadicPattern { .. }
            | Parameter::Error { .. } => false,
        }
    }

    /// Return true when a parameter is optional.
    fn accessor_parameter_is_optional(&self, parameter: &Parameter) -> bool {
        match parameter {
            Parameter::Named { is_optional, .. } | Parameter::Pattern { is_optional, .. } => {
                *is_optional
            }
            Parameter::VariadicNamed { .. }
            | Parameter::VariadicPattern { .. }
            | Parameter::Error { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::TestProgram;

    /// Reject constructor named getters in classes.
    #[test]
    fn test_reject_constructor_named_getter() {
        let test = TestProgram::memory_sequential();

        // source: class A { get constructor() {} }
        let module_id = test.add_module("test.js", "class A { get constructor() {} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA501");
    }

    /// Reject constructor named setters in classes.
    #[test]
    fn test_reject_constructor_named_setter() {
        let test = TestProgram::memory_sequential();

        // source: class A { set constructor(v) {} }
        let module_id = test.add_module("test.js", "class A { set constructor(v) {} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA501");
    }

    /// Reject constructor named generators in classes.
    #[test]
    fn test_reject_constructor_named_generator() {
        let test = TestProgram::memory_sequential();

        // source: class A { *constructor() {} }
        let module_id = test.add_module("test.js", "class A { *constructor() {} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA501");
    }

    /// Reject static members named prototype.
    #[test]
    fn test_reject_static_prototype_member() {
        let test = TestProgram::memory_sequential();

        // source: class A { static "prototype"() {} }
        let module_id = test.add_module("test.js", r#"class A { static "prototype"() {} }"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA501");
    }

    /// Allow static methods named constructor.
    #[test]
    fn test_allow_static_constructor_named_method() {
        let test = TestProgram::memory_sequential();

        // source: class A { static constructor() {} }
        let module_id = test.add_module("test.js", "class A { static constructor() {} }");
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA501");
    }

    /// Reject duplicate constructor implementations with constructor named string keys.
    #[test]
    fn test_reject_duplicate_constructor_named_string_method() {
        let test = TestProgram::memory_sequential();

        // source: class A { constructor() {} 'constructor'() {} }
        let module_id = test.add_module(
            "test.js",
            r#"class A { constructor() {} 'constructor'() {} }"#,
        );
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA501");
    }

    /// Reject JavaScript class method signatures without bodies.
    #[test]
    fn test_reject_javascript_class_method_without_body() {
        let test = TestProgram::memory_sequential();

        // source: class A { constructor() {} 'constructor'() }
        let module_id =
            test.add_module("test.js", r#"class A { constructor() {} 'constructor'() }"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA503");
    }

    /// Reject non-simple class method parameters with strict directive prologues in JS/TS.
    #[test]
    fn test_reject_class_method_non_simple_parameters_with_use_strict() {
        let test = TestProgram::memory_sequential();

        // source: class A { m([]){ "use strict"; } }
        let module_id = test.add_module("test.js", r#"class A { m([]){ "use strict"; } }"#);
        test.apply_destack_config(
            module_id,
            r#"{"compilerOptions":{"checkTs":true,"checkJs":true}}"#,
        );
        test.analyze_module(module_id);
        test.compile();
        test.check_has_diagnostic("EA503");
    }

    /// Allow non-simple class method parameters with strict directive prologues in Destack.
    #[test]
    fn test_allow_class_method_non_simple_parameters_with_use_strict_in_destack() {
        let test = TestProgram::memory_sequential();

        // source: class A { m([]){ "use strict"; } }
        let module_id = test.add_module("test.ds", r#"class A { m([]){ "use strict"; } }"#);
        test.analyze_module(module_id);
        test.compile();
        test.check_no_diagnostic_code("EA503");
    }
}
