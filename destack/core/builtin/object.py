import inspect
import textwrap
from collections.abc import Mapping
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Optional,
    Self,
    assert_never,
    cast,
    dataclass_transform,
)

from .._utils import (
    execute_arbitrary_code,
    frozendict,
    frozenlist,
    get_superclasses,
)
from ._const import (
    ACTIVE_SESSION,
    EMPTY_DICT,
    EPSILON,
    EPSILON_EXPONENT,
    METAKIND_PROPERTY_ID,
    METATYPE_PROPERTY_ID,
    UNSET,
)
from ._hoisted import (
    Encoding,
    EnumType,
    PrimitiveType,
    ScalarType,
    TypeCardinality,
    ValueFactory,
)
from .casing import StringCasing, to_casing
from .declaration import (
    ActionDeclaration,
    ConstantDeclaration,
    MethodDeclaration,
    NodeDeclaration,
    ObjectDeclaration,
    TypeDeclaration,
    declare_method,
)
from .enum import OptionDeclaration
from .property import _PROPERTY_SPECIFIERS, PropertyDeclaration
from .types import Int64
from .universe import NodeType, ObjectKind, ObjectStability, StructType
from .uuid import UUID, uuid4, uuid7

if TYPE_CHECKING:
    from destack import BinaryReader, BinaryWriter, EncoderOptions, Hasher, Node, Session


__is_finalized__ = False


def _is_finalized() -> bool:
    return __is_finalized__


def _set_finalized():
    global __is_finalized__
    __is_finalized__ = True


_processed_classes: dict[type["Object"], type["Object"]] = {}


class ObjectGenerator:
    """Generate code for an Object class."""

    __slots__ = ("check_required",)

    def __init__(self, *, check_required: bool) -> None:
        self.check_required = check_required

    #
    # Init
    #

    def generate_init[ObjectT: Object](
        self, cls: type[ObjectT], declaration: ObjectDeclaration
    ) -> tuple[str, dict[str, Any]]:
        """Generate an __init__ for an Object class."""

        extra_glbls: dict[str, Any] = {}

        # header properties
        header_properties = {
            prop.name: prop for prop in cls.__properties__.values() if not prop.is_static
        }
        if declaration.kind == ObjectKind.NODE:
            header_properties.pop("_ref")
            header_properties.pop("_is_new")
        properties_in_order = list(header_properties.values())
        properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
        required_properties = [
            p
            for p in properties_in_order
            if not p.is_static
            and p.default_value is UNSET
            and p.default_factory is None
            and p.default_factory_callable is None
            and not p.is_internal
            and p.type.cardinality == TypeCardinality.SCALAR
            and p.type.scalar_type
            != ScalarType.NODE_REFERENCE  # passed either as node or node_ptr, defer check
        ]

        # header
        if properties_in_order:
            method_header_lines = ["def __init__(self, *"]
            # explicit session kw-only arg first
            method_header_lines.append("_session=None")
            # first add properties without defaults that are not managed
            for prop in required_properties:
                method_header_lines.append(prop.name)
            # then add properties with defaults or that are managed
            for prop in properties_in_order:
                if prop in required_properties:
                    continue
                elif prop.default_value is UNSET:
                    default_str = "None"
                elif isinstance(prop.default_value, OptionDeclaration):
                    default_str = (
                        f"{prop.default_value.component.__name__}.{prop.default_value.name}"
                    )
                    extra_glbls[prop.default_value.component.__name__] = (
                        prop.default_value.component
                    )
                elif prop.default_value is None or isinstance(
                    prop.default_value, (bool, int, float, str, bytes, UUID)
                ):
                    default_str = repr(prop.default_value)
                else:
                    default_name = f"_default_{prop.name}"
                    extra_glbls[default_name] = prop.default_value
                    default_str = default_name
                method_header_lines.append(f"{prop.name}={default_str}")
                if prop.type.scalar_type == ScalarType.NODE_REFERENCE:
                    method_header_lines.append(f"{prop.name}_ptr=None")

            method_header_lines.append(")")
            method_header = ", ".join(method_header_lines)
        else:
            method_header = "def __init__(self, *, _session=None)"

        # body
        # NOTE: frozen objects can use direct assignment, mutable objects can't
        #  (because of the custom __setattr__, that would add overhead for every set)
        extra_glbls["ACTIVE_SESSION"] = ACTIVE_SESSION
        extra_glbls["EMPTY_LIST"] = frozenlist()
        extra_glbls["EMPTY_DICT"] = frozendict()
        extra_glbls["uuid4"] = uuid4
        extra_glbls["uuid7"] = uuid7

        method_body_lines = []
        body_properties = {prop.name: prop for prop in declaration.properties}
        if not declaration.is_immutable and declaration.kind == ObjectKind.NODE:
            method_body_lines.append("__setattr__ = object.__setattr__")
            set_template_str = "__setattr__(self, '{0}', {1})"
        else:
            set_template_str = "self.{0} = {1}"

        # setup
        if declaration.kind == ObjectKind.NODE:
            assert isinstance(declaration, NodeDeclaration), (
                f"unexpected declaration: {declaration!r}"
            )
            # node setup
            body_properties.pop("id")
            if NodeType.ENTITY in declaration.inherits:
                body_properties.pop("created_at")
                body_properties.pop("created_epoch")
                body_properties.pop("created_by")
                body_properties.pop("updated_at")
                body_properties.pop("updated_epoch")
                body_properties.pop("updated_by")
            elif NodeType.EVENT in declaration.inherits:
                body_properties.pop("created_at")
                body_properties.pop("created_epoch")
                body_properties.pop("created_by")
                body_properties.pop("client")
                body_properties.pop("client_created_at")
                body_properties.pop("client_remote_epoch")
                body_properties.pop("client_local_epoch")
            else:
                raise NotImplementedError(f"unexpected node {cls.__name__}")
            body_properties.pop("_ref")
            body_properties.pop("_is_new")
            method_body_lines.append(f"""\
# session
if _session is None:
    _session = ACTIVE_SESSION.get()
    if _session is None:
        raise RuntimeError("no active session for {cls.__name__}")

# node identity
if id is None:
    """)
            if NodeType.ENTITY in declaration.inherits:
                method_body_lines.append("""\
    id = uuid4()
    _now = _session.context.now()
    created_at = _now
    created_epoch = _session.remote_epoch
    created_by_ptr = _session.context.actor_ptr
    updated_at = _now
    updated_epoch = _session.remote_epoch
    updated_by_ptr = _session.context.actor_ptr
""")
            elif NodeType.EVENT in declaration.inherits:
                method_body_lines.append("""\
    id = uuid7()
    _now = _session.context.now()
    created_epoch = _session.remote_epoch
    created_at = _now
    created_by_ptr = _session.context.actor_ptr
    client_ptr = _session.context.client_ptr
    client_nonce = _session.context.client_nonce
    client_remote_epoch = _session.remote_epoch
    client_local_epoch = _session.local_epoch
    client_created_at = _now
""")
            else:
                raise NotImplementedError(f"unexpected node {cls.__name__}")
            method_body_lines.append(f"""\
    _is_new = True
else:
    _is_new = False
{set_template_str.format("id", "id")}
""")

            if NodeType.ENTITY in declaration.inherits:
                method_body_lines.append(f"""\
{set_template_str.format("created_at", "created_at")}
{set_template_str.format("created_epoch", "created_epoch")}
{set_template_str.format("created_by_ptr", "created_by_ptr")}
{set_template_str.format("updated_at", "updated_at")}
{set_template_str.format("updated_epoch", "updated_epoch")}
{set_template_str.format("updated_by_ptr", "updated_by_ptr")}
""")
            elif NodeType.EVENT in declaration.inherits:
                method_body_lines.append(f"""\
{set_template_str.format("created_at", "created_at")}
{set_template_str.format("created_epoch", "created_epoch")}
{set_template_str.format("created_by_ptr", "created_by_ptr")}
{set_template_str.format("client_ptr", "client_ptr")}
{set_template_str.format("client_nonce", "client_nonce")}
{set_template_str.format("client_created_at", "client_created_at")}
{set_template_str.format("client_remote_epoch", "client_remote_epoch")}
{set_template_str.format("client_local_epoch", "client_local_epoch")}
""")
            else:
                raise NotImplementedError(f"unexpected node {cls.__name__}")
            method_body_lines.append(f"""\
{set_template_str.format("_ref", "None")}
{set_template_str.format("_is_new", "_is_new")}
""")

        # property assignments
        method_body_lines.append("# properties")
        body_properties_in_order = list(body_properties.values())
        body_properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
        for prop in body_properties_in_order:
            if prop.is_static:
                # computed, can't assign
                continue

            arg_name = prop.name
            self_name = (
                prop.name
                if prop.type.scalar_type != ScalarType.NODE_REFERENCE
                else f"{prop.name}_ptr"
            )

            # cast node to node_ptr
            if prop.type.scalar_type == ScalarType.NODE_REFERENCE:
                method_body_lines.append(f"""\
if {arg_name} is not None:
    {self_name} = {arg_name}.to_ref()""")

            # init default factory
            if prop.default_factory_callable is not None:
                extra_glbls[f"_{prop.name}_default"] = prop.default_factory_callable
                method_body_lines.append(f"""\
if {self_name} is None:
    {self_name} = _{prop.name}_default()""")
            elif prop.default_factory is not None:
                default_factory_str = self.generate_prop_default(
                    cls, declaration, prop, target_expr=self_name
                )
                method_body_lines.append(f"""\
if {self_name} is None:
{textwrap.indent(default_factory_str, " " * 4)}
""")

            # check/init required properties
            if prop.type.is_required:
                if prop.type.cardinality == TypeCardinality.SCALAR:
                    if self.check_required:
                        method_body_lines.append(f"""\
if {self_name} is None:
    raise AttributeError(f"{cls.__name__}.{prop.name} is required")""")
                elif prop.type.cardinality == TypeCardinality.LIST:
                    method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = {"[]" if not declaration.is_immutable else "EMPTY_LIST"}""")
                elif prop.type.cardinality == TypeCardinality.MAP:
                    method_body_lines.append(f"""\
