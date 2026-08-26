use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::GenericSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render generic templates with their parameter signatures,
        //  keeping only the ones that declare parameters
        for (_, template) in self.iter_templates() {
            if template.parameters.is_empty() {
                continue;
            }

            let bindings = builder
                .bindings
                .unwrap_or_else(|| panic!("generic snapshot is missing its binding table"));
            let parent = bindings
                .scope_ancestors(template.scope)
                .find_map(|scope| self.template_by_scope(scope.id));
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
                    parent.map(|parent| format!("template#{}", parent.0)),
                )
                .tuple_field(
                    "parameters",
                    template.parameters.iter().map(|parameter| {
                        generic_template_parameter_label(
                            *parameter,
                            self.get_parameter(*parameter),
                            builder,
                        )
                    }),
                );

            builder.push(row);
        }

        // render the instantiations the checked bodies recorded
        for instantiation in self.iter_instantiations() {
            let arguments = selection_arguments(&instantiation.key);
            let row = SnapshotRow::new(
                builder.anchor_node(instantiation.source),
                "generic",
                "instantiation",
            )
            .field(
                "id",
                builder.generic_instance_label(
                    instantiation.key.symbol,
                    &selection_types(&instantiation.key),
                ),
            )
            .field(
                "template",
                builder.symbol_path_label(instantiation.key.symbol),
            )
            .verbatim_field(
                "arguments",
                builder.generic_instance_arguments_label(&arguments),
            )
            .optional_field(
                "owner",
                instantiation
                    .owner
                    .map(|owner| builder.symbol_path_label(owner)),
            );

            builder.push(row);
        }

        // render the instances this segment closed, with the types they evaluated
        for (instance_id, instance) in self.iter_instances() {
            let arguments = selection_arguments(&instance.key);
            let mut row =
                SnapshotRow::new(builder.anchor_node(instance.source), "generic", "instance")
                    .field("id", instance_label(builder, instance))
                    .field("template", builder.symbol_path_label(instance.key.symbol))
                    .verbatim_field(
                        "arguments",
                        builder.generic_instance_arguments_label(&arguments),
                    );

            // fold the types evaluation moved beyond substitution into the row
            let evaluated = self
                .iter_instance_types()
                .filter(|(id, _, _, is_evaluated)| *id == instance_id && *is_evaluated)
                .map(|(_, source, resolved, _)| {
                    format!(
                        "{} => {}",
                        builder.global_type_label(source),
                        builder.global_type_label(resolved)
                    )
                })
                .collect::<Vec<_>>();
            if !evaluated.is_empty() {
                row = row.verbatim_field("evaluated", format!("({})", evaluated.join(", ")));
            }

            builder.push(row);
        }

        // summarize the segment, skipping segments that declare nothing
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

/// Return one closed generic instance label.
fn instance_label(builder: &DirSnapshotBuilder<'_>, instance: &dir::Instance) -> String {
    builder.generic_instance_label(instance.key.symbol, &selection_types(&instance.key))
}

/// Return the argument types one key binds.
fn selection_arguments(key: &dir::InstanceKey) -> Vec<dir::GlobalTypeId> {
    key.arguments
        .iter()
        .map(|argument| argument.argument)
        .collect()
}

/// Return the receiver and argument types one key closes, receiver first.
fn selection_types(key: &dir::InstanceKey) -> Vec<dir::GlobalTypeId> {
    key.receiver
        .into_iter()
        .chain(key.arguments.iter().map(|argument| argument.argument))
        .collect()
}

/// Return one generic template parameter label.
pub(super) fn generic_template_parameter_label(
    parameter_id: dir::LocalGenericParameterId,
    parameter: &dir::GenericParameterBinding,
    builder: &DirSnapshotBuilder<'_>,
) -> String {
    // print the parameter head with its modifiers
    let name = generic_parameter_name(parameter.key, builder);

    // print tick parameters bare, their kind is implied
    if builder.is_tick_parameter(parameter, &name) {
        return name;
    }
    let mut name = builder.generic_parameter_binding_head_label(parameter, name);

    // unannotated parameters show the variance check derived
    if parameter.variance.is_none()
        && let Some(derived) = builder
            .generics
            .as_ref()
            .and_then(|generics| generics.variance(parameter_id))
    {
        name = format!("{} {name}", derived.as_str());
    }

    // print the constraint and default suffixes
    let constraint = parameter
        .constraint
        .map(|constraint| format!(": {}", builder.global_type_label(constraint)))
        .unwrap_or_default();
    let default = parameter
        .default
        .map(|default| format!(" = {}", builder.global_type_label(default)))
        .unwrap_or_default();

    format!("{name}{constraint}{default}")
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
    /// Render one applied generic declaration label.
    pub(crate) fn generic_instance_label(
        &self,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) -> String {
        let symbol = self.reference_symbol_label(symbol);
        let arguments = arguments
            .iter()
            .map(|argument| self.global_type_label(*argument))
            .collect::<Vec<_>>()
            .join(", ");

        format!("{symbol}<{arguments}>")
    }

    /// Render one applied generic argument tuple label.
    pub(crate) fn generic_instance_arguments_label(
        &self,
        arguments: &[dir::GlobalTypeId],
    ) -> String {
        let arguments = arguments
            .iter()
            .map(|argument| self.global_type_label(*argument))
            .collect::<Vec<_>>()
            .join(", ");

        format!("({arguments})")
    }

    /// Return whether one parameter prints as its bare tick name.
    pub(super) fn is_tick_parameter(
        &self,
        parameter: &dir::GenericParameterBinding,
        label: &str,
    ) -> bool {
        parameter.memory_parameter() == Some(dir::MemoryParameter::Region)
            && label
                .rsplit('.')
                .next()
                .is_some_and(|name| name.starts_with('\''))
    }

    /// Add generic parameter modifiers to one label.
    pub(super) fn generic_parameter_binding_head_label(
        &self,
        parameter: &dir::GenericParameterBinding,
        mut label: String,
    ) -> String {
        if parameter.is_variadic {
            label = format!("...{label}");
        }
        if let Some(variance) = parameter.variance {
            label = format!("{} {label}", variance.as_str());
        }
        if parameter.is_const {
            label = format!("const {label}");
        }

        label
    }
}
