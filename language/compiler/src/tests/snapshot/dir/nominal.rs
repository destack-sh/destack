use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::NominalSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render nominal definitions in declaration order
        for (symbol, definition) in self.iter_definitions() {
            add_nominal_definition_rows(builder, self, symbol, definition);
        }

        let count = self.iter_definitions().count();
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "nominal", "summary")
            .count_field("definitions", count);
        builder.push(row);
    }
}

/// Add rows for one nominal definition.
fn add_nominal_definition_rows(
    builder: &mut DirSnapshotBuilder<'_>,
    segment: &dir::NominalSegment,
    symbol: dir::GlobalSymbolId,
    definition: &dir::NominalDefinition,
) {
    let source = segment.definition_source(symbol);

    match definition {
        dir::NominalDefinition::Struct(definition) => {
            add_declaration_row(builder, symbol, "struct", source, definition.template);
            add_heritage(builder, symbol, "implements", &definition.implements);
            add_members(
                builder,
                symbol,
                &definition.fields,
                &definition.static_fields,
                &definition.methods,
                &definition.static_methods,
            );
            add_associated(
                builder,
                symbol,
                &definition.associated_types,
                &definition.associated_consts,
            );
        }
        dir::NominalDefinition::Class(definition) => {
            add_declaration_row(builder, symbol, "class", source, definition.template);
            add_optional_heritage(builder, symbol, "extends", definition.extends.as_ref());
            add_heritage(builder, symbol, "implements", &definition.implements);
            add_members(
                builder,
                symbol,
                &definition.fields,
                &definition.static_fields,
                &definition.methods,
                &definition.static_methods,
            );
            add_associated(
                builder,
                symbol,
                &definition.associated_types,
                &definition.associated_consts,
            );
        }
        dir::NominalDefinition::Interface(definition) => {
            add_declaration_row(builder, symbol, "interface", source, definition.template);
            add_heritage(builder, symbol, "extends", &definition.extends);
            add_members(
                builder,
                symbol,
                &definition.fields,
                &definition.static_fields,
                &definition.methods,
                &definition.static_methods,
            );
            add_signatures(builder, symbol, "call", &definition.call_signatures);
            add_signatures(
                builder,
                symbol,
                "construct",
                &definition.construct_signatures,
            );
            add_signatures(builder, symbol, "index", &definition.index_signatures);
            add_associated(
                builder,
                symbol,
                &definition.associated_types,
                &definition.associated_consts,
            );
        }
        dir::NominalDefinition::Enum(definition) => {
            add_declaration_row(builder, symbol, "enum", source, definition.template);
            add_heritage(builder, symbol, "implements", &definition.implements);
            add_variants(builder, symbol, &definition.variants);
            add_members(
                builder,
                symbol,
                &[],
                &definition.static_fields,
                &definition.methods,
                &definition.static_methods,
            );
            add_associated(
                builder,
                symbol,
                &definition.associated_types,
                &definition.associated_consts,
            );
        }
        dir::NominalDefinition::Newtype(definition) => {
            add_declaration_row(builder, symbol, "newtype", source, definition.template);
        }
    }
}

/// Add one nominal declaration row.
fn add_declaration_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    kind: &'static str,
    source: dir::GlobalNodeIdAny,
    template: Option<dir::LocalGenericTemplateId>,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(symbol), "nominal", kind)
        .field("symbol", builder.symbol_path_label(symbol))
        .optional_field("source", builder.node_source(source))
        .optional_field("template", template.map(|template| format!("{template:?}")));

    builder.push(row);
}

/// Add one optional nominal heritage row.
fn add_optional_heritage(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    relation: &'static str,
    heritage: Option<&dir::NominalHeritage>,
) {
    if let Some(heritage) = heritage {
        add_one_heritage(builder, owner, relation, heritage);
    }
}

/// Add nominal heritage rows.
fn add_heritage(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    relation: &'static str,
    heritages: &[dir::NominalHeritage],
) {
    for heritage in heritages {
        add_one_heritage(builder, owner, relation, heritage);
    }
}