if {arg_name} is None:
    {arg_name} = {"{}" if not declaration.is_immutable else "EMPTY_DICT"}""")

            # regular assignment
            if declaration.kind == ObjectKind.NODE and not declaration.is_immutable:
                method_body_lines.append(f"__setattr__(self, '{self_name}', {self_name})")
            else:
                method_body_lines.append(f"self.{self_name} = {self_name}")

        method_body = "\n".join(method_body_lines) or "pass"
        if len(method_body.splitlines()) < 2:
            method_body += "\npass"  # just in case we don't have any real lines
        method_body = textwrap.indent(method_body, " " * 4)
        init_str = f"{method_header}:\n{method_body}"
        return init_str, extra_glbls

    def generate_prop_default(
        self,
        cls: type["Object"],
        object: ObjectDeclaration,
        prop: PropertyDeclaration,
        target_expr: str,
    ) -> str:
        """Generate the default factory for an unset property."""

        assert prop.default_factory is not None, f"no default factory for {prop!r}"
        if prop.default_factory == ValueFactory.UUID4:
            return f"{target_expr} = uuid4()"
        elif prop.default_factory == ValueFactory.UUID7:
            return f"{target_expr} = uuid7()"
        elif prop.default_factory == ValueFactory.NOW:
            if cls.__declaration__.kind == ObjectKind.NODE:
                return f"""\
{target_expr} = _session.context.now()"""
            else:
                return f"""\
assert _session is not None, "no active Session for {cls.__name__}"
{target_expr} = _session.context.now()"""
        elif prop.default_factory == ValueFactory.REMOTE_EPOCH:
            assert object.kind is not None, f"unexpected declaration: {object!r}"
            if object.kind == ObjectKind.NODE:
                return f"""\
{target_expr} = _session.remote_epoch"""
            elif object.kind == ObjectKind.STRUCT:
                return f"""\
