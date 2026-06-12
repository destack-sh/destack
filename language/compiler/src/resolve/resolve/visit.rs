use destack_artifact::GlobalEnvironment;
use destack_dir as dir;
use dir::NodeVisitor as _;

use crate::CompilerResult;
use crate::resolve::state::ResolveState;

impl ResolveState<'_> {
    /// Resolve every collected reference against required inputs.
    pub(in crate::resolve) fn resolve(
        &mut self,
        environment: &GlobalEnvironment,
    ) -> CompilerResult<()> {
        // explicit module clauses resolve through exports
        self.resolve_module_clauses()?;

        // globals resolve through the profile's precomputed table
        self.resolve_profile_globals(environment)?;

        // source-visible language globals and namespace paths follow
        self.resolve_language_globals(&environment.language)?;
        self.resolve_path_references()?;

        // syntax-required language item modules resolve last
        self.resolve_syntax_language_items(&environment.language)
    }

    /// Walk active roots and collect references and syntax language items.
    ///
    /// Example:
    /// ```ds
    /// import { value } from "./dep.ds";
    ///
    /// value;
    /// ```
    pub(in crate::resolve) fn walk(&mut self, roots: &[dir::LocalNodeId<dir::Expression>]) {
        let tree = self.view.tree();
        self.stats.roots += roots.len();

        for root in roots {
            let expression = tree.get(*root);
            self.visit_expression(tree, *root, expression);
        }

        let decorators = self
            .view
            .get_all_decorators()
            .into_values()
            .flatten()
            .collect::<Vec<_>>();

        // collect references from attached decorator expressions
        for decorator in decorators {
            let node = tree.get(decorator);
            self.visit_decorator(tree, decorator, node);
        }
    }
}

impl dir::NodeVisitor for ResolveState<'_> {
    /// Return resolve visitor options.
    fn options(&self) -> &dir::NodeVisitorOptions {
        &self.options
    }

    /// Visit one expression.
    ///
    /// Example:
    /// ```ds
    /// dep.value;
    /// ```
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.stats.expressions += 1;
        self.walk_expression(tree, id, expression);
    }

    /// Visit one type expression.
    ///
    /// Example:
    /// ```ds
    /// let value: dep.Model;
    /// ```
    fn visit_type_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeExpression>,
        type_expression: &dir::TypeExpression,
    ) {
        self.stats.type_expressions += 1;
        self.walk_type_expression(tree, id, type_expression);
    }

    /// Visit one declaration.
    ///
    /// Example:
    /// ```ds
    /// async function load() {}
    /// ```
    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        if let dir::Declaration::Function(function) = declaration {
            self.require_function_language_items(&function.signature);
            self.enter_function(&function.signature);
            dir::walk_declaration(self, tree, id, declaration);
            self.leave_function();

            return;
        }

        dir::walk_declaration(self, tree, id, declaration);
    }

    /// Visit one object property.
    ///
    /// Example:
    /// ```ds
    /// const service = {
    ///     async load() {}
    /// };
    /// ```
    fn visit_property(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Property>,
        property: &dir::Property,
    ) {
        if let dir::Property::Method { signature, .. } = property {
            self.require_function_language_items(signature);
            self.enter_function(signature);
            dir::walk_property(self, tree, id, property);
            self.leave_function();

            return;
        }

        dir::walk_property(self, tree, id, property);
    }

    /// Visit one structural type member.
    ///
    /// Example:
    /// ```ds
    /// type Service = {
    ///     load(): Promise<void>;
    /// };
    /// ```
    fn visit_type_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::TypeMember>,
        member: &dir::TypeMember,
    ) {
        if let dir::TypeMember::Method { signature, .. } = member {
            self.require_function_language_items(signature);
        }

        dir::walk_type_member(self, tree, id, member);
    }

    /// Visit one class or interface member.
    ///
    /// Example:
    /// ```ds
    /// class Service {
    ///     async load() {}
    /// }
    /// ```
    fn visit_member(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Member>,
        member: &dir::Member,
    ) {
        if let dir::Member::Method { signature, .. } = member {
            self.require_function_language_items(signature);
            self.enter_function(signature);
            dir::walk_member(self, tree, id, member);
            self.leave_function();

            return;
        }

        dir::walk_member(self, tree, id, member);
    }

    /// Visit one decorator.
    ///
    /// Example:
    /// ```ds
    /// @trace
    /// function f() {}
    /// ```
    fn visit_decorator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        if self.is_static_if_decorator_expression(tree, decorator.expression) {
            self.walk_static_if_decorator_arguments(tree, decorator.expression);

            return;
        }

        dir::walk_decorator(self, tree, id, decorator);
    }
}

impl ResolveState<'_> {
    /// Return whether one decorator expression is syntactic `@if`.
    ///
    /// Example:
    /// ```ds
    /// @if(Target.isNative)
    /// function f() {}
    /// ```
    fn is_static_if_decorator_expression(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) -> bool {
        match tree.get(expression) {
            dir::Expression::Parenthesized { expression } => {
                self.is_static_if_decorator_expression(tree, *expression)
            }
            dir::Expression::Call { left, .. } => {
                self.is_static_if_decorator_expression(tree, *left)
            }
            dir::Expression::Identifier { name } => self.strings.get(*name) == "if",
            dir::Expression::QualifiedReference { path, .. } if path.segments.len() == 1 => {
                self.strings.get(path.segments[0]) == "if"
            }
            _ => false,
        }
    }

    /// Walk condition arguments of syntactic `@if`.
    ///
    /// Example:
    /// ```ds
    /// @if(Target.isNative)
    /// function f() {}
    /// ```
    fn walk_static_if_decorator_arguments(
        &mut self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
    ) {
        match tree.get(expression) {
            dir::Expression::Parenthesized { expression } => {
                self.walk_static_if_decorator_arguments(tree, *expression);
            }
            dir::Expression::Call { arguments, .. } => {
                for argument in arguments {
                    let Some(value) = tree.get(*argument).value() else {
                        continue;
                    };
                    let expression = tree.get(value);

                    self.visit_expression(tree, value, expression);
                }
            }
            _ => {}
        }
    }
}
