use std::sync::Arc;

use tspp_dir as dir;
use tspp_source::{DiagnosticCollection, File};

use super::compiler::Compiler;
use super::marker::{Marker, MarkerError};
use super::place::MarkerTarget;
use crate::{MetavariableTable, Predicate, PredicateError, PredicateUse, PredicateUses};

impl Compiler {
    /// Compile one predicate expression against existing metavariable declarations.
    pub(crate) fn compile_predicate(
        &mut self,
        file: Arc<File>,
        metavariables: &MetavariableTable,
    ) -> Result<Predicate, DiagnosticCollection> {
        self.register(file.clone());
        let mut parse = Self::parse(file);

        // reject malformed predicate source
        if !parse.errors.is_empty() {
            return Err(parse.diagnostics());
        }

        // index ancestors for marker resolution
        parse.tree.index_parents(&parse.roots);

        // require one complete expression root
        let [root] = parse.roots.as_slice() else {
            let error = PredicateError::ExpectedRoot {
                anchor: parse.file.id.into(),
                found: parse.roots.len(),
            };

            return Err(self.report(error, &parse.file));
        };
        let root = *root;
        let tokens = parse.take_token_spans();
        self.strings().extend(&parse.strings);
        let mut uses = PredicateUses::new(parse.tree.node_count());

        // bind every predicate marker to an existing metavariable
        for token in tokens {
            let marker = Marker::parse(&parse.file, token)
                .map_err(|error| self.predicate_marker_error(error, &parse.file))?;
            let Some(marker) = marker else {
                continue;
            };
            if marker.is_nodes {
                let error = PredicateError::RepeatedMetavariable {
                    anchor: marker.span.into(),
                };

                return Err(self.report(error, &parse.file));
            }
            let name = self.strings().intern(&marker.name);
            let Some(variable) = metavariables.find_id(name) else {
                let error = PredicateError::UnboundMetavariable {
                    anchor: marker.span.into(),
                    name: marker.name,
                };

                return Err(self.report(error, &parse.file));
            };
            let target = marker
                .resolve(&parse.tree)
                .map_err(|error| self.predicate_marker_error(error, &parse.file))?;
            let MarkerTarget::Node(expression) = target else {
                let error = PredicateError::InvalidMetavariable {
                    anchor: marker.span.into(),
                };

                return Err(self.report(error, &parse.file));
            };
            if expression.ty != dir::NodeType::Expression {
                let error = PredicateError::InvalidMetavariable {
                    anchor: marker.span.into(),
                };

                return Err(self.report(error, &parse.file));
            }

            uses.insert(PredicateUse {
                variable,
                expression: dir::LocalNodeId::new(expression.id),
            });
        }

        self.validate_predicate(&parse.tree, root, &uses, &parse.file)?;

        Ok(Predicate {
            tree: parse.tree,
            root,
            uses,
        })
    }

    /// Validate one predicate condition.
    fn validate_predicate(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
        uses: &PredicateUses,
        file: &File,
    ) -> Result<(), DiagnosticCollection> {
        if self.is_predicate_condition(tree, expression, uses) {
            return Ok(());
        }
        let node = tree.get(expression);

        // diagnose recognized operators with unsupported operands
        if let dir::Expression::Binary { operator, .. } = node
            && matches!(
                operator,
                dir::BinaryOperator::And
                    | dir::BinaryOperator::Or
                    | dir::BinaryOperator::Equal
                    | dir::BinaryOperator::NotEqual
                    | dir::BinaryOperator::EqualStrict
                    | dir::BinaryOperator::NotEqualStrict
                    | dir::BinaryOperator::LessThan
                    | dir::BinaryOperator::LessThanOrEqual
                    | dir::BinaryOperator::GreaterThan
                    | dir::BinaryOperator::GreaterThanOrEqual
            )
        {
            return Err(self.invalid_predicate_operands(tree, expression, *operator, file));
        }

        let anchor = self.predicate_anchor(tree, expression, file)?;
        let error = PredicateError::UnsupportedExpression { anchor };

        Err(self.report(error, file))
    }

