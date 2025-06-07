import abc
import base64
import contextvars
import inspect
import textwrap
from collections.abc import Collection, Mapping
from enum import Enum, IntEnum
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Self,
    Union,
    assert_never,
    cast,
    dataclass_transform,
    final,
)

import structlog
from bitarray import bitarray
from fastuuid import UUID, uuid4
from opentelemetry import trace

from bench.pb2 import AnyObjectData
from bench.utils.code import exec_, format_code
from bench.utils.env import IS_DEV
from bench.utils.frozen import frozendict, frozenlist
from bench.utils.func import dualmethod, get_superclasses

from .const import (
    ACTIVE_SESSION,
    EMPTY_DICT,
    REGION,
    UNSET,
    DefaultFactory,
    EdgeType,
    EnumType,
    NodeType,
    PrimitiveType,
    ScalarType,
    StructType,
    TraitType,
    TypeCardinality,
)
from .property import _PROPERTY_SPECIFIERS, IntoType, Property, property_runtime_

if TYPE_CHECKING:
    from bench.language import (
        Field,
        Graph,
        Node,
        QueryConnection,
        Session,
        Supergraph,
    )

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

FieldOrProperty = Union[
    Field if TYPE_CHECKING else "Field", Property if TYPE_CHECKING else "Property", Any
]
NodeTypeOrClass = Union[NodeType, type["Node"]]


class _SetupStage(IntEnum):
    INITIALIZING = 1
    FINALIZING = 2
    COMPLETED = 3


_SETUP_STAGE = _SetupStage.INITIALIZING


def _is_setup_complete() -> bool:
    return _SETUP_STAGE == _SetupStage.COMPLETED


def _set_setup_finalizing():
    global _SETUP_STAGE
    _SETUP_STAGE = _SetupStage.FINALIZING


def _set_setup_complete():
    global _SETUP_STAGE
    _SETUP_STAGE = _SetupStage.COMPLETED


def get_tk_b64_from_ck(ck: UUID) -> str:
    """Gets the stable across templates first 6 bytes of the ck."""
    return base64.b64encode(ck.bytes).decode()


_processed_classes: dict[type["BuiltinObjectBase"], type["BuiltinObjectBase"]] = {}


