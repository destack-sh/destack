import base64
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Collection, cast
from uuid import UUID

from bench.language.const import EnumType, PrimitiveType, StructType, TypeKind
from bench.language.node import Node, Struct, struct
from bench.language.setup import ENUM_CLASS_BY_TYPE

if TYPE_CHECKING:
    from bench.language import Code, Text
    from bench.language.field import TypeInfoBase
    from bench.language.value import ScalarValue, Object, SomeValue


class NodeVisitor:
    def __init__(self):
        self._reference_by_ck: dict[UUID, Node] = {}

    def __str__(self):
        return f"{len(self._reference_by_ck)} nodes"

    def __repr__(self):
        return f"<NodeVisitor {str(self)}>"

    @property
    def references(self) -> Collection["Node"]:
        return self._reference_by_ck.values()

    def visit_reference(self, node: "Node"):
        self._reference_by_ck[node.ck] = node


@struct(StructType.PROJECTION)
class Projection(Struct):
    """
    A projection into the graph.
    NOTE :Incomplete :Architecture: figure out projection
     - how do we filter and LoD this?
     - how do we represent unloaded nodes?
     - how do we make projections reproducible and inspectable in the editor?
    """

    ...


def render_value_scalar(value: "ScalarValue", typ: "TypeInfoBase") -> str:
    """Renders single scalar value into Bench python."""
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            value_b64 = base64.b64encode(cast(bytes, value)).decode("utf-8")
            return f"base64.b64decode({repr(value_b64)})"
        elif typ.primitive_type == PrimitiveType.DATETIME:
            value_iso = cast(datetime, value).isoformat()
            return f"datetime.fromisoformat({repr(value_iso)})"
        else:
            return repr(value)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        raise NotImplementedError("nocheckin")
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        value = enum_cls(value)
        return f"{enum_cls.__name__}.{value.name}"
    elif typ.kind == TypeKind.STRUCT:
        return render_struct(cast(Struct, value))


def render_object_scalar(value: "Object", typ: "TypeInfoBase") -> str:
    """Renders single Object into Bench python (recursively)."""
    assert typ.base_type is not None, f"{value!r} has no base type"
    repr_by_name: dict[str, str] = {}
    for field in typ._base_fields:
        field_type = field._to_resolved()
        field_value = cast(SomeValue, getattr(value, field.name, None))
        field_value_repr = render_value(field_value, field_type)
        repr_by_name[field.name] = field_value_repr
    return f"{typ.base_type.name}({', '.join(f'{k}={v}' for k, v in repr_by_name.items())})"


def render_value(value: "SomeValue | None", typ: "TypeInfoBase") -> str:
    """Renders a value into Bench python (recursively)."""
    from bench.language.value import Object

    if value is None:
        return "None"
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        # nested object
        if not typ.is_list:
            assert type(value) is Object, f"{value!r} is not an object (expected {typ!r})"
            return render_object_scalar(value, typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            return f"[{', '.join(render_object_scalar(cast(Object, v), typ) for v in value)}]"
    else:
        # scalar
        if not typ.is_list:
            return render_value_scalar(cast("ScalarValue", value), typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            return f"[{', '.join(render_value_scalar(v, typ) for v in value)}]"


def render_struct(object: Node | Struct) -> str:
    """Renders the given Node/Struct to Bench python."""
    if object.metatype == StructType.TEXT:
        # Text: just the markdown
        markdown = cast("Text", object).to_markdown()
        return f"Text.from_markdown({repr(markdown)})"
    else:
        # default: prop-by-prop
        repr_by_name: dict[str, str] = {}
        for prop in object.__properties__.values():
            if (
                prop.type_info is None
                or prop.id < 30  # skip system properties
                or prop.is_tree_reference  # skip node properties
                or prop.name in ("order_key",)
            ):
                continue  # ignore

            prop_value = getattr(object, prop.name)
            if (
                prop_value is None
                or prop_value == prop.default
                or prop.is_list
                and len(prop_value) == 0
            ):
                continue  # skip empty values
            prop_repr = render_value(prop_value, prop.type_info)
            repr_by_name[prop.name] = prop_repr
        struct_repr = (
            f"{object.__class__.__name__}({', '.join(f'{k}={v}' for k, v in repr_by_name.items())})"
        )
        return struct_repr
