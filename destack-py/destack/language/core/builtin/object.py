import base64
import contextvars
import inspect
import json
import textwrap
import time
from collections.abc import Mapping
from enum import Enum
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Self,
    assert_never,
    cast,
    dataclass_transform,
    final,
)

import structlog
from opentelemetry import trace

from destack.utils.code import exec_, format_code
from destack.utils.env import IS_DEV, IS_TEST
from destack.utils.frozen import frozendict, frozenlist
from destack.utils.func import dualmethod, get_superclasses
from destack.utils.hash import hash_bool, hash_bytes, hash_float, hash_int, hash_string
from destack.utils.string import Casing, to_casing
from destack.utils.uuid import UUID, to_nano_id, uuid4, uuid7

from .common import (
    EdgeType,
    Encoding,
    EnumType,
    NodeType,
    ObjectKind,
    PrimitiveType,
    ScalarType,
    StructType,
    TraitType,
    TypeCardinality,
    ValueFactory,
)
from .const import (
    ACTIVE_BRANCH,
    ACTIVE_EVENT,
    ACTIVE_SESSION,
    ACTIVE_SNAPSHOT,
    ACTIVE_SPACE,
    EMPTY_DICT,
    ENCODERS,
    REGION,
    UNSET,
)
from .meta import ConstantDeclaration, builtin_method
from .property import (
    _PROPERTY_SPECIFIERS,
    PropertyDeclaration,
    TypeDeclaration,
    builtin_property_runtime,
)

if TYPE_CHECKING:
    from destack.language import BinaryReader, BinaryWriter, Graph, Node, Session

# pyright: reportIncompatibleVariableOverride=false

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

__is_finalized__ = False


def _is_finalized() -> bool:
    return __is_finalized__


def _set_finalized():
    global __is_finalized__
    __is_finalized__ = True


def get_tk_b64_from_ck(ck: UUID) -> str:
    """Gets the stable across templates first 4 bytes of the ck."""
    return base64.b64encode(ck.bytes).decode()


_processed_classes: dict[type["BuiltinObject"], type["BuiltinObject"]] = {}