def _generate_init_impl[ObjectT: BuiltinObjectBase](
    cls: type[ObjectT],
    is_node: bool,
    is_root_node: bool,
    is_frozen: bool,
    traits: tuple[TraitType, ...],
    properties: dict[str, Property],
) -> tuple[str, dict[str, Any]]:
    """Generates an __init__ for a BuiltinObject class."""

    extra_glbls: dict[str, Any] = {}

    # header
    header_properties = dict(properties)
    if is_node:
        header_properties.pop("_hash")
        header_properties.pop("_ref")
        header_properties.pop("_is_new")
        header_properties.pop("_is_attached")
        header_properties.pop("_dirty")
    properties_in_order = list(header_properties.values())
    properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
    method_header_lines = ["def __init__(self, *"]
    required_properties = [
        p
        for p in properties_in_order
        if not p.is_computed
        and p.default is UNSET
        and p.default_factory is None
        and not p.is_managed
        and p.cardinality == TypeCardinality.SCALAR
        and p.scalar_type
        != ScalarType.NODE_REFERENCE  # passed either as node or node_ptr, defer check
    ]
    # first add properties without defaults that are not managed
    for prop in required_properties:
        method_header_lines.append(prop.name)
    # then add properties with defaults or that are managed
    for prop in properties_in_order:
        if prop.is_computed or prop in required_properties:
            continue
        elif prop.default is UNSET:
            default_str = "None"
        elif isinstance(prop.default, Enum):
            default_str = f"{prop.default.__class__.__name__}.{prop.default.name}"
            extra_glbls[prop.default.__class__.__name__] = prop.default.__class__
        elif prop.default is None or isinstance(prop.default, (bool, int, float, str, bytes, UUID)):
            default_str = repr(prop.default)
        else:
            default_name = f"_default_{prop.name}"
            extra_glbls[default_name] = prop.default
            default_str = default_name
        method_header_lines.append(f"{prop.name}={default_str}")

    method_header_lines.append(")")
    method_header = ", ".join(method_header_lines)

    # body
    # NOTE: Structs can use direct assignment, Nodes shouldn't (because of custom __setattr__)
    extra_glbls["ACTIVE_SESSION"] = ACTIVE_SESSION
    extra_glbls["EMPTY_LIST"] = frozenlist()
    extra_glbls["EMPTY_DICT"] = frozendict()
    extra_glbls["uuid4"] = uuid4
    extra_glbls["REGION"] = REGION

    method_body_lines = [
        "__setattr__ = object.__setattr__",
    ]
    body_properties = dict(properties)
    body_properties.pop("_supergraph")

    # setup
    if is_node:
        # node setup
        body_properties.pop("id")
        body_properties.pop("ck", None)
        if TraitType.TRACKED in traits:
            body_properties.pop("created_at")
            body_properties.pop("updated_at")
        body_properties.pop("_session")
        body_properties.pop("_graph")
        body_properties.pop("_connection")
        body_properties.pop("_hash")
        body_properties.pop("_ref")
        body_properties.pop("_is_new")
        body_properties.pop("_is_attached")
        body_properties.pop("_dirty")
        method_body_lines.append(f"""\
# session
if _session is None:
    _session = ACTIVE_SESSION.get()
    if _session is None:
        raise RuntimeError("no active session for {cls.__name__}")
__setattr__(self, "_session", _session)
if _supergraph is None:
    _supergraph = _session.supergraph
__setattr__(self, "_supergraph", _supergraph)

# node identity
if id is None:
    id = uuid4()
    """)
        if TraitType.TRACKED in traits:
            method_body_lines.append("""\
    now = self._session.oracle.utc()
    created_at = now
    updated_at = now
""")
        method_body_lines.append(f"""\
    _is_new = True
    _is_attached = {"True" if is_root_node else "_graph is not None"}
else:
    _is_new = False
    _is_attached = True # if we already have an id, assume we're attached
__setattr__(self, "id", id)
""")

        if TraitType.TRACKED in traits:
            method_body_lines.append("""\
__setattr__(self, "created_at", created_at)
__setattr__(self, "updated_at", updated_at)
""")
        method_body_lines.append("""\
__setattr__(self, "_hash", id.int)
__setattr__(self, "_ref", None)
__setattr__(self, "_is_new", _is_new)
__setattr__(self, "_is_attached", _is_attached)
__setattr__(self, "_dirty", None)
""")

    else:
        # struct setup
        method_body_lines.append("""\
# session
if _supergraph is None:
    if (session := ACTIVE_SESSION.get()) is not None:
        _supergraph = session.supergraph
self._supergraph = _supergraph
""")

    # property assignments
    method_body_lines.append("# properties")
    body_properties_in_order = list(body_properties.values())
    body_properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
    for prop in body_properties_in_order:
        if prop.is_computed:
            # computed, can't assign
            continue
        elif (ptr_prop := prop.ptr_prop) is not None:
            # derive ptr_prop from prop if prop is set
            if prop.cardinality == TypeCardinality.SCALAR:
                method_body_lines.append(f"""\
if {prop.name} is not None:
    {ptr_prop.name} = {prop.name}.to_ref()""")
            elif prop.cardinality == TypeCardinality.LIST:
                method_body_lines.append(f"""\
if {prop.name}:
    {ptr_prop.name} = tuple(x.to_ref() for x in {prop.name})""")
            else:
                raise RuntimeError(f"unsupported cardinality: {prop!r}")
            # (don't need to actually assign since these come before the ptr_prop in the list)
            continue

        # check if node is passed if required and scalar
        if (
            prop.is_required
            and prop.cardinality == TypeCardinality.SCALAR
            and prop.scalar_type == ScalarType.NODE_REFERENCE
        ):
            method_body_lines.append(f"""\
if {prop.name} is None:
    raise AttributeError(f"{cls.__name__}.{prop.name} is required")""")

        # init default factory
        if prop.default_factory is not None:
            if prop.default_factory == DefaultFactory.UUID:
                method_body_lines.append(f"""\
if {prop.name} is None:
    {prop.name} = uuid4()""")
            elif prop.default_factory == DefaultFactory.NOW:
                if is_node:
                    method_body_lines.append(f"""\
if {prop.name} is None:
    assert self._session is not None, "no session for {cls.__name__}"
    {prop.name} = self._session.oracle.utc()""")
                else:
                    method_body_lines.append(f"""\
if {prop.name} is None:
    session = ACTIVE_SESSION.get()
    if session is None:
        raise RuntimeError("no active session for {cls.__name__}")
    {prop.name} = session.oracle.utc()""")
            elif prop.default_factory == DefaultFactory.REGION:
                method_body_lines.append(f"""\
if {prop.name} is None:
    {prop.name} = REGION""")
            else:
                assert_never(prop.default_factory)

        # init list/map if unset
        if prop.cardinality == TypeCardinality.LIST:
            method_body_lines.append(f"""\
if {prop.name} is None:
    {prop.name} = {"[]" if not is_frozen else "EMPTY_LIST"}""")
        elif prop.cardinality == TypeCardinality.MAP:
            method_body_lines.append(f"""\
if {prop.name} is None:
    {prop.name} = {"{}" if not is_frozen else "EMPTY_DICT"}""")

        # regular assignment
        if is_node:
            method_body_lines.append(f"__setattr__(self, '{prop.name}', {prop.name})")
        else:
            method_body_lines.append(f"self.{prop.name} = {prop.name}")

    if is_node:
        method_body_lines.append("""\
# graph
if _graph is None:
    from bench.language.core import SingletonGraph
                                 
    _graph = SingletonGraph(_supergraph, self)
    _supergraph.add_graph(_graph)
else:
    _graph.add(self)
__setattr__(self, "_graph", _graph)
__setattr__(self, "_connection", _connection)
""")

    method_body = "\n".join(method_body_lines) or "pass"
    method_body = textwrap.indent(method_body, "    ")
    init_str = f"{method_header}:\n{method_body}"
    if IS_DEV:
        init_str = format_code(init_str)
    return init_str, extra_glbls