assert _session is not None, "no active Session for {cls.__name__}"
{target_expr} = _session.remote_epoch"""
            elif object.kind in (ObjectKind.HANDLE, ObjectKind.MODULE):
                raise NotImplementedError(f"cannot use {prop.default_factory} for {cls.__name__}")
            else:
                assert_never(object.kind)
        elif prop.default_factory == ValueFactory.LOCAL_EPOCH:
            assert object.kind is not None, f"unexpected declaration: {object!r}"
            if object.kind == ObjectKind.NODE:
                return f"""\
{target_expr} = _session.local_epoch"""
            elif object.kind == ObjectKind.STRUCT:
                return f"""\
assert _session is not None, "no active Session for {cls.__name__}"
{target_expr} = _session.local_epoch"""
            elif object.kind in (ObjectKind.HANDLE, ObjectKind.MODULE):
                raise NotImplementedError(f"cannot use {prop.default_factory} for {cls.__name__}")
            else:
                assert_never(object.kind)
        elif prop.default_factory == ValueFactory.ACTOR:
            return f"""\
{target_expr}_ptr = _session.context.actor_ptr"""
        elif prop.default_factory == ValueFactory.CLIENT:
            return f"""\
{target_expr}_ptr = _session.context.client_ptr"""
        elif prop.default_factory == ValueFactory.CLIENT_NONCE:
            return f"""\
{target_expr} = _session.context.client_nonce"""
        elif prop.default_factory == ValueFactory.REGION:
            return f"""\
{target_expr} = REGION"""
        elif prop.default_factory == ValueFactory.SELF:
            assert object.kind == ObjectKind.NODE, (
                f"{cls.__name__} is not a Node, cannot use self in {prop!r}"
            )
            return f"""\
{target_expr} = self.to_ref()"""
        elif prop.default_factory == ValueFactory.SPACE:
            assert object.kind == ObjectKind.NODE, (
                f"{cls.__name__} is not a Node, cannot use self in {prop!r}"
            )
            return f"""\
space = ACTIVE_SPACE.get()
if space is None:
    raise RuntimeError("no active Space for {cls.__name__}")
{target_expr} = space.to_ref()"""
        elif prop.default_factory == ValueFactory.BRANCH:
            return f"""\
branch = ACTIVE_BRANCH.get()
if branch is None:
    raise RuntimeError("no active Branch for {cls.__name__}")
{target_expr} = branch.to_ref()"""
        elif prop.default_factory == ValueFactory.SNAPSHOT:
            return f"""\
snapshot = ACTIVE_SNAPSHOT.get()
if snapshot is None:
    raise RuntimeError("no active Snapshot for {cls.__name__}")
{target_expr} = snapshot.to_ref()"""
        elif prop.default_factory == ValueFactory.NAME:
            assert object.kind == ObjectKind.NODE, (
                f"{cls.__name__} is not a Node, cannot use {prop.default_factory} in {prop!r}"
            )
            return f"""\
{target_expr} = "{cls.__name__}" """
        else:
            assert_never(prop.default_factory)

    #
    # Repr
    #

    def generate_repr[ObjectT: Object](
        self,
        cls: type[ObjectT],
    ) -> tuple[str, dict[str, Any]]:
        """Generate Object.__repr__."""
        repr_properties = [prop for prop in cls.__properties__.values() if prop.is_repr]
        if not repr_properties:
            if cls.__declaration__.kind == ObjectKind.NODE:
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

        # property parts
        repr_parts_lines: list[str] = []
        repr_parts_lines.append("_property_reprs: list[str] = []")
        has_required_repr_props = False
        for prop in repr_properties:
            source_prop_name = prop.name
            if prop.type.scalar_type == ScalarType.NODE_REFERENCE:
                source_prop_name = f"{prop.name}_ptr"
            source_expr = f"self.{source_prop_name}"
            target_expr = f"_{source_prop_name}_repr"
            repr_impl = self.generate_repr_value(
                prop.type, f"_{prop.name}", source_expr, target_expr
            )
            if prop.type.is_required:
                repr_parts_lines.append(f"""\
{repr_impl}
_property_reprs.append(f'{prop.name}={{{target_expr}}}')
""")
            else:
                repr_parts_lines.append(f"""\
{target_expr} = UNSET
{repr_impl}
if {target_expr} is not UNSET:
    _property_reprs.append(f'{prop.name}={{{target_expr}}}')
""")
            if prop.type.is_required and prop.type.cardinality == TypeCardinality.SCALAR:
                has_required_repr_props = True

        # wrap in repr
        repr_parts_str = "\n".join(repr_parts_lines)
        if cls.__declaration__.kind == ObjectKind.NODE:
            if has_required_repr_props:
                inner_repr_impl = f"""\
{repr_parts_str}
return f"<{cls.__name__} \\"{{self.path}}\\" {{' '.join(_property_reprs)}}>"
"""
            else:
                inner_repr_impl = f"""\
{repr_parts_str}
if _property_reprs:
    return f"<{cls.__name__} \\"{{self.path}}\\" {{' '.join(_property_reprs)}}>"
else:
    return f"<{cls.__name__} \\"{{self.path}}\\">"
"""
        else:
            if has_required_repr_props:
                inner_repr_impl = f"""\
{repr_parts_str}
return f"<{cls.__name__} {{' '.join(_property_reprs)}}>"
"""
            else:
                inner_repr_impl = f"""\
{repr_parts_str}
if _property_reprs:
    return f"<{cls.__name__} {{' '.join(_property_reprs)}}>"
else:
    return f"<{cls.__name__}>"
"""

        inner_repr_impl = textwrap.indent(inner_repr_impl, " " * 4)
        repr_impl = f"""\
def __repr__(self) -> str:
{inner_repr_impl}
__str__ = __repr__
"""

        return repr_impl, {"UNSET": UNSET}

    def generate_repr_value(
        self,
        type: TypeDeclaration,
        key: str,
        source_expr: str,
        target_expr: str,
    ) -> str:
        """Generate repr code for a value of the given type."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if type.is_required:
                scalar_repr = self.generate_repr_scalar_value(type, source_expr)
                return f"{target_expr} = f'{{{scalar_repr}}}'"
            else:
                value_source_expr = f"{key}_value"
                scalar_repr = self.generate_repr_scalar_value(type, value_source_expr)
                return f"""\
if ({value_source_expr} := {source_expr}) is not None:
    {target_expr} = f'{{{scalar_repr}}}'"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_source_expr = f"{key}_item"
            item_repr_expr = f"_{item_source_expr}_repr"
            item_repr = self.generate_repr_value(
                type.value_type, f"{key}_value", item_source_expr, item_repr_expr
            )
            list_impl = f"""\