def _generate_init[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    is_node: bool,
    is_entity: bool,
    is_root_node: bool,
    is_frozen: bool,
    traits: tuple[TraitType, ...],
    inherits: tuple[StructType, ...] | tuple[NodeType, ...],
    properties: dict[str, PropertyDeclaration],
) -> tuple[str, dict[str, Any]]:
    """Generates an __init__ for a BuiltinObject class."""

    extra_glbls: dict[str, Any] = {}

    # header properties
    header_properties = dict(properties)
    if is_node:
        header_properties.pop("_ref")
        header_properties.pop("_is_new")
    properties_in_order = list(header_properties.values())
    properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
    required_properties = [
        p
        for p in properties_in_order
        if not p.is_computed
        and p.default is UNSET
        and p.default_factory is None
        and not p.is_internal
        and p.cardinality == TypeCardinality.SCALAR
        and p.scalar_type
        != ScalarType.NODE_REFERENCE  # passed either as node or node_ptr, defer check
    ]

    # header
    method_header_lines = ["def __init__(self, *"]
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
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            method_header_lines.append(f"{prop.name}_ptr=None")

    method_header_lines.append(")")
    method_header = ", ".join(method_header_lines)

    # body
    # NOTE: frozen objects can use direct assignment, mutable objects can't
    #  (because of the custom __setattr__, that would add overhead for every set)
    extra_glbls["ACTIVE_SESSION"] = ACTIVE_SESSION
    extra_glbls["ACTIVE_SPACE"] = ACTIVE_SPACE
    extra_glbls["ACTIVE_BRANCH"] = ACTIVE_BRANCH
    extra_glbls["ACTIVE_SNAPSHOT"] = ACTIVE_SNAPSHOT
    extra_glbls["ACTIVE_EVENT"] = ACTIVE_EVENT
    extra_glbls["EMPTY_LIST"] = frozenlist()
    extra_glbls["EMPTY_DICT"] = frozendict()
    extra_glbls["uuid4"] = uuid4
    extra_glbls["uuid7"] = uuid7
    extra_glbls["to_nano_id"] = to_nano_id
    extra_glbls["REGION"] = REGION

    method_body_lines = []
    body_properties = dict(properties)
    if not is_frozen and is_node:
        method_body_lines.append("__setattr__ = object.__setattr__")
        set_template_str = "__setattr__(self, '{0}', {1})"
    else:
        set_template_str = "self.{0} = {1}"

    # setup
    if is_node:
        # node setup
        body_properties.pop("id")
        if NodeType.ENTITY in inherits:
            body_properties.pop("created_at")
            body_properties.pop("created_epoch")
            body_properties.pop("created_by")
            body_properties.pop("updated_at")
            body_properties.pop("updated_epoch")
            body_properties.pop("updated_by")
        elif NodeType.EVENT in inherits:
            body_properties.pop("created_at")
            body_properties.pop("created_epoch")
            body_properties.pop("created_by")
            body_properties.pop("client")
            body_properties.pop("client_created_at")
            body_properties.pop("client_epoch")
        else:
            raise NotImplementedError(f"unexpected node {cls.__name__} extends {inherits}")
        body_properties.pop("_session")
        body_properties.pop("_ref")
        body_properties.pop("_is_new")
        method_body_lines.append(f"""\
# session
if _session is None:
    _session = ACTIVE_SESSION.get()
    if _session is None:
        raise RuntimeError("no active session for {cls.__name__}")
{set_template_str.format("_session", "_session")}

# node identity
if id is None:
    """)
        if NodeType.ENTITY in inherits:
            method_body_lines.append("""\
    id = uuid4()
    _now = self._session.oracle.now()
    _epoch = self._session.epoch
    created_at = _now
    created_epoch = _epoch
    created_by_ptr = self._session.actor_ptr
    updated_at = _now
    updated_epoch = _epoch
    updated_by_ptr = self._session.actor_ptr
""")
        elif NodeType.EVENT in inherits:
            method_body_lines.append("""\
    id = uuid7()
    _now = self._session.oracle.now()
    _epoch = self._session.epoch
    created_epoch = _epoch
    created_at = _now
    created_by_ptr = self._session.actor_ptr
    client_ptr = self._session.client_ptr
    client_nonce = self._session.client_nonce
    client_epoch = _epoch
    client_created_at = _now
""")
        else:
            raise NotImplementedError(f"unexpected node {cls.__name__} extends {inherits}")
        method_body_lines.append(f"""\
    _is_new = True
else:
    _is_new = False
{set_template_str.format("id", "id")}
""")

        if NodeType.ENTITY in inherits:
            method_body_lines.append(f"""\
{set_template_str.format("created_at", "created_at")}
{set_template_str.format("created_epoch", "created_epoch")}
{set_template_str.format("created_by_ptr", "created_by_ptr")}
{set_template_str.format("updated_at", "updated_at")}
{set_template_str.format("updated_epoch", "updated_epoch")}
{set_template_str.format("updated_by_ptr", "updated_by_ptr")}
""")
        elif NodeType.EVENT in inherits:
            method_body_lines.append(f"""\
{set_template_str.format("created_at", "created_at")}
{set_template_str.format("created_epoch", "created_epoch")}
{set_template_str.format("created_by_ptr", "created_by_ptr")}
{set_template_str.format("client_ptr", "client_ptr")}
{set_template_str.format("client_nonce", "client_nonce")}
{set_template_str.format("client_created_at", "client_created_at")}
{set_template_str.format("client_epoch", "client_epoch")}
""")
        else:
            raise NotImplementedError(f"unexpected node {cls.__name__} extends {inherits}")
        method_body_lines.append(f"""\
{set_template_str.format("_ref", "None")}
{set_template_str.format("_is_new", "_is_new")}
""")

    # property assignments
    method_body_lines.append("# properties")
    body_properties_in_order = list(body_properties.values())
    body_properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
    for prop in body_properties_in_order:
        if prop.is_computed:
            # computed, can't assign
            continue

        arg_name = prop.name
        self_name = (
            prop.name if prop.scalar_type != ScalarType.NODE_REFERENCE else f"{prop.name}_ptr"
        )

        # cast node to node_ptr
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            method_body_lines.append(f"""\
if {arg_name} is not None:
    {self_name} = {arg_name}.to_ref()""")

        # init default factory
        if prop.default_factory is not None:
            if prop.default_factory == ValueFactory.UUID4:
                method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = uuid4()""")
            elif prop.default_factory == ValueFactory.UUID7:
                method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = uuid7()""")
            elif prop.default_factory == ValueFactory.NOW:
                if is_node:
                    method_body_lines.append(f"""\
if {arg_name} is None:
    assert self._session is not None, "no session for {cls.__name__}"
    {arg_name} = self._session.oracle.now()""")
                else:
                    method_body_lines.append(f"""\
if {arg_name} is None:
    session = ACTIVE_SESSION.get()
    if session is None:
        raise RuntimeError("no active session for {cls.__name__}")
    {arg_name} = session.oracle.now()""")
            elif prop.default_factory == ValueFactory.EPOCH:
                if is_node:
                    method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = self._session.epoch""")
                else:
                    method_body_lines.append(f"""\
