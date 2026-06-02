use destack_dir as dir;

use crate::check::{
    AwaitTerm, Dump, DumpContext, IdentityTerm, ImportMetaTerm, IndexKind, IndexSetTerm, IndexTerm,
    InstanceCheckTerm, KeyMembershipTerm, OperatorTerm, OperatorTermKind, RangeValueTerm,
    ReceiverTerm, SuperTerm, TaggedTemplateTerm, TemplateTerm, TreeTerm, TryFailureTerm, TryTerm,
    TryTermKind, TypeTerm, TypeValueTerm, YieldTerm,
};

use super::argument::dump_arguments;
use super::format::{dump_list, dump_record};
use super::literal::dump_strings;
use super::operand::dump_type_operands;
use super::shape::dump_members;

impl Dump for TypeTerm {
    /// Render one type term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Type(ty) => dump_record("TypeTerm.Type", [("value", context.type_label(*ty))]),
            Self::Literal(literal) => {
                dump_record("TypeTerm.Literal", [("value", literal.dump(context))])
            }
            Self::Parameter(parameter) => {
                dump_record("TypeTerm.Parameter", [("slot", parameter.dump(context))])
            }
            Self::This => dump_record("TypeTerm.This", []),
            Self::Reference {
                origin,
                symbol,
                arguments,
            } => {
                let origin = origin.dump(context);
                let symbol = context.symbol_label(*symbol);
                let arguments = dump_arguments(arguments, context);

                dump_record(
                    "TypeTerm.Reference",
                    [
                        ("source", origin),
                        ("symbol", symbol),
                        ("arguments", arguments),
                    ],
                )
            }
            Self::Array { element } => {
                dump_record("TypeTerm.Array", [("element", element.dump(context))])
            }
            Self::FixedArray { element, length } => dump_record(
                "TypeTerm.FixedArray",
                [
                    ("element", element.dump(context)),
                    ("length", length.dump(context)),
                ],
            ),
            Self::Slice { element } => {
                dump_record("TypeTerm.Slice", [("element", element.dump(context))])
            }
            Self::Tuple { form, elements } => {
                let elements = elements
                    .iter()
                    .map(|element| element.dump(context))
                    .collect::<Vec<_>>()
                    .join(",");

                dump_record(
                    "TypeTerm.Tuple",
                    [
                        ("form", form.dump(context)),
                        ("elements", dump_list(elements)),
                    ],
                )
            }
            Self::Form { form, payload } => {
                let form = context.check.term(*form).dump(context);

                dump_record(
                    "TypeTerm.Form",
                    [("form", form), ("payload", payload.dump(context))],
                )
            }
            Self::Dynamic { constraint } => dump_record(
                "TypeTerm.Dynamic",
                [("constraint", constraint.dump(context))],
            ),
            Self::Member(member) => {
                let member = context.check.term(*member);
                let key = context.static_key_label(&member.key);
                let arguments = dump_arguments(&member.arguments, context);

                dump_record(
                    "TypeTerm.Member",
                    [
                        ("owner", member.owner.dump(context)),
                        ("key", key),
                        ("arguments", arguments),
                    ],
                )
            }
            Self::Operation(operation) => dump_record(
                "TypeTerm.Operation",
                [("operation", context.check.term(*operation).dump(context))],
            ),
            Self::Union { elements } => dump_record(
                "TypeTerm.Union",
                [("elements", dump_type_operands(elements, context))],
            ),
            Self::Intersection { elements } => dump_record(
                "TypeTerm.Intersection",
                [("elements", dump_type_operands(elements, context))],
            ),
            Self::StaticValue { value } => {
                dump_record("TypeTerm.StaticValue", [("value", value.dump(context))])
            }
            Self::Call(call) => dump_record(
                "TypeTerm.Call",
                [
                    ("id", call.index().to_string()),
                    ("term", context.check.term(*call).dump(context)),
                ],
            ),
            Self::Construct(construct) => dump_record(
                "TypeTerm.Construct",
                [
                    ("id", construct.index().to_string()),
                    ("term", context.check.term(*construct).dump(context)),
                ],
            ),
            Self::Shape(shape) => {
                let shape = context.check.term(*shape);
                let members = shape
                    .members
                    .iter()
                    .map(|member| member.dump(context))
                    .collect::<Vec<_>>()
                    .join(", ");

                dump_record("TypeTerm.Shape", [("members", dump_members(members))])
            }
            Self::Function(function) => {
                let function = context.check.term(*function);

                dump_record("TypeTerm.Function", [("term", function.dump(context))])
            }
            Self::Closure {
                function,
                environment,
            } => dump_record(
                "TypeTerm.Closure",
                [
                    ("function", function.dump(context)),
                    ("environment", environment.dump(context)),
                ],
            ),
            Self::Range {
                start,
                end,
                is_inclusive,
            } => dump_record(
                "TypeTerm.Range",
                [
                    ("start", start.dump(context)),
                    ("end", end.dump(context)),
                    ("inclusive", is_inclusive.to_string()),
                ],
            ),
            Self::RangeValue(term) => dump_record(
                "TypeTerm.RangeValue",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Tree(term) => dump_record(
                "TypeTerm.Tree",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::TypeValue(term) => dump_record(
                "TypeTerm.TypeValue",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::ImportMeta(term) => dump_record(
                "TypeTerm.ImportMeta",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Receiver(term) => dump_record(
                "TypeTerm.Receiver",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Super(term) => dump_record(
                "TypeTerm.Super",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Operator(term) => dump_record(
                "TypeTerm.Operator",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Index(term) => dump_record(
                "TypeTerm.Index",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::IndexSet(term) => dump_record(
                "TypeTerm.IndexSet",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::KeyMembership(term) => dump_record(
                "TypeTerm.KeyMembership",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::InstanceCheck(term) => dump_record(
                "TypeTerm.InstanceCheck",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Identity(term) => dump_record(
                "TypeTerm.Identity",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Await(term) => dump_record(
                "TypeTerm.Await",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Try(term) => dump_record(
                "TypeTerm.Try",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Yield(term) => dump_record(
                "TypeTerm.Yield",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::TryFailure(term) => dump_record(
                "TypeTerm.TryFailure",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::Template(term) => dump_record(
                "TypeTerm.Template",
                [("term", context.check.term(*term).dump(context))],
            ),
            Self::TaggedTemplate(term) => dump_record(
                "TypeTerm.TaggedTemplate",
                [("term", context.check.term(*term).dump(context))],
            ),
        }
    }
}

impl Dump for RangeValueTerm {
    /// Render one runtime range value term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "RangeValueTerm",
            [
                ("source", context.node_label(self.source)),
                ("start", self.start.dump(context)),
                ("end", self.end.dump(context)),
                ("end_kind", self.end_kind.dump(context)),
            ],
        )
    }
}

impl Dump for TreeTerm {
    /// Render one tree expression term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TreeTerm",
            [
                ("source", context.node_label(self.source)),
                ("tag", self.tag.dump(context)),
                (
                    "generic_arguments",
                    dump_arguments(&self.generic_arguments, context),
                ),
                ("arguments", dump_type_operands(&self.arguments, context)),
                ("elements", dump_type_operands(&self.elements, context)),
            ],
        )
    }
}

impl Dump for TypeValueTerm {
    /// Render one reflected type value term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TypeValueTerm",
            [
                ("source", context.node_label(self.source)),
                ("type", self.ty.dump(context)),
            ],
        )
    }
}

impl Dump for ImportMetaTerm {
    /// Render one import metadata term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "ImportMetaTerm",
            [("source", context.node_label(self.source))],
        )
    }
}

impl Dump for ReceiverTerm {
    /// Render one contextual receiver term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "ReceiverTerm",
            [
                ("source", context.node_label(self.source)),
                ("kind", self.kind.dump(context)),
                ("type", self.ty.dump(context)),
            ],
        )
    }
}

impl Dump for SuperTerm {
    /// Render one super receiver term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "SuperTerm",
            [
                ("source", context.node_label(self.source)),
                ("receiver", self.receiver.dump(context)),
            ],
        )
    }
}

impl Dump for OperatorTerm {
    /// Render one runtime operator term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "OperatorTerm",
            [
                ("source", context.node_label(self.source)),
                ("kind", self.kind.dump(context)),
                ("receiver", self.receiver.dump(context)),
                ("argument", self.argument.dump(context)),
            ],
        )
    }
}

