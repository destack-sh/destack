use dir::NodeVisitor as _;
use tspp_artifact::{DirExported, EnvironmentBound};
use tspp_dir as dir;

use crate::CompilerResult;
use crate::resolve::state::ResolveState;
use crate::r#static::StaticGuard;

impl ResolveState<'_> {
    /// Resolve every collected reference against required inputs.
    pub(in crate::resolve) fn resolve(
        &mut self,
        environment: &EnvironmentBound,
        exported: &DirExported,
    ) -> CompilerResult<()> {
        // imports resolve before exported local imports and source paths
        self.resolve_imports()?;

        // resolve references owned by exported names
        self.resolve_export_references(exported)?;

        // globals resolve through the profile's precomputed table
        self.resolve_profile_globals(environment)?;

        // resolve source paths after globals
        self.resolve_path_references()?;

        // resolve language item symbols after profile globals
        self.resolve_language_items(&environment.language)
    }

    /// Walk active roots and collect references and language item uses.
    ///
    /// Example:
    /// ```tspp
    /// import { value } from "./dep.tspp";
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
    /// Visit one expression.
    ///
    /// Example:
    /// ```tspp
    /// dep.value;
    /// ```
    fn visit_expression(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Expression>,
        expression: &dir::Expression,
    ) {
        self.stats.expressions += 1;

        // retain imports
        if matches!(expression, dir::Expression::Import { .. }) {
            self.stats.import_clauses += 1;
            self.import_expressions.push(id);
        }

        // count direct re-exports
        if matches!(
            expression,
            dir::Expression::Export {
                target: Some(_),
                ..
            }
        ) {
            self.stats.reexport_clauses += 1;
        }

        self.walk_expression(tree, id, expression);
    }

    /// Visit one type expression.
    ///
    /// Example:
    /// ```tspp
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
    /// ```tspp
    /// async function load() {}
    /// ```
    fn visit_declaration(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Declaration>,
        declaration: &dir::Declaration,
    ) {
        if let dir::Declaration::Function(function) = declaration {
            self.use_function_language_items(&function.signature);
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
    /// ```tspp
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
            self.use_function_language_items(signature);
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
    /// ```tspp
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
            self.use_function_language_items(signature);
        }

        dir::walk_type_member(self, tree, id, member);
    }

    /// Visit one class or interface member.
    ///
    /// Example:
    /// ```tspp
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
            self.use_function_language_items(signature);
            self.enter_function(signature);
            dir::walk_member(self, tree, id, member);
            self.leave_function();

            return;
        }

        dir::walk_member(self, tree, id, member);
    }

    /// Visit one pattern.
    ///
    /// Example:
    /// ```tspp
    /// const { name } = user;
    /// const [first] = users;
    /// ```
    fn visit_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Pattern>,
        pattern: &dir::Pattern,
    ) {
        match pattern {
            dir::Pattern::Range {
                start,
                end,
                end_kind,
            } => {
                self.use_range_language_item(start.is_some(), end.is_some(), *end_kind);
            }
            dir::Pattern::Sequence { .. } => {
                self.use_sequence_pattern_language_items();
            }
            dir::Pattern::Object { .. } => {
                self.use_apparent_member_language_items();
            }
            _ => {}
        }

        dir::walk_pattern(self, tree, id, pattern);
    }

    /// Visit one pattern field.
    ///
    /// Example:
    /// ```tspp
    /// const { [key]: value } = object;
    /// ```
    fn visit_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::PatternField>,
        pattern_field: &dir::PatternField,
    ) {
        if matches!(pattern_field, dir::PatternField::Computed { .. }) {
            self.use_language_item(dir::LanguageItem::Index);
        }

        dir::walk_pattern_field(self, tree, id, pattern_field);
    }

    /// Visit one assignment pattern.
    ///
    /// Example:
    /// ```tspp
    /// [target] = values;
    /// ({ name: target } = user);
    /// ```
    fn visit_assign_pattern(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPattern>,
        pattern: &dir::AssignPattern,
    ) {
        match pattern {
            dir::AssignPattern::Sequence { .. } => {
                self.use_sequence_pattern_language_items();
            }
            dir::AssignPattern::Object { .. } => {
                self.use_apparent_member_language_items();
            }
            _ => {}
        }

        dir::walk_assign_pattern(self, tree, id, pattern);
    }

    /// Visit one assignment pattern field.
    ///
    /// Example:
    /// ```tspp
    /// ({ [key]: target } = object);
    /// ```
    fn visit_assign_pattern_field(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::AssignPatternField>,
        field: &dir::AssignPatternField,
    ) {
        if matches!(field, dir::AssignPatternField::Computed { .. }) {
            self.use_language_item(dir::LanguageItem::Index);
        }

        dir::walk_assign_pattern_field(self, tree, id, field);
    }

    /// Visit one decorator.
    ///
    /// Example:
    /// ```tspp
    /// @trace
    /// function f() {}
    /// ```
    fn visit_decorator(
        &mut self,
        tree: &dir::Tree,
        id: dir::LocalNodeId<dir::Decorator>,
        decorator: &dir::Decorator,
    ) {
        let view = dir::View::new(tree);
        match StaticGuard::classify(view, self.strings, id) {
            // resolve ordinary decorator expressions
            StaticGuard::Ordinary => dir::walk_decorator(self, tree, id, decorator),

            // resolve the static condition without resolving the intrinsic name
            StaticGuard::Condition(condition) => {
                let condition_node = tree.get(condition);
                self.visit_expression(tree, condition, condition_node);
            }

            // malformed guard
            StaticGuard::Rejected(_) => {}
        }
    }
}
