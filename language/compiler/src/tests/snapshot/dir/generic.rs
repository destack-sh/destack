use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render generic templates with their parameter signatures
        for (_, template) in self.iter_templates() {
            let anchor = builder.anchor_node(template.source);
            let row = SnapshotRow::new(anchor, "generic", "template")
                .field("source", builder.node_label(template.source))
                .optional_field(
                    "parent",
                    template
                        .parent
                        .map(|parent| format!("template#{}", parent.0)),
                )
                .list_field(
                    "parameters",
                    template.parameters.iter().map(|parameter| {
                        generic_template_parameter_label(self.get_parameter(*parameter), builder)
                    }),
                );

            builder.push(row);
        }

        let template_count = self.template_count();
        let parameter_count = self.parameter_count();
        if template_count == 0 && parameter_count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "generic", "summary")
            .count_field("templates", template_count)
            .count_field("parameters", parameter_count);
        builder.push(row);
    }
}

/// Return one generic template parameter label.
fn generic_template_parameter_label(
    parameter: &dir::GenericParameterBinding,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    // spell the parameter head with its modifiers
    let mut name = generic_parameter_name(parameter.key, builder);
    if parameter.is_comptime {
        name = format!("comptime {name}");
    }
    if parameter.is_variadic {
        name = format!("...{name}");
    }
    let name = generic_variance_label(parameter.variance, name);

    // spell the constraint and default suffixes
    let constraint = parameter
        .constraint
        .map(|constraint| format!(": {}", builder.global_type_label(constraint)))
        .unwrap_or_default();
    let default = parameter
        .default
        .map(|default| format!(" = {}", builder.global_type_label(default)))
        .unwrap_or_default();

    generic_origin_label(format!("{name}{constraint}{default}"), parameter.origin)
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
        dir::GenericParameterInduction::ParameterConstraint => "parameter_constraint",
        dir::GenericParameterInduction::StorageConstraint => "storage_constraint",
        dir::GenericParameterInduction::Form => "form",
        dir::GenericParameterInduction::Comptime => "comptime",
    }
}
