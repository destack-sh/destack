use destack_dir::{
    DependencySource, GlobalSymbolId, LocalNodeIdAny, LocalTypeId, Path, StaticArgument, StaticKey,
    StringId, SymbolSpaceOrder, Type, TypeTable,
};
use destack_workspace::{Module, ProfileId};

use crate::Compiler;

#[allow(clippy::too_many_arguments)]
impl Compiler {
    /// Resolve a type import into a concrete exported symbol.
    pub(crate) fn resolve_import_type_symbol(
        &self,
        module: &Module,
        profile: ProfileId,
        node: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
    ) -> Option<GlobalSymbolId> {
        // reject empty or nested qualifiers for now
        let member_key = self.import_type_member_key(qualifier)?;

        // resolve the module target from the import specifier
        let dir = module.dir(profile);
        let node = node.into_global(module.id);
        let target = self
            .resolve_import(
                module,
                dir,
                profile,
                node,
                DependencySource::ImportStatement,
                target,
            )
            .ok()?;

        // resolve the exported symbol from the target module
        let symbol = self
            .resolve_export_symbol_for_target(
                module.id,
                node,
                target,
                profile,
                SymbolSpaceOrder::TypeOnly,
                member_key,
            )
            .ok()
            .flatten()?;

        // ensure exported types are available for the resolved symbol
        let _ = self.require_analyze_module_export(symbol.module_id, profile);

        Some(symbol)
    }

    /// Resolve a type import into a local reference type.
    pub(crate) fn resolve_import_type_reference(
        &self,
        module: &Module,
        profile: ProfileId,
        source_id: LocalNodeIdAny,
        target: StringId,
        qualifier: Option<&Path>,
        static_arguments: Option<&[StaticArgument]>,
        types: &mut TypeTable,
    ) -> Option<LocalTypeId> {
        let symbol =
            self.resolve_import_type_symbol(module, profile, source_id, target, qualifier)?;
        let static_arguments = static_arguments.map(|arguments| arguments.to_vec());
        let reference = Type::Reference {
            symbol,
            static_arguments,
        };
        Some(types.insert_type_from_any(reference, source_id))
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