def _generate_repr_impl[ObjectT: BuiltinObjectBase](
    cls: type[ObjectT],
) -> tuple[str, dict[str, Any]]:
    """Generates BuiltinObject.__repr__."""
    repr_properties = [prop for prop in cls.__properties__.values() if prop.is_repr]
    if not repr_properties:
        if cls.__is_node__:
            repr_impl = f"""\
def __repr__(self) -> str:
    return f"<{cls.__name__} {{self.path}}>"
__str__ = __repr__
"""
        else:
            repr_impl = f"""\
def __repr__(self) -> str:
    return "<{cls.__name__}>"
__str__ = __repr__
"""
        return repr_impl, {}

    def _get_scalar_repr(prop: IntoType, value_expr: str) -> str:
        """Get repr expression for a scalar value."""
        if prop.scalar_type == ScalarType.ENUM:
            return f"{value_expr}.name"
        elif prop.scalar_type in (
            ScalarType.PRIMITIVE,
            ScalarType.STRUCT,
            ScalarType.NODE_REFERENCE,
            ScalarType.NODE_VALUE,
        ):
            if prop.primitive_type == PrimitiveType.UUID:
                return f"str({value_expr})"
            elif prop.primitive_type in (
                PrimitiveType.DATETIME,
                PrimitiveType.DATE,
                PrimitiveType.TIME,
            ):
                return f"{value_expr}.isoformat()"
            else:
                return f"{value_expr}!r"
        else:
            assert_never(prop.scalar_type)

    # property parts
    repr_parts_lines: list[str] = []
    repr_parts_lines.append("property_reprs = []")
    has_required_repr_props = False

    for prop in repr_properties:
        prop_name = prop.name
        if prop.cardinality == TypeCardinality.SCALAR:
            if prop.is_required:
                scalar_expr = _get_scalar_repr(prop, f"self.{prop_name}")
                repr_parts_lines.append(f"property_reprs.append(f'{prop_name}={{{scalar_expr}}}')")
                has_required_repr_props = True
            else:
                scalar_expr = _get_scalar_repr(prop, f"{prop_name}")
                repr_parts_lines.extend(
                    f"""\
if ({prop_name} := self.{prop_name}) is not None:
    property_reprs.append(f'{prop_name}={{{scalar_expr}}}')""".splitlines()
                )
        elif prop.cardinality == TypeCardinality.LIST:
            if prop.scalar_type == ScalarType.ENUM:
                list_expr = f"'[' + ', '.join(x.name for x in self.{prop_name}) + ']'"
            else:
                list_expr = f"self.{prop_name}!r"
            repr_parts_lines.extend(
                f"""\
if self.{prop_name}:
    property_reprs.append(f'{prop_name}={{{list_expr}}}')""".splitlines()
            )
        elif prop.cardinality == TypeCardinality.MAP:
            assert prop.key_type is not None, f"{prop!r} has no key type"
            if prop.key_type.scalar_type == ScalarType.ENUM:
                key_repr = _get_scalar_repr(prop.key_type, "k")
                value_repr = _get_scalar_repr(prop, "v")
                map_expr = f"'{{' + ', '.join(f'{{{key_repr}}}: {{{value_repr}}}' for k, v in self.{prop_name}.items()) + '}}'"
            else:
                map_expr = f"self.{prop_name}!r"
            repr_parts_lines.extend(
                f"""\
if self.{prop_name}:
    property_reprs.append(f'{prop_name}={{{map_expr}}}')""".splitlines()
            )
        else:
            assert_never(prop.cardinality)

    # wrap in repr
    repr_parts_str = "\n".join(repr_parts_lines)
    if cls.__is_node__:
        if has_required_repr_props:
            inner_repr_impl = f"""\
{repr_parts_str}
return f"<{cls.__name__} {{self.path}} {{' '.join(property_reprs)}}>"
"""
        else:
            inner_repr_impl = f"""\
{repr_parts_str}
if property_reprs:
    return f"<{cls.__name__} {{self.path}} {{' '.join(property_reprs)}}>"
else:
    return f"<{cls.__name__} {{self.path}}>"
"""
    else:
        if has_required_repr_props:
            inner_repr_impl = f"""\
{repr_parts_str}
return f"<{cls.__name__} {{' '.join(property_reprs)}}>"
"""
        else:
            inner_repr_impl = f"""\
{repr_parts_str}
if property_reprs:
    return f"<{cls.__name__} {{' '.join(property_reprs)}}>"
else:
    return f"<{cls.__name__}>"
"""

    if cls.__is_frozen__:
        # cache _repr in __repr__ (frozen Struct)
        inner_repr_impl = inner_repr_impl.replace("return ", "self._repr = ")
        inner_repr_impl = textwrap.indent(inner_repr_impl, "    ")
        inner_repr_impl = f"if self._repr is None:\n{inner_repr_impl}\nreturn self._repr"
        inner_repr_impl = textwrap.indent(inner_repr_impl, "    ")
        repr_impl = f"""\
def __repr__(self) -> str:
{inner_repr_impl}
__str__ = __repr__
"""
    else:
        # no cache
        inner_repr_impl = textwrap.indent(inner_repr_impl, "    ")
        repr_impl = f"""\
def __repr__(self) -> str:
{inner_repr_impl}
__str__ = __repr__
"""

    return repr_impl, {}


def _generate_ref_impl[NodeT: "Node"](
    cls: type[NodeT], node_type: NodeType
) -> tuple[str, dict[str, Any]]:
    """Generates Node.__to_ref__ method."""
    from ..common.relation import NodeReference

    assert cls.__is_node__, f"{cls.__name__} is not a Node"
    if node_type == NodeType.BENCH:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        node_type=NodeType.{node_type.name},
        id=self.id,
        bench_id=self.id,
    )
"""
    elif node_type == NodeType.CUSTOM_NODE_INSTANCE:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        node_type=NodeType.{node_type.name},
        id=self.id,
        definition_id=self.definition_id,
        bench_id=self.bench_id,
    )
"""
    elif TraitType.IN_BENCH in cls.__traits__:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        node_type=NodeType.{node_type.name},
        id=self.id,
        bench_id=self.bench_id,
    )