if {arg_name} is None:
    session = ACTIVE_SESSION.get()
    if session is None:
        raise RuntimeError("no active session for {cls.__name__}")
    {arg_name} = session.epoch""")
            elif prop.default_factory == ValueFactory.ACTOR:
                method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name}_ptr = self._session.actor_ptr""")
            elif prop.default_factory == ValueFactory.CLIENT:
                method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name}_ptr = self._session.client_ptr""")
            elif prop.default_factory == ValueFactory.CLIENT_NONCE:
                method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = self._session.client_nonce""")
            elif prop.default_factory == ValueFactory.REGION:
                method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = REGION""")
            elif prop.default_factory == ValueFactory.SELF:
                assert is_node, f"{cls.__name__} is not a Node, cannot use self in {prop!r}"
                method_body_lines.append(f"""\
if {self_name} is None:
    {self_name} = self.to_ref()""")
            elif prop.default_factory == ValueFactory.SPACE:
                assert is_node, f"{cls.__name__} is not a Node, cannot use self in {prop!r}"
                method_body_lines.append(f"""\
if {self_name} is None:
    space = ACTIVE_SPACE.get()
    if space is None:
        raise RuntimeError("no active Space for {cls.__name__}")
    {self_name} = space.to_ref()""")
            elif prop.default_factory == ValueFactory.BRANCH:
                method_body_lines.append(f"""\
if {self_name} is None:
    branch = ACTIVE_BRANCH.get()
    if branch is None:
        raise RuntimeError("no active Branch for {cls.__name__}")
    {self_name} = branch.to_ref()""")
            elif prop.default_factory == ValueFactory.SNAPSHOT:
                method_body_lines.append(f"""\
if {self_name} is None:
    snapshot = ACTIVE_SNAPSHOT.get()
    if snapshot is None:
        raise RuntimeError("no active Snapshot for {cls.__name__}")
    {self_name} = snapshot.to_ref()""")
            elif prop.default_factory == ValueFactory.NAME:
                assert is_node, (
                    f"{cls.__name__} is not a Node, cannot use {prop.default_factory} in {prop!r}"
                )
                method_body_lines.append(f"""\
if {self_name} is None:
    {self_name} = "{cls.__name__}" """)
            else:
                assert_never(prop.default_factory)

        # check if node is passed if required and scalar
        if prop.is_required and prop.cardinality == TypeCardinality.SCALAR:
            method_body_lines.append(f"""\
if {self_name} is None:
    raise AttributeError(f"{cls.__name__}.{prop.name} is required")""")

        # init list/map if unset
        if prop.cardinality == TypeCardinality.LIST:
            method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = {"[]" if not is_frozen else "EMPTY_LIST"}""")
        elif prop.cardinality == TypeCardinality.MAP:
            method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = {"{}" if not is_frozen else "EMPTY_DICT"}""")

        # regular assignment
        if is_node and not is_frozen:
            method_body_lines.append(f"__setattr__(self, '{self_name}', {self_name})")
        else:
            method_body_lines.append(f"self.{self_name} = {self_name}")

    if is_node:
        if is_entity:
            method_body_lines.append(f"""\
# graph
{set_template_str.format("_graph", "_graph")}
""")
        else:  # is event
            method_body_lines.append(f"""\
# graph
{set_template_str.format("_graph", "_graph")}
""")

    method_body = "\n".join(method_body_lines) or "pass"
    method_body = textwrap.indent(method_body, "    ")
    init_str = f"{method_header}:\n{method_body}"
    if IS_DEV:
        init_str = format_code(init_str)
    return init_str, extra_glbls


def _generate_repr[ObjectT: BuiltinObject](
    cls: type[ObjectT],
) -> tuple[str, dict[str, Any]]:
    """Generates BuiltinObject.__repr__."""
    repr_properties = [prop for prop in cls.__properties__.values() if prop.is_repr]
    if not repr_properties:
        if cls.__is_node__:
            repr_impl = f"""\
def __repr__(self) -> str:
    return f"<{cls.__name__} \\"{{self.path}}\\">"
__str__ = __repr__
"""
        else:
            repr_impl = f"""\
def __repr__(self) -> str:
    return "<{cls.__name__}>"
