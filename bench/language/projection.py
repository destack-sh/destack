import base64
from datetime import datetime, timedelta
from typing import TYPE_CHECKING, Collection, cast
from uuid import UUID

from bench.language.const import EnumType, NodeType, PrimitiveType, StructType, TypeKind
from bench.language.graph import NodeList
from bench.language.node import BuiltinObject, Node, Struct, struct_
from bench.language.setup import ENUM_CLASS_BY_TYPE

if TYPE_CHECKING:
    from bench.language import Code, Text
    from bench.language.field import TypeInfoBase
    from bench.language.value import ScalarValue, SomeValue, ValueObject


class NodeVisitor:
    def __init__(self):
        self._reference_by_ck: dict[UUID, Node] = {}

    def __str__(self):
        return f"{len(self._reference_by_ck)} nodes"

    def __repr__(self):
        return f"<NodeVisitor {self!s}>"

    @property
    def references(self) -> Collection["Node"]:
        return self._reference_by_ck.values()

    def visit_reference(self, node: "Node"):
        self._reference_by_ck[node.ck] = node


@struct_(StructType.PROJECTION)
class Projection(Struct):
    """
    A projection into the graph.
    NOTE :Incomplete :Architecture: figure out projection
     - how do we filter and LoD this?
     - how do we represent unloaded nodes?
     - how do we make projections reproducible and inspectable in the editor?
    """

    ...


def project_node(node: Node) -> list[Node]:
    """
    Gather the references and descendants of the given node, recursively.
    See Projection for details.
    """
    seen_by_ck: dict[UUID, Node] = {}
    to_visit: list[Node] = [node]

    def _visit_node(node: Node):
        if node.ck in seen_by_ck:
            return
        seen_by_ck[node.ck] = node

        # visit children
        for prop in node.__node_child_properties__.values():
            prop_value = cast(NodeList, getattr(node, prop.name))
            for child in prop_value:
                _visit_node(child)
        # visit references in all contained structs
        for struc in node._walk_struct():
            for prop in struc.__node_reference_properties__.values():
                if prop.id is None or prop.id < 30:  # skip system properties (incl. parent)
                    continue
                elif prop.is_list:
                    prop_value = cast(NodeList | None, getattr(struc, prop.name))
                    if prop_value:
                        for child in prop_value:
                            _visit_node(child)
                else:
                    ref = cast(Node | None, getattr(struc, prop.name))
                    if ref:
                        _visit_node(ref)

    # traverse
    while to_visit:
        node = to_visit.pop()
        _visit_node(node)

    return list(seen_by_ck.values())


def render_value_scalar(value: "ScalarValue", typ: "TypeInfoBase") -> str:
    """Renders single scalar value into Bench python."""
    if typ.kind == TypeKind.PRIMITIVE:
        if typ.primitive_type == PrimitiveType.BYTES:
            value_b64 = base64.b64encode(cast(bytes, value)).decode("utf-8")
            return f"base64.b64decode({value_b64!r})"
        elif typ.primitive_type == PrimitiveType.DATETIME:
            value_iso = cast(datetime, value).isoformat()
            return f"datetime.fromisoformat({value_iso!r})"
        elif typ.primitive_type == PrimitiveType.INTERVAL:
            return f"timedelta(seconds={cast(timedelta, value).total_seconds()})"
        else:
            return repr(value)
    elif typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE:
        assert isinstance(value, Node), f"{value!r} is not a node (expected {typ!r})"
        # TODO :Broken :Projection: render/alias node reference properly :NodeAliasing
        ident = value.py_ident
        assert ident is not None, f"{value!r} has no identifier"
        return ident
    elif typ.kind == TypeKind.ENUM:
        enum_cls = ENUM_CLASS_BY_TYPE[cast(EnumType, typ.bench_type)]
        value = enum_cls(value)
        return f"{enum_cls.__name__}.{value.name}"
    elif typ.kind == TypeKind.STRUCT:
        return render_builtin_object(cast(Struct, value))
    else:
        raise RuntimeError(f"unexpected type {typ!r}")


