import abc
import base64
import contextvars
import inspect
import textwrap
from enum import Enum, IntEnum
from sys import intern
from typing import (
    TYPE_CHECKING,
    Any,
    ClassVar,
    Collection,
    Iterable,
    Mapping,
    Optional,
    Self,
    TypeGuard,
    Union,
    cast,
    dataclass_transform,
    final,
)

import structlog
from bitarray import bitarray
from fastuuid import UUID
from opentelemetry import trace

from bench.language.registry import (
    BUILTIN_OBJECT_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from bench.pb2 import AnyObjectData, AnyStructData, ScopeData, lang_pb2
from bench.utils.code import format_code
from bench.utils.env import IS_DEV
from bench.utils.func import dualmethod, get_superclasses
from bench.utils.utils import frozendict

from .const import (
    ACTIVE_SESSION,
    EMPTY_DICT,
    UNSET,
    EnumType,
    NodeReferenceKind,
    NodeType,
    PrimitiveType,
    Region,
    StructType,
)
from .graph import Supergraph
from .property import _PROPERTY_SPECIFIERS, Property, property_

if TYPE_CHECKING:
    from bench.language import Field, Node, NodeReference, PropertyReference, Session

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


_processed_classes: dict[type["BuiltinObject"], type["BuiltinObject"]] = {}


def _generate_init_for_cls[ObjectT: BuiltinObject](
    cls: type[ObjectT], is_node: bool, header_properties: dict[str, Property]
) -> str:
    """Generates an __init__ for a BuiltinObject class."""

    # prune properties
    original_properties = header_properties

    # sort properties (by id, runtime by alpha)
    properties_in_order = list(header_properties.values())
    properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))

    # header
    method_header_lines = ["def __init__(self, *"]
    for prop in properties_in_order:
        if prop.is_computed:
            continue
        if prop.default is UNSET:
            default_str = "UNSET"
        elif isinstance(prop.default, Enum):
            default_str = repr(prop.default.value)
        else:
            default_str = repr(prop.default)
        method_header_lines.append(f"{prop.name}={default_str}")
    method_header_lines.append(")")
    method_header = ", ".join(method_header_lines)

    # body
    body_properties = dict(original_properties)
    body_properties.pop("_session")
    body_properties.pop("_supergraph")
    method_body_lines = [
        # general setup
        """\
# session / supergraph
if _session is None:
    self._session = ACTIVE_SESSION.get()
    if self._session is None:
        raise RuntimeError("no session")
if _supergraph is None:
    self._supergraph = self._session.supergraph
"""
    ]

    # node setup
    if is_node:
        body_properties.pop("id")
        body_properties.pop("ck", None)
        body_properties.pop("created_at")
        body_properties.pop("updated_at")
        body_properties.pop("_is_new")
        body_properties.pop("_graph")
        body_properties.pop("_hash")
        # node init
        if "ck" not in original_properties:
            method_body_lines.append("""\
# init node
if id is None:
    self.id = uuid4()
    now = self._session._oracle.utc()
    self.created_at = now
    self.updated_at = now
    self._is_new = True
self._hash = self.id.int
""")
        else:
            method_body_lines.append("""\
# init node
if id is None:
    self.id = uuid4()
    self.ck = self.id
    now = self._session._oracle.utc()
    self.created_at = now
    self.updated_at = now
    self._is_new = True
self._hash = self.id.int
""")

        # node graph
        method_body_lines.append("""\
# init graph
self._graph = _graph
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
            # set ptr_prop from prop if prop is set
            if prop.cardinality == "scalar":
                method_body_lines.append(f"""\
if {prop.name} is not None:
    {ptr_prop.name} = {prop.name}.to_ref()""")
            else:
                method_body_lines.append(f"""\
if {prop.name}:
    {ptr_prop.name} = tuple(x.to_ref() for x in {prop.name})""")
        else:
            # regular assignment
            method_body_lines.append(f"self.{prop.name} = {prop.name}")
    method_body = "\n".join(method_body_lines)
    method_body = textwrap.indent(method_body, "    ")

    init_str = f"{method_header}:\n{method_body}"
    if IS_DEV:
        init_str = format_code(init_str)
    return init_str


def _generate_property_property(prop: Property) -> str:
    """The computed get/set property for a property reference."""

    ptr_prop = prop.ptr_prop
    assert ptr_prop is not None, f"no wired prop for {prop!r}"

    if prop.cardinality == "scalar":
        # property scalar
        return f"""\
@property
def {prop.name}(self: "BuiltinObject") -> "Property | None":
    value_ptr: PropertyReference | None = self.{ptr_prop.name}
    if value_ptr is not None:
        return value_ptr.resolve()
    else:
        return None

@{prop.name}.setter
def {prop.name}(self: "BuiltinObject", value: "Property | None"):
    if value is None:
        {ptr_prop.name} = None
    else:
        {ptr_prop.name} = value.to_ref()
"""
    else:
        # property list
        return f"""\
@property
def {prop.name}(self: "BuiltinObject") -> tuple["Property", ...]:
    value_ptrs: list[PropertyReference] = self.{ptr_prop.name}
    assert type(value_ptrs) is list, f"invalid {prop}: {{value_ptrs!r}}"
    return tuple(p.resolve() for p in value_ptrs)

@{prop.name}.setter
def {prop.name}(self: "BuiltinObject", values: list["Property"]):
    self.{ptr_prop.name} = [p.to_ref() for p in values]
"""


def _generate_node_property(prop: Property) -> str:
    """The computed get/set property for a node reference. Resolved against the active supergraph."""

    ptr_prop = prop.ptr_prop
    assert ptr_prop is not None, f"no wired prop for {prop!r}"

    if prop.cardinality == "scalar":
        return f"""\
@property
def {prop.name}(self: "BuiltinObject") -> "Node | None":
    node_ptr: NodeReference | None = self.{ptr_prop.name}
    if node_ptr is not None:
        return self._supergraph.get(node_ptr)
    else:
        return None

@{prop.name}.setter
def {prop.name}(self: "BuiltinObject", value: "Node | None"):
    if value is None:
        self.{ptr_prop.name} = None
    else:
        self.{ptr_prop.name} = value.to_ref()
"""
    else:
        return f"""\
@property
def {prop.name}(self: "BuiltinObject") -> tuple["Node", ...]:
    node_ptrs: list[NodeReference] = self.{ptr_prop.name}
    assert type(node_ptrs) is list, f"invalid {prop}: {{node_ptrs!r}}"
    return tuple(self._supergraph.get(p) for p in node_ptrs)

@{prop.name}.setter
def {prop.name}(self: "BuiltinObject", nodes: list["Node"]):
    self.{ptr_prop.name} = [n.to_ref() for n in nodes]
"""


def _generate_node_key_property(obj_key: str, ptr_key: str, prop: Property) -> str:
    """The computed get property from a specific attribute of a node pointer."""

    ptr_prop = prop.ptr_prop
    assert ptr_prop is not None, f"no wired prop for {prop!r}"

    if prop.cardinality == "scalar":
        return f"""\
@property
def {prop.name}_{obj_key}(self: "BuiltinObject") -> "Node | None":
    node_ptr: NodeReference | None = self.{ptr_prop.name}
    if node_ptr is not None:
        return node_ptr.{ptr_key}
    else:
        return None
"""
    else:
        return f"""\
@property
def {prop.name}_{obj_key}(self: "BuiltinObject") -> tuple["Node", ...]:
    node_ptrs: list[NodeReference] = self.{ptr_prop.name}
    assert type(node_ptrs) is list, f"invalid {prop}: {{node_ptrs!r}}"
    return tuple(node_ptr.{ptr_key} for node_ptr in node_ptrs)
"""


def _generate_ancestor_property(object_type: NodeType | StructType, prop: Property) -> str:
    """The computer get property for Node ancestors."""

    node_types_str = ", ".join(str(t.value) for t in prop.nodes)

    if prop.node_kind == NodeReferenceKind.NODE_ANCESTOR_OR_SELF and object_type in prop.nodes:
        return f"""\
@property
def {prop.name}(self: "Node") -> "Node":
    return self
"""
    else:
        return f"""\
@property
def {prop.name}(self: "Node") -> "Node | None":
    node = self.parent
    while node is not None:
        if node.metatype in ({node_types_str}):
            return node
        node = node.parent
    return None
"""


def _process_object_cls[ObjectT: BuiltinObject](
    cls: type[ObjectT],
    object_type: NodeType | StructType | None,
    is_concrete: bool = False,
    is_struct: bool = False,
    is_node: bool = False,
) -> tuple[type[ObjectT], dict[str, "Property"]]:
    """Process a BuiltinObject base class and return the processed class and its properties."""
    assert isinstance(cls, type), f"expected type, got {cls} ({type(cls)})"
    assert cls not in _processed_classes, f"class {cls.__name__} has already been processed"

    metatype = Property(
        id=1,
        name="metatype",
        default=None,
        py_type_raw=NodeType if is_node else StructType,
        is_internal=True,
        cardinality="scalar",
        is_required=True,
        is_computed=True,  # is set statically by class decorator
        is_wired=True,
        is_stored=False,
        primitive_type=PrimitiveType.INT16,
        enum_type=EnumType.NODE_TYPE if is_node else EnumType.STRUCT_TYPE,
        component=cls,
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
        prop.py_type_raw = cls.__annotations__.get(name, None)
        properties[name] = prop
    cls.__declared_properties__ = frozendict(properties)

    # collect properties from ancestor components (closest first)
    for component in reversed(components[1:]):
        for name, prop in component.__declared_properties__.items():
            existing = properties.get(name)
            if existing is not None:
                if existing.name in ("metatype", "parent"):
                    continue  # may be narrowed
                raise RuntimeError(
                    f"property '{name}' from '{component.__name__}' conflicts with '{cls.__name__}': {prop!r}, {existing!r}"
                )
            prop = prop.clone()
            prop.component = cls
            properties[name] = prop

    # finalize properties
    for prop in tuple(properties.values()):
        prop.finalize(object_type)
        if prop.ptr_prop is not None:
            properties[prop.ptr_prop.name] = prop.ptr_prop

    # index properties
    cls.__properties__ = frozendict(properties)
    properties_by_id: dict[int, Property] = {}
    for prop in properties.values():
        if prop.id is not None and prop.id is not UNSET and not prop.runtime_prop:
            existing = properties_by_id.get(prop.id, None)
            if existing is None:
                properties_by_id[prop.id] = prop
            elif not prop.runtime_prop:
                # contributed reference properties can share an id
                raise ValueError(f"property id conflict: {prop!r}, {existing!r}")
    props = properties.values()
    cls.__properties_by_id__ = frozendict(properties_by_id)
    cls.__node_properties__ = frozendict(
        {p.name: p for p in props if p.is_node_reference and not p.runtime_prop}
    )
    cls.__struct_properties__ = frozendict({p.name: p for p in props if p.is_struct is True})
    cls.__wired_properties__ = frozendict(
        {p.name: p for p in props if p.is_wired is True and p.ptr_prop is None}
    )
    cls.__stored_properties__ = frozendict(
        {p.name: p for p in props if p.is_stored is True and p.ptr_prop is None}
    )

    # assign property ordinals
    cls.__properties_in_order__ = tuple(sorted(properties_by_id.values(), key=lambda p: p.id))
    for i, prop in enumerate(cls.__properties_in_order__):
        prop.component = cls
        prop.ord = i
        if prop.ptr_prop:
            prop.ptr_prop.ord = i
    cls.__properties_id_in_order__ = tuple(p.id for p in cls.__properties_in_order__)
    cls.__max_property_ord__ = len(cls.__properties_in_order__) - 1
    cls.__properties_mask_set__ = bitarray(cls.__max_property_ord__ + 1)
    cls.__properties_mask_set__.setall(True)
    cls.__properties_mask_unset__ = bitarray(cls.__max_property_ord__ + 1)

    # define slots & __init__ (in leaf classes, otherwise their slots might clash)
    if is_concrete:
        assert object_type is not None, f"concrete objects need a type: {cls.__name__}"
        cls_dict = dict(cls.__dict__)
        cls_dict.pop("__dict__", None)
        cls_dict.pop("__weakref__", None)
        for prop in properties.values():
            cls_dict.pop(prop.name, None)
        cls_dict["__slots__"] = tuple(cls.__wired_properties__.keys())
        init_str = _generate_init_for_cls(cls, is_node=is_node, header_properties=properties)
        exec(init_str, {"ACTIVE_SESSION": ACTIVE_SESSION}, cls_dict)

        # add computed properties to concrete classes
        for prop in properties.values():
            if prop.runtime_prop is not None:
                continue  # not a contributed property
            # computed property property
            if prop.is_property_reference:
                property_property_str = _generate_property_property(prop)
                exec(property_property_str, {}, cls_dict)
            # computed node property
            elif prop.node_kind in (
                NodeReferenceKind.NODE_PARENT,
                NodeReferenceKind.NODE_REGULAR,
                NodeReferenceKind.NODE_TEMPLATE,
            ):
                node_property_str = _generate_node_property(prop)
                exec(node_property_str, {}, cls_dict)
            # computed node ancestor property
            elif prop.node_kind in (
                NodeReferenceKind.NODE_ANCESTOR,
                NodeReferenceKind.NODE_ANCESTOR_OR_SELF,
            ):
                ancestor_property_str = _generate_ancestor_property(object_type, prop)
                exec(ancestor_property_str, {}, cls_dict)
            # computed _x node reference properties (e.g., parent_id, node_ck, node_type, ...)
            if prop.is_node_reference:
                for obj_key, ptr_key in (("id", "id"), ("ck", "ck"), ("type", "node_type")):
                    if obj_key == "type" and (not prop.nodes or len(prop.nodes) <= 1):
                        continue  # no need for *_type if only one possible node type
                    node_key_property_str = _generate_node_key_property(obj_key, ptr_key, prop)
                    exec(node_key_property_str, {}, cls_dict)

        # create the new class
        _original_cls = cls
        cls = cast(type[ObjectT], type(cls.__name__, cls.__bases__, cls_dict))
        _processed_classes[_original_cls] = cls
        del _original_cls

        # update cls references in props
        for prop in properties.values():
            prop.component = cls

    return cls, properties  # type: ignore


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def object_[_ObjectT: BuiltinObject](
    struct_type: StructType | None = None,
    is_concrete: bool = False,
    is_struct: bool = False,
    is_node: bool = False,
):
    """
    Mark a class as an object component (or concrete struct for a StructType).
    """

    def decorate(cls_in: type[_ObjectT]) -> type[_ObjectT]:
        cls, _properties = _process_object_cls(
            cls=cast(Any, cls_in),
            object_type=struct_type,
            is_concrete=is_concrete,
            is_struct=is_struct,
            is_node=is_node,
        )

        # register struct
        if struct_type:
            cls.metatype = struct_type
            if struct_type in STRUCT_CLASS_BY_TYPE:
                raise ValueError(
                    f"struct class conflict for {struct_type}: {cls}, {STRUCT_CLASS_BY_TYPE[struct_type]}"
                )
            STRUCT_CLASS_BY_TYPE[struct_type] = cls
        return cast(type[_ObjectT], cls)

    return decorate


@dataclass_transform(kw_only_default=True, field_specifiers=_PROPERTY_SPECIFIERS)
def struct_[_ObjectT: BuiltinObject](struct_type: StructType):
    """Register a class as a concrete struct for the given struct type."""

    def decorate(cls: type[_ObjectT]) -> type[_ObjectT]:
        cls = object_(struct_type=struct_type, is_concrete=True)(cls)
        return cast(type[_ObjectT], cls)

    return decorate


_HANDLING_ATTRIBUTE_ERROR = contextvars.ContextVar("handling_attribute_error", default=False)


@object_()
class BuiltinObject[ObjectDataT: AnyObjectData](abc.ABC):
    """The base for all intrinsic objects like Structs and Nodes and all their derivatives."""

    __is_struct__: ClassVar[bool] = False
    __is_node__: ClassVar[bool] = False

    __properties__: ClassVar[dict[str, Property]] = {}
    __properties_by_id__: ClassVar[dict[int, Property]] = {}

    __declared_properties__: ClassVar[dict[str, Property]] = {}
    __node_properties__: ClassVar[dict[str, Property]] = {}
    __struct_properties__: ClassVar[dict[str, Property]] = {}
    __wired_properties__: ClassVar[dict[str, Property]] = {}
    __stored_properties__: ClassVar[dict[str, Property]] = {}

    __properties_in_order__: ClassVar[tuple[Property, ...]]
    __properties_id_in_order__: ClassVar[tuple[int, ...]]
    __max_property_ord__: ClassVar[int] = UNSET
    __properties_mask_set__: ClassVar[bitarray] = UNSET
    __properties_mask_unset__: ClassVar[bitarray] = UNSET

    _session: "Session" = property_()
    _supergraph: "Supergraph" = property_()

    def __content_str__(self) -> str:
        return ""  # empty by default

    @final
    def __str__(self):
        return self.__content_str__()

    def equals(
        self,
        other: Self | Any,
        identity_map: Mapping[UUID, "NodeReference"] = EMPTY_DICT,
    ) -> bool:
        """Checks if the content of the two objects is equal (recursively)."""
        # nocheckin: generate equals for BuiltinObject (and hash for Structs)
        raise NotImplementedError

    def validate(self) -> None:
        """Validate the object."""
        # nocheckin: genreate validate for BuiltinObject
        raise NotImplementedError

    def _stable_hash(self) -> int:
        """Hash of content properties."""
        raise NotImplementedError

    def _clone_kwargs(self, reset: bool = True):
        """Clone kwargs for a new instance."""
        copy_kwargs = {}
        for prop in self.__wired_properties__.values():
            prop_value = getattr(self, prop.name)
            if reset and (prop.id is None or prop.id < 30):
                continue  # ignore tracking/autoset properties
            if prop.is_struct:
                if prop.cardinality == "list":
                    if prop_value:
                        copy_kwargs[prop.name] = [item.clone() for item in prop_value]
                elif prop.cardinality == "scalar":
                    if prop_value is not None:
                        copy_kwargs[prop.name] = prop_value.clone()
                else:
                    raise RuntimeError(f"unsupported property: {prop.cardinality}")
            else:
                if prop.cardinality == "list":
                    if prop_value:
                        copy_kwargs[prop.name] = list(prop_value)
                elif prop.cardinality == "scalar":
                    copy_kwargs[prop.name] = prop_value
                else:
                    raise RuntimeError(f"unsupported property: {prop.cardinality}")
        return copy_kwargs

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
        exclude: Collection[NodeReferenceKind],
    ):
        """Replaces Node references with new Nodes. Missing Nodes are kept as is."""
        for prop in self.__node_properties__.values():
            if prop.node_kind in exclude:
                continue
            prop_value = getattr(self, prop.name)
            if prop.cardinality == "scalar":
                if prop_value is None:
                    continue
                new_node = new_node_by_id.get(prop_value.id)
                if new_node is not None:
                    setattr(self, prop.name, new_node)
            elif prop.cardinality == "list":
                new_nodes = [new_node_by_id.get(node.id) for node in prop_value]
                prop_value.set(new_nodes)
            else:
                raise RuntimeError(f"unsupported property: {prop.cardinality}")

    @property
    def active_session(self) -> "Session":
        """The currently active session (errors if none)"""
        assert self._session is not None, f"no session for {self!r}"
        return self._session

    def _walk_struct(self) -> Iterable["BuiltinObject"]:
        yield self
        for prop in self.__struct_properties__.values():
            value: Struct | list[Struct] | None = getattr(self, prop.name)
            if value is None:
                continue
            elif prop.cardinality == "scalar":
                yield from (cast(Struct, value))._walk_struct()
            elif prop.cardinality == "list":
                for item in cast(list, value):
                    yield from cast(Struct, item)._walk_struct()

    def __bool__(self):
        return True  # support truthy checks for objects

    @final
    def _to_data(self) -> ObjectDataT:
        """Convert to wire format"""
        from bench.proto.wiring import pack_builtin_object

        return pack_builtin_object(self)  # type: ignore

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
class Struct[StructDataT: AnyStructData](BuiltinObject[StructDataT], abc.ABC):
    """A Struct is a value with some properties."""

    metatype: ClassVar[StructType]  # type: ignore

    __is_struct__: ClassVar[bool] = True

    @final
    def __repr__(self):
        content_str = str(self)
        if content_str:
            return f"<{self.__class__.__name__} {content_str}>"
        else:
            return f"<{self.__class__.__name__}>"

    def __eq__(self, other):
        if other is self:
            return True
        elif other is None:
            return False
        else:
            return self.equals(other)


def is_node[T: Node](obj: Any, node_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, Node) and obj.metatype == node_cls.metatype


def is_struct[T: Struct | Struct](obj: Any, struct_cls: type[T]) -> TypeGuard[T]:
    return isinstance(obj, (Struct, Struct)) and obj.metatype == struct_cls.metatype


#
# Utility types
#


@struct_(StructType.SCOPE)
class Scope(Struct[ScopeData]):
    """The scope for an operation on the Bench graph."""

    region: Optional[Region] = property_(30)
    bench_id: Optional[UUID] = property_(31)
    package_ids: list[UUID] = property_(32)

    def __content_str__(self) -> str:
        return repr_scope(self)


def repr_scope(scope: Scope | ScopeData) -> str:
    if scope.package_ids:
        return f"[bench_id={scope.bench_id}, package_ids={', '.join(str(id) for id in scope.package_ids)}]"
    elif scope.bench_id:
        return f"[bench_id={scope.bench_id}]"
    else:
        return "[*]"


EMPTY_SCOPE_DATA = ScopeData(metatype=lang_pb2.StructType.STRUCT_TYPE_SCOPE)


@struct_(StructType.PROPERTY_REFERENCE)
class PropertyReference(Struct):
    """
    A reference to a builtin object's Property.
    If type is unset, this refers to a base property in one of the base BuiltinObject types.
    """

    node_type: NodeType | None = property_(30)
    struct_type: StructType | None = property_(31)
    id: int = property_(32)

    def __content_str__(self):
        object_cls = self.object_cls
        if object_cls is None:
            return "???"
        prop = object_cls.__properties_by_id__.get(self.id)
        if prop is None:
            return f"{object_cls.__name__}.??? [id={self.id}]"
        else:
            return f"{object_cls.__name__}.{prop.name} [id={self.id}]"

    @property
    def object_cls(self) -> type[BuiltinObject] | None:
        if self.node_type is not None:
            return BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.node_type)
        elif self.struct_type is not None:
            return BUILTIN_OBJECT_CLASS_BY_TYPE.get(self.struct_type)
        else:
            return None

    def resolve_or_error(self) -> Property:
        """Resolves the property reference to a Property."""
        resolved = self.resolve()
        if resolved is None:
            raise ValueError(f"could not resolve {self!r}")
        return resolved

    def resolve(self) -> Property | None:
        """Resolves the property reference to a Property."""
        from .node import Node

        object_cls = self.object_cls
        prop = (object_cls or Node).__properties_by_id__.get(self.id)
        return prop