_{key}_items_repr: list[str] = []
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_repr, " " * 4)}
    _{key}_items_repr.append({item_repr_expr})
{target_expr} = f'[{{", ".join(_{key}_items_repr)}}]'"""

            if type.is_required:
                return list_impl
            else:
                list_expr = f"{key}_list"
                return f"""\
if ({list_expr} := {source_expr}):
{textwrap.indent(list_impl.replace(source_expr, list_expr), " " * 4)}"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            tuple_parts = [f"_{key}_elements_repr: list[str] = []"]
            for i, element_type in enumerate(type.element_types):
                element_source_expr = f"{key}_element_{i}"
                element_repr_expr = f"_{element_source_expr}_repr"
                element_repr = self.generate_repr_value(
                    element_type,
                    element_source_expr,
                    f"{source_expr}[{i}]",
                    element_repr_expr,
                )
                tuple_parts.append(element_repr)
                tuple_parts.append(f"_{key}_elements_repr.append({element_repr_expr})")
            tuple_parts.append(f"{target_expr} = f'({', '.join(f'{{_{key}_elements_repr}}')})'")
            if type.is_required:
                return "\n".join(tuple_parts)
            else:
                tuple_expr = f"{key}_tuple"
                return f"""\
if ({tuple_expr} := {source_expr}) is not None:
{textwrap.indent("\n".join(tuple_parts).replace(source_expr, tuple_expr), " " * 4)}"""

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key"
            value_source_expr = f"{key}_value"
            key_target_expr = f"{key}_key_repr"
            value_target_expr = f"{key}_value_repr"
            map_pairs_target_expr = f"{key}_pairs"
            key_repr = self.generate_repr_value(
                type.key_type, f"{key}_key", key_source_expr, key_target_expr
            )
            value_repr = self.generate_repr_value(
                type.value_type, f"{key}_value", value_source_expr, value_target_expr
            )

            map_impl = f"""\
{map_pairs_target_expr} = []
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_repr, " " * 4)}
{textwrap.indent(value_repr, " " * 4)}
    {map_pairs_target_expr}.append(f'{{{key_target_expr}}}={{{value_target_expr}}}')
{target_expr} = f'{{{", ".join({map_pairs_target_expr})}}}'"""

            if type.is_required:
                return map_impl
            else:
                map_expr = f"{key}_map"
                return f"""\
if ({map_expr} := {source_expr}):
{textwrap.indent(map_impl.replace(source_expr, map_expr), " " * 4)}"""

        else:
            assert_never(type.cardinality)

    def generate_repr_scalar_value(self, type: TypeDeclaration, value_expr: str) -> str:
        """Get repr expression for a scalar value."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"
        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return f"{value_expr}!r"
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return f"{value_expr}!r"
            elif type.primitive_type in (
                PrimitiveType.INT8,
                PrimitiveType.INT16,
                PrimitiveType.INT32,
                PrimitiveType.INT64,
                PrimitiveType.INT128,
                PrimitiveType.UINT8,
                PrimitiveType.UINT16,
                PrimitiveType.UINT32,
                PrimitiveType.UINT64,
                PrimitiveType.UINT128,
            ):
                return f"{value_expr}!r"
            elif type.primitive_type in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            ):
                return f"{value_expr}:0.{EPSILON_EXPONENT}"
            elif type.primitive_type in (
                PrimitiveType.DATETIME,
                PrimitiveType.DATE,
                PrimitiveType.TIME,
            ):
                return f"{value_expr}.isoformat()"
            elif type.primitive_type == PrimitiveType.DURATION:
                return f"{value_expr}!r"
            elif type.primitive_type == PrimitiveType.STRING:
                return f"{value_expr}!r"
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return f"{value_expr}!r"
            elif type.primitive_type == PrimitiveType.UUID:
                return f"str({value_expr})"
            elif type.primitive_type == PrimitiveType.BYTES:
                return f"{value_expr}!r"
            elif type.primitive_type == PrimitiveType.JSON:
                return f"{value_expr}!r"
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return f"{value_expr}.name"
        # node_reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return f"{value_expr}!r"
        # node_value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return f"{value_expr}!r"
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            return f"{value_expr}!r"
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            return f"{value_expr}!r"
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot get repr for union: {type!r}")
        #
        else:
            assert_never(type.scalar_type)

    #
    # Equals
    #

    def generate_equals[ObjectT: Object](
        self,
        cls: type[ObjectT],
        is_node: bool,
    ) -> tuple[str, dict[str, Any]]:
        """Generate Object.equals method."""

        eq_properties = [
            prop for prop in cls.__properties__.values() if prop.is_eq and not prop.is_runtime_only
        ]
        cmp_strs = []
        for i, prop in enumerate(eq_properties):
            prop_name = prop.name
            if prop.type.scalar_type == ScalarType.NODE_REFERENCE:
                prop_name = f"{prop_name}_ptr"
            self_source_expr = f"self.{prop_name}"
            other_source_expr = f"other.{prop_name}"
            cmp_str = self.generate_equals_value(
                prop.type, f"_{i}", self_source_expr, other_source_expr
            )
            cmp_strs.append(cmp_str)
        body_str = "\n".join(cmp_strs)
        body_str = textwrap.indent(body_str, " " * 4)

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

    def generate_equals_value(
        self, type: TypeDeclaration, key: str, self_source_expr: str, other_source_expr: str
    ) -> str:
        """Generate equality check code for a value of the given type."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            scalar_cmps_str, is_simple = self.generate_equals_scalar_value(type)
            if type.is_required or is_simple:
                # required scalar
                return f"""\
if not ({scalar_cmps_str.format(self_val=self_source_expr, other_val=other_source_expr)}):
    return False"""
            else:
                # optional scalar
                return f"""\
if ({self_source_expr} is None) != ({other_source_expr} is None):
    return False
if {self_source_expr} is not None and not ({scalar_cmps_str.format(self_val=self_source_expr, other_val=other_source_expr)}):
    return False"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_expr = f"{key}_value"
            value_cmp = self.generate_equals_value(
                type.value_type,
                f"{key}_value",
                f"{self_source_expr}[{item_expr}]",
                f"{other_source_expr}[{item_expr}]",
            )
            main_cmp = f"""\