impl Dump for IndexTerm {
    /// Render one index access term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "IndexTerm",
            [
                ("source", context.node_label(self.source)),
                ("kind", self.kind.dump(context)),
                ("receiver", self.receiver.dump(context)),
                ("index", self.index.dump(context)),
                ("key", self.key.dump(context)),
            ],
        )
    }
}

impl Dump for IndexSetTerm {
    /// Render one index set term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "IndexSetTerm",
            [
                ("source", context.node_label(self.source)),
                ("receiver", self.receiver.dump(context)),
                ("index", self.index.dump(context)),
                ("value", self.value.dump(context)),
                ("key", self.key.dump(context)),
            ],
        )
    }
}

impl Dump for KeyMembershipTerm {
    /// Render one key membership term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "KeyMembershipTerm",
            [
                ("source", context.node_label(self.source)),
                ("key", self.key.dump(context)),
                ("receiver", self.receiver.dump(context)),
            ],
        )
    }
}

impl Dump for InstanceCheckTerm {
    /// Render one instance check term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "InstanceCheckTerm",
            [
                ("source", context.node_label(self.source)),
                ("value", self.value.dump(context)),
                ("target", self.target.dump(context)),
            ],
        )
    }
}

impl Dump for IdentityTerm {
    /// Render one identity check term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "IdentityTerm",
            [
                ("source", context.node_label(self.source)),
                ("operator", self.operator.text().to_string()),
                ("left", self.left.dump(context)),
                ("right", self.right.dump(context)),
            ],
        )
    }
}