__str__ = __repr__
"""
        return repr_impl, {}

    def _get_scalar_repr(prop: TypeDeclaration, value_expr: str) -> str:
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
return f"<{cls.__name__} \\"{{self.path}}\\" {{' '.join(property_reprs)}}>"
"""
        else:
            inner_repr_impl = f"""\
{repr_parts_str}
if property_reprs:
    return f"<{cls.__name__} \\"{{self.path}}\\" {{' '.join(property_reprs)}}>"
else:
    return f"<{cls.__name__} \\"{{self.path}}\\">"
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

    if cls.__is_frozen__ and not cls.__is_node__:
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


def _generate_ref[NodeT: "Node"](
    cls: type[NodeT], node_type: NodeType
) -> tuple[str, dict[str, Any]]:
    """Generates Node.__to_ref__ method."""
    from .relation import NodeReference

    assert cls.__is_node__, f"{cls.__name__} is not a Node"
    if node_type == NodeType.SPACE:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        type=NodeType.{node_type.name},
        id=self.id,
        space_id=self.id,
        branch_id=self.branch_ptr.id,
        snapshot_id=self.snapshot_ptr.id,
    )
"""
    elif node_type == NodeType.BRANCH:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        type=NodeType.{node_type.name},
        id=self.id,
        space_id=self.space_ptr.id,
        branch_id=self.id,
        snapshot_id=self.snapshot_ptr.id,
    )
"""
    elif node_type == NodeType.SNAPSHOT:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        type=NodeType.{node_type.name},
        id=self.id,
        space_id=self.space_ptr.id,
        branch_id=self.branch_ptr.id,
        snapshot_id=self.id,
    )
"""
    else:
        ref_impl = f"""\
def __to_ref__(self) -> "NodeReference":
    return NodeReference(
        type=NodeType.{node_type.name},
        id=self.id,
        space_id=self.space_ptr.id,
        branch_id=self.branch_ptr.id,
        snapshot_id=self.snapshot_ptr.id,
        definition_id=self.definition_ptr.id if self.definition_ptr is not None else None,
    )
"""

    return ref_impl, {"NodeReference": NodeReference, "NodeType": NodeType}


#
# Equality
#


def _generate_equals[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    is_node: bool,
) -> tuple[str, dict[str, Any]]:
    """Generate BuiltinObject.equals method."""

    eq_properties = [prop for prop in cls.__properties__.values() if prop.is_eq and prop.is_wired]
    assert eq_properties, f"{cls.__name__} has no properties to compare"
    cmp_strs = []
    for prop in eq_properties:
        cmp_str = _generate_property_cmp_impl(prop)
        cmp_strs.append(cmp_str)
    body_str = "\n".join(cmp_strs)
    body_str = textwrap.indent(body_str, "    ")

    equals_impl = f"""\
def equals(self, other) -> bool:
{body_str}
    return True
"""

    if not is_node:
        equals_impl += """\
__eq__ = equals
"""

    return equals_impl, {}


def _generate_property_cmp_impl(prop: PropertyDeclaration) -> str:
    """Generate equality check code for a single property."""
    prop_name = prop.name
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_name = f"{prop_name}_ptr"

    scalar_cmps_str, is_simple = _generate_scalar_cmp_impl(prop)
    if prop.cardinality == TypeCardinality.SCALAR:
        # scalar
        if prop.is_required or is_simple:
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


def _generate_scalar_cmp_impl(prop: PropertyDeclaration) -> tuple[str, bool]:
    """Generate the core scalar comparison logic. Returns a format string with {self_val} and {other_val} placeholders."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type and prop.primitive_type in (
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return "{self_val} == {other_val} or abs({self_val} - {other_val}) < 1e-10", False
        else:
            return "{self_val} == {other_val}", True
    elif prop.scalar_type == ScalarType.ENUM:
        return "{self_val} == {other_val}", True
    elif prop.scalar_type == ScalarType.NODE_REFERENCE or prop.scalar_type == ScalarType.NODE_VALUE:
        return "{self_val}.id == {other_val}.id", False
    elif prop.scalar_type == ScalarType.STRUCT:
        return "{self_val}.equals({other_val})", False
    else:
        assert_never(prop.scalar_type)


def _generate_hash[ObjectT: BuiltinObject](
    cls: type[ObjectT],
) -> tuple[str, dict[str, Any]]:
    """Generate BuiltinObject.hash method."""
    hash_properties = [
        prop for prop in cls.__properties__.values() if prop.is_hash and prop.is_wired
    ]
    assert hash_properties, f"{cls.__name__} has no properties to hash"
    hash_parts: list[str] = ["h = 1"]
    for prop in hash_properties:
        prop_hash_impl = _generate_property_hash_impl(prop)
        hash_parts.append(prop_hash_impl)
    hash_parts_str = "\n".join(hash_parts)

    if cls.__is_frozen__ and not cls.__is_node__:
        hash_impl = f"""\
def hash(self) -> int:
    if self._hash is not None:
        return self._hash

{textwrap.indent(hash_parts_str, "    ")}

    self._hash = h
    return h
"""
    else:
        hash_impl = f"""\
def hash(self) -> int:
{textwrap.indent(hash_parts_str, "    ")}
    return h
