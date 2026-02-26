use crate::analyze::common::{ModuleTreeView, TypeContext};
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{
    DependencyKind, DependencySource, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, Path,
    StaticArgument, StaticKey, StringId, SymbolSpaceOrder, Type,
};

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a type import into a concrete exported symbol.
    pub(crate) fn resolve_import_type_symbol(
        &self,
        view: ModuleTreeView<'_>,
        node: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
    ) -> AnalyzeResult<Option<GlobalSymbolId>> {
        // reject empty or nested qualifiers for now
        let Some(member_key) = self.import_type_member_key(qualifier) else {
            return Ok(None);
        };

        // resolve the module target from the import specifier
        let dir = view.module.dir(view.profile);
        let node = node.into_global(view.module.id);
        let target = match self.resolve_import(
            view.module,
            dir,
            view.profile,
            node,
            DependencySource::ImportStatement,
            target,
            DependencyKind::Type,
        ) {
            Ok(target) => target,
            Err(error) => {
                self.error(error);
                return Ok(None);
            }
        };

        // resolve the exported symbol from the target module
        let symbol = match self.resolve_export_symbol_for_target(
            view.module.id,
            node,
            target,
            view.profile,
            SymbolSpaceOrder::TypeOnly,
            member_key,
        ) {
            Ok(symbol) => symbol,
            Err(error) => {
                self.error(error);
                return Ok(None);
            }
        };
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        // ensure exported types are available for the resolved symbol
        self.require_analyze_module_interface(symbol.module_id, view.profile)?;

        Ok(Some(symbol))
    }

    /// Resolve a type import into a local reference type.
    pub(crate) fn resolve_import_type_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
        static_arguments: Option<&[StaticArgument]>,
    ) -> AnalyzeResult<Option<LocalTypeId>> {
        let symbol =
            self.resolve_import_type_symbol(ctx.module_tree_view(), source_id, target, qualifier)?;
        let Some(symbol) = symbol else {
            return Ok(None);
        };

        let static_arguments = static_arguments.map(|arguments| arguments.to_vec());
        let reference = Type::Reference {
            symbol,
            static_arguments,
        };
        Ok(Some(ctx.types.insert_type_from_any(reference, source_id)))
    }

    /// Query a type import reference in non-AnalyzeResult paths.
    pub(crate) fn query_import_type_reference(
        &self,
        ctx: &mut TypeContext<'_>,
        source_id: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
        static_arguments: Option<&[StaticArgument]>,
    ) -> Option<LocalTypeId> {
        match self.resolve_import_type_reference(
            &mut ctx.reborrow(),
            source_id,
            target,
            qualifier,
            static_arguments,
        ) {
            Ok(reference_type_id) => reference_type_id,
            Err(AnalyzeError::Yield { .. }) => None,
            Err(error) => {
                self.error(error);
                None
            }
        }
    }

    /// Convert an import qualifier path into a static key.
    fn import_type_member_key(&self, qualifier: Option<&Path>) -> Option<StaticKey> {
        let qualifier = qualifier?;
        if qualifier.segments.len() != 1 {
            return None;
        }
        qualifier.last_segment().map(StaticKey::Name)
    }
}
