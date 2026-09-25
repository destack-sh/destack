use tspp_dir as dir;

use super::resolution::add_access_path_fields;
use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::FlowTable<'_> {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render the nodes flow proves unreachable
        for node in self.unreachable_nodes() {
            let node = node.into_global(self.module_id);
            let row = SnapshotRow::new(builder.anchor_node(node), "flow", "unreachable")
                .optional_field("source", builder.node_source(node));

            builder.push(row);
        }

        // render the nodes flow proves never return
        for node in self.diverging_nodes() {
            let node = node.into_global(self.module_id);
            let row = SnapshotRow::new(builder.anchor_node(node), "flow", "diverging")
                .optional_field("source", builder.node_source(node));

            builder.push(row);
        }

        // render the loops flow proves run their body at most once
        for node in self.single_pass_nodes() {
            let node = node.into_global(self.module_id);
            let row = SnapshotRow::new(builder.anchor_node(node), "flow", "single_pass")
                .optional_field("source", builder.node_source(node));

            builder.push(row);
        }

        // render the recorded symbol uses
        for (symbol, binding_use) in self.binding_uses() {
            let symbol = dir::GlobalSymbolId {
                module_id: self.module_id,
                local_id: symbol,
            };
            let row = SnapshotRow::new(builder.anchor_symbol(symbol), "flow", "use")
                .field("symbol", builder.symbol_label(symbol))
                .verbatim_field("uses", binding_use_label(binding_use));

            builder.push(row);
        }

        // render the recorded stable access uses
        for occurrence in self.access_occurrences() {
            let node = occurrence.node.into_global(self.module_id);
            let row = SnapshotRow::new(builder.anchor_node(node), "flow", "access")
                .optional_field("source", builder.node_source(node));
            let row = add_access_path_fields(builder, row, &occurrence.path)
                .verbatim_field("uses", binding_use_label(occurrence.uses));

            builder.push(row);
        }

        // render the recorded foreign symbol uses
        for (symbol, binding_use) in self.foreign_uses() {
            let row = SnapshotRow::new(SnapshotAnchor::End, "flow", "foreign")
                .field("symbol", builder.symbol_label(symbol))
                .verbatim_field("uses", binding_use_label(binding_use));

            builder.push(row);
        }
    }
}

/// Return one binding use label.
fn binding_use_label(binding_use: dir::BindingUse) -> String {
    let mut labels = Vec::new();
    if binding_use.contains(dir::BindingUse::READ) {
        labels.push("read");
    }
    if binding_use.contains(dir::BindingUse::WRITE) {
        labels.push("written");
    }
    if binding_use.contains(dir::BindingUse::CAPTURE) {
        labels.push("captured");
    }
    if binding_use.contains(dir::BindingUse::MUTATE) {
        labels.push("mutated");
    }
    if binding_use.contains(dir::BindingUse::MUTABLE) {
        labels.push("mutable");
    }
    if binding_use.contains(dir::BindingUse::MOVE) {
        labels.push("moved");
    }

    labels.join("+")
}
