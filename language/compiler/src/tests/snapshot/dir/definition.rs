use tspp_dir as dir;

use super::generic::generic_template_parameter_label;
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
            let row = declaration_row(builder, symbol, "struct", source, definition.template);
            builder.push(row);
            add_conformances(builder, symbol, &definition.implements);
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
            add_conformances(builder, symbol, &definition.implements);
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
            add_enum_row(builder, symbol, source, definition);
            add_conformances(builder, symbol, &definition.implements);
            add_members(builder, symbol, &definition.members);
        }
        dir::Definition::Newtype(definition) => {
            add_newtype_row(builder, symbol, source, definition);
            add_members(builder, symbol, &definition.members);
        }
        dir::Definition::Extension(extension) => {
            add_extension_row(builder, symbol, source, extension);
            add_conformances(builder, symbol, &extension.implements);
            add_template_predicates(builder, symbol, extension.template);
            add_members(builder, symbol, &extension.members);
        }
    }

    // render where predicates for every other templated declaration
    if !matches!(definition, dir::Definition::Extension(_)) {
        add_template_predicates(builder, symbol, definition.template());
    }
}

/// Add one enum declaration row.
fn add_enum_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    source: dir::GlobalNodeIdAny,
    definition: &dir::EnumDefinition,
) {
    let backing = (definition.backing != dir::EnumBackingType::DEFAULT)
        .then(|| enum_backing_label(definition.backing));
    let row = declaration_row(builder, symbol, "enum", source, definition.template)
        .optional_field("backing", backing);

    builder.push(row);
}

/// Render one enum backing type.
fn enum_backing_label(backing: dir::EnumBackingType) -> String {
    match backing {
        dir::EnumBackingType::String => "string".to_string(),
        dir::EnumBackingType::Integer(integer) => integer.as_str(),
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
            definition
                .template
                .and_then(|template| template_label(builder, symbol, template)),
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
            definition
                .template
                .and_then(|template| template_label(builder, symbol, template)),
        )
        .type_field("backing", builder.global_type_label(definition.backing))
        .optional_field(
            "backing_visibility",
            visibility_label(definition.backing_visibility),
        );
    let constructors = builder
        .members
        .as_ref()
        .and_then(|members| members.newtype_constructors(symbol))
        .map(<[_]>::to_vec)
        .unwrap_or_default();
    let row = match constructors.is_empty() {
        true => row,
        false => row.list_field(
            "constructors",
            constructors
                .iter()
                .map(|constructor| builder.global_type_label(constructor.ty)),
        ),
    };

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
        .optional_field(
            "template",
            template.and_then(|template| template_label(builder, symbol, template)),
        )
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
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", relation)
        .field("symbol", builder.symbol_path_label(owner))
        .optional_field("source", builder.node_source(heritage.source))
        .field("target", builder.global_type_label(heritage.ty));

    builder.push(row);
}

/// Add explicit interface conformance rows.
fn add_conformances(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    conformances: &[dir::NominalConformance],
) {
    // render each implemented interface and its selected members
    for conformance in conformances {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "implements")
            .field("symbol", builder.symbol_path_label(owner))
            .optional_field("source", builder.node_source(conformance.source))
            .field("target", builder.global_type_label(conformance.interface));
        builder.push(row);

        let members = builder
            .members
            .as_ref()
            .and_then(|members| members.conformance_members(conformance.source))
            .map(<[_]>::to_vec)
            .unwrap_or_default();
        for member in &members {
            let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "conformance")
                .field("symbol", builder.symbol_path_label(owner))
                .field("member", builder.symbol_path_label(member.member))
                .field("requirement", builder.symbol_path_label(member.requirement));
            builder.push(row);
        }
    }
}