def render_object_scalar(value: "ValueObject", typ: "TypeInfoBase") -> str:
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
    from bench.language.value import ValueObject

    if value is None:
        return "None"
    typ = typ._to_resolved()
    assert typ.kind != TypeKind.ALIAS, f"unresolved type {typ!r}"
    if typ.kind == TypeKind.OBJECT:
        # nested object
        if not typ.is_list:
            assert type(value) is ValueObject, f"{value!r} is not an object (expected {typ!r})"
            return render_object_scalar(value, typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            return f"[{', '.join(render_object_scalar(cast(ValueObject, v), typ) for v in value)}]"
    else:
        # scalar
        if not typ.is_list:
            return render_value_scalar(cast("ScalarValue", value), typ)
        else:
            assert isinstance(value, list), f"{value!r} is not a list (expected {typ!r})"
            return f"[{', '.join(render_value_scalar(v, typ) for v in value)}]"


def render_builtin_object(value: BuiltinObject) -> str:
    """Renders the given Node/Struct to Bench python."""
    if value.metatype == StructType.TEXT:
        # Text: just the markdown
        markdown = cast("Text", value).to_markdown()
        return f"Text.from_markdown({markdown!r})"
    elif value.metatype == StructType.CODE:
        # Code: just the code
        code = cast("Code", value).to_string()
        return f"Code.from_string({code!r})"
    else:
        # default: prop-by-prop
        repr_by_name: dict[str, str] = {}
        for prop in value.__properties__.values():
            if (
                prop._type_info is None
                or not prop.is_introspectable
                or prop.id < 30  # skip system properties
                or prop.is_tree_reference  # skip node properties
                or prop.name == "order_key"
            ):
                continue  # ignore
            prop_value = getattr(value, prop.name)
            if (
                prop_value is None
                or prop_value == prop.default
                or (prop.is_list and len(prop_value) == 0)
            ):
                continue  # skip empty values
            prop_repr = render_value(prop_value, prop._type_info)
            repr_by_name[prop.name] = prop_repr
        struct_repr = (
            f"{value.__class__.__name__}({', '.join(f'{k}={v}' for k, v in repr_by_name.items())})"
        )
        return struct_repr


# TODO :UX :Cleanup: simplify/prettify Node/Struct rendering and creation
#  (like Field(name=..., zone=.., kind=.., primitive_type=..) -> Field.input("name", str))


def render_node(
    roots: Node | tuple[Node, ...] | list[Node], node_types: tuple[NodeType, ...] = ()
) -> str:
    """Renders the given nodes and their descendants to Bench python."""
    roots = roots if isinstance(roots, (tuple, list)) else (roots,)
    seen_by_ck: dict[UUID, Node] = {}
    to_visit: list[Node] = list(roots)
    to_visit.reverse()  # keep order (we'll pop from the end)

    # TODO :Broken: defer setting not-yet-defined node alias

    def _render_node(node: Node) -> list[str]:
        """Renders the node and any in-page children immediately, deferring the rest to to_visit."""
        descendants_lines: list[str] = []
        node_alias = node.py_ident  # :NodeAliasing
        assert node_alias is not None, f"{node!r} has no identifier"

        # walk children
        for prop in node.__node_child_properties__.values():
            if node_types != () and (
                not prop.reference_nodes or prop.reference_nodes[0] not in node_types
            ):
                continue  # skip unwanted types
            prop_value = cast(NodeList, getattr(node, prop.name))
            prop_children = tuple(prop_value)
            if not prop_children:
                continue  # skip empty values

            prop_lines: list[str] = [f"{node_alias}.{prop.name}.extend("]
            nested_lines: list[str] = []
            for child in prop_children:
                child_lines = _render_node(child)
                prop_lines.append(child_lines[0] + ", ")
                nested_lines.extend(child_lines[1:])
            prop_lines.append(")")

            descendants_lines.extend(prop_lines)
            descendants_lines.extend(nested_lines)

        # wrap with alias if needed :NodeAliasing
        if descendants_lines:
            return [f"{node_alias} = {render_builtin_object(node)}", *descendants_lines]
        else:
            return [render_builtin_object(node)]

    all_lines: list[str] = []
    # traverse
    while to_visit:
        node = to_visit.pop()
        if node.ck in seen_by_ck:
            continue
        seen_by_ck[node.ck] = node
        new_lines = _render_node(node)
        all_lines.extend(new_lines)

    rendered = "\n".join(all_lines)
    return rendered


def render(value: Node | Struct) -> str:
    """Renders the given Node/Struct to Bench python and prettifies it."""
    from bench.language.code import format_code

    rendered = render_node(value) if isinstance(value, Node) else render_builtin_object(value)
    rendered = format_code(rendered)
    return rendered
