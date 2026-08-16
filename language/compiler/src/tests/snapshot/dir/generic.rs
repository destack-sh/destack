use std::collections::BTreeSet;

use destack_dir as dir;
use smallvec::SmallVec;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

/// One applied generic declaration discovered from checked DIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GenericInstanceSnapshot {
    /// The declaration template label.
    pub(super) template: Option<String>,
    /// The complete argument tuple label.
    pub(super) arguments: String,
    /// The rows this instance derives from.
    pub(super) owners: BTreeSet<usize>,
    /// Whether the instance derives from a rowless call and never retires.
    pub(super) is_rowless: bool,
}

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

        // render the instances this segment closed
        for (_, instance) in self.iter_instances() {
            let arguments = instance_arguments(instance);
            let row = SnapshotRow::new(builder.anchor_node(instance.source), "instance", "closed")
                .field("id", instance_label(builder, instance))
                .field(
                    "template",
                    builder.symbol_path_label(instance.selection.symbol),
                )
                .verbatim_field(
                    "arguments",
                    builder.generic_instance_arguments_label(&arguments),
                );

            builder.push(row);
        }

        // render the types materialized under those instances
        for (instance_id, source, resolved) in self.iter_instance_types() {
            let instance = self.get_local_instance(instance_id).unwrap_or_else(|| {
                panic!("generic snapshot is missing the instance {instance_id:?}")
            });
            let row = SnapshotRow::new(builder.anchor_node(instance.source), "instance", "type")
                .field("instance", instance_label(builder, instance))
                .type_field("source", builder.global_type_label(source))
                .type_field("type", builder.global_type_label(resolved));

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
    builder.generic_instance_label(instance.selection.symbol, &instance_arguments(instance))
}

/// Return the selected argument types of one closed generic instance.
fn instance_arguments(instance: &dir::Instance) -> Vec<dir::GlobalTypeId> {
    instance
        .selection
        .arguments
        .iter()
        .map(|argument| argument.argument)
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
    /// Add generic instance rows from one type.
    pub(crate) fn add_generic_instances_in_type(
        &mut self,
        anchor: SnapshotAnchor,
        source: Option<String>,
        type_id: dir::GlobalTypeId,
    ) {
        let mut visited = BTreeSet::new();

        self.add_generic_instances_in_type_depth(anchor, source.as_deref(), type_id, &mut visited);
    }

    /// Add one selected generic instance.
    pub(crate) fn add_generic_instance(
        &mut self,
        anchor: SnapshotAnchor,
        source: Option<String>,
        symbol: dir::GlobalSymbolId,
        arguments: &[dir::GlobalTypeId],
    ) {
        if arguments.is_empty() {
            return;
        }

        let template = self.symbol_path_label(symbol);
        let id = self.generic_instance_label(symbol, arguments);
        let arguments = self.generic_instance_arguments_label(arguments);

        self.add_generic_instance_row(anchor, source.as_deref(), &id, Some(&template), &arguments);
    }

    /// Add end-of-snapshot rows for discovered generic instances.
    pub(crate) fn add_generic_instance_index_rows(&mut self) {
        let rows = self
            .generic_instances
            .iter()
            .filter(|(_, instance)| instance.is_rowless || !instance.owners.is_empty())
            .map(|(id, instance)| {
                SnapshotRow::new(SnapshotAnchor::End, "generic", "instance")
                    .field("id", id)
                    .optional_field("template", instance.template.clone())
                    .verbatim_field("arguments", instance.arguments.clone())
            })
            .collect::<Vec<_>>();

        for row in rows {
            self.push(row);
        }
    }

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

    /// Return whether one parameter spells as its bare tick name.
    pub(super) fn is_tick_parameter(
        &self,
        parameter: &dir::GenericParameterBinding,
        label: &str,
    ) -> bool {
        parameter.memory_parameter() == Some(dir::MemoryParameter::Lifetime)
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

    /// Add generic instances from one type and its children.
    fn add_generic_instances_in_type_depth(
        &mut self,
        anchor: SnapshotAnchor,
        source: Option<&str>,
        type_id: dir::GlobalTypeId,
        visited: &mut BTreeSet<dir::GlobalTypeId>,
    ) {
        if !visited.insert(type_id) {
            return;
        }

        let types = if type_id.module_id == self.tree.module_id {
            self.types.as_ref()
        } else {
            self.foreign_types.get(&type_id.module_id)
        };
        let Some(types) = types else {
            return;
        };
        let Some(ty) = types.get_type_maybe(type_id.local_id) else {
            return;
        };

        // collect the instance arguments and child type ids before recursing,
        //  since the recursion needs a mutable borrow of self
        let instance = match ty {
            dir::Type::Application(instance) => {
                let arguments: SmallVec<[dir::GlobalTypeId; 8]> =
                    SmallVec::from_slice(types.type_ids(instance.arguments));

                Some((instance.symbol, arguments))
            }
            _ => None,
        };
        let mut children = SmallVec::<[dir::GlobalTypeId; 8]>::new();
        types.for_each_child(&ty, |child| children.push(child));

        if let Some((symbol, arguments)) = instance {
            self.add_generic_instance(anchor, source.map(str::to_string), symbol, &arguments);
        }

        for child in children {
            self.add_generic_instances_in_type_depth(anchor, source, child, visited);
        }
    }

    /// Add one generic instance source and index row.
    pub(crate) fn add_generic_instance_row(
        &mut self,
        anchor: SnapshotAnchor,
        source: Option<&str>,
        id: &str,
        template: Option<&str>,
        arguments: &str,
    ) {
        // attribute the instance to the row it derives from, so a later
        //  layer shadowing that row also retires the instance
        let owner = self.last_row;
        let instance = self
            .generic_instances
            .entry(id.to_string())
            .or_insert_with(|| GenericInstanceSnapshot {
                template: None,
                arguments: String::new(),
                owners: BTreeSet::new(),
                is_rowless: false,
            });
        instance.template = template.map(str::to_string);
        instance.arguments = arguments.to_string();
        match owner {
            Some(row) => {
                instance.owners.insert(row);
            }
            None => instance.is_rowless = true,
        }

        let Some(source) = source else {
            return;
        };
        let key = (anchor.sort_key(), source.to_string(), id.to_string());
        if !self.generic_instance_sources.insert(key) {
            return;
        }

        let row = SnapshotRow::new(anchor, "generic", "instance")
            .field("source", source)
            .field("id", id);
        self.push(row);
    }
}