/// Add one extension definition row.
fn add_extension_row(
    builder: &mut DirSnapshotBuilder<'_>,
    symbol: dir::GlobalSymbolId,
    source: dir::GlobalNodeIdAny,
    extension: &dir::ExtensionDefinition,
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

/// Render one declaration template's parameter list, omitting empty representations.
fn template_label(
    builder: &DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    template: dir::LocalGenericTemplateId,
) -> Option<String> {
    let Some(generics) = builder.generic_table(owner.module_id) else {
        return Some(format!("{template:?}"));
    };
    let parameters = generics.get_template(template).parameters.as_slice();
    if parameters.is_empty() {
        return None;
    }
    let parameters = parameters
        .iter()
        .map(|parameter| {
            generic_template_parameter_label(
                *parameter,
                generics.get_parameter(*parameter),
                builder,
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    Some(format!("({parameters})"))
}

/// Add where predicate rows from one declaration's template.
fn add_template_predicates(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    template: Option<dir::LocalGenericTemplateId>,
) {
    let Some(template) = template else {
        return;
    };
    let Some(generics) = builder.generic_table(owner.module_id) else {
        return;
    };
    let predicates = generics.get_template(template).predicates.clone();
    for predicate in predicates {
        let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "where")
            .field("symbol", builder.symbol_path_label(owner))
            .optional_field("source", builder.node_source(predicate.source))
            .field(
                "relation",
                match predicate.relation {
                    dir::WhereRelation::Satisfies => "satisfies",
                    dir::WhereRelation::Equal => "equal",
                },
            )
            .type_field("left", builder.global_type_label(predicate.left))
            .type_field("right", builder.global_type_label(predicate.right));

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
            dir::DefinitionMember::EnumVariant(variant) => {
                add_enum_variant(builder, owner, variant);
            }
            dir::DefinitionMember::CallSignature(signature) => {
                add_signature(builder, owner, "call", signature);
            }
            dir::DefinitionMember::ConstructSignature(signature) => {
                add_signature(builder, owner, "construct", signature);
            }
            dir::DefinitionMember::IndexSignature(signature) => {
                add_index_signature(builder, owner, signature);
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
        .optional_field("visibility", visibility_label(field.visibility))
        .optional_field("abstract", field.is_abstract.then(|| "true".to_string()))
        .optional_field("override", field.is_override.then(|| "true".to_string()))
        .type_field("type", builder.global_symbol_type_label(field.symbol));

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
        .field("symbol", builder.symbol_path_label(method.symbol))
        .optional_field("source", builder.node_source(method.source))
        .field("slot", member_slot_label(method.slot, builder))
        .optional_field("static", static_label(method.space))
        .optional_field("visibility", visibility_label(method.visibility))
        .optional_field("role", method.role.map(DirSnapshotBuilder::variant_label))
        .optional_field("abstraction", abstraction)
        .optional_field("override", method.is_override.then(|| "true".to_string()))
        .type_field("type", builder.global_symbol_type_label(method.symbol));

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
    );

    builder.push(row);
}

/// Add one associated const row.
fn add_associated_const(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    constant: &dir::AssociatedConstDefinition,
) {
    let row = SnapshotRow::new(
        builder.anchor_symbol(owner),
        "definition",
        "associated.const",
    )
    .field("symbol", builder.symbol_path_label(constant.symbol))
    .optional_field("source", builder.node_source(constant.source))
    .field("key", builder.static_key(constant.key))
    .type_field("type", builder.global_symbol_type_label(constant.symbol));

    builder.push(row);
}

/// Add one declared enum variant row.
fn add_enum_variant(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    variant: &dir::EnumVariantDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "variant")
        .field("symbol", builder.symbol_path_label(variant.symbol))
        .optional_field("source", builder.node_source(variant.source))
        .field("key", builder.static_key(variant.key))
        .field("value", builder.scalar_literal_label(&variant.value.into()));

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
        .type_field("type", builder.global_type_label(signature.ty));

    builder.push(row);
}

/// Add one index signature row.
fn add_index_signature(
    builder: &mut DirSnapshotBuilder<'_>,
    owner: dir::GlobalSymbolId,
    signature: &dir::IndexSignatureDefinition,
) {
    let row = SnapshotRow::new(builder.anchor_symbol(owner), "definition", "signature")
        .field("kind", "index")
        .optional_field("source", builder.node_source(signature.source))
        .type_field("key", builder.global_type_label(signature.key_type))
        .type_field("type", builder.global_type_label(signature.value_type));

    builder.push(row);
}

/// Return the label of one protected or private visibility.
fn visibility_label(visibility: dir::Visibility) -> Option<String> {
    match visibility {
        dir::Visibility::Public => None,
        visibility => Some(visibility.label().to_string()),
    }
}

/// Return one static-space marker label.
fn static_label(space: dir::MemberSpace) -> Option<String> {
    match space {
        dir::MemberSpace::Instance => None,
        dir::MemberSpace::Static => Some("true".to_string()),
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
