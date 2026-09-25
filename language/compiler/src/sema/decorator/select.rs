use tspp_core::FxIndexSet;
use tspp_dir as dir;

use crate::sema::{
    CheckState, DecoratorApplication, FlowSite, NewtypeMatch, Origin, OverloadRule, PlaceUse,
    SelectedDecorator, ValueUse,
};
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Select the backing for one decorator application.
    pub(in crate::sema) fn select_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Option<SelectedDecorator>> {
        // read the decorating module
        let module = site.node.module_id;

        // collect explicit decorator type arguments
        let mut type_arguments = Vec::with_capacity(application.expression.generic_arguments.len());
        for argument in &application.expression.generic_arguments {
            let source = argument.into_global_any(module);
            type_arguments.push(self.require_node_type(source)?);
        }

        // type the target as its resolved newtype declaration
        let target = application.expression.target.into_global_any(module);
        let reference = dir::Type::Reference(dir::TypeReference::new(application.symbol));
        let reference = self.intern_type(reference)?;
        self.commit_node_type(target, reference)?;

        // decide the target reference for its own resolution fact
        if self.resolutions(module).name_resolution(target).is_none() {
            self.commit_name(target, dir::NameResolution::new(application.symbol))?;
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
            OverloadRule::Exclusive,
            ValueUse::Const,
        )?;
        let (key, backing, signature) = match matched {
            NewtypeMatch::Selected(signature, _) => {
                (signature.key, signature.backing, signature.signature)
            }
            NewtypeMatch::Refused => {
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
            NewtypeMatch::Rejected(notes) => {
                self.report_no_matching_decorator(site.origin(), &notes)?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
            NewtypeMatch::Ambiguous => {
                self.report_ambiguous_decorator(site.origin())?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
        };

        // commit the coercions the selected backing matched
        for (source, coercion) in &signature.coercions {
            self.commit_coercion(*source, coercion.clone())?;
        }

        // type argument values their selection left uncommitted
        for argument in &application.expression.arguments {
            if let Some(value) = self.argument_expression(module, *argument)
                && self.committed_node_type(value).is_none()
            {
                let value_site = self.visit_site(value)?;
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
        let arguments = signature.bind_arguments(Origin::Node(site.node, None), self)?;
        let return_type = signature.return_type;

        // commit the selected newtype backing as the decorator's resolution
        let resolution = dir::DecoratorResolution {
            target,
            selection: dir::DecoratorSelection::Newtype {
                key,
                backing,
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

    /// Select one compiler-owned derive decorator and its interfaces.
    fn select_derive_decorator(
        &mut self,
        site: FlowSite,
        application: DecoratorApplication,
    ) -> CompilerResult<Option<SelectedDecorator>> {
        // read the decorating module
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

        // select every written argument as a derivable interface
        let mut interfaces = Vec::with_capacity(application.expression.arguments.len());
        let mut selected = FxIndexSet::default();
        for argument in application.expression.arguments.iter().copied() {
            // resolve the written argument reference
            let Some(expression) = self.argument_expression(module, argument) else {
                return Err(CompilerError::Internal {
                    message: format!("derive argument {argument:?} has no expression"),
                });
            };
            let expression = expression.into_typed::<dir::Expression>().local_id;
            let source = expression.into_global_any(module);
            let origin = Origin::Node(source, None);
            let Some(symbol) = self.reference_symbol(source)? else {
                self.report_invalid_derive_interface(origin)?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            };

            // require a compiler-known derivable interface
            let Some(interface) = self
                .environment_bound
                .language
                .item(symbol)
                .and_then(dir::AutoInterface::from_language_item)
                .filter(|interface| interface.is_derivable())
            else {
                self.report_invalid_derive_interface(origin)?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            };

            // reject duplicate interfaces at the repeated argument
            if !selected.insert(interface) {
                self.report_duplicate_derive_interface(origin, symbol)?;
                self.commit_error_node(site.node)?;

                return Ok(None);
            }
            let reference =
                self.intern_type(dir::Type::Reference(dir::TypeReference::new(symbol)))?;
            self.commit_node_type(source, reference)?;
            interfaces.push(interface);
        }

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
            selection: dir::DecoratorSelection::Derive { interfaces },
            ty,
        };
        self.commit_node_type(site.node, ty)?;

        Ok(Some(SelectedDecorator {
            application,
            resolution,
        }))
    }
}