"""
    else:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        node_type=NodeType.{node_type.name},
        id=self.id,
    )
"""

    return ref_impl, {"NodeReference": NodeReference, "NodeType": NodeType}


#
# Equality
#


def _generate_equals_impl[ObjectT: BuiltinObjectBase](
    cls: type[ObjectT],
    is_node: bool,
) -> tuple[str, dict[str, Any]]:
    """Generates BuiltinObject.equals method."""

    eq_properties = [
        prop
        for prop in cls.__properties__.values()
        if prop.is_eq and prop.is_wired and prop.ptr_prop is None
    ]
    assert eq_properties, f"{cls.__name__} has no properties to compare"
    cmp_strs = []
    for prop in eq_properties:
        cmp_str = _generate_property_cmp_impl(prop)
        cmp_strs.append(cmp_str)
    body_str = "\n".join(cmp_strs)
    body_str = textwrap.indent(body_str, "    ")

    equals_impl = f"""\
def equals(self, other, _identity_map: dict["UUID", "UUID"] = EMPTY_DICT) -> bool:
{body_str}
    return True
"""

    if not is_node:
        equals_impl += """\
__eq__ = equals
"""

    return equals_impl, {}


def _generate_property_cmp_impl(prop: Property) -> str:
    """Generate equality check code for a single property."""
    prop_name = prop.name

    scalar_cmps_str = _generate_scalar_cmp_impl(prop)
    if prop.cardinality == TypeCardinality.SCALAR:
        # scalar
        if prop.is_required:
            # required scalar
            return f"""\
if not ({scalar_cmps_str.format(self_val=f"self.{prop_name}", other_val=f"other.{prop_name}")}):
    return False"""
        else:
            # optional scalar
            return f"""\
if (self.{prop_name} is None) != (other.{prop_name} is None) or (self.{prop_name} is not None and not ({scalar_cmps_str.format(self_val=f"self.{prop_name}", other_val=f"other.{prop_name}")})):
    return False"""
    elif prop.cardinality == TypeCardinality.LIST:
        # list (always required)
        return f"""\
if len(self.{prop_name}) != len(other.{prop_name}):
    return False
for i in range(len(self.{prop_name})):
    if not ({scalar_cmps_str.format(self_val=f"self.{prop_name}[i]", other_val=f"other.{prop_name}[i]")}):
        return False"""
    elif prop.cardinality == TypeCardinality.MAP:
        # map (always required)
        if prop.scalar_type in (
            ScalarType.STRUCT,
            ScalarType.NODE_REFERENCE,
            ScalarType.NODE_VALUE,
        ):
            # maps with complex values need key-by-key comparison
            return f"""\
if len(self.{prop_name}) != len(other.{prop_name}):
    return False
for key in self.{prop_name}:
    if key not in other.{prop_name}:
        return False
    if not ({scalar_cmps_str.format(self_val=f"self.{prop_name}[key]", other_val=f"other.{prop_name}[key]")}):
        return False"""
        else:
            # maps with primitive/enum values can use direct comparison
            return f"""\
if self.{prop_name} != other.{prop_name}:
    return False"""
    else:
        assert_never(prop.cardinality)


def _generate_scalar_cmp_impl(prop: Property) -> str:
    """Generate the core scalar comparison logic. Returns a format string with {self_val} and {other_val} placeholders."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type and prop.primitive_type.is_float:
            return "{self_val} == {other_val} or abs({self_val} - {other_val}) < 1e-10"
        else:
            return "{self_val} == {other_val}"
    elif prop.scalar_type == ScalarType.ENUM:
        return "{self_val} == {other_val}"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE or prop.scalar_type == ScalarType.NODE_VALUE:
        return "{self_val}.id == {other_val}.id or _identity_map.get({self_val}.id, {self_val}.id) == _identity_map.get({other_val}.id, {other_val}.id)"
    elif prop.scalar_type == ScalarType.STRUCT:
        return "{self_val}.equals({other_val}, _identity_map=_identity_map)"
    else:
        assert_never(prop.scalar_type)


#
# Validation
#


def _generate_validate_impl[ObjectT: BuiltinObjectBase](
    cls: type[ObjectT],
) -> tuple[str, dict[str, Any]]:
    """Generates BuiltinObject.validate method."""
    validate_impl = """\
def validate(self) -> None:
    raise NotImplementedError
"""
    return validate_impl, {}


#
# Computed
#


def _generate_path_impl[NodeT: Node](cls: type[NodeT]) -> tuple[str, dict[str, Any]]:
    """Generates Node.path property (and Node._path_key helper)."""
    assert cls.__is_node__, f"{cls.__name__} is not a Node"

    # Node._path_key
    if "slug" in cls.__properties__:
        if "name" in cls.__properties__:
            path_key_str = """\
@property
def _path_key(self) -> str:
    return self.slug or self.name
"""
        else:
            path_key_str = """\
@property
def _path_key(self) -> str:
    return self.slug or f"{self.metatype.bench_name}[id={self.id}]"
"""
    elif "name" in cls.__properties__:
        path_key_str = """\
@property
def _path_key(self) -> str:
    return self.name
"""
    elif "title" in cls.__properties__:
        path_key_str = """\
@property
def _path_key(self) -> str:
    return self.title
"""
    else:
        path_key_str = """\
