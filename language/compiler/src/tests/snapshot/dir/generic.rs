use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render generic templates with their parameter signatures
        for (_, template) in self.iter_templates() {
            let anchor = builder.anchor_node(template.source);
            let row = SnapshotRow::new(anchor, "generic", "template")
                .optional_field(
                    "symbol",
                    template
                        .symbol
                        .map(|symbol| builder.symbol_path_label(symbol)),
                )
                .optional_field(
                    "source",
                    template
                        .symbol
                        .is_none()
                        .then(|| builder.node_label(template.source)),
                )
                .optional_field(
                    "parent",
                    template
                        .parent
                        .map(|parent| format!("template#{}", parent.0)),
                )
                .tuple_field(
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
    let name = generic_parameter_name(parameter.key, builder);
    let name = builder.generic_parameter_binding_head_label(parameter, name);

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

impl DirSnapshotBuilder<'_> {
    /// Add generic parameter modifiers to one label.
    pub(super) fn generic_parameter_binding_head_label(
        &self,
        parameter: &dir::GenericParameterBinding,
        mut label: String,
    ) -> String {
        if parameter.is_variadic {
            label = format!("...{label}");
        }
        if parameter.is_const {
            label = format!("const {label}");
        }
        if parameter.is_comptime {
            label = format!("comptime {label}");
        }
        if let Some(variance) = parameter.variance {
            label = format!("{} {label}", variance.as_str());
        }

        label
    }
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