"""
    return hash_impl, {
        "hash_string": hash_string,
        "hash_bytes": hash_bytes,
        "hash_int": hash_int,
        "hash_float": hash_float,
        "hash_bool": hash_bool,
        "json": json,
    }


_SCALAR_HASH_TEMPLATE = "h = ((h * 31) + {value_expr}) & 0xFFFFFFFF"


def _generate_property_hash_impl(prop: PropertyDeclaration) -> str:
    """Generate a hash method for a single property."""
    prop_name = prop.name
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_name = f"{prop_name}_ptr"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            scalar_hash_str = _generate_scalar_hash_impl(prop, f"self.{prop_name}")
            return _SCALAR_HASH_TEMPLATE.format(value_expr=scalar_hash_str)
        else:
            scalar_hash_str = _generate_scalar_hash_impl(prop, prop_name)
            return f"""\
if ({prop_name} := self.{prop_name}) is not None:
    {_SCALAR_HASH_TEMPLATE.format(value_expr=scalar_hash_str)}"""
    elif prop.cardinality == TypeCardinality.LIST:
        scalar_hash_str = _generate_scalar_hash_impl(prop, "_item")
        return f"""\
if ({prop_name} := self.{prop_name}):
    for _item in {prop_name}:
        {_SCALAR_HASH_TEMPLATE.format(value_expr=scalar_hash_str)}"""
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"{prop.name} has no key type"
        scalar_hash_str = _generate_scalar_hash_impl(prop.key_type, "_key")
        scalar_hash_str = _generate_scalar_hash_impl(prop, "_value")
        return f"""\
