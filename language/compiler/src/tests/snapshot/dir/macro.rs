use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable, value};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::MacroTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for invocation in self.iter() {
            let row = SnapshotRow::new(
                builder.anchor_node(invocation.target_node),
                "macro",
                "invocation",
            )
            .field("target", builder.node_label(invocation.target_node))
            .field(
                "trigger",
                value::macro_trigger_label(builder, &invocation.trigger),
            )
            .field(
                "implementation",
                builder.symbol_label(invocation.implementation),
            )
            .field(
                "state",
                if invocation.state.is_some() {
                    "yes"
                } else {
                    "no"
                },
            );
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "macro", "summary")
            .field("invocations", self.invocations.len().to_string());
        builder.push(row);
    }
}
