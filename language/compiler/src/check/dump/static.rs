use destack_dir as dir;

use crate::check::{Dump, DumpContext, StaticTerm, TypeRelation};

use super::argument::dump_arguments;
use super::format::{dump_list, dump_record};

impl Dump for StaticTerm {
    /// Render one static term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Static(value) => dump_record(
                "StaticTerm.Static",
                [("value", context.static_label(*value))],
            ),
            Self::Literal(literal) => {
                dump_record("StaticTerm.Literal", [("value", literal.dump(context))])
            }
            Self::Parameter(parameter) => {
                dump_record("StaticTerm.Parameter", [("slot", parameter.dump(context))])
            }
            Self::Expression(expression) => dump_record(
                "StaticTerm.Expression",
                [("source", context.node_label(expression.clone().into()))],
            ),
            Self::Member {
                source,
                owner,
                key,
                arguments,
            } => {
                let source = context.node_label(*source);
                let key = context.static_key_label(key);
                let arguments = dump_arguments(arguments, context);

                dump_record(
                    "StaticTerm.Member",
                    [
                        ("source", source),
                        ("owner", owner.dump(context)),
                        ("key", key),
                        ("arguments", arguments),
                    ],
                )
            }
            Self::Equal {
                left,
                right,
                is_negated,
            } => dump_record(
                "StaticTerm.Equal",
                [
                    ("left", left.dump(context)),
                    ("right", right.dump(context)),
                    ("is_negated", is_negated.to_string()),
                ],
            ),
            Self::TypeRelation {
                relation,
                left,
                right,
            } => dump_record(
                "StaticTerm.TypeRelation",
                [
                    ("relation", relation.dump(context)),
                    ("left", left.dump(context)),
                    ("right", right.dump(context)),
                ],
            ),
            Self::Conditional {
                condition,
                then_value,
                else_value,
            } => dump_record(
                "StaticTerm.Conditional",
                [
                    ("condition", condition.dump(context)),
                    ("then", then_value.dump(context)),
                    ("else", else_value.dump(context)),
                ],
            ),
            Self::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| element.dump(context))
                    .collect::<Vec<_>>()
                    .join(",");

                dump_record("StaticTerm.Union", [("elements", dump_list(elements))])
            }
            Self::Layout(layout) => {
                dump_record("StaticTerm.Layout", [("query", layout.dump(context))])
            }
            Self::Intrinsic { item, arguments } => dump_record(
                "StaticTerm.Intrinsic",
                [
                    ("item", item.key()),
                    ("arguments", dump_arguments(arguments, context)),
                ],
            ),
        }
    }
}

