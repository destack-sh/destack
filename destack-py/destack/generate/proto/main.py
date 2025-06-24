from collections.abc import Collection
from itertools import chain
from pathlib import Path
from typing import TYPE_CHECKING, Any, cast

from destack.language import (
    NODE_TYPES,
    BuiltinObjectBase,
    Enum,
    EnumType,
    NodeType,
    StructType,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    get_builtin_class,
)
from destack.utils.string import Casing, to_casing

from ..core import LANGUAGE_PROTO
from .core import (
    ProtoEnum,
    ProtoField,
    ProtoFieldType,
    ProtoMessage,
    ProtoObject,
    ProtoSchema,
)
from .map import _map_object_type_to_proto

if TYPE_CHECKING:
    pass


def _generate_proto_schema(
    name: str,
    unions: dict[str, tuple[str, Collection[EnumType | NodeType | StructType]]],
    extras: list[ProtoEnum | ProtoMessage],
    postfix: str,
) -> ProtoSchema:
    # walk all destack types to populate the cache
    cache: dict[type[BuiltinObjectBase] | type[Enum], ProtoObject] = {}
    for destack_cls in chain(
        NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values(), ENUM_CLASS_BY_TYPE.values()
    ):
        cache[destack_cls] = _map_object_type_to_proto(destack_cls, cache)

    # collect all proto types
    proto_types: list[ProtoEnum | ProtoMessage] = []
    for proto_type in cache.values():
        if isinstance(proto_type, (ProtoEnum, ProtoMessage)):
            proto_types.append(proto_type)
    # sort: enum -> struct -> node (and by name within each group)
    proto_types.sort(key=lambda t: (isinstance(t, ProtoMessage), isinstance(t, ProtoEnum), t.name))

    # add custom union types
    for union_name, (wrapper_field_name, unioned_types) in unions.items():
        sub_fields = [
            ProtoField(
                id=t.value,
                name=to_casing(t.name, Casing.SNAKE),
                type=cast(Any, _map_object_type_to_proto(get_builtin_class(t), cache)),
            )
            for t in unioned_types
        ]
        wrapper_field = ProtoField(
            id=None, name=wrapper_field_name, type=ProtoFieldType.ONE_OF, sub_fields=sub_fields
        )
        wrapper_message = ProtoMessage(
            name=union_name, reserved_names=[], reserved_ids=[], fields=[wrapper_field]
        )
        proto_types.append(wrapper_message)
    proto_types.extend(extras)

    # apply postfix to messages
    if postfix:
        for proto_type in proto_types:
            if proto_type not in extras:
                proto_type.name += postfix

    return ProtoSchema.from_types(name, proto_types)


def generate():
    proto_schema = _generate_proto_schema(
        name="symbol.destack",
        unions={"SomeNode": ("node", NODE_TYPES)},
        extras=[],
        postfix="Proto",
    )
    Path(LANGUAGE_PROTO).write_text(proto_schema.to_proto_source())