for {item_expr} in range(len({self_source_expr})):
{textwrap.indent(value_cmp, " " * 4)}
"""
            if type.is_required:
                return f"""\
if len({self_source_expr}) != len({other_source_expr}):
    return False
{main_cmp}"""
            else:
                return f"""\
if ({self_source_expr} is not None) != ({other_source_expr} is not None):
    return False
if {self_source_expr} is not None:
    if len({self_source_expr}) != len({other_source_expr}):
        return False
{textwrap.indent(main_cmp, " " * 4)}
"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            tuple_parts: list[str] = []
            for i, element_type in enumerate(type.element_types):
                element_cmp = self.generate_equals_value(
                    element_type,
                    f"{key}_element_{i}",
                    f"{self_source_expr}[{i}]",
                    f"{other_source_expr}[{i}]",
                )
                tuple_parts.append(element_cmp)
            return "\n".join(tuple_parts)

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_expr = f"{key}_key"

            if type.scalar_type in (
                ScalarType.STRUCT,
                ScalarType.NODE_REFERENCE,
                ScalarType.NODE_VALUE,
            ):
                # maps with complex values need key-by-key comparison
                value_cmp = self.generate_equals_value(
                    type.value_type,
                    f"{key}_value",
                    f"{self_source_expr}[{key_expr}]",
                    f"{other_source_expr}[{key_expr}]",
                )
                main_cmp = f"""\
for {key_expr} in {self_source_expr}:
    if {key_expr} not in {other_source_expr}:
        return False
{textwrap.indent(value_cmp, " " * 4)}
"""
                if type.is_required:
                    return f"""\
if len({self_source_expr}) != len({other_source_expr}):
    return False
{main_cmp}"""
                else:
                    self_map_expr = f"{key}_self_map"
                    other_map_expr = f"{key}_other_map"
                    return f"""\
if ({self_source_expr} is not None) != ({other_source_expr} is not None):
    return False
if {self_source_expr} is not None:
    {self_map_expr} = {self_source_expr}
    {other_map_expr} = {other_source_expr}
    if len({self_map_expr}) != len({other_map_expr}):
        return False
{textwrap.indent(value_cmp, " " * 4)}    
"""
            else:
                # maps with primitive/enum values can use direct comparison
                return f"""\
if {self_source_expr} != {other_source_expr}:
    return False"""
        else:
            assert_never(type.cardinality)

    def generate_equals_scalar_value(self, type: TypeDeclaration) -> tuple[str, bool]:
        """Generate the core scalar comparison logic. Returns a format string with {self_val} and {other_val} placeholders."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return "{self_val} == {other_val}", True
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return "{self_val} == {other_val}", True
            elif type.primitive_type in (
                PrimitiveType.INT8,
                PrimitiveType.INT16,
                PrimitiveType.INT32,
                PrimitiveType.INT64,
                PrimitiveType.INT128,
                PrimitiveType.UINT8,
                PrimitiveType.UINT16,
                PrimitiveType.UINT32,
                PrimitiveType.UINT64,
                PrimitiveType.UINT128,
            ):
                return "{self_val} == {other_val}", True
            elif type.primitive_type in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            ):
                return (
                    f"{{self_val}} == {{other_val}} or abs({{self_val}} - {{other_val}}) < {EPSILON}",
                    False,
                )
            elif type.primitive_type in (
                PrimitiveType.DATETIME,
                PrimitiveType.DATE,
                PrimitiveType.TIME,
                PrimitiveType.DURATION,
            ):
                return "{self_val} == {other_val}", True
            elif type.primitive_type in (
                PrimitiveType.STRING,
                PrimitiveType.CHARACTER,
                PrimitiveType.UUID,
                PrimitiveType.BYTES,
            ):
                return "{self_val} == {other_val}", True
            elif type.primitive_type == PrimitiveType.JSON:
                return "{self_val} == {other_val}", True
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return "{self_val} == {other_val}", True
        # node_reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return "{self_val}.id == {other_val}.id", False
        # node_value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return "{self_val}.id == {other_val}.id", False
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            return "{self_val}.equals({other_val})", False
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            return "{self_val} is {other_val}", False
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot get equals for union: {type!r}")
        #
        else:
            assert_never(type.scalar_type)

    #
    # Hash
    #

    def generate_hash[ObjectT: Object](
        self,
        cls: type[ObjectT],
    ) -> tuple[str, dict[str, Any]]:
        """Generate Object.hash method."""
        hash_properties = [
            prop
            for prop in cls.__properties__.values()
            if prop.is_hash and not prop.is_runtime_only
        ]
        hash_parts: list[str] = []
        for prop in hash_properties:
            prop_name = prop.name
            if prop.type.scalar_type == ScalarType.NODE_REFERENCE:
                prop_name = f"{prop_name}_ptr"
            source_expr = f"self.{prop_name}"
            prop_hash_impl = self.generate_hash_value(
                prop.type, f"_{prop_name}", source_expr, "_hasher"
            )
            hash_parts.append(prop_hash_impl)
        hash_parts_str = "\n".join(hash_parts)

        hash_impl = f"""\
def hash(self, _hasher: "Hasher | None" = None) -> Int64:
    if _hasher is None:
        from destack.core import Hasher
        _hasher = Hasher()
{textwrap.indent(hash_parts_str, " " * 4)}
    return _hasher.digest()
"""
        return hash_impl, {"Int64": Int64}

    def generate_hash_value(
        self, type: TypeDeclaration, key: str, source_expr: str, hasher_expr: str
    ) -> str:
        """Generate hash code for a value of the given type."""
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if type.is_required:
                scalar_hash_str = self.generate_hash_scalar_value(type, source_expr, hasher_expr)
                return scalar_hash_str
            else:
                scalar_source_expr = f"{key}_value"
                scalar_hash_str = self.generate_hash_scalar_value(
                    type, scalar_source_expr, hasher_expr
                )
                return f"""\
