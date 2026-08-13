use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    BodyState, DecoratorApplication, FlowSite, NewtypeMatch, NewtypeOverload, NewtypeRejection,
    Origin, PlaceUse, SelectedDecorator, ValueUse,
};
use crate::{CompilerError, CompilerResult};

/// One selected compiler-owned derive provider.
struct SelectedDeriveProvider {
    /// The source provider argument.
    argument: dir::GlobalNodeId<dir::Argument>,
    /// The exact provider backing.
    newtype: dir::NewtypeSelection,
    /// The instantiated provider type.
    ty: dir::GlobalTypeId,
}

impl BodyState<'_, '_> {
    /// Check one decorator application.
    pub(in crate::sema) fn check_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Option<SelectedDecorator>> {
        let module = site.node.module_id;

        // collect explicit decorator type arguments
        let mut type_arguments = Vec::with_capacity(application.expression.generic_arguments.len());
        for argument in &application.expression.generic_arguments {
            let source = argument.into_global_any(module);
            type_arguments.push(self.require_node_type(source)?);
        }

        // type the target as its resolved newtype declaration
        let target = application.expression.target.into_global_any(module);
        let reference = dir::Type::Reference(dir::TypeReference {
            symbol: application.symbol,
        });
        let reference = self.intern_type(reference)?;
        self.commit_node_type(target, reference)?;

        // decide the target reference for its own resolution fact
        if self.resolutions(module).name_resolution(target).is_none() {
            self.check
                .commit_name(target, dir::NameResolution::new(application.symbol))?;
        }

        // dispatch compiler-owned derive to its intrinsic backing
        if self.environment_bound.language.item(application.symbol)
            == Some(dir::LanguageItem::Derive)
        {
            return self.select_derive_decorator(site, application);
        }

        // select the decorator backing
        let matched = self.match_newtype(
            site.origin(),
            application.symbol,
            &application.expression.arguments,
            &type_arguments,
            None,
            NewtypeOverload::Unambiguous,
            ValueUse::Comptime,
        )?;
        let (selection, signature) = match matched {
            NewtypeMatch::Selected(signature) | NewtypeMatch::ReturnMismatch(signature) => {
                (signature.selection, signature.signature)
            }
            NewtypeMatch::Invalid { rejection, .. } => {
                self.report_decorator_rejection(
                    site.origin(),
                    NewtypeRejection::Signature(rejection),
                )?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
            NewtypeMatch::Rejected(rejection) => {
                self.report_decorator_rejection(site.origin(), rejection)?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
        };

        // type argument values their selection left uncommitted
        for argument in &application.expression.arguments {
            if let Some(value) = self.argument_expression(module, *argument)
                && self.check.committed_node_type(value).is_none()
            {
                let value_site = self.check.visit_site(value)?;
                self.attempt_node(value_site, PlaceUse::Read, None)?;
            }
        }

        // name the decorator target and bind its written arguments
        let target = match self.environment_bound.language.item(application.symbol) {
            Some(item) => dir::DecoratorTarget::LanguageItem {
                symbol: application.symbol,
                item,
            },
            None => dir::DecoratorTarget::Symbol {
                symbol: application.symbol,
            },
        };
        let arguments = self.selected_argument_bindings(
            site.node,
            module,
            &application.expression.arguments,
            &signature,
        )?;
        let return_type = signature.return_type;

        // commit the selected newtype backing as the decorator's resolution
        let resolution = dir::DecoratorResolution {
            target,
            selection: dir::DecoratorSelection::Newtype {
                newtype: selection,
                arguments,
            },
            ty: return_type,
        };
        self.commit_node_type(site.node, resolution.ty)?;

        Ok(Some(SelectedDecorator {
            application,
            resolution,
        }))
    }

    /// Select one compiler-owned derive decorator and its providers.
    fn select_derive_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Option<SelectedDecorator>> {
        let module = site.node.module_id;

        // reject written generic arguments, derive takes none
        if !application.expression.generic_arguments.is_empty() {
            let name = self.format_symbol(application.symbol);
            self.report_wrong_generic_arity(
                module,
                site.node.local_id,
                name,
                0,
                application.expression.generic_arguments.len(),
            );
            self.commit_error_node(site.node)?;

            return Ok(None);
        }

        // select every provider as nominal newtype data
        let mut providers = Vec::with_capacity(application.expression.arguments.len());
        let mut selected_symbols = FxIndexSet::default();
        for argument in application.expression.arguments.iter().copied() {
            let selected = self.select_derive_provider(module, argument)?;
            let Some(selected) = selected else {
                self.commit_error_node(site.node)?;

                return Ok(None);
            };
            // reject duplicate providers without a second report
            if !selected_symbols.insert(selected.newtype.symbol) {
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
            providers.push(selected);
        }

        // retain exact provider selections in argument order
        let providers = providers
            .into_iter()
            .map(|provider| dir::DeriveProvider {
                argument: provider.argument,
                newtype: provider.newtype,
                ty: provider.ty,
            })
            .collect();

        // construct the opaque compiler-defined derive type
        let type_arguments = self.intern_type_ids(&[])?;
        let ty = self.intern_type(dir::Type::Application(dir::GenericApplication {
            symbol: application.symbol,
            arguments: type_arguments,
        }))?;
        let resolution = dir::DecoratorResolution {
            target: dir::DecoratorTarget::LanguageItem {
                symbol: application.symbol,
                item: dir::LanguageItem::Derive,
            },
            selection: dir::DecoratorSelection::Derive { providers },
            ty,
        };
        self.commit_node_type(site.node, ty)?;

        Ok(Some(SelectedDecorator {
            application,
            resolution,
        }))
    }

    /// Select one compiler-owned derive provider.
    fn select_derive_provider(
        &mut self,
        module: ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<Option<SelectedDeriveProvider>> {
        let Some(expression) = self.argument_expression(module, argument) else {
            return Err(CompilerError::Internal {
                message: format!("derive provider argument {argument:?} has no expression"),
            });
        };
        let expression = expression.into_typed::<dir::Expression>().local_id;
        let origin = Origin::Node(expression.into_global_any(module), None);
        let node = self.module_view(module).get(expression).clone();

        // select the exact newtype backing of a configured provider
        let (selection, ty) = if let dir::Expression::Call {
            left,
            generic_arguments,
            arguments,
            ..
        } = node
        {
            let target = left.into_global_any(module);
            let Some(symbol) = self.reference_symbol(target) else {
                self.report_invalid_derive_provider(origin)?;

                return Ok(None);
            };
            // require a newtype provider
            if self
                .symbol_kind_maybe(symbol)?
                .is_some_and(|kind| !matches!(kind, dir::SymbolKind::Newtype))
            {
                self.report_invalid_derive_provider(origin)?;

                return Ok(None);
            }
            let reference = dir::Type::Reference(dir::TypeReference { symbol });
            let reference = self.intern_type(reference)?;
            self.commit_node_type(target, reference)?;

            // collect written generic arguments
            let mut type_arguments = Vec::with_capacity(generic_arguments.len());
            for argument in &generic_arguments {
                type_arguments.push(self.require_node_type(argument.into_global_any(module))?);
            }

            // select the only viable provider backing
            let matched = self.match_newtype(
                origin,
                symbol,
                &arguments,
                &type_arguments,
                None,
                NewtypeOverload::Unambiguous,
                ValueUse::Comptime,
            )?;
            let (selection, signature) = match matched {
                NewtypeMatch::Selected(signature) | NewtypeMatch::ReturnMismatch(signature) => {
                    (signature.selection, signature.signature)
                }
                NewtypeMatch::Invalid { rejection, .. } => {
                    self.report_decorator_rejection(
                        origin,
                        NewtypeRejection::Signature(rejection),
                    )?;
                    self.commit_error_node(expression.into_global_any(module))?;

                    return Ok(None);
                }
                NewtypeMatch::Rejected(rejection) => {
                    self.report_decorator_rejection(origin, rejection)?;
                    self.commit_error_node(expression.into_global_any(module))?;

                    return Ok(None);
                }
            };

            // retain the selected provider construction for static evaluation
            let resolution = dir::ConstructDecision::new(
                dir::ConstructTarget::Newtype(selection.clone()),
                self.selected_argument_bindings(
                    expression.into_global_any(module),
                    module,
                    &arguments,
                    &signature,
                )?,
                signature.return_type,
            );
            self.commit_decision(
                expression.into_global_any(module),
                dir::Decision::Construct(resolution),
            )?;
            self.commit_node_type(expression.into_global_any(module), signature.return_type)?;

            (selection, signature.return_type)
        }
        // otherwise select the zero-argument backing of a bare provider
        else {
            let source = expression.into_global_any(module);
            let Some(symbol) = self.reference_symbol(source) else {
                self.report_invalid_derive_provider(origin)?;

                return Ok(None);
            };

            // accept capability interfaces as compiler-owned derives
            let is_capability = self
                .environment_bound
                .language
                .item(symbol)
                .and_then(dir::AutoInterface::from_language_item)
                .is_some_and(dir::AutoInterface::is_derivable);
            if is_capability {
                let reference = dir::Type::Reference(dir::TypeReference { symbol });
                let ty = self.intern_type(reference)?;
                self.commit_node_type(source, ty)?;

                return Ok(Some(SelectedDeriveProvider {
                    argument: argument.into_global(module),
                    newtype: dir::NewtypeSelection {
                        symbol,
                        backing: ty,
                        generic_arguments: Vec::new(),
                    },
                    ty,
                }));
            }

            // require a newtype provider
            if self
                .symbol_kind_maybe(symbol)?
                .is_some_and(|kind| !matches!(kind, dir::SymbolKind::Newtype))
            {
                self.report_invalid_derive_provider(origin)?;

                return Ok(None);
            }
            let matched = self.match_newtype(
                origin,
                symbol,
                &[],
                &[],
                None,
                NewtypeOverload::Unambiguous,
                ValueUse::Comptime,
            )?;
            let (selection, return_type) = match matched {
                NewtypeMatch::Selected(signature) | NewtypeMatch::ReturnMismatch(signature) => {
                    (signature.selection, signature.signature.return_type)
                }
                NewtypeMatch::Invalid { rejection, .. } => {
                    self.report_decorator_rejection(
                        origin,
                        NewtypeRejection::Signature(rejection),
                    )?;
                    self.commit_error_node(source)?;

                    return Ok(None);
                }
                NewtypeMatch::Rejected(rejection) => {
                    self.report_decorator_rejection(origin, rejection)?;
                    self.commit_error_node(source)?;

                    return Ok(None);
                }
            };
            self.commit_node_type(source, return_type)?;

            (selection, return_type)
        };

        // require the compiler-owned Tagged provider
        if self.environment_bound.language.item(selection.symbol) != Some(dir::LanguageItem::Tagged)
        {
            self.report_invalid_derive_provider(origin)?;

            return Ok(None);
        }

        Ok(Some(SelectedDeriveProvider {
            argument: argument.into_global(module),
            newtype: selection,
            ty,
        }))
    }

    /// Report one rejected decorator newtype selection.
    fn report_decorator_rejection(
        &mut self,
        origin: Origin,
        rejection: NewtypeRejection,
    ) -> CompilerResult<()> {
        match rejection {
            NewtypeRejection::Signature(rejection) => {
                self.report_signature_rejection(origin, rejection)
            }
            NewtypeRejection::NoMatch(notes) => self.report_no_matching_decorator(origin, &notes),
            NewtypeRejection::Ambiguous => self.report_ambiguous_decorator(origin),
        }
    }
}