@property
def _path_key(self) -> str:
    return f"{self.metatype.bench_name}[id={self.id}]"
"""

    # Node.path
    if cls.__root_type__ is None:
        path_str = """\
path = _path_key
"""
    else:
        path_str = """\
@property
def path(self) -> str:
    path_parts: list[str] = []
    node = self
    while node is not None:
        path_parts.append(node._path_key)
        node = node.parent
    if not self._is_attached:
        path_parts.append("<detached>")
    return "/".join(reversed(path_parts))
"""

    path_impl = f"{path_key_str}\n{path_str}"
    return path_impl, {}


def _generate_property_property_impl(prop: Property) -> str:
    """The computed get/set property for a property reference."""

    ptr_prop = prop.ptr_prop
    assert ptr_prop is not None, f"no wired prop for {prop!r}"

    if prop.cardinality == TypeCardinality.SCALAR:
        # property scalar
        return f"""\
@property
def {prop.name}(self: "BuiltinObjectBase") -> "Property | None":
    value_ptr: PropertyReference | None = self.{ptr_prop.name}
    if value_ptr is not None:
        return value_ptr.resolve()
    else:
        return None

@{prop.name}.setter
def {prop.name}(self: "BuiltinObjectBase", value: "Property | None"):
    if value is None:
        {ptr_prop.name} = None
    else:
        {ptr_prop.name} = value.to_ref()
"""
    elif prop.cardinality == TypeCardinality.LIST:
        # property list
        return f"""\
@property
def {prop.name}(self: "BuiltinObjectBase") -> tuple["Property", ...]:
    value_ptrs: list[PropertyReference] = self.{ptr_prop.name}
    assert type(value_ptrs) is list, f"invalid {prop}: {{value_ptrs!r}}"
    return tuple(p.resolve() for p in value_ptrs)

@{prop.name}.setter
def {prop.name}(self: "BuiltinObjectBase", values: list["Property"]):
    self.{ptr_prop.name} = [p.to_ref() for p in values]
"""
    else:
        raise RuntimeError(f"unsupported cardinality: {prop.cardinality}")


def _generate_node_property_impl(prop: Property) -> str:
    """The computed get/set property for a node reference. Resolved against the active supergraph."""
    # NOTE :Performance: we could inline Supergraph.get into node property getters

    ptr_prop = prop.ptr_prop
    assert ptr_prop is not None, f"no wired prop for {prop!r}"
    is_node = prop.component.__is_node__

    if prop.cardinality == TypeCardinality.SCALAR:
        if is_node:
            getter = f"""\
@property
def {prop.name}(self: "BuiltinObjectBase") -> "Node | None":
    node_ptr: NodeReference | None = self.{ptr_prop.name}
    if node_ptr is not None:
        return self._supergraph.get(node_ptr.id)
    else:
        return None
"""
        else:
            getter = f"""\
@property
def {prop.name}(self: "BuiltinObjectBase") -> "Node | None":
    node_ptr: NodeReference | None = self.{ptr_prop.name}
    if node_ptr is not None:
        if self._supergraph is None:
            return None
        return self._supergraph.get(node_ptr.id)
    else:
        return None
"""

        if is_node:
            setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "BuiltinObjectBase", value: "Node | None"):
    if value is None:
        self._do_set("{ptr_prop.name}", None)
    else:
        self._do_set("{ptr_prop.name}", value.to_ref())
"""
        else:
            setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "BuiltinObjectBase", value: "Node | None"):
    if value is None:
        self.{ptr_prop.name} = None
    else:
        self.{ptr_prop.name} = value.to_ref()
"""

    elif prop.cardinality == TypeCardinality.LIST:
        if is_node:
            getter = f"""\
@property
def {prop.name}(self: "BuiltinObjectBase") -> tuple["Node", ...]:
    node_ptrs: list[NodeReference] = self.{ptr_prop.name}
    return tuple(self._supergraph.get(p.id) for p in node_ptrs)
"""
        else:
            getter = f"""\
@property
def {prop.name}(self: "BuiltinObjectBase") -> tuple["Node", ...]:
    node_ptrs: list[NodeReference] = self.{ptr_prop.name}
    if self._supergraph is None:
        return ()
    return tuple(self._supergraph.get(p.id) for p in node_ptrs)
"""

        if is_node:
            setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "BuiltinObjectBase", nodes: list["Node"]):
    self._do_set("{ptr_prop.name}", [n.to_ref() for n in nodes])
"""
        else:
            setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "BuiltinObjectBase", nodes: list["Node"]):
    self.{ptr_prop.name} = [n.to_ref() for n in nodes]
"""
    else:
        raise RuntimeError(f"unsupported cardinality: {prop.cardinality}")

    return getter + "\n\n" + setter


def _generate_node_key_property_impl(obj_key: str, ptr_key: str, prop: Property) -> str:
    """The computed get property from a specific attribute of a node pointer."""

    ptr_prop = prop.ptr_prop
    assert ptr_prop is not None, f"no wired prop for {prop!r}"

    if prop.cardinality == TypeCardinality.SCALAR:
        return f"""\
@property
def {prop.name}_{obj_key}(self: "BuiltinObjectBase") -> "Node | None":
    node_ptr: NodeReference | None = self.{ptr_prop.name}
    if node_ptr is not None:
        return node_ptr.{ptr_key}
    else:
        return None
"""
    else:
        return f"""\
