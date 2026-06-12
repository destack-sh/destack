use destack_dir as dir;

use super::{DirSnapshotBuilder, SnapshotTable};
use crate::tests::snapshot::{SnapshotAnchor, SnapshotRow};

impl SnapshotTable for dir::DefinitionSegment {
    fn add_snapshot_rows(&self, builder: &mut DirSnapshotBuilder<'_>) {
        // render definitions in declaration order
        for (symbol, definition) in self.iter_definitions() {
            add_definition_rows(builder, self, symbol, definition);
        }

        let count = self.iter_definitions().count();
        if count == 0 {
            return;
        }

        let row = SnapshotRow::new(SnapshotAnchor::End, "definition", "summary")
            .count_field("definitions", count);
        builder.push(row);
    }
}

/// Add rows for one definition.
fn add_definition_rows(
    builder: &mut DirSnapshotBuilder<'_>,
    segment: &dir::DefinitionSegment,
    symbol: dir::GlobalSymbolId,
    definition: &dir::Definition,
) {
    let source = segment.definition_source(symbol);

    match definition {
        dir::Definition::TypeAlias(definition) => {
            add_type_alias_row(builder, symbol, source, definition);
        }
        dir::Definition::Struct(definition) => {
            add_declaration_row(builder, symbol, "struct", source, definition.template);
            add_heritage(builder, symbol, "implements", &definition.implements);
            add_members(builder, symbol, &definition.members);
        }
        dir::Definition::Class(definition) => {
            let row = declaration_row(builder, symbol, "class", source, definition.template)
                .optional_field(
                    "abstract",
                    definition.is_abstract.then(|| "true".to_string()),
                )
                .optional_field("final", definition.is_final.then(|| "true".to_string()));
            builder.push(row);
            add_optional_heritage(builder, symbol, "extends", definition.extends.as_ref());
            add_heritage(builder, symbol, "implements", &definition.implements);
            add_members(builder, symbol, &definition.members);
        }
        dir::Definition::Interface(definition) => {
            let row = declaration_row(builder, symbol, "interface", source, definition.template)
                .optional_field("nominal", definition.is_nominal.then(|| "true".to_string()));
            builder.push(row);
            add_heritage(builder, symbol, "extends", &definition.extends);
            add_members(builder, symbol, &definition.members);
        }
        dir::Definition::Enum(definition) => {
            add_declaration_row(builder, symbol, "enum", source, definition.template);
            add_heritage(builder, symbol, "implements", &definition.implements);
            add_members(builder, symbol, &definition.members);
        }
        dir::Definition::Newtype(definition) => {
            add_newtype_row(builder, symbol, source, definition);
        }
        dir::Definition::Extension(extension) => {
            add_extension_row(builder, symbol, source, extension);
            add_heritage(builder, symbol, "implements", &extension.implements);
            add_extension_where_clauses(builder, symbol, &extension.where_clauses);
            add_members(builder, symbol, &extension.members);
        }
    }
}

/// Add one type alias declaration row.
fn add_type_alias_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    source: dir::GlobalNodeIdAny,
    definition: &dir::TypeAliasDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(symbol), "definition", "type")
        .field("symbol", builder.symbol_path_label(symbol))
        .optional_field("source", builder.node_source(source))
        .optional_field(
            "template",
            definition.template.map(|template| format!("{template:?}")),
        )
        .type_field("value", builder.global_type_label(definition.value));

    builder.push(row);
}

/// Add one newtype declaration row.
fn add_newtype_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    source: dir::GlobalNodeIdAny,
    definition: &dir::NewtypeDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(symbol), "definition", "newtype")
        .field("symbol", builder.symbol_path_label(symbol))
        .optional_field("source", builder.node_source(source))
        .optional_field(
            "template",
            definition.template.map(|template| format!("{template:?}")),
        )
        .type_field("value", builder.global_type_label(definition.value));

    builder.push(row);
}

/// Build one definition declaration row.
fn declaration_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    kind: &'static str,
    source: dir::GlobalNodeIdAny,
    template: Option<dir::LocalGenericTemplateId>,
) -> SnapshotRow {
    SnapshotRow::new(builder.anchor_symbol(symbol), "definition", kind)
        .field("symbol", builder.symbol_path_label(symbol))
        .optional_field("source", builder.node_source(source))
        .optional_field("template", template.map(|template| format!("{template:?}")))
}

/// Add one definition declaration row.
fn add_declaration_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    kind: &'static str,
    source: dir::GlobalNodeIdAny,
    template: Option<dir::LocalGenericTemplateId>,
) {
    let row = declaration_row(builder, symbol, kind, source, template);

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
    let arguments = (!heritage.arguments.is_empty()).then(|| {
        heritage
            .arguments
            .iter()
            .map(|argument| builder.global_type_label(*argument))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", relation)
        .field("symbol", builder.symbol_path_label(owner))
        .optional_field("source", builder.node_source(heritage.source))
        .field("target", builder.symbol_path_label(heritage.symbol))
        .optional_field("arguments", arguments);

    builder.push(row);
}

/// Add one extension definition row.
fn add_extension_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    source: dir::GlobalNodeIdAny,
    extension: &dir::Extension,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(symbol), "definition", "extension")
        .field("symbol", builder.symbol_path_label(extension.symbol))
        .optional_field("source", builder.node_source(source))
        .field("form", DirSnapshotBuilder::variant_label(extension.form))
        .type_field(
            "target",
            builder.global_type_label(extension.target.r#type()),
        );

    builder.push(row);
}

/// Add extension where clause rows.
fn add_extension_where_clauses(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    where_clauses: &[dir::ExtensionWhereClause],
) {
    for where_clause in where_clauses {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "where")
            .field("symbol", builder.symbol_path_label(owner))
            .optional_field("source", builder.node_source(where_clause.source))
            .type_field("left", builder.global_type_label(where_clause.left))
            .type_field("right", builder.global_type_label(where_clause.right));

        builder.push(row);
    }
}