    /// Return whether one expression is a supported predicate condition.
    fn is_predicate_condition(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
        uses: &PredicateUses,
    ) -> bool {
        if uses.get(expression).is_some() {
            return true;
        }

        match tree.get(expression) {
            dir::Expression::Literal(dir::Literal::Boolean(_)) => true,
            dir::Expression::Unary {
                operator: dir::UnaryOperator::Not,
                right,
            } => self.is_predicate_condition(tree, *right, uses),
            dir::Expression::Binary {
                left,
                operator,
                right,
            } if operator.is_logical_boolean() => {
                self.is_predicate_condition(tree, *left, uses)
                    && self.is_predicate_condition(tree, *right, uses)
            }
            dir::Expression::Binary {
                left,
                operator:
                    dir::BinaryOperator::Equal
                    | dir::BinaryOperator::NotEqual
                    | dir::BinaryOperator::EqualStrict
                    | dir::BinaryOperator::NotEqualStrict
                    | dir::BinaryOperator::LessThan
                    | dir::BinaryOperator::LessThanOrEqual
                    | dir::BinaryOperator::GreaterThan
                    | dir::BinaryOperator::GreaterThanOrEqual,
                right,
            } => {
                self.is_predicate_value(tree, *left, uses)
                    && self.is_predicate_value(tree, *right, uses)
            }
            dir::Expression::Satisfies {
                expression,
                target_type,
            } => uses.get(*expression).is_some() && self.is_predicate_type(tree, *target_type),
            _ => false,
        }
    }

    /// Return whether one expression is a side-effect-free predicate value.
    fn is_predicate_value(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
        uses: &PredicateUses,
    ) -> bool {
        if uses.get(expression).is_some() {
            return true;
        }

        match tree.get(expression) {
            dir::Expression::Identifier { .. } | dir::Expression::Literal(_) => true,
            dir::Expression::Member {
                left,
                name: Some(_),
                ..
            } => self.is_predicate_value(tree, *left, uses),
            _ => false,
        }
    }

    /// Return whether one closed predicate target type has implemented assignability.
    fn is_predicate_type(
        &self,
        tree: &dir::Tree,
        target: dir::LocalNodeId<dir::TypeExpression>,
    ) -> bool {
        match tree.get(target) {
            dir::TypeExpression::Keyword { .. } | dir::TypeExpression::Literal { .. } => true,
            dir::TypeExpression::Reference {
                generic_arguments, ..
            } => generic_arguments.iter().all(|argument| {
                matches!(
                    tree.get(*argument),
                    dir::GenericArgument::Type { value }
                        if self.is_predicate_type(tree, *value)
                )
            }),
            dir::TypeExpression::Union { elements }
            | dir::TypeExpression::Intersection { elements } => elements
                .iter()
                .all(|element| self.is_predicate_type(tree, *element)),
            _ => false,
        }
    }

    /// Report invalid values supplied to one predicate operator.
    fn invalid_predicate_operands(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
        operator: dir::BinaryOperator,
        file: &File,
    ) -> DiagnosticCollection {
        let anchor = match self.predicate_anchor(tree, expression, file) {
            Ok(anchor) => anchor,
            Err(diagnostics) => return diagnostics,
        };
        let error = PredicateError::InvalidOperands {
            anchor,
            operator: operator.text().to_string(),
        };

        self.report(error, file)
    }

    /// Resolve one predicate expression to a diagnostic anchor.
    fn predicate_anchor(
        &self,
        tree: &dir::Tree,
        expression: dir::LocalNodeId<dir::Expression>,
        file: &File,
    ) -> Result<tspp_artifact::DiagnosticAnchor, DiagnosticCollection> {
        let Some(span) = tree.get_span_by_id(expression.id) else {
            let error = PredicateError::Internal {
                anchor: file.id.into(),
                message: "predicate expression has no source span".to_string(),
            };

            return Err(self.report(error, file));
        };

        Ok(span.into())
    }

    /// Report one marker error in a predicate expression.
    fn predicate_marker_error(&self, error: MarkerError, file: &File) -> DiagnosticCollection {
        match error {
            MarkerError::InvalidSource { span } => self.report(
                PredicateError::Internal {
                    anchor: span.into(),
                    message: "predicate token is outside its source".to_string(),
                },
                file,
            ),
            MarkerError::Invalid { span } => self.report(
                PredicateError::InvalidMetavariable {
                    anchor: span.into(),
                },
                file,
            ),
            MarkerError::InvalidRepeated { span } => self.report(
                PredicateError::RepeatedMetavariable {
                    anchor: span.into(),
                },
                file,
            ),
            MarkerError::MissingNodeSpan { span } => self.report(
                PredicateError::Internal {
                    anchor: span.into(),
                    message: "repeated placeholder element has no source span".to_string(),
                },
                file,
            ),
        }
    }
}