if ({scalar_source_expr} := {source_expr}) is not None:
    {scalar_hash_str}"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_source_expr = f"{key}_item"
            value_hash_str = self.generate_hash_value(
                type.value_type, f"{key}_value", item_source_expr, hasher_expr
            )
            if type.is_required:
                return f"""\
for {item_source_expr} in {source_expr}:
    {value_hash_str}"""
            else:
                list_source_expr = f"{key}_list"
                return f"""\
if ({list_source_expr} := {source_expr}):
    for {item_source_expr} in {list_source_expr}:
        {textwrap.indent(value_hash_str, " " * 4)}"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            tuple_parts: list[str] = []
            for i, element_type in enumerate(type.element_types):
                element_hash = self.generate_hash_value(
                    element_type, f"{key}_element_{i}", f"{source_expr}[{i}]", hasher_expr
                )
                tuple_parts.append(element_hash)
            return "\n".join(tuple_parts)

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key"
            value_source_expr = f"{key}_value"
            key_hash_str = self.generate_hash_value(
                type.key_type, f"{key}_key", key_source_expr, hasher_expr
            )
            value_hash_str = self.generate_hash_value(
                type.value_type, f"{key}_value", value_source_expr, hasher_expr
            )
            if type.is_required:
                return f"""\
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
    {key_hash_str}
    {value_hash_str}"""
            else:
                map_source_expr = f"{key}_map"
                return f"""\
if ({map_source_expr} := {source_expr}):
    for {key_source_expr}, {value_source_expr} in {map_source_expr}.items():
        {textwrap.indent(key_hash_str, " " * 4)}
        {textwrap.indent(value_hash_str, " " * 4)}"""

        else:
            assert_never(type.cardinality)

    def generate_hash_scalar_value(
        self, type: TypeDeclaration, source_expr: str, hasher_expr: str
    ) -> str:
        """Generate hash expression for a scalar value."""
        assert type.scalar_type is not None, f"no scalar type for {type!r}"
        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return f"{hasher_expr}.hash_none()"
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return f"{hasher_expr}.hash_bool({source_expr})"
            elif type.primitive_type == PrimitiveType.INT8:
                return f"{hasher_expr}.hash_int8({source_expr})"
            elif type.primitive_type == PrimitiveType.INT16:
                return f"{hasher_expr}.hash_int16({source_expr})"
            elif type.primitive_type == PrimitiveType.INT32:
                return f"{hasher_expr}.hash_int32({source_expr})"
            elif type.primitive_type == PrimitiveType.INT64:
                return f"{hasher_expr}.hash_int64({source_expr})"
            elif type.primitive_type == PrimitiveType.INT128:
                return f"{hasher_expr}.hash_int128({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT8:
                return f"{hasher_expr}.hash_uint8({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT16:
                return f"{hasher_expr}.hash_uint16({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT32:
                return f"{hasher_expr}.hash_uint32({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT64:
                return f"{hasher_expr}.hash_uint64({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT128:
                return f"{hasher_expr}.hash_uint128({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT16:
                return f"{hasher_expr}.hash_float16({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return f"{hasher_expr}.hash_float32({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return f"{hasher_expr}.hash_float64({source_expr})"
            elif type.primitive_type == PrimitiveType.DATETIME:
                return f"{hasher_expr}.hash_datetime({source_expr})"
            elif type.primitive_type == PrimitiveType.DATE:
                return f"{hasher_expr}.hash_date({source_expr})"
            elif type.primitive_type == PrimitiveType.TIME:
                return f"{hasher_expr}.hash_time({source_expr})"
            elif type.primitive_type == PrimitiveType.DURATION:
                return f"{hasher_expr}.hash_duration({source_expr})"
            elif type.primitive_type == PrimitiveType.STRING:
                return f"{hasher_expr}.hash_string({source_expr})"
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return f"{hasher_expr}.hash_character({source_expr})"
            elif type.primitive_type == PrimitiveType.UUID:
                return f"{hasher_expr}.hash_uuid({source_expr})"
            elif type.primitive_type == PrimitiveType.BYTES:
                return f"{hasher_expr}.hash_bytes({source_expr})"
            elif type.primitive_type == PrimitiveType.JSON:
                return f"{hasher_expr}.hash_json({source_expr})"
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return f"{hasher_expr}.hash_uint32({source_expr})"
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            return f"{hasher_expr}.hash_uint64({source_expr}.hash())"
        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return f"{hasher_expr}.hash_uint64({source_expr}.hash())"
        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return f"{hasher_expr}.hash_uint64({source_expr}.hash())"
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot hash Handle: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot hash union: {type!r}")
        else:
            assert_never(type.scalar_type)

    #
    # Path
    #

    def generate_path[NodeT: Node](self, cls: type[NodeT]) -> tuple[str, dict[str, Any]]:
        """Generate Node.path property (and Node._path_key helper)."""
        assert cls.__declaration__.kind == ObjectKind.NODE, f"{cls.__name__} is not a Node"

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

        path_key_str = self.generate_path_key_property(cls)
        path_impl = f"{path_key_str}\n{path_str}"
        return path_impl, {}

    def generate_path_key_property(self, cls: type["Object"]) -> str:
        """Generate a Node's path "key" property."""
        assert cls.__declaration__.kind == ObjectKind.NODE, f"{cls.__name__} is not a Node"

        # Node._path_key
        if "slug" in cls.__properties__:
            if "name" in cls.__properties__:
                return """\
@property
def _path_key(self) -> str:
    return self.slug or self.name
"""
            else:
                return """\
@property
def _path_key(self) -> str:
    return self.slug or f"{self.metatype.destack_name}[id={self.id}]"
"""
        elif "name" in cls.__properties__:
            return """\
@property
def _path_key(self) -> str:
    return self.name
"""
        else:
            return f"""\
@property
def _path_key(self) -> str:
    return f"{cls.__name__}[id={{self.id}}]"
"""

    def generate_path_key[NodeT: Node](self, cls: type[NodeT]) -> str:
        """Generate a Node's path "key"."""
        assert cls.__declaration__.kind == ObjectKind.NODE, f"{cls.__name__} is not a Node"
        if "slug" in cls.__properties__:
            if "name" in cls.__properties__:
                return """self.slug or self.name"""
            else:
                return """self.slug or f"{self.metatype.destack_name}[id={self.id}]" """
        elif "name" in cls.__properties__:
            return """self.name"""
        else:
            return f"""f"{cls.__name__}[id={{self.id}}]"""

    #
    # Node properties
    #

    def generate_node_property(self, prop: PropertyDeclaration) -> str:
        """The computed get/set property for a Node reference."""
        # NOTE :Performance: we could inline Graph.get into node property getters

        assert prop.type.cardinality == TypeCardinality.SCALAR, (
            f"node properties must be scalar: {prop!r}"
        )
        is_node = prop.component.__declaration__.kind == ObjectKind.NODE

        if is_node:
            getter = f"""\
@property
def {prop.name}(self: "Object") -> "Node | None":
    node_ptr: NodeReference | None = self.{prop.name}_ptr
    if node_ptr is not None:
        return _session.graph.get(node_ptr.id, node_ptr.space_id, node_ptr.branch_id, node_ptr.snapshot_id)
    else:
        return None
"""
        else:
            getter = f"""\
@property
def {prop.name}(self: "Object") -> "Node | None":
    node_ptr: NodeReference | None = self.{prop.name}_ptr
    if node_ptr is not None:
        if _session is None:
            return None
        return _session.graph.get(node_ptr.id, node_ptr.space_id, node_ptr.branch_id, node_ptr.snapshot_id)
    else:
        return None
"""

        if is_node:
            setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "Object", value: "Node | None"):
    if value is None:
        self.set("{prop.name}_ptr", None)
    else:
        self.set("{prop.name}_ptr", value.to_ref())
