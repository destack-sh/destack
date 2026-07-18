use destack_core::FxIndexSet;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{
    Answer, BodyState, DecoratorApplication, FlowSite, NewtypeMatch, NewtypeRejection, Origin,
    PlaceUse, SelectedDecorator, answer,
};
use crate::{CompilerError, CompilerResult};

/// One selected compiler-owned derive provider.
struct SelectedDeriveProvider {
    /// The provider expression origin.
    origin: Origin,
    /// The source provider argument.
    argument: dir::GlobalNodeId<dir::Argument>,
    /// The exact provider backing.
    newtype: dir::NewtypeSelection,
    /// The instantiated provider type.
    ty: dir::GlobalTypeId,
}

impl BodyState<'_, '_> {
    /// Check one decorator application to a final selection.
    pub(in crate::check) fn check_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Option<SelectedDecorator>> {
        match self.select_decorator(site, application)? {
            Answer::Ready(selected) => Ok(selected),
            Answer::Pending(_) => {
                self.report_cannot_infer_node(site.node)?;
                self.commit_error_node(site.node)?;

                Ok(None)
            }
        }
    }

    /// Select one decorator as compile-time newtype data.
    fn select_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Answer<Option<SelectedDecorator>>> {
        let module = site.node.module_id;

        // collect explicit decorator type arguments
        let mut type_arguments = Vec::with_capacity(application.expression.generic_arguments.len());
        for argument in &application.expression.generic_arguments {
            let source = argument.into_global_any(module);
            type_arguments.push(answer!(self.node_type(source)?));
        }

        // type the target as its resolved newtype declaration
        let target = application.expression.target.into_global_any(module);
        if self.node_type_maybe(target).is_none() {
            let reference = dir::Type::Reference(dir::TypeReference {
                symbol: application.symbol,
            });
            let reference = self.intern_type(module, reference)?;
            self.commit_node_type(target, reference)?;
        }

        // compiler-owned derive dispatch has an intrinsic backing
        if self.global.language.item(application.symbol) == Some(dir::LanguageItem::Derive) {
            return self.select_derive_decorator(site, application);
        }

        // select the decorator backing
        let matched = answer!(self.match_newtype(
            site.origin(),
            application.symbol,
            &application.expression.arguments,
            &type_arguments,
            None,
        )?);
        let (selection, parameters, return_type) = match matched {
            NewtypeMatch::Selected {
                selection,
                parameters,
                return_type,
            } => (selection, parameters, return_type),
            NewtypeMatch::Rejected(rejection) => {
                match rejection {
                    NewtypeRejection::Signature(rejection) => {
                        self.report_signature_rejection(
                            site.origin(),
                            module,
                            &application.expression.arguments,
                            rejection,
                        )?;
                    }
                    NewtypeRejection::Candidates(notes) => {
                        let arguments =
                            answer!(self
                                .infer_argument_types(site, &application.expression.arguments,)?);
                        self.report_no_matching_construct(site.origin(), &arguments, &notes)?;
                    }
                }
                self.commit_error_node(site.node)?;

                return Ok(Answer::Ready(None));
            }
        };

        // commit the decorator-specific resolution and final argument checks
        let target = match self.global.language.item(application.symbol) {
            Some(item) => dir::DecoratorTarget::LanguageItem {
                symbol: application.symbol,
                item,
            },
            None => dir::DecoratorTarget::Symbol {
                symbol: application.symbol,
            },
        };
        let arguments =
            self.argument_bindings(module, &application.expression.arguments, &parameters);
        self.check_arguments(site, &application.expression.arguments, &arguments)?;

        let resolution = dir::DecoratorResolution {
            target,
            selection: dir::DecoratorSelection::Newtype {
                newtype: selection,
                arguments,
            },
            ty: return_type,
        };
        self.commit_node_type(site.node, resolution.ty)?;

        Ok(Answer::Ready(Some(SelectedDecorator {
            application,
            resolution,
        })))
    }

    /// Select one compiler-owned derive decorator and its providers.
    fn select_derive_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Answer<Option<SelectedDecorator>>> {
        let module = site.node.module_id;

        // derive has no generic parameters
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

            return Ok(Answer::Ready(None));
        }

