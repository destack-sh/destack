use tspp_dir as dir;

use crate::sema::{DecoratorApplication, DecoratorExpression, WalkState};
use crate::{CompilerError, CompilerResult};

impl WalkState<'_, '_> {
    /// Walk one decorator application.
    ///
    /// Example:
    /// ```tspp
    /// @inline
    /// function f() {}
    /// ```
    pub(in crate::sema) fn walk_decorator(
        &mut self,
        decorator: DecoratorExpression,
        owner: dir::GlobalNodeIdAny,
    ) -> CompilerResult<()> {
        let arguments = decorator.arguments.clone();
        if !self.declare_decorator(decorator, owner)? {
            return Ok(());
        }

        // walk decorator value expressions
        for argument in &arguments {
            self.walk_argument(*argument, self.tree.get(*argument))?;
        }

        Ok(())
    }

    /// Declare one decorator application without walking its values.
    pub(in crate::sema) fn declare_decorator(
        &mut self,
        decorator: DecoratorExpression,
        owner: dir::GlobalNodeIdAny,
    ) -> CompilerResult<bool> {
        self.enter_node(decorator.decorator)?;
        self.enter_node(decorator.target)?;

        // declare each authored decorator once, however often typing revisits its node
        if !self.check.walked_decorators.insert(decorator.decorator) {
            return Ok(false);
        }

        // resolve the decorator declaration exactly once
        let Some(symbol) = self.walk_decorator_target(decorator.target)? else {
            return Ok(false);
        };
        // skip kind validation on foreign targets while declaring
        if self
            .check
            .symbol_kind(symbol)
            .map(|kind| !matches!(kind, dir::SymbolKind::Newtype))?
        {
            self.check
                .report_invalid_decorator_target(self.module, decorator.target.into_any());

            return Ok(false);
        }

        // walk explicit decorator type arguments
        self.walk_generic_arguments(&decorator.generic_arguments)?;

        // retain the resolved application in module walk order
        self.check.decorators.push(DecoratorApplication {
            expression: decorator,
            owner,
            symbol,
        });

        Ok(true)
    }

    /// Walk one decorator target.
    ///
    /// Example:
    /// ```tspp
    /// @repr("C")
    /// struct Header {}
    /// ```
    fn walk_decorator_target(
        &mut self,
        target: dir::LocalNodeId<dir::Expression>,
    ) -> CompilerResult<Option<dir::GlobalSymbolId>> {
        // require a lexical name or namespace path
        let is_member = match self.tree.get(target) {
            dir::Expression::Identifier { .. } => false,
            dir::Expression::Member { .. } => true,
            _ => {
                self.check
                    .report_invalid_decorator_target(self.module, target.into_any());

                return Ok(None);
            }
        };
        // reject member chains that form no lexical path
        let Some(path) = self.tree.tree().reference_path(target) else {
            self.check
                .report_invalid_decorator_target(self.module, target.into_any());

            return Ok(None);
        };
        let reference = self
            .check
            .module(self.module)
            .resolved
            .references
            .get(target.into_global_any(self.module))
            .cloned();
        let Some(reference) = reference else {
            // classify members without namespace resolution as value projections
            if is_member {
                self.check
                    .report_invalid_decorator_target(self.module, target.into_any());

                return Ok(None);
            }

            return Err(CompilerError::Internal {
                message: format!("decorator target {target:?} has no resolved reference"),
            });
        };

        // resolve the one visible declaration named by the path
        let symbol = match reference {
            dir::Reference::Bound(symbols) => {
                let symbols = self.check.present_symbols(&symbols);

                match symbols.as_slice() {
                    [] => {
                        self.check.report_unresolved_reference(
                            self.module,
                            target.into_any(),
                            &path,
                        )?;

                        None
                    }
                    [symbol] => Some(*symbol),
                    _ => {
                        self.check.report_ambiguous_reference(
                            self.module,
                            target.into_any(),
                            &path,
                        )?;

                        None
                    }
                }
            }
            dir::Reference::TypeLiteral(_) | dir::Reference::Missing => {
                self.check
                    .report_unresolved_reference(self.module, target.into_any(), &path)?;

                None
            }
            dir::Reference::Ambiguous(_) => {
                self.check
                    .report_ambiguous_reference(self.module, target.into_any(), &path)?;

                None
            }
            dir::Reference::Namespace { .. } | dir::Reference::Projected { .. } => {
                self.check
                    .report_invalid_decorator_target(self.module, target.into_any());

                None
            }
        };

        if let Some(symbol) = symbol {
            self.check.commit_name(
                target.into_global_any(self.module),
                dir::NameResolution::new(symbol),
            )?;
        }

        Ok(symbol)
    }
}