@property
def {prop.name}_{obj_key}(self: "BuiltinObjectBase") -> tuple["Node", ...]:
    node_ptrs: list[NodeReference] = self.{ptr_prop.name}
    return tuple(node_ptr.{ptr_key} for node_ptr in node_ptrs)
"""


def _generate_node_ancestor_property_impl(
    object_type: NodeType | StructType, prop: Property
) -> str:
    """The machine get property for Node ancestors."""

    node_types_str = ", ".join(str(t.value) for t in prop.node_types or ())
    assert node_types_str, f"no node types for {prop!r}"

    if prop.edge_type == EdgeType.ANCESTOR and object_type in (prop.node_types or ()):
        return f"""\
@property
def {prop.name}(self: "Node") -> "Node":
    return self

@property
def {prop.name}_ptr(self: "Node") -> "NodeType":
    return self.to_ref()
"""
    else:
        return f"""\
@property
def {prop.name}(self: "Node") -> "Node | None":
    node = self.parent
    while node is not None:
        if node.metatype in ({node_types_str},):
            return node
        node = node.parent
    return None

@property
def {prop.name}_ptr(self: "Node") -> "NodeType":
    node = self
    while node is not None:
        if node.metatype in ({node_types_str},):
            return node.to_ref()
        node = node.parent
    return None
