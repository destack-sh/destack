import types
import typing
from typing import (
    TYPE_CHECKING,
    TypeAliasType,
)

from .declaration import TypeDeclaration
from .hoisted import (
    PRIMITIVE_TYPE_BY_ANNOTATION,
    EnumType,
    PrimitiveType,
    ScalarType,
    TypeCardinality,
)
from .string import Casing, to_casing
from .universe import HandleType, NodeType, StructType

if TYPE_CHECKING:
    pass

type_ = type


def resolve_enum_type(class_name: str) -> EnumType | None:
    """Get the EnumType for the given enum name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if enum_type := EnumType.__options_by_alias__.get(enum_name):
        return enum_type
    enum_name = class_name.upper()
    if enum_type := EnumType.__options_by_alias__.get(enum_name):
        return enum_type
    return None


def resolve_struct_type(class_name: str) -> StructType | None:
    """Get the StructType for the given struct name."""
    struct_name = to_casing(class_name, Casing.ALL_CAPS)
    if struct_type := StructType.__options_by_alias__.get(struct_name):
        return struct_type
    struct_name = class_name.upper()
    if struct_type := StructType.__options_by_alias__.get(struct_name):
        return struct_type
    return None


def resolve_handle_type(class_name: str) -> HandleType | None:
    """Get the HandleType for the given handle name."""
    handle_name = to_casing(class_name, Casing.ALL_CAPS)
    if handle_type := HandleType.__options_by_alias__.get(handle_name):
        return handle_type
    handle_name = class_name.upper()
    if handle_type := HandleType.__options_by_alias__.get(handle_name):
        return handle_type
    return None


def resolve_node_types(class_name: str) -> tuple[NodeType, ...] | None:
    """Get the NodeType for the given node name."""
    enum_name = to_casing(class_name, Casing.ALL_CAPS)
    if node_type := NodeType.__options_by_alias__.get(enum_name):
        return (node_type,)
    if node_type := NodeType.__options_by_alias__.get(class_name.upper()):
        return (node_type,)
    if class_name == "Node":
        return tuple(NodeType)
    return None


NONE_TYPE_DECLARATION = TypeDeclaration(
    cardinality=TypeCardinality.SCALAR,
    scalar_type=ScalarType.PRIMITIVE,
    primitive_type=PrimitiveType.NONE,
    is_required=False,
)


def parse_type_declaration(
    py_type: type | str | typing.ForwardRef,
    *,
    is_builtin: bool = False,
) -> TypeDeclaration:
    """
    Parses the TypeDeclaration from a given py type.
    NOTE: builtin members have some additional limitations (no unions, flat types, etc.).
    """

    # try to resolve
    if not isinstance(py_type, type):
        if isinstance(py_type, typing.ForwardRef):
            py_type = py_type.__forward_arg__

    origin = typing.get_origin(py_type)

    # list
    if origin is list:
        element_type_arg = typing.get_args(py_type)[0]
        element_annotation = parse_type_declaration(element_type_arg, is_builtin=is_builtin)
        return TypeDeclaration(
            cardinality=TypeCardinality.LIST,
            value_type=element_annotation,
            is_required=True,
        )

    # tuple
    if origin is tuple:
        type_args = typing.get_args(py_type)
        if not type_args:
            raise ValueError(f"cannot infer type of empty tuple annotation: {py_type!r}")
        # check for homogeneous tuple notation: tuple[int, ...]
        if len(type_args) == 2 and type_args[1] is ...:
            raise ValueError(f"cannot infer type of tuple: {py_type!r}")
        # heterogeneous tuple
        element_types = [parse_type_declaration(arg, is_builtin=is_builtin) for arg in type_args]
        return TypeDeclaration(
            cardinality=TypeCardinality.TUPLE,
            element_types=element_types,
            is_required=True,
        )

    # union/optional
    is_required = True
    if origin in (typing.Union, types.UnionType):
        union_args = typing.get_args(py_type)
        is_required = not any(t is type(None) for t in union_args)
        non_none_types = tuple(t for t in union_args if t is not type(None))
        assert len(non_none_types) > 0, f"empty union: {py_type!r}"

        if len(non_none_types) == 1:
            # it's just an optional of that type
            result = parse_type_declaration(non_none_types[0], is_builtin=is_builtin)
            result.is_required = is_required
            return result
        else:
            # check if all types are Node types
            all_node_types: list[NodeType] = []
            all_are_nodes = True
            for union_type in non_none_types:
                union_class_name = _get_class_name(union_type)
                if union_class_name and (node_t := resolve_node_types(union_class_name)):
                    all_node_types.extend(node_t)
                else:
                    all_are_nodes = False
                    break

            if all_are_nodes and all_node_types:
                # union of node types
                return TypeDeclaration(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.NODE_REFERENCE,
                    node_types=tuple(all_node_types),
                    is_required=is_required,
                )
            else:
                # general union
                element_types = tuple(
                    parse_type_declaration(arg, is_builtin=is_builtin) for arg in non_none_types
                )
                return TypeDeclaration(
                    cardinality=TypeCardinality.SCALAR,
                    scalar_type=ScalarType.UNION,
                    element_types=tuple(element_types),
                    is_required=is_required,
                )

    # map
    if origin is dict:
        key_type_arg, value_type_arg = typing.get_args(py_type)
        key_annotation = parse_type_declaration(key_type_arg, is_builtin=is_builtin)
        value_annotation = parse_type_declaration(value_type_arg, is_builtin=is_builtin)
        if is_builtin:
            assert key_annotation.cardinality == TypeCardinality.SCALAR, (
                f"builtin Types don't support map keys: {py_type!r}"
            )
            assert value_annotation.cardinality == TypeCardinality.SCALAR, (
                f"builtin Types don't support map values: {py_type!r}"
            )
        return TypeDeclaration(
            cardinality=TypeCardinality.MAP,
            key_type=key_annotation,
            value_type=value_annotation,
            is_required=is_required,
        )

    # determine scalar type
    return parse_type_declaration_scalar(
        py_type,
        is_builtin=is_builtin,
        is_required=is_required,
    )


def parse_type_declaration_scalar(
    py_type: type | str | typing.ForwardRef,
    *,
    is_builtin: bool = False,
    is_required: bool = True,
) -> TypeDeclaration:
    """
    Parses the TypeDeclaration from a given py type.
    """
    scalar_type: ScalarType | None = None
    primitive_type: PrimitiveType | None = None
    enum_type: EnumType | None = None
    struct_type: StructType | None = None
    handle_type: HandleType | None = None
    node_types: list[NodeType] | None = None
    is_self = False
    is_any = False
    class_name = _get_class_name(py_type) if py_type is not type(None) else "None"
    assert class_name is not None, f"undetermined class name: {py_type!r}"
    if isinstance(py_type, (type, TypeAliasType)) and (
        primitive_t := PRIMITIVE_TYPE_BY_ANNOTATION.get(py_type)
    ):
        if is_builtin and py_type in (float, int):
            # shouldn't use float/int directly, use a specific precision/size
            raise ValueError(f"unspecific primitive type: {py_type!r}")
        scalar_type = ScalarType.PRIMITIVE
        primitive_type = primitive_t
    elif class_name == "None":
        scalar_type = ScalarType.PRIMITIVE
        primitive_type = PrimitiveType.NONE
        is_required = False
    elif class_name == "Self":
        scalar_type = ScalarType.NODE_REFERENCE
        is_self = True
    elif class_name == "Any":
        scalar_type = ScalarType.PRIMITIVE
        primitive_type = PrimitiveType.NONE  # handled manually
        is_any = True
    elif class_name == "Object":
        scalar_type = ScalarType.STRUCT
        is_any = True
    elif enum_t := resolve_enum_type(class_name):
        scalar_type = ScalarType.ENUM
        enum_type = enum_t
    elif struct_t := resolve_struct_type(class_name):
        scalar_type = ScalarType.STRUCT
        struct_type = struct_t
    elif node_t := resolve_node_types(class_name):
        scalar_type = ScalarType.NODE_REFERENCE
        node_types = list(node_t)
    elif handle_t := resolve_handle_type(class_name):
        scalar_type = ScalarType.HANDLE
        handle_type = handle_t
    else:
        raise ValueError(f"undetermined scalar type: {py_type!r} ({type(py_type)})")

    # default: scalar
    return TypeDeclaration(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=scalar_type,
        primitive_type=primitive_type,
        enum_type=enum_type,
        struct_type=struct_type,
        handle_type=handle_type,
        node_types=node_types,
        is_any=is_any,
        is_self=is_self,
        is_required=is_required,
    )


def _get_class_name(
    py_type: type | typing.ForwardRef | typing.TypeAliasType | typing._SpecialForm | str,
) -> str:
    if isinstance(py_type, str):
        name = py_type
    elif isinstance(py_type, (type, typing.TypeAliasType)):
        name = py_type.__name__
    elif isinstance(py_type, typing.ForwardRef):
        name = py_type.__forward_arg__
    elif isinstance(py_type, typing._SpecialForm):
        name = getattr(py_type, "__name__")
    else:
        raise ValueError(f"unexpected type: {py_type!r}")

    # strip generic type parameters
    if "[" in name:
        name = name.split("[")[0]

    return name