/// Add one nominal heritage row.
fn add_one_heritage(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    relation: &'static str,
    heritage: &dir::NominalHeritage,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", relation)
        .field("symbol", builder.symbol_path_label(owner))
        .optional_field("source", builder.node_source(heritage.source))
        .field("target", builder.symbol_path_label(heritage.symbol))
        .optional_field(
            "instance",
            heritage
                .instance
                .map(|instance| builder.generic_instance_label(instance)),
        );

    builder.push(row);
}

/// Add field and method rows.
fn add_members(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    fields: &[dir::FieldDefinition],
    static_fields: &[dir::FieldDefinition],
    methods: &[dir::MethodDefinition],
    static_methods: &[dir::MethodDefinition],
) {
    for field in fields {
        add_field(builder, owner, field, false);
    }

    for field in static_fields {
        add_field(builder, owner, field, true);
    }

    for method in methods {
        add_method(builder, owner, method, false);
    }

    for method in static_methods {
        add_method(builder, owner, method, true);
    }
}

/// Add one field row.
fn add_field(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    field: &dir::FieldDefinition,
    is_static: bool,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", "field")
        .field("symbol", builder.symbol_path_label(field.symbol))
        .optional_field("source", builder.node_source(field.source))
        .field("key", builder.static_key(field.key))
        .optional_field("static", is_static.then(|| "true".to_string()))
        .type_field("type", builder.global_type_label(field.ty));

    builder.push(row);
}

/// Add one method row.
fn add_method(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    method: &dir::MethodDefinition,
    is_static: bool,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", "method")
        .optional_field(
            "symbol",
            method
                .symbol
                .map(|symbol| builder.symbol_path_label(symbol)),
        )
        .optional_field("source", builder.node_source(method.source))
        .field("slot", member_slot_label(method.slot, builder))
        .optional_field("static", is_static.then(|| "true".to_string()))
        .type_field("type", builder.global_type_label(method.ty));

    builder.push(row);
}

/// Add symbol-free signature rows.
fn add_signatures(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    kind: &'static str,
    signatures: &[dir::SignatureDefinition],
) {
    for signature in signatures {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", "signature")
            .field("kind", kind)
            .optional_field("source", builder.node_source(signature.source))
            .type_field("type", builder.global_type_label(signature.ty));

        builder.push(row);
    }
}

/// Add associated member rows.
fn add_associated(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    types: &[dir::AssociatedTypeDefinition],
    consts: &[dir::AssociatedConstDefinition],
) {
    for ty in types {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", "associated.type")
            .field("symbol", builder.symbol_path_label(ty.symbol))
            .optional_field("source", builder.node_source(ty.source))
            .optional_field(
                "constraint",
                ty.constraint
                    .map(|constraint| builder.global_type_label(constraint)),
            )
            .optional_field(
                "value",
                ty.value.map(|value| builder.global_type_label(value)),
            );

        builder.push(row);
    }

    for value in consts {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", "associated.const")
            .field("symbol", builder.symbol_path_label(value.symbol))
            .optional_field("source", builder.node_source(value.source))
            .type_field("type", builder.global_type_label(value.ty))
            .optional_field(
                "value",
                value.value.map(|value| builder.global_static_label(value)),
            );

        builder.push(row);
    }
}

/// Add enum variant rows.
fn add_variants(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    variants: &[dir::VariantDefinition],
) {
    for variant in variants {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "nominal", "variant")
            .field("symbol", builder.symbol_path_label(variant.symbol))
            .optional_field("source", builder.node_source(variant.source))
            .field("key", builder.static_key(variant.key))
            .optional_field(
                "value",
                variant
                    .value
                    .map(|value| builder.global_static_label(value)),
            );

        builder.push(row);
    }
}

/// Return one member slot label.
fn member_slot_label(slot: dir::MemberSlot, builder: &DirSnapshotBuilder<'_>) -> String {
    match slot {
        dir::MemberSlot::Key(key) => builder.static_key(key),
        dir::MemberSlot::Constructor => "constructor".to_string(),
        dir::MemberSlot::New => "new".to_string(),
        dir::MemberSlot::Call => "call".to_string(),
    }
}