"""


def _process_object_cls[ObjectT: BuiltinObjectBase](
    cls: type[ObjectT],
    object_type: NodeType | StructType | None,
    is_frozen: bool = False,
    is_concrete: bool = False,
    is_struct: bool = False,
    is_node: bool = False,
    is_root_node: bool = False,
    traits: tuple[TraitType, ...] = (),
) -> tuple[type[ObjectT], dict[str, "Property"]]:
    """Process a BuiltinObject base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"
    assert cls not in _processed_classes, f"class {cls.__name__} has already been processed"

    cls.__is_struct__ = is_struct
    cls.__is_node__ = is_node
    cls.__is_frozen__ = is_frozen

    metatype = Property(
        id=1,
        name="metatype",
        default=None,
        py_type=NodeType if is_node else StructType,
        cardinality=TypeCardinality.SCALAR,
        is_required=True,
        is_computed=True,  # is set statically by class decorator
        is_wired=True,
        is_stored=False,
        primitive_type=PrimitiveType.INT16,
        enum_type=EnumType.NODE_TYPE if is_node else EnumType.STRUCT_TYPE,
        component=cls,
    )

    # collect all components from class hierarchy (including self)
    components: list[type[BuiltinObjectBase]] = []
    for base_cls in get_superclasses(cls):
        base_cls = _processed_classes.get(base_cls, base_cls)
        if base_cls.__name__ == "ABC":
            continue
        if hasattr(base_cls, "__properties__"):
            components.append(base_cls)

    # collect properties from this class definition
    properties: dict[str, Property] = {"metatype": metatype}
    for name, prop in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(prop).__name__.startswith("_")
            or inspect.ismethod(prop)
            or inspect.isfunction(prop)
            or isinstance(prop, (property, classmethod, staticmethod, dualmethod))
        ):
            continue  # ignore reserved names and non-fields
        if not isinstance(prop, Property):
            raise TypeError(f"{cls.__name__}.{name} is not a Property: {prop} ({type(prop)})")
        prop.name = intern(name)
        prop.component = cls
        prop.py_type = cls.__annotations__.get(name, None)
        properties[name] = prop
    cls.__declared_properties__ = frozendict(properties)

    # collect properties from ancestor components (closest first)
    for component in reversed(components[1:]):
        for name, prop in component.__declared_properties__.items():
            existing = properties.get(name)
            if existing is not None:
                if existing.name in ("metatype", "parent", "_supergraph"):
                    continue  # may be narrowed
                raise RuntimeError(
                    f"property '{name}' from '{component.__name__}' conflicts with '{cls.__name__}': {prop!r}, {existing!r}"
                )
            prop = prop.clone()
            prop.component = cls
            properties[name] = prop

    # determine property types
    for prop in tuple(properties.values()):
        prop.determine(object_type)
        if prop.ptr_prop is not None:
            properties[prop.ptr_prop.name] = prop.ptr_prop

    # index properties
    cls.__properties__ = frozendict(properties)
    properties_by_id: dict[int, Property] = {}
    for prop in properties.values():
        if prop.id is not None and prop.runtime_prop is None:
            existing = properties_by_id.get(prop.id, None)
            if existing is None:
                properties_by_id[prop.id] = prop
            elif not prop.runtime_prop:
                # contributed reference properties can share an id
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
    props = properties.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__node_properties__ = frozendict(
        {
            p.name: p
            for p in props
            if p.scalar_type == ScalarType.NODE_REFERENCE and not p.runtime_prop
        }
    )
    cls.__wired_properties__ = frozendict(
        {p.name: p for p in props if p.is_wired is True and p.ptr_prop is None}
    )
    cls.__stored_properties__ = frozendict(
        {p.name: p for p in props if p.is_stored is True and p.ptr_prop is None}
    )
    if is_frozen:
        cls.__tracked_properties__ = frozendict()
    else:
        cls.__tracked_properties__ = frozendict(
            {
                p.name: p
                for p in props
                if not p.is_managed and not p.is_computed and p.ptr_prop is None
            }
        )

    # assign property ordinals
    cls.__properties_in_order__ = tuple(
        sorted(properties_by_id.values(), key=lambda p: cast(int, p.id))
    )
    for i, prop in enumerate(cls.__properties_in_order__):
        prop.component = cls
        prop.ord = i
        if prop.ptr_prop:
            prop.ptr_prop.ord = i
    cls.__properties_id_in_order__ = tuple(cast(int, p.id) for p in cls.__properties_in_order__)
    cls.__max_property_ord__ = len(cls.__properties_in_order__) - 1
    cls.__properties_mask_set__ = bitarray(cls.__max_property_ord__ + 1)
    cls.__properties_mask_set__.setall(True)
    cls.__properties_mask_unset__ = bitarray(cls.__max_property_ord__ + 1)

    cls_dict = dict(cls.__dict__)

    # define final methods in leaf classes
    if is_concrete:
        assert object_type is not None, f"concrete objects need a type: {cls.__name__}"

        # __init__
        glbls = {
            "ACTIVE_SESSION": ACTIVE_SESSION,
            "EMPTY_LIST": frozenlist(),
            "EMPTY_DICT": frozendict(),
            "uuid4": uuid4,
        }
        init_str, init_glbls = _generate_init_impl(
            cls,
            is_node=is_node,
            is_frozen=is_frozen,
            is_root_node=is_root_node,
            traits=traits,
            properties=properties,
        )
        exec_(init_str, {**glbls, **init_glbls}, cls_dict, f"{cls.__name__}:init")
        # __repr__
        repr_str, repr_glbls = _generate_repr_impl(cls)
        exec_(repr_str, {**glbls, **repr_glbls}, cls_dict, f"{cls.__name__}:repr")
        # equals
        equals_str, equals_glbls = _generate_equals_impl(cls, is_node=is_node)
        exec_(equals_str, {**glbls, **equals_glbls}, cls_dict, f"{cls.__name__}:equals")
        # validate
        validate_str, validate_glbls = _generate_validate_impl(cls)
        exec_(validate_str, {**glbls, **validate_glbls}, cls_dict, f"{cls.__name__}:validate")
        if is_node:
            # __to_ref__
            ref_str, ref_glbls = _generate_ref_impl(cast(type["Node"], cls), NodeType(object_type))
            exec_(ref_str, {**glbls, **ref_glbls}, cls_dict, f"{cls.__name__}:to_ref")
            # path
            path_str, path_glbls = _generate_path_impl(cast(type["Node"], cls))
            exec_(path_str, {**glbls, **path_glbls}, cls_dict, f"{cls.__name__}:path")
        # pack/unpack are generated after setup because we need all classes

        # add computed properties to concrete classes
        for prop in properties.values():
            if prop.runtime_prop is not None:
                continue  # not a contributed property
            # computed property property
            if prop.struct_type == StructType.PROPERTY_REFERENCE:
                property_property_str = _generate_property_property_impl(prop)
                exec_(
                    property_property_str,
                    {},
                    cls_dict,
                    f"{cls.__name__}:property_property:{prop.name}",
                )
            # computed node property
            elif prop.edge_type in (
                EdgeType.PARENT,
                EdgeType.REGULAR,
                EdgeType.TEMPLATE,
            ):
                node_property_str = _generate_node_property_impl(prop)
                exec_(node_property_str, {}, cls_dict, f"{cls.__name__}:node_property:{prop.name}")
            # computed node ancestor property
            elif prop.edge_type == EdgeType.ANCESTOR:
                ancestor_property_str = _generate_node_ancestor_property_impl(object_type, prop)
                exec_(
                    ancestor_property_str,
                    {},
                    cls_dict,
                    f"{cls.__name__}:ancestor_property:{prop.name}",
                )
            # computed _x node reference properties (e.g., parent_id, node_ck, node_type, ...)
            if prop.scalar_type == ScalarType.NODE_REFERENCE:
                for obj_key, ptr_key in (("id", "id"), ("ck", "ck"), ("type", "node_type")):
                    if obj_key == "type" and (not prop.node_types or len(prop.node_types) <= 1):
                        continue  # no need for *_type if only one possible node type
                    node_key_property_str = _generate_node_key_property_impl(obj_key, ptr_key, prop)
                    exec_(
                        node_key_property_str,
                        {},
                        cls_dict,
                        f"{cls.__name__}:node_key_property:{prop.name}:{obj_key}",
                    )

        # freeze
        if is_frozen:
            pass  # do nothing since a custom __setattr__ kills :Performance?

    # slots
    cls_dict.pop("__dict__", None)
    cls_dict.pop("__weakref__", None)
    if is_concrete:  # (only define actual slots in leaf, otherwise slots clash)
        for prop in properties.values():
            if isinstance(cls_dict.get(prop.name), Property):
                cls_dict.pop(prop.name, None)
        cls_dict["__slots__"] = tuple(
            p.name for p in properties.values() if p.ptr_prop is None and not p.is_computed
        )
    else:
        cls_dict["__slots__"] = ()

    # create the new class
    original_cls = cls
    cls = cast(type[ObjectT], type(cls.__name__, cls.__bases__, cls_dict))
    _processed_classes[original_cls] = cls
    del original_cls

    # update cls references in props
    for prop in properties.values():
        prop.component = cls

    return cls, properties  # type: ignore


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def object_[ObjectT: BuiltinObjectBase](
    object_type: NodeType | StructType | None = None,
    frozen: bool = False,
    concrete: bool = False,
    struct: bool = False,
    node: bool = False,
):
    """
    Mark a class as an object component (or concrete struct for a StructType).
    """

    def decorate(cls_in: type[ObjectT]) -> type[ObjectT]:
        cls, _properties = _process_object_cls(
            cls=cast(Any, cls_in),
            object_type=object_type,
            is_frozen=frozen,
            is_concrete=concrete,
            is_struct=struct,
            is_node=node,
        )
        return cast(type[ObjectT], cls)

    return decorate


