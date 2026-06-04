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
                    template.parameters.iter().map(|parameter| {
                        generic_template_parameter_label(self.get_parameter(*parameter), builder)
                    }),
                );

            builder.push(row);
        }

        // render generic instance sites
        for (node_id, instance_id) in self.node_instances() {
            let row = SnapshotRow::new(builder.anchor_node(node_id), "generic", "instance")
                .optional_field("source", builder.node_source(node_id))
                .field("id", builder.generic_instance_label(instance_id));

            builder.push(row);
        }

        // render generic instances
        for (instance_id, instance) in self.iter_instances() {
            let template = self.get_template(instance.template);
            let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "instance")
                .field("id", builder.generic_instance_label(instance_id))
                .field("symbol", builder.symbol_path_label(template.owner))
                .list_field(
                    "arguments",
                    instance
                        .arguments
                        .iter()
                        .map(|argument| builder.static_argument_label(argument)),
                );

            builder.push(row);
        }

        let template_count = self.template_count();
        let parameter_count = self.parameter_count();
        let instance_count = self.instance_count();
        let application_site_count = self.node_instance_count();
        if template_count == 0
            && parameter_count == 0
            && instance_count == 0
            && application_site_count == 0
        {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "summary")
            .count_field("templates", template_count)
            .count_field("parameters", parameter_count)
            .count_field("instances", instance_count)
            .count_field("application_sites", application_site_count);
        builder.push(row);
    }
}

/// Return one generic template parameter label.
fn generic_template_parameter_label(
    slot: &dir::GenericParameterBinding,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    match slot {
        dir::GenericParameterBinding::Type {
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
        dir::GenericParameterBinding::VariadicType {
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
        dir::GenericParameterBinding::Static {
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
        dir::GenericParameterBinding::VariadicStatic {
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
fn generic_parameter_name(
    key: dir::GenericParameterKey,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    match key {
        dir::GenericParameterKey::Symbol(symbol) => builder.symbol_label(symbol),
        dir::GenericParameterKey::Generated(_) => builder.generic_parameter_key_label(key),
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
fn generic_origin_label(name: String, origin: dir::GenericParameterOrigin) -> String {
    match origin {
        dir::GenericParameterOrigin::Explicit => name,
        dir::GenericParameterOrigin::Induced(induction) => {
            let induction = generic_parameter_induction_label(induction);

            format!("{name} origin=induced.{induction}")
        }
    }
}

/// Return one induced generic reason label.
fn generic_parameter_induction_label(induction: dir::GenericParameterInduction) -> &'static str {
    match induction {
        dir::GenericParameterInduction::Constraint => "constraint",
        dir::GenericParameterInduction::Form => "form",
        dir::GenericParameterInduction::Comptime => "comptime",
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