impl Dump for dir::StaticTerm {
    /// Render one committed DIR static term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Parameter(parameter) => dump_record(
                "DirStaticTerm.Parameter",
                [
                    ("owner", context.symbol_label(parameter.owner)),
                    ("key", context.generic_slot_key(parameter.key)),
                    ("index", parameter.index.get().to_string()),
                ],
            ),
            Self::Symbol { symbol } => dump_record(
                "DirStaticTerm.Symbol",
                [("symbol", context.symbol_label(*symbol))],
            ),
            Self::Access { access } => {
                dump_record("DirStaticTerm.Access", [("value", access.dump(context))])
            }
            Self::Space { space } => {
                dump_record("DirStaticTerm.Space", [("value", space.dump(context))])
            }
            Self::Place { place } => {
                dump_record("DirStaticTerm.Place", [("value", place.dump(context))])
            }
            Self::Lifetime { lifetime } => dump_record(
                "DirStaticTerm.Lifetime",
                [("value", lifetime.dump(context))],
            ),
            Self::ScalarLiteral { value } => dump_record(
                "DirStaticTerm.ScalarLiteral",
                [("value", value.dump(context))],
            ),
            Self::TypeLiteral { value } => dump_record(
                "DirStaticTerm.TypeLiteral",
                [("value", value.dump(context))],
            ),
            Self::Declaration {
                declaration,
                generic_arguments,
            } => dump_record(
                "DirStaticTerm.Declaration",
                [
                    ("declaration", declaration.dump(context)),
                    ("generic_arguments", generic_arguments.dump(context)),
                ],
            ),
            Self::Type { ty } => {
                dump_record("DirStaticTerm.Type", [("type", context.type_label(*ty))])
            }
            Self::Array { elements } => dump_record(
                "DirStaticTerm.Array",
                [("elements", dump_dir_static_terms(elements, context))],
            ),
            Self::FixedArray { value, length } => dump_record(
                "DirStaticTerm.FixedArray",
                [
                    ("value", value.dump(context)),
                    ("length", length.dump(context)),
                ],
            ),
            Self::Tuple { elements } => dump_record(
                "DirStaticTerm.Tuple",
                [("elements", dump_dir_static_terms(elements, context))],
            ),
            Self::Object { properties } => dump_record(
                "DirStaticTerm.Object",
                [(
                    "properties",
                    dump_dir_static_properties(properties, context),
                )],
            ),
            Self::Struct { ty, properties } => dump_record(
                "DirStaticTerm.Struct",
                [
                    ("type", context.type_label(*ty)),
                    (
                        "properties",
                        dump_dir_static_properties(properties, context),
                    ),
                ],
            ),
            Self::Union { elements } => {
                let elements = elements
                    .iter()
                    .map(|element| context.static_label(*element))
                    .collect::<Vec<_>>()
                    .join(",");

                dump_record("DirStaticTerm.Union", [("elements", dump_list(elements))])
            }
        }
    }
}

impl Dump for Vec<dir::StaticArgument> {
    /// Render one committed DIR static argument vector.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_dir_static_arguments(self, context)
    }
}

impl Dump for dir::StaticArgument {
    /// Render one committed DIR static argument.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "DirStaticArgument",
            [
                ("name", self.name.dump(context)),
                ("value", context.static_label(self.value)),
            ],
        )
    }
}

impl Dump for dir::StaticProperty {
    /// Render one committed DIR static property.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Field { key, value } => dump_record(
                "DirStaticProperty.Field",
                [("key", key.dump(context)), ("value", value.dump(context))],
            ),
            Self::Method {
                key,
                signature,
                body,
            } => dump_record(
                "DirStaticProperty.Method",
                [
                    ("key", key.dump(context)),
                    ("signature", signature.dump(context)),
                    ("body", body.dump(context)),
                ],
            ),
            Self::Spread { value } => {
                dump_record("DirStaticProperty.Spread", [("value", value.dump(context))])
            }
        }
    }
}

impl Dump for dir::StaticKey {
    /// Render one static key.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        context.static_key_label(self)
    }
}

impl Dump for TypeRelation {
    /// Render one type relation.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Equal => "equal".to_string(),
            Self::Assignable => "assignable".to_string(),
            Self::Castable => "castable".to_string(),
            Self::Satisfies => "satisfies".to_string(),
            Self::Extends => "extends".to_string(),
            Self::Implements => "implements".to_string(),
        }
    }
}

/// Render one DIR static term list.
fn dump_dir_static_terms(terms: &[dir::StaticTerm], context: &DumpContext<'_, '_>) -> String {
    let terms = terms
        .iter()
        .map(|term| term.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(terms)
}

/// Render one DIR static argument list.
fn dump_dir_static_arguments(
    arguments: &[dir::StaticArgument],
    context: &DumpContext<'_, '_>,
) -> String {
    let arguments = arguments
        .iter()
        .map(|argument| argument.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(arguments)
}

/// Render one DIR static property list.
fn dump_dir_static_properties(
    properties: &[dir::StaticProperty],
    context: &DumpContext<'_, '_>,
) -> String {
    let properties = properties
        .iter()
        .map(|property| property.dump(context))
        .collect::<Vec<_>>()
        .join(",");

    dump_list(properties)
}