_HANDLING_ATTRIBUTE_ERROR = contextvars.ContextVar("handling_attribute_error", default=False)


class BuiltinObjectBase[ObjectDataT: AnyObjectData](abc.ABC):
    """The base for all intrinsic objects like Structs and Nodes and all their derivatives."""

    __is_frozen__: ClassVar[bool] = False
    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False
    __is_trait__: ClassVar[bool] = False

    __properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}

    __declared_properties__: ClassVar[dict[str, Property]] = {}
    __node_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}
    __tracked_properties__: ClassVar[dict[str, Property]] = {}

    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = UNSET
    __properties_mask_set__: ClassVar[bitarray] = UNSET
    __properties_mask_unset__: ClassVar[bitarray] = UNSET

    def equals(
        self,
        other: Self | Any,
        _identity_map: Mapping[UUID, UUID] = EMPTY_DICT,
    ) -> bool:
        """Checks if the content of the two objects is equal (recursively)."""
        raise NotImplementedError  # generated

    @final
    def validate(self) -> None:
        """Validate the object."""
        raise NotImplementedError  # generated

    def hash(self) -> int:
        """Hash of content properties."""
        raise NotImplementedError  # generated

    def _clone_kwargs(self, reset: bool = True):
        """Clone kwargs for a new instance."""
        raise NotImplementedError

    def clone(self, *, reset: bool = True, **kwargs) -> Self:
        """
        Create a clone of this object and its descendants (structs/nodes) with the same content.
        """
        copy_kwargs = self._clone_kwargs(reset=reset)
        copy_kwargs.update(kwargs)
        return self.__class__(**copy_kwargs)

    def replace_references(
        self,
        new_node_by_id: Mapping[UUID, "Node"],
        exclude: Collection[EdgeType],
    ):
        """Replaces Node references with new Nodes. Missing Nodes are kept as is."""
        raise NotImplementedError  # generated

    def __bool__(self):
        return True  # support truthy checks for objects

    @classmethod
    def __pack_proto__(cls, _object: Self) -> ObjectDataT:
        """Convert to wire format"""
        raise NotImplementedError  # generated

    @classmethod
    def __unpack_proto__(
        cls,
        _object_data: ObjectDataT,
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        """Convert from wire format"""
        raise NotImplementedError  # generated

    @final
    def to_proto(self) -> ObjectDataT:
        """Convert to wire format"""
        raise NotImplementedError  # generated (usually = __pack_proto__)

    @classmethod
    def from_proto(
        cls,
        _object_data: ObjectDataT,
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        """Convert from wire format"""
        raise NotImplementedError  # generated

    @classmethod
    def __pack_value__(cls, _object: Self) -> dict:
        """Convert to value format"""
        raise NotImplementedError  # generated

    @classmethod
    def __unpack_value__(
        cls,
        _object_value: dict,
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        """Convert from value format"""
        raise NotImplementedError  # generated

    @final
    def to_value(self) -> dict:
        """Convert to value format"""
        raise NotImplementedError  # generated (usually = __pack_value__)

    @classmethod
    def from_value(
        cls,
        _object_value: dict,
        _session: "Session | None" = None,
        _supergraph: "Supergraph | None" = None,
        _graph: "Graph | None" = None,
        _connection: "QueryConnection | None" = None,
    ) -> Self:
        """Convert from value format"""
        raise NotImplementedError  # generated

    @classmethod
    def property(cls, name: str) -> Property:
        """Get a Property by name."""
        prop = cls.__properties__.get(name)
        if prop is None:
            raise ValueError(f"no property '{name}' in {cls.__name__}")
        return prop

    @classmethod
    def _unmask_properties_ids(cls, mask: bitarray) -> tuple[int, ...]:
        return tuple(cls.__properties_id_in_order__[i] for i in mask.search(True))

    @classmethod
    def _unmask_properties(cls, mask: bitarray) -> tuple[Property, ...]:
        return tuple(cls.__properties_in_order__[i] for i in mask.search(True))

    @classmethod
    def _mask_properties(cls, properties: Collection[Property]) -> bitarray:
        mask = bitarray(cls.__max_property_ord__ + 1)
        for prop in properties:
            assert prop.ord is not None, f"{prop!r} has no ordinal"
            mask[prop.ord] = True
        return mask

    @classmethod
    def _mask_properties_ids(cls, properties: Collection[int]) -> bitarray:
        mask = bitarray(cls.__max_property_ord__ + 1)
        for prop_id in properties:
            prop = cls.__properties_by_id__[prop_id]
            assert prop.ord is not None, f"{prop!r} has no ordinal"
            mask[prop.ord] = True
        return mask


@object_()
class BuiltinObjectMutable[ObjectDataT: AnyObjectData](BuiltinObjectBase[ObjectDataT]):
    """A mutable BuiltinObject."""

    _supergraph: "Supergraph | None" = property_runtime_()


@object_(frozen=True)
class BuiltinObjectFrozen[ObjectDataT: AnyObjectData](BuiltinObjectBase[ObjectDataT]):
    """A frozen BuiltinObject."""

    _supergraph: "Supergraph | None" = property_runtime_()
