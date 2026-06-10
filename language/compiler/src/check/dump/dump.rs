use destack_dir as dir;
use destack_source::ModuleId;

use crate::check::{Dump, DumpContext, GenericParameterId, Origin, VariableId, VariableKind};

use super::format::dump_record;

impl<T: Dump> Dump for Option<T> {
    /// Render one optional dump value.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        self.as_ref()
            .map_or_else(|| "none".to_string(), |value| value.dump(context))
    }
}

impl<T: Dump + ?Sized> Dump for Box<T> {
    /// Render one boxed dump value.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        self.as_ref().dump(context)
    }
}

impl Dump for VariableId {
    /// Render one variable id.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        let variable = context.check.variable(*self);
        let label = context.variable_label(*self);
        let source = context.render(&variable.source);
        let kind = match variable.kind {
            VariableKind::Type => "type",
            VariableKind::Static => "static",
        };

        dump_record(
            "Variable",
            [
                ("id", label),
                ("kind", kind.to_string()),
                ("source", source),
            ],
        )
    }
}

impl Dump for Origin {
    /// Render one check origin.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Node(node) => dump_record(
                "Origin.Node",
                [
                    ("value", context.node_label(*node)),
                    ("at", context.node_source_label(*node)),
                ],
            ),
            Self::Symbol(symbol) => dump_record(
                "Origin.Symbol",
                [
                    ("value", context.symbol_label(*symbol)),
                    ("at", context.symbol_source_label(*symbol)),
                ],
            ),
            Self::Type(ty) => dump_record("Origin.Type", [("value", context.type_label(*ty))]),
        }
    }
}

impl Dump for GenericParameterId {
    /// Render one generic parameter id.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "GenericParameter",
            [
                ("id", context.generic_parameter_label(*self)),
                ("module", context.module_label(self.module_id)),
                ("local", self.local_id.0.to_string()),
            ],
        )
    }
}

impl Dump for ModuleId {
    /// Render one module id.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record("Module", [("value", context.module_label(*self))])
    }
}

impl Dump for dir::GlobalNodeIdAny {
    /// Render one global node id.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "GlobalNode",
            [
                ("value", context.node_label(*self)),
                ("at", context.node_source_label(*self)),
            ],
        )
    }
}

impl Dump for dir::GlobalSymbolId {
    /// Render one global symbol id.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "GlobalSymbol",
            [
                ("value", context.symbol_label(*self)),
                ("at", context.symbol_source_label(*self)),
            ],
        )
    }
}
