use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render generic templates
        for (_, template) in self.iter_templates() {
            let anchor = builder.anchor_symbol(template.owner);
            let row = SnapshotRow::new(anchor, "generic", "template")
                .field("symbol", builder.symbol_path_label(template.owner))
                .list_field(
                    "parameters",
                    template.slots.iter().map(|slot| {
                        generic_template_parameter_label(self.get_slot(*slot), builder)
                    }),
                );

            builder.push(row);
        }

        // render generic application sites
        for (node_id, application_id) in self.node_applications() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "generic", "application")
                .optional_field("source", builder.node_source(node_id))
                .field("id", builder.generic_application_label(application_id));

            builder.push(row);
        }

        // render generic applications
        for (application_id, application) in self.iter_applications() {
            let template = self.get_template(application.template);
            let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "application")
                .field("id", builder.generic_application_label(application_id))
                .field("symbol", builder.symbol_path_label(template.owner))
                .list_field(
                    "arguments",
                    application
                        .arguments
                        .iter()
                        .map(|argument| builder.static_argument_label(argument)),
                );

            builder.push(row);
        }

        let template_count = self.template_count();
        let slot_count = self.slot_count();
        let application_count = self.application_count();
        let application_site_count = self.node_application_count();
        if template_count == 0
            && slot_count == 0
            && application_count == 0
            && application_site_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "summary")
            .count_field("templates", template_count)
            .count_field("slots", slot_count)
            .count_field("applications", application_count)
            .count_field("application_sites", application_site_count);
        builder.push(row);
    }
}

/// Return one generic template parameter label.
fn generic_template_parameter_label(
    slot: &dir::GenericSlot,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    match slot {
        dir::GenericSlot::Type {
            key,
            variance,
            constraint,
            default,
            origin,
            ..
        } => {
            let name = generic_parameter_name(*key, builder);
            let name = generic_variance_label(*variance, name);
            let label = generic_type_parameter_label(name, *constraint, *default, builder);

            generic_origin_label(label, *origin)
        }
        dir::GenericSlot::VariadicType {
            key,
            variance,
            constraint,
            default,
            origin,
            ..
        } => {
            let name = format!("...{}", generic_parameter_name(*key, builder));
            let name = generic_variance_label(*variance, name);
            let label = generic_type_parameter_label(name, *constraint, *default, builder);

            generic_origin_label(label, *origin)
        }
        dir::GenericSlot::Static {
            key,
            constraint,
            default,
            origin,
            ..
        } => {
            let name = format!("comptime {}", generic_parameter_name(*key, builder));
            let label = generic_static_parameter_label(name, *constraint, *default, builder);

            generic_origin_label(label, *origin)
        }
        dir::GenericSlot::VariadicStatic {
            key,
            constraint,
            default,
            origin,
            ..
        } => {
            let name = format!("...comptime {}", generic_parameter_name(*key, builder));
            let label = generic_static_parameter_label(name, *constraint, *default, builder);

            generic_origin_label(label, *origin)
        }
    }
}

/// Return one generic parameter source name.
fn generic_parameter_name(key: dir::GenericSlotKey, builder: &DirSnapshotBuilder<'_>) -> String {
    match key {
        dir::GenericSlotKey::Symbol(symbol) => builder.symbol_label(symbol),
        dir::GenericSlotKey::Generated(_) => builder.generic_slot_key_label(key),
    }
}

/// Add one optional variance prefix.
fn generic_variance_label(variance: Option<dir::VarianceModifier>, name: String) -> String {
    if let Some(variance) = variance {
        return format!("{} {name}", variance.as_str());
    }

    name
}

/// Add one induced origin suffix.
fn generic_origin_label(name: String, origin: dir::GenericSlotOrigin) -> String {
    match origin {
        dir::GenericSlotOrigin::Explicit => name,
        dir::GenericSlotOrigin::Induced(induction) => {
            let induction = generic_slot_induction_label(induction);

            format!("{name} origin=induced.{induction}")
        }
    }
}

/// Return one induced generic reason label.
fn generic_slot_induction_label(induction: dir::GenericSlotInduction) -> &'static str {
    match induction {
        dir::GenericSlotInduction::Application => "application",
        dir::GenericSlotInduction::Constraint => "constraint",
        dir::GenericSlotInduction::Form => "form",
        dir::GenericSlotInduction::Comptime => "comptime",
    }
}

/// Return one type generic parameter label.
fn generic_type_parameter_label(
    name: String,
    constraint: Option<dir::GlobalTypeId>,
    default: Option<dir::GlobalTypeId>,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    let constraint = constraint
        .map(|constraint| format!(": {}", builder.global_type_label(constraint)))
        .unwrap_or_default();
    let default = default
        .map(|default| format!(" = {}", builder.global_type_label(default)))
        .unwrap_or_default();

    format!("{name}{constraint}{default}")
}

/// Return one static generic parameter label.
fn generic_static_parameter_label(
    name: String,
    constraint: Option<dir::GlobalTypeId>,
    default: Option<dir::GlobalStaticId>,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    let constraint = constraint
        .map(|constraint| format!(": {}", builder.global_type_label(constraint)))
        .unwrap_or_default();
    let default = default
        .map(|default| format!(" = {}", builder.global_static_label(default)))
        .unwrap_or_default();

    format!("{name}{constraint}{default}")
}
