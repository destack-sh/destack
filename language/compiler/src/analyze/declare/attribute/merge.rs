use crate::{AnalyzeError, Compiler};
use destack_dir::{
    Binding, Decorator, DeprecatedNotice, ExperimentalNotice, ExternBinding, IntrinsicBinding,
    LanguageItemBinding, LifetimeAnnotation, LocalNodeId, SymbolDecorators, UnrollHint,
};
use destack_workspace::{Module, ProfileId};

impl Compiler {
    /// Merge an extern binding into the symbol metadata.
    pub(crate) fn merge_extern_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        binding: ExternBinding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.extern_binding.as_ref() else {
            decorators.extern_binding = Some(binding);
            return;
        };

        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "extern decorator is already set",
            );
        }
    }

    /// Merge a binding into the symbol metadata.
    pub(crate) fn merge_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        binding: Binding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.binding.as_ref() else {
            decorators.binding = Some(binding);
            return;
        };

        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "binding decorator is already set",
            );
        }
    }

    /// Merge a language item binding into the symbol metadata.
    pub(crate) fn merge_language_item_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        binding: LanguageItemBinding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.language_item.as_ref() else {
            decorators.language_item = Some(binding);
            return;
        };

        // reject conflicting bindings
        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "languageItem decorator is already set",
            );
        }
    }

    /// Merge an intrinsic binding into the symbol metadata.
    pub(crate) fn merge_intrinsic_binding(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        binding: IntrinsicBinding,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.intrinsic_binding.as_ref() else {
            decorators.intrinsic_binding = Some(binding);
            return;
        };

        if existing != &binding {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "intrinsic decorator is already set",
            );
        }
    }

    /// Merge a lifetime annotation into the symbol metadata.
    pub(crate) fn merge_lifetime_annotation(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        lifetime: LifetimeAnnotation,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.lifetime.as_ref() else {
            decorators.lifetime = Some(lifetime);
            return;
        };

        // reject conflicting annotations
        if existing != &lifetime {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "lifetime decorator is already set",
            );
        }
    }

    /// Merge a deprecated notice into the symbol metadata.
    pub(crate) fn merge_deprecated_notice(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        notice: DeprecatedNotice,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.deprecated.as_ref() else {
            decorators.deprecated = Some(notice);
            return;
        };

        if existing != &notice {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "deprecated decorator is already set",
            );
        }
    }

    /// Merge an experimental notice into the symbol metadata.
    pub(crate) fn merge_experimental_notice(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        notice: ExperimentalNotice,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.experimental.as_ref() else {
            decorators.experimental = Some(notice);
            return;
        };

        if existing != &notice {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "experimental decorator is already set",
            );
        }
    }

    /// Merge an unroll hint into the symbol metadata.
    pub(crate) fn merge_unroll_hint(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        hint: UnrollHint,
        decorators: &mut SymbolDecorators,
    ) {
        let Some(existing) = decorators.unroll.as_ref() else {
            decorators.unroll = Some(hint);
            return;
        };

        if existing != &hint {
            self.report_invalid_well_known_decorator(
                module,
                profile,
                annotation_id,
                "unroll decorator is already set",
            );
        }
    }

    /// Emit a diagnostic for an invalid well-known decorator.
    pub(crate) fn report_invalid_well_known_decorator(
        &self,
        module: &Module,
        profile: ProfileId,
        annotation_id: LocalNodeId<Decorator>,
        message: &str,
    ) {
        let message = self.repository.strings.intern(message);
        self.error(AnalyzeError::InvalidWellKnownDecorator {
            node: annotation_id
                .into_global_any(module.id)
                .into_anchored(Some(profile)),
            message,
        });
    }
}