if ({prop_name} := self.{prop_name}):
    for _key, _value in {prop_name}.items():
        {_SCALAR_HASH_TEMPLATE.format(value_expr=scalar_hash_str)}"""
    else:
        assert_never(prop.cardinality)


def _generate_scalar_hash_impl(prop: TypeDeclaration | PropertyDeclaration, value_expr: str) -> str:
    """Generate a hash method for a single scalar property."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.BOOLEAN:
            return f"hash_bool({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return f"hash_int({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return f"hash_float({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.DATETIME,
            PrimitiveType.DATE,
            PrimitiveType.TIME,
        ):
            return f"hash_string({value_expr}.isoformat())"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"hash_float({value_expr}.total_seconds())"
        elif prop.primitive_type == PrimitiveType.STRING:
            return f"hash_string({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"{value_expr}.int"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"hash_bytes({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return f"hash_string(json.dumps({value_expr}))"
        else:
            assert_never(prop.primitive_type)
    elif prop.scalar_type == ScalarType.ENUM:
        return value_expr
    elif prop.scalar_type == ScalarType.STRUCT:
        return f"{value_expr}.hash()"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise NotImplementedError(f"cannot hash node value: {prop!r}")
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"{value_expr}.id.int"
    else:
        assert_never(prop.scalar_type)


#
# Validation
#


def _generate_validate[ObjectT: BuiltinObject](
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


def _generate_path[NodeT: Node](cls: type[NodeT]) -> tuple[str, dict[str, Any]]:
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
    return self.slug or f"{self.metatype.destack_name}[id={self.id}]"
"""
    elif "name" in cls.__properties__:
        path_key_str = """\
@property
def _path_key(self) -> str:
    return self.name
"""
    else:
        path_key_str = f"""\
@property
def _path_key(self) -> str:
    return f"{cls.__name__}[id={{self.id}}]"
"""

    # Node.path
    if cls.metatype == NodeType.SPACE:
        path_str = """\
path = _path_key
"""
    else:
        path_str = f"""\
@property
def path(self) -> str:
    path_parts: list[str] = []
    node = self
    last_node = self
    while node is not None:
        path_parts.append(node._path_key)
        last_node = node
        node = node.parent
    if last_node.metatype != {NodeType.SPACE.value}:
        path_parts.append("<detached>")
    return "/".join(reversed(path_parts))
"""

    path_impl = f"{path_key_str}\n{path_str}"
    return path_impl, {}


def _generate_node_property_impl(prop: PropertyDeclaration) -> str:
    """The computed get/set property for a node reference. Resolved against the active supergraph."""
    # NOTE :Performance: we could inline Supergraph.get into node property getters

    assert prop.cardinality == TypeCardinality.SCALAR, f"node properties must be scalar: {prop!r}"
    is_node = prop.component.__is_node__

    if is_node:
        getter = f"""\
@property
def {prop.name}(self: "BuiltinObject") -> "Node | None":
    node_ptr: NodeReference | None = self.{prop.name}_ptr
    if node_ptr is not None:
        return self._session.graph.get(node_ptr.id, node_ptr.space_id, node_ptr.branch_id, node_ptr.snapshot_id)
    else:
        return None
"""
    else:
        getter = f"""\
@property
def {prop.name}(self: "BuiltinObject") -> "Node | None":
    node_ptr: NodeReference | None = self.{prop.name}_ptr
    if node_ptr is not None:
        if self._session is None:
            return None
        return self._session.graph.get(node_ptr.id, node_ptr.space_id, node_ptr.branch_id, node_ptr.snapshot_id)
    else:
        return None
"""

    if is_node:
        setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "BuiltinObject", value: "Node | None"):
    if value is None:
        self._do_set("{prop.name}_ptr", None)
    else:
        self._do_set("{prop.name}_ptr", value.to_ref())
"""
    else:
        setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "BuiltinObject", value: "Node | None"):
    if value is None:
        self.{prop.name}_ptr = None
    else:
        self.{prop.name}_ptr = value.to_ref()
"""

    return getter + "\n\n" + setter


_time_spent_in_process_object_cls = 0


def _process_object_cls[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    object_type: NodeType | StructType | None,
    is_frozen: bool,
    is_concrete: bool,
    is_struct: bool,
    is_node: bool,
    is_entity: bool,
    is_root_node: bool,
    is_abstract: bool,
    base_type: StructType | NodeType | None,
    traits: tuple[TraitType, ...],
    inherits: tuple[StructType, ...] | tuple[NodeType, ...] = (),
) -> tuple[type[ObjectT], dict[str, "PropertyDeclaration"]]:
    """Process a BuiltinObject base class and return the processed class and its properties."""
    global _time_spent_in_process_object_cls
    start = time.time()
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"
    assert cls not in _processed_classes, f"class {cls.__name__} has already been processed"

    cls.__is_struct__ = is_struct
    cls.__is_node__ = is_node
    cls.__is_frozen__ = is_frozen

    metatype = PropertyDeclaration(
        id=1,
        name="metatype",
        default=None,
        py_type=NodeType if is_node else StructType,
        cardinality=TypeCardinality.SCALAR,
        is_required=True,
        is_computed=True,  # is set statically by class decorator
        is_wired=True,
        is_stored=False,
        primitive_type=PrimitiveType.SINT32,
        enum_type=EnumType.NODE_TYPE if is_node else EnumType.STRUCT_TYPE,
        component=cls,
    )

    # check for redundant components
    if IS_DEV or IS_TEST:
        for component in cls.__bases__:
            if component.__name__ in ("ABC", "object", "Generic"):
                continue
            for other_component in cls.__bases__:
                if component.__name__ != other_component.__name__ and component in get_superclasses(
                    other_component
                ):
                    raise AssertionError(
                        f"'{cls.__name__}' has redundant component '{component.__name__}' (already inherits from '{other_component.__name__}')"
                    )

    # collect all components from class hierarchy (including self)
    components: list[type[BuiltinObject]] = []
    for base_cls in get_superclasses(cls):
        base_cls = _processed_classes.get(base_cls, base_cls)
        if base_cls.__name__ == "ABC":
            continue
        if hasattr(base_cls, "__properties__"):
            components.append(base_cls)

    # collect properties from this class definition
    properties: dict[str, PropertyDeclaration] = {"metatype": metatype}
    for name, attribute in list(cls.__dict__.items()):
        if (
            name.startswith("__")
            or type(attribute).__name__.startswith("_")
            or inspect.ismethod(attribute)
            or inspect.isfunction(attribute)
            or isinstance(attribute, (property, classmethod, staticmethod, dualmethod))
        ):
            continue  # ignore reserved names and non-fields
        elif isinstance(attribute, PropertyDeclaration):
            attribute.name = intern(name)
            attribute.component = cls
            if attribute.original_component is UNSET:
                attribute.original_component = cls
            attribute.py_type = cls.__annotations__.get(name, None)
            properties[name] = attribute
        elif isinstance(attribute, ConstantDeclaration):
            attribute.name = intern(name)
            attribute.component = cls
            if attribute.original_component is UNSET:
                attribute.original_component = cls
        else:
            raise TypeError(
                f"{cls.__name__}.{name} is not a Property or Constant: {attribute} ({type(attribute)})"
            )
    cls.__declared_properties__ = frozendict(properties)

    # add properties from ancestor components (closest first)
    for component in components[1:]:
        for name, prop in component.__declared_properties__.items():
            existing = properties.get(name)
            if existing is not None:
                if existing.name in ("id", "metatype", "parent", "definition", "_graph"):
                    continue  # may be narrowed/duplicated
                elif (
                    component.__is_trait__ or component.__is_abstract__
                ) and existing.id == prop.id:
                    continue  # may be overridden by the trait
                else:
                    # error: property conflicts with ancestor component
                    raise RuntimeError(
                        f"property '{name}' from '{component.__name__}' conflicts with '{cls.__name__}': {prop!r}, {existing!r}"
                    )
            # add property
            prop = prop.clone()
            prop.component = cls
            if prop.original_component is UNSET:
                prop.original_component = component
            properties[name] = prop

    # determine property types
    for prop in tuple(properties.values()):
        prop.determine(object_type, is_root_node=is_root_node and not is_abstract)

    # index properties
    cls.__properties__ = frozendict(properties)
    properties_by_id: dict[int, PropertyDeclaration] = {}
    properties_by_alias: dict[str, PropertyDeclaration] = {}
    for prop in properties.values():
        # index by id
        if prop.id is not None:
            existing = properties_by_id.get(prop.id, None)
            if existing is None:
                properties_by_id[prop.id] = prop
            else:
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
        # index by aliases (if not runtime only)
        if prop.is_wired:
            lower_camel_name = to_casing(prop.name, Casing.LOWER_CAMEL)
            upper_camel_name = to_casing(prop.name, Casing.CAMEL)
            if prop.scalar_type == ScalarType.NODE_REFERENCE:
                aliases = (
                    prop.name,
                    prop.name + "_ptr",
                    lower_camel_name,
                    lower_camel_name + "Ptr",
                    upper_camel_name,
                    upper_camel_name + "Ptr",
                )
            else:
                aliases = (prop.name, lower_camel_name, upper_camel_name)
            for alias in aliases:
                existing = properties_by_alias.get(alias)
                if existing is None:
                    properties_by_alias[alias] = prop
                elif existing is not prop:
                    raise ValueError(f"property alias conflict: {prop!r}, {existing!r}")
    props = properties.values()
    cls.__properties_by_alias__ = frozendict(properties_by_alias)
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__node_properties__ = frozendict(
        {p.name: p for p in props if p.scalar_type == ScalarType.NODE_REFERENCE}
    )
    cls.__wired_properties__ = frozendict({p.name: p for p in props if p.is_wired is True})
    cls.__stored_properties__ = frozendict({p.name: p for p in props if p.is_stored is True})
    if is_frozen:
        cls.__tracked_properties__ = frozendict()
    else:
        cls.__tracked_properties__ = frozendict(
            {p.name: p for p in props if not p.is_internal and not p.is_computed}
        )

    # assign property ordinals
    cls.__properties_in_order__ = tuple(
        sorted(properties_by_id.values(), key=lambda p: cast(int, p.id))
    )
    for i, prop in enumerate(cls.__properties_in_order__):
        prop.component = cls
        if prop.original_component is UNSET:
            prop.original_component = cls
        prop.ord = i
    cls.__properties_id_in_order__ = tuple(cast(int, p.id) for p in cls.__properties_in_order__)

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
        if not is_abstract:
            init_str, init_glbls = _generate_init(
                cls,
                is_node=is_node,
                is_entity=is_entity,
                is_frozen=is_frozen,
                is_root_node=is_root_node,
                properties=properties,
                traits=traits,
                inherits=inherits,
            )
            exec_(init_str, {**glbls, **init_glbls}, cls_dict, f"{cls.__name__}:init")
            # __repr__
            repr_str, repr_glbls = _generate_repr(cls)
            exec_(repr_str, {**glbls, **repr_glbls}, cls_dict, f"{cls.__name__}:repr")
            # equals
            equals_str, equals_glbls = _generate_equals(cls, is_node=is_node)
            exec_(equals_str, {**glbls, **equals_glbls}, cls_dict, f"{cls.__name__}:equals")
            # hash
            hash_str, hash_glbls = _generate_hash(cls)
            exec_(hash_str, {**glbls, **hash_glbls}, cls_dict, f"{cls.__name__}:hash")
            # validate
            validate_str, validate_glbls = _generate_validate(cls)
            exec_(validate_str, {**glbls, **validate_glbls}, cls_dict, f"{cls.__name__}:validate")
            if is_node:
                # __to_ref__
                ref_str, ref_glbls = _generate_ref(cast(type["Node"], cls), NodeType(object_type))
                exec_(ref_str, {**glbls, **ref_glbls}, cls_dict, f"{cls.__name__}:to_ref")
                # path
                path_str, path_glbls = _generate_path(cast(type["Node"], cls))
                exec_(path_str, {**glbls, **path_glbls}, cls_dict, f"{cls.__name__}:path")
        # pack/unpack are generated after setup because we need all classes

        # add computed properties to concrete classes
        for prop in properties.values():
            if prop.edge_type in (EdgeType.PARENT, EdgeType.REGULAR):
                node_property_str = _generate_node_property_impl(prop)
                exec_(node_property_str, {}, cls_dict, f"{cls.__name__}:node_property:{prop.name}")

        # freeze
        if is_frozen:
            pass  # do nothing since a custom __setattr__ kills :Performance?

    # slots
    cls_dict.pop("__dict__", None)
    cls_dict.pop("__weakref__", None)
    if is_concrete:  # (only define actual slots in leaf, otherwise slots clash)
        for prop in properties.values():
            if isinstance(cls_dict.get(prop.name), PropertyDeclaration):
                cls_dict.pop(prop.name, None)
        cls_dict["__slots__"] = tuple(
            p.name if p.scalar_type != ScalarType.NODE_REFERENCE else f"{p.name}_ptr"
            for p in properties.values()
            if not p.is_computed
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
        if prop.original_component is UNSET:
            prop.original_component = cls
        else:
            prop.original_component = _processed_classes.get(
                prop.original_component, prop.original_component
            )

    _time_spent_in_process_object_cls += time.time() - start

    return cls, properties  # type: ignore


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def _builtin_object[ObjectT: BuiltinObject](
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
            is_root_node=False,
            is_entity=False,
            is_abstract=True,
            base_type=None,
            traits=(),
        )
        cls.__is_abstract__ = True
        return cast(type[ObjectT], cls)

    return decorate


_HANDLING_ATTRIBUTE_ERROR = contextvars.ContextVar("handling_attribute_error", default=False)


@_builtin_object()
class BuiltinObject:
    """The base for all intrinsic objects like Structs and Nodes."""

    metatype: ClassVar[NodeType | StructType] = UNSET
    __kind__: ClassVar[ObjectKind] = UNSET

    __is_frozen__: ClassVar[bool] = False
    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False
    __is_trait__: ClassVar[bool] = False
    __is_abstract__: ClassVar[bool] = False

    __properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __properties_by_alias__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __properties_by_id__: ClassVar[dict[int, PropertyDeclaration]] = {}

    __declared_properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __node_properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __wired_properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __stored_properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __tracked_properties__: ClassVar[dict[str, PropertyDeclaration]] = {}

    __properties_in_order__: ClassVar[tuple[PropertyDeclaration, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]

    __slots__: ClassVar[tuple[str, ...]] = ()

    """The Session this BuiltinObject is in."""
    _session: "Session | None" = builtin_property_runtime()
    """The Graph this BuiltinObject is in."""
    _graph: "Graph | None" = builtin_property_runtime()

    @classmethod
    def property(cls, name: str) -> PropertyDeclaration:
        """Get a Property by name."""
        prop = cls.__properties_by_alias__.get(name)
        if prop is not None:
            return prop
        raise ValueError(f"no property '{name}' in {cls.__name__}")

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

    def __bool__(self):
        return True  # support truthy checks for objects

    #
    # Encoding
    #

    @builtin_method(30)
    def pack(self, encoding: Encoding) -> Any:
        """Pack this BuiltinObject into some encoded format."""
        encoder = ENCODERS.get(encoding)
        assert encoder is not None, f"no Encoder defined for {encoding}"
        packed_object = encoder.pack_object(self.__kind__, self.metatype, self)
        return packed_object

    @builtin_method(31)
    def pack_binary(self, encoding: Encoding, writer: "BinaryWriter") -> None:
        """Pack this BuiltinObject into the byte representation of its encoded format."""
        encoder = ENCODERS.get(encoding)
        assert encoder is not None, f"no Encoder defined for {encoding}"
        encoder.pack_object_binary(self.__kind__, self.metatype, self, writer)

    @builtin_method(32)
    @classmethod
    def unpack(cls, encoding: Encoding, value: Any, session: "Session | None") -> Self:
        """Unpack a BuiltinObject from some encoded format."""
        encoder = ENCODERS.get(encoding)
        assert encoder is not None, f"no Encoder defined for {encoding}"
        unpacked_object = encoder.unpack_object(cls.__kind__, cls.metatype, value, session)
        return cast(Self, unpacked_object)

    @builtin_method(33)
    @classmethod
    def unpack_binary(
        cls, encoding: Encoding, reader: "BinaryReader", session: "Session | None"
    ) -> Self:
        """Unpack a BuiltinObject from the byte representation of its encoded format."""
        encoder = ENCODERS.get(encoding)
        assert encoder is not None, f"no Encoder defined for {encoding}"
        unpacked_object = encoder.unpack_object_binary(cls.__kind__, cls.metatype, reader, session)
        return cast(Self, unpacked_object)

    @builtin_method(34)
    @classmethod
    def unpack_binary_base64(
        cls, encoding: Encoding, value: str, session: "Session | None"
    ) -> Self:
        reader = BinaryReader(base64.b64decode(value))
        return cls.unpack_binary(encoding, reader, session)