"""
        else:
            setter = f"""\
@{prop.name}.setter
def {prop.name}(self: "Object", value: "Node | None"):
    if value is None:
        self.{prop.name}_ptr = None
    else:
        self.{prop.name}_ptr = value.to_ref()
"""

        return getter + "\n\n" + setter


_METAKIND_TYPE = TypeDeclaration(
    cardinality=TypeCardinality.SCALAR,
    primitive_type=PrimitiveType.INT32,
    enum_type=EnumType.OBJECT_KIND,
    is_required=True,
)
_METATYPE_TYPE = TypeDeclaration(
    cardinality=TypeCardinality.SCALAR,
    primitive_type=PrimitiveType.INT32,
    is_required=True,
)
_generator = ObjectGenerator(check_required=False)


def _process_object_cls[ObjectT: Object](
    cls: type[ObjectT], declaration: ObjectDeclaration
) -> tuple[type[ObjectT], ObjectDeclaration]:
    """Process an Object base class and return the processed class and its properties."""

    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"
    assert cls not in _processed_classes, f"class {cls.__name__} has already been processed"

    cls.__declaration__ = declaration

    # metatype
    metakind_property = PropertyDeclaration(
        id=METAKIND_PROPERTY_ID,
        name="metakind",
        description="The kind of the Object.",
        py_type=Any,
        type=_METAKIND_TYPE,
        is_internal=True,
        is_runtime_only=True,
        is_static=True,
        is_identity=True,
        component=cls,
    )
    metatype_property = PropertyDeclaration(
        id=METATYPE_PROPERTY_ID,
        name="metatype",
        description="The type of the Object.",
        py_type=Any,
        is_internal=True,
        is_runtime_only=True,
        is_static=True,
        is_identity=True,
        type=_METATYPE_TYPE,
        component=cls,
    )

    # check for redundant components
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
    components: list[type[Object]] = []
    for base_cls in get_superclasses(cls):
        base_cls = _processed_classes.get(base_cls, base_cls)
        if hasattr(base_cls, "__properties__"):
            components.append(base_cls)

    # walk the class definition and collect class-level stuff
    properties: dict[str, PropertyDeclaration] = {
        metakind_property.name: metakind_property,
        metatype_property.name: metatype_property,
    }
    for name, attribute in list(cls.__dict__.items()):
        if (
            inspect.ismethod(attribute)
            or inspect.isfunction(attribute)
            or isinstance(attribute, (property, classmethod, staticmethod))
            or name in ("metakind", "metatype")
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
            # constants are replaced with their value during finalization
            attribute.name = intern(name)
            attribute.component = cls
            if attribute.original_component is None:
                attribute.original_component = cls
        elif isinstance(attribute, (MethodDeclaration, ActionDeclaration)):
            # methods/actions are replaced later with their callables
            attribute.name = intern(name)
            attribute.component = cls
            if attribute.original_component is None:
                attribute.original_component = cls
        elif not name.startswith("__"):
            raise TypeError(
                f"{cls.__name__}.{name} is not a Property or Constant: {attribute} ({type(attribute)})"
            )

    # add properties from ancestor components (closest first)
    for component in components[1:]:
        for prop in component.__declaration__.properties:
            existing = properties.get(prop.name)
            if (
                existing is not None
                and existing.original_component is not component
                and existing.original_component is not prop.original_component
            ):
                if existing.name in (
                    "id",
                    "metatype",
                    "metakind",
                    "parent",
                    "definition",
                ):
                    continue  # may be narrowed/duplicated
                elif component.__declaration__.is_abstract and existing.id == prop.id:
                    continue  # may be overridden
                else:
                    # error: property conflicts with ancestor component
                    raise RuntimeError(
                        f"property '{prop.name}' from '{component.__name__}' conflicts with '{cls.__name__}': {prop!r}, {existing!r}"
                    )
            # add property
            prop = prop.clone()
            prop.component = cls
            if prop.original_component is UNSET:
                prop.original_component = component
            properties[prop.name] = prop

    # determine property types
    for prop in properties.values():
        prop.determine(declaration.type, is_root_node=declaration.type == NodeType.SPACE)

    # index properties
    cls.__declaration__.properties = sorted(properties.values(), key=lambda p: p.id)
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
        if not prop.is_runtime_only:
            lower_camel_name = to_casing(prop.name, StringCasing.LOWER_CAMEL)
            upper_camel_name = to_casing(prop.name, StringCasing.UPPER_CAMEL)
            if prop.type.scalar_type == ScalarType.NODE_REFERENCE:
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
    cls.__properties_by_alias__ = frozendict(properties_by_alias)
    cls.__properties_by_id__ = frozendict(properties_by_id)

    cls_dict = dict(cls.__dict__)

    # define final methods in leaf classes
    glbls = {
        "ACTIVE_SESSION": ACTIVE_SESSION,
        "EMPTY_LIST": frozenlist(),
        "EMPTY_DICT": frozendict(),
        "uuid4": uuid4,
    }
    if declaration.is_abstract:
        init_str = f"""\