        // select every provider as nominal newtype data
        let mut providers = Vec::with_capacity(application.expression.arguments.len());
        let mut selected_symbols = FxIndexSet::default();
        for argument in application.expression.arguments.iter().copied() {
            let selected = answer!(self.select_derive_provider(module, argument)?);
            let Some(selected) = selected else {
                self.commit_error_node(site.node)?;

                return Ok(Answer::Ready(None));
            };
            if !selected_symbols.insert(selected.newtype.symbol) {
                self.report_duplicate_derive_provider(selected.origin, selected.newtype.symbol)?;
                self.commit_error_node(site.node)?;

                return Ok(Answer::Ready(None));
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
        let type_arguments = self.intern_type_ids(module, &[])?;
        let ty = self.intern_type(
            module,
            dir::Type::Instance(dir::GenericInstance {
                symbol: application.symbol,
                arguments: type_arguments,
            }),
        )?;
        let resolution = dir::DecoratorResolution {
            target: dir::DecoratorTarget::LanguageItem {
                symbol: application.symbol,
                item: dir::LanguageItem::Derive,
            },
            selection: dir::DecoratorSelection::Derive { providers },
            ty,
        };
        self.commit_node_type(site.node, ty)?;

        Ok(Answer::Ready(Some(SelectedDecorator {
            application,
            resolution,
        })))
    }

    /// Select one compiler-owned derive provider.
    fn select_derive_provider(
        &mut self,
        module: ModuleId,
        argument: dir::LocalNodeId<dir::Argument>,
    ) -> CompilerResult<Answer<Option<SelectedDeriveProvider>>> {
        let Some(expression) = self.argument_expression(module, argument) else {
            return Err(CompilerError::Internal {
                message: format!("derive provider argument {argument:?} has no expression"),
            });
        };
        let expression = expression.into_typed::<dir::Expression>().local_id;
        let origin = self.node_site(expression.into_global_any(module))?.origin();
        let node = self.module_view(module).get(expression).clone();

        // configured providers are ordinary newtype constructions
        let (selection, ty) = if matches!(node, dir::Expression::Call { .. }) {
            let expression_site = self.node_site(expression.into_global_any(module))?;
            answer!(self.infer_node_type(expression_site, PlaceUse::Read)?);
            let resolution = self
                .resolutions(module)
                .construct_resolution(expression.into_global_any(module))
                .cloned();
            let Some(resolution) = resolution else {
                let ty = self.require_node_type(expression.into_global_any(module))?;
                if !self.ty(ty)?.is_error() {
                    self.report_invalid_derive_provider(origin, ty)?;
                }

                return Ok(Answer::Ready(None));
            };
            let dir::ConstructTarget::Newtype(selection) = resolution.target else {
                self.report_invalid_derive_provider(origin, resolution.return_type)?;

                return Ok(Answer::Ready(None));
            };

            (selection, resolution.return_type)
        }
        // bare providers select their zero-argument backing
        else {
            let source = expression.into_global_any(module);
            let Some(symbol) = self.reference_symbol(source) else {
                let ty = answer!(self.infer_node_type(self.node_site(source)?, PlaceUse::Read)?);
                if !self.ty(ty)?.is_error() {
                    self.report_invalid_derive_provider(origin, ty)?;
                }

                return Ok(Answer::Ready(None));
            };
            let symbol = self.resolve_symbol_alias(symbol)?;
            let matched = answer!(self.match_newtype(origin, symbol, &[], &[], None)?);
            let NewtypeMatch::Selected {
                selection,
                return_type,
                ..
            } = matched
            else {
                let ty = answer!(self.infer_node_type(self.node_site(source)?, PlaceUse::Read)?);
                if !self.ty(ty)?.is_error() {
                    self.report_invalid_derive_provider(origin, ty)?;
                }

                return Ok(Answer::Ready(None));
            };
            self.commit_node_type(source, return_type)?;

            (selection, return_type)
        };

        // require the compiler-owned Tagged provider
        if self.global.language.item(selection.symbol) != Some(dir::LanguageItem::Tagged) {
            self.report_invalid_derive_provider(origin, ty)?;

            return Ok(Answer::Ready(None));
        }

        Ok(Answer::Ready(Some(SelectedDeriveProvider {
            origin,
            argument: argument.into_global(module),
            newtype: selection,
            ty,
        })))
    }
}