/// Add member rows in declaration order.
fn add_members(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    members: &[dir::DefinitionMember],
) {
    for member in members {
        match member {
            dir::DefinitionMember::Field(field) => add_field(builder, owner, field),
            dir::DefinitionMember::Method(method) => add_method(builder, owner, method),
            dir::DefinitionMember::AssociatedType(ty) => add_associated_type(builder, owner, ty),
            dir::DefinitionMember::AssociatedConst(value) => {
                add_associated_const(builder, owner, value);
            }
            dir::DefinitionMember::Variant(variant) => add_variant(builder, owner, variant),
            dir::DefinitionMember::CallSignature(signature) => {
                add_signature(builder, owner, "call", signature);
            }
            dir::DefinitionMember::ConstructSignature(signature) => {
                add_signature(builder, owner, "construct", signature);
            }
            dir::DefinitionMember::IndexSignature(signature) => {
                add_signature(builder, owner, "index", signature);
            }
        }
    }
}

/// Add one field row.
fn add_field(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    field: &dir::FieldDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "field")
        .field("symbol", builder.symbol_path_label(field.symbol))
        .optional_field("source", builder.node_source(field.source))
        .field("key", builder.static_key(field.key))
        .optional_field("static", static_label(field.space))
        .optional_field("abstract", field.is_abstract.then(|| "true".to_string()))
        .optional_field("override", field.is_override.then(|| "true".to_string()))
        .optional_field("condition", condition_label(builder, field.condition))
        .type_field("type", builder.global_type_label(field.ty));

    builder.push(row);
}

/// Add one method row.
fn add_method(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    method: &dir::MethodDefinition,
) {
    let abstraction = match method.abstraction {
        dir::MethodAbstraction::Concrete => None,
        dir::MethodAbstraction::Virtual => Some("virtual".to_string()),
        dir::MethodAbstraction::Abstract => Some("abstract".to_string()),
    };
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "method")
        .optional_field(
            "symbol",
            method
                .symbol
                .map(|symbol| builder.symbol_path_label(symbol)),
        )
        .optional_field("source", builder.node_source(method.source))
        .field("slot", member_slot_label(method.slot, builder))
        .optional_field("static", static_label(method.space))
        .optional_field("role", method.role.map(DirSnapshotBuilder::variant_label))
        .optional_field("abstraction", abstraction)
        .optional_field("override", method.is_override.then(|| "true".to_string()))
        .optional_field("condition", condition_label(builder, method.condition))
        .type_field("type", builder.global_type_label(method.ty));

    builder.push(row);
}

/// Add one associated type row.
fn add_associated_type(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    ty: &dir::AssociatedTypeDefinition,
) {
    let row = SnapshotRow::new(
        builder.anchor_symbol(owner),
        "definition",
        "associated.type",
    )
    .field("symbol", builder.symbol_path_label(ty.symbol))
    .optional_field("source", builder.node_source(ty.source))
    .field("key", builder.static_key(ty.key))
    .optional_field(
        "constraint",
        ty.constraint
            .map(|constraint| builder.global_type_label(constraint)),
    )
    .optional_field(
        "value",
        ty.value.map(|value| builder.global_type_label(value)),
    )
    .optional_field("condition", condition_label(builder, ty.condition));

    builder.push(row);
}

/// Add one associated const row.
fn add_associated_const(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    value: &dir::AssociatedConstDefinition,
) {
    let row = SnapshotRow::new(
        builder.anchor_symbol(owner),
        "definition",
        "associated.const",
    )
    .field("symbol", builder.symbol_path_label(value.symbol))
    .optional_field("source", builder.node_source(value.source))
    .field("key", builder.static_key(value.key))
    .type_field("type", builder.global_type_label(value.ty))
    .optional_field(
        "value",
        value.value.map(|value| builder.global_static_label(value)),
    )
    .optional_field("condition", condition_label(builder, value.condition));

    builder.push(row);
}

/// Add one enum variant row.
fn add_variant(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    variant: &dir::VariantDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "variant")
        .field("symbol", builder.symbol_path_label(variant.symbol))
        .optional_field("source", builder.node_source(variant.source))
        .field("key", builder.static_key(variant.key))
        .optional_field(
            "value",
            variant
                .value
                .map(|value| builder.global_static_label(value)),
        )
        .optional_field("condition", condition_label(builder, variant.condition));

    builder.push(row);
}

/// Add one symbol-free signature row.
fn add_signature(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    kind: &'static str,
    signature: &dir::SignatureDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "signature")
        .field("kind", kind)
        .optional_field("source", builder.node_source(signature.source))
        .optional_field("condition", condition_label(builder, signature.condition))
        .type_field("type", builder.global_type_label(signature.ty));

    builder.push(row);
}

/// Return one static-space marker label.
fn static_label(space: dir::MemberSpace) -> Option<String> {
    match space {
        dir::MemberSpace::Instance => None,
        dir::MemberSpace::Static => Some("true".to_string()),
    }
}

/// Return one rendered @if condition label.
fn condition_label(
    builder: &mut DirSnapshotBuilder<'_>,
    condition: Option<dir::GlobalTypeId>,
) -> Option<String> {
    condition.map(|condition| builder.global_type_label(condition))
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