def __init__(self):
    raise NotImplementedError(f"abstract {cls.__name__} cannot be instantiated")
"""
        execute_arbitrary_code(init_str, glbls, cls_dict, f"{cls.__name__}.__init__")
    else:
        # __init__
        init_str, init_glbls = _generator.generate_init(cls, declaration)
        execute_arbitrary_code(
            init_str, {**glbls, **init_glbls}, cls_dict, f"{cls.__name__}.__init__"
        )
        # __repr__
        repr_str, repr_glbls = _generator.generate_repr(cls)
        execute_arbitrary_code(
            repr_str, {**glbls, **repr_glbls}, cls_dict, f"{cls.__name__}.__repr__"
        )
        # equals
        equals_str, equals_glbls = _generator.generate_equals(
            cls, is_node=declaration.kind == ObjectKind.NODE
        )
        execute_arbitrary_code(
            equals_str, {**glbls, **equals_glbls}, cls_dict, f"{cls.__name__}.equals"
        )
        # hash
        hash_str, hash_glbls = _generator.generate_hash(cls)
        execute_arbitrary_code(hash_str, {**glbls, **hash_glbls}, cls_dict, f"{cls.__name__}.hash")
        if declaration.kind == ObjectKind.NODE:
            assert (
                isinstance(declaration.type, OptionDeclaration)
                and declaration.type.component == NodeType
            ), f"unexpected type: {declaration.type!r}"
            # path
            path_str, path_glbls = _generator.generate_path(cast(type["Node"], cls))
            execute_arbitrary_code(
                path_str, {**glbls, **path_glbls}, cls_dict, f"{cls.__name__}.path"
            )
        # computed Node properties
        for prop in properties.values():
            if prop.edge_type is not None:
                node_property_str = _generator.generate_node_property(prop)
                execute_arbitrary_code(
                    node_property_str, {}, cls_dict, f"{cls.__name__}.{prop.name}"
                )

    # slots
    cls_dict.pop("__dict__", None)
    cls_dict.pop("__weakref__", None)
    if not declaration.is_abstract:  # (only define actual slots in leaf, otherwise slots clash)
        for prop in properties.values():
            if isinstance(cls_dict.get(prop.name), PropertyDeclaration):
                cls_dict.pop(prop.name, None)
        cls_dict["__slots__"] = tuple(
            p.name if p.type.scalar_type != ScalarType.NODE_REFERENCE else f"{p.name}_ptr"
            for p in properties.values()
            if not p.is_static
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

    return cls, properties  # type: ignore


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def _declare_object[ObjectT: Object](
    object_type: NodeType | StructType | None = None,
    frozen: bool = False,
):
    """
    Mark a class as an object component (or concrete struct for a StructType).
    """

    def decorate(cls_in: type[ObjectT]) -> type[ObjectT]:
        declaration = ObjectDeclaration(
            # meta
            cls=cls_in,
            kind=ObjectKind.STRUCT,
            type=object_type,
            id=0,
            name=cls_in.__name__,
            description=cls_in.__doc__ or "",
            stability=ObjectStability.DYNAMIC,
            is_abstract=True,
            is_immutable=frozen,
            is_final=False,
            # inheritance
            base_type=None,
            inherits=[],
            inherited_by=[],
            extended_by=[],
            # content
            properties=[],
        )
        cls, _properties = _process_object_cls(cast(Any, cls_in), declaration)
        cls.__is_abstract__ = True
        return cast(type[ObjectT], cls)

    return decorate


@_declare_object()
class Object:
    """The base for all intrinsic Objects."""

    # Object.metakind: 0
    metakind: ClassVar[ObjectKind] = UNSET
    # Object.metatype: 1
    metatype: ClassVar[NodeType | StructType] = UNSET
    __declaration__: ClassVar[ObjectDeclaration] = UNSET

    # runtime index
    __properties__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __properties_by_alias__: ClassVar[dict[str, PropertyDeclaration]] = {}
    __properties_by_id__: ClassVar[dict[int, PropertyDeclaration]] = {}

    __slots__: ClassVar[tuple[str, ...]] = ()

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
        raise NotImplementedError

    def hash(self, _hasher: "Hasher | None" = None) -> Int64:
        """Hash of content properties."""
        raise NotImplementedError

    def __bool__(self):
        return True  # support truthy checks for objects

    #
    # Encoding
    #

    @declare_method(30)
    def pack(
        self,
        encoding: Encoding,
        options: Optional["EncoderOptions"] = None,
    ) -> Any:
        """Pack this Object into some encoded format."""
        raise NotImplementedError

    @declare_method(31)
    def pack_binary(
        self,
        encoding: Encoding,
        writer: "BinaryWriter",
        options: Optional["EncoderOptions"] = None,
    ) -> None:
        """Pack this Object into the byte representation of its encoded format."""
        raise NotImplementedError

    @declare_method(201)
    @classmethod
    def unpack(
        cls,
        encoding: Encoding,
        value: Any,
        session: Optional["Session"] = None,
        options: Optional["EncoderOptions"] = None,
    ) -> Self:
        """Unpack an Object from some encoded format."""
        raise NotImplementedError

    @declare_method(202)
    @classmethod
    def unpack_binary(
        cls,
        encoding: Encoding,
        reader: "BinaryReader",
        session: Optional["Session"] = None,
        options: Optional["EncoderOptions"] = None,
    ) -> Self:
        """Unpack an Object from the byte representation of its encoded format."""
        raise NotImplementedError

    @declare_method(203)
    @classmethod
    def unpack_binary_base64(
        cls,
        encoding: Encoding,
        value: str,
        session: Optional["Session"] = None,
        options: Optional["EncoderOptions"] = None,
    ) -> Self:
        """Unpack an Object from a base64 encoded string."""
        raise NotImplementedError
