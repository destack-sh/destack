use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::ImportTable {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        for (symbol_id, target) in &self.symbol_targets {
            let anchor = builder.anchor_symbol(symbol_id.into_global(self.module_id));
            let row = match target {
                dir::ImportTarget::Symbol(target) => SnapshotRow::new(anchor, "import", "symbol")
                    .field("symbol", builder.local_symbol_label(*symbol_id))
                    .field("target", builder.symbol_path_label(*target)),
                dir::ImportTarget::Namespace(module_id) => {
                    SnapshotRow::new(anchor, "import", "namespace")
                        .field("symbol", builder.local_symbol_label(*symbol_id))
                        .field("module", builder.module_path(*module_id))
                }
            };
            builder.push(row);
        }

        for (key, symbols) in &self.global_symbols {
            let symbols = symbols
                .iter()
                .map(|symbol| builder.symbol_path_label(*symbol));
            let row = SnapshotRow::new(SnapshotAnchor::End, "import", "global")
                .field("key", builder.static_key(*key))
                .list_field("symbols", symbols);
            builder.push(row);
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "import", "summary")
            .count_field("symbols", self.symbol_targets.len())
            .count_field("globals", self.global_symbols.len());
        builder.push(row);
    }
}