impl Dump for AwaitTerm {
    /// Render one await term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "AwaitTerm",
            [
                ("source", context.node_label(self.source)),
                ("value", self.value.dump(context)),
            ],
        )
    }
}

impl Dump for TryTerm {
    /// Render one try term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TryTerm",
            [
                ("source", context.node_label(self.source)),
                ("value", self.value.dump(context)),
                ("kind", self.kind.dump(context)),
            ],
        )
    }
}

impl Dump for YieldTerm {
    /// Render one yield term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "YieldTerm",
            [
                ("source", context.node_label(self.source)),
                ("value", self.value.dump(context)),
                ("yield_type", self.yield_type.dump(context)),
                ("resume_type", self.resume_type.dump(context)),
                (
                    "delegate_return_type",
                    self.delegate_return_type.dump(context),
                ),
                ("cardinality", self.cardinality.dump(context)),
            ],
        )
    }
}

impl Dump for TryFailureTerm {
    /// Render one try failure term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TryFailureTerm",
            [
                ("source", context.node_label(self.source)),
                ("value", self.value.dump(context)),
            ],
        )
    }
}

impl Dump for TemplateTerm {
    /// Render one template term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TemplateTerm",
            [
                ("source", context.node_label(self.source)),
                ("strings", dump_strings(&self.strings, context)),
                ("spans", dump_type_operands(&self.spans, context)),
            ],
        )
    }
}

impl Dump for TaggedTemplateTerm {
    /// Render one tagged template term.
    fn dump(&self, context: &DumpContext<'_, '_>) -> String {
        dump_record(
            "TaggedTemplateTerm",
            [
                ("source", context.node_label(self.source)),
                ("tag", self.tag.dump(context)),
                (
                    "generic_arguments",
                    dump_arguments(&self.generic_arguments, context),
                ),
                ("strings", dump_strings(&self.strings, context)),
                ("spans", dump_type_operands(&self.spans, context)),
            ],
        )
    }
}

impl Dump for dir::TupleForm {
    /// Render one tuple form.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Tuple => "tuple".to_string(),
            Self::Array => "array".to_string(),
        }
    }
}

impl Dump for dir::RangeEnd {
    /// Render one range end.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Open => "open".to_string(),
            Self::Inclusive => "inclusive".to_string(),
        }
    }
}

impl Dump for dir::ReceiverKind {
    /// Render one receiver kind.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::This => "this".to_string(),
            Self::Super => "super".to_string(),
        }
    }
}

impl Dump for OperatorTermKind {
    /// Render one operator kind.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Unary(operator) => dump_record(
                "OperatorTermKind.Unary",
                [("operator", operator.text().to_string())],
            ),
            Self::Binary(operator) => dump_record(
                "OperatorTermKind.Binary",
                [("operator", operator.text().to_string())],
            ),
        }
    }
}

impl Dump for IndexKind {
    /// Render one index kind.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Element => "element".to_string(),
            Self::Slice => "slice".to_string(),
        }
    }
}

impl Dump for TryTermKind {
    /// Render one try kind.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Maybe => "maybe".to_string(),
            Self::Must => "must".to_string(),
        }
    }
}

impl Dump for dir::YieldCardinality {
    /// Render one yield cardinality.
    fn dump(&self, _context: &DumpContext<'_, '_>) -> String {
        match self {
            Self::Scalar => "scalar".to_string(),
            Self::Generator => "generator".to_string(),
        }
    }
}
