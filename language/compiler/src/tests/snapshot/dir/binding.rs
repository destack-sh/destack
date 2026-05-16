use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::BindingSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        let node_scopes = self.node_scopes().collect::<Vec<_>>();

        // render scope rows before symbols so the summary remains readable
        for scope_id in self.scope_ids() {
            let scope = self.get_scope_by_id(scope_id);
            let anchor = builder.anchor_scope(self.module_id, scope_id, scope, &node_scopes);
            let row = SnapshotRow::new(anchor, "binding", "scope")
                .field("scope", builder.scope_label(scope_id))
                .field("kind", value::debug_label(scope.kind))
                .optional_field("parent", builder.optional_scope_label(scope.parent))
                .optional_field("owner", builder.optional_local_symbol_label(scope.owner));
            builder.push(row);
        }

        // render source declarations with stable, source-shaped names
        for symbol_id in self.symbol_ids() {
            let symbol = self.get_symbol(symbol_id);
            let global_symbol_id = symbol_id.into_global(self.module_id);
            let mut row =
                SnapshotRow::new(builder.anchor_symbol(global_symbol_id), "binding", "symbol")
                    .field("name", builder.local_symbol_label(symbol_id))
                    .field("role", value::debug_label(symbol.role))
                    .field("form", value::debug_label(symbol.form))
                    .field("scope", builder.scope_cursor_label(symbol.scope));

            if let Some(mutability) = symbol.binding_mutability {
                row = row.field("mutability", value::debug_label(mutability));
            }

            if symbol.origin != dir::SymbolOrigin::Module {
                row = row.field("origin", value::debug_label(symbol.origin));
            }

            if let Some(export_kind) = symbol.export_kind {
                row = row.field("export", value::debug_label(export_kind));
            }

            builder.push(row);
        }

        // render replacement segments when a patch masks old bindings
        for (symbol_id, symbol) in self.replaced_symbols() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "binding", "replaced_symbol")
                .field("name", value::local_symbol_label(symbol_id))
                .field("role", value::debug_label(symbol.role))
                .field("form", value::debug_label(symbol.form));
            builder.push(row);
        }

        // render exact node cursors only for targeted binding tests
        if builder.binding_nodes {
            for (node_id, scope) in node_scopes {
                let row = SnapshotRow::new(builder.anchor_node(node_id), "binding", "node")
                    .field("node", builder.node_label(node_id))
                    .field("scope", builder.scope_cursor_label(scope))
                    .optional_field("source", builder.node_source(node_id));
                builder.push(row);
            }
        }

        // summarize the dense table shape without dumping every node scope
        let row = SnapshotRow::new(SnapshotAnchor::End, "binding", "summary")
            .field("symbols", self.symbol_count().to_string())
            .field("scopes", self.scope_count().to_string())
            .field(
                "declarations",
                self.declaration_symbols().count().to_string(),
            )
            .field("node_scopes", self.node_scopes().count().to_string())
            .field(
                "replaced_symbols",
                self.replaced_symbols().count().to_string(),
            )
            .field(
                "replaced_scopes",
                self.replaced_scopes().count().to_string(),
            );
        builder.push(row);
    }
}
