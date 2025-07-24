import textwrap
from itertools import chain
from typing import TYPE_CHECKING, Any

from destack.language.core import BuiltinObject, NodeType, ObjectKind, StructType
from destack.language.registry import ENUM_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from destack.utils.code import exec_
from destack.utils.log import get_logger
from destack.utils.telemetry import get_tracer
from destack.utils.uuid import UUID

if TYPE_CHECKING:
    pass

from .core import KompaktObjectEncoder

# pyright: reportIncompatibleVariableOverride=false


logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def _generate_kompakt_object_encoder(cls: type["BuiltinObject"]) -> tuple[str, str, dict[str, Any]]:
    """Generate the KompaktObjectEncoder class for a BuiltinObject."""

    pack_kompakt = textwrap.indent(_generate_pack_kompakt(cls), " " * 8)
    unpack_kompakt = textwrap.indent(_generate_unpack_kompakt(cls), " " * 8)
    encoder_name = f"{cls.__name__}KompaktEncoder"

    impl = f"""
class {encoder_name}(KompaktObjectEncoder):
    @override
    def pack_object(self, _object: "{cls.__name__}", writer: BinaryWriter) -> None:
{pack_kompakt}

    @override
    def unpack_object(self, _reader: BinaryReader, _session: Session | None) -> "{cls.__name__}":
{unpack_kompakt}
"""

    return (
        encoder_name,
        impl,
        {
            "KompaktObjectEncoder": KompaktObjectEncoder,
        },
    )


def _generate_pack_kompakt(cls: type["BuiltinObject"]) -> str:
    """Generate the pack_object method implementation."""
    return ""


def _generate_unpack_kompakt(cls: type["BuiltinObject"]) -> str:
    """Generate the unpack_object method implementation."""
    return ""


# registry of Kompakt encoders by (ObjectKind, NodeType|StructType)
KOMPAKT_OBJECT_ENCODERS: dict[tuple[ObjectKind, NodeType | StructType], KompaktObjectEncoder] = {}


def _generate():
    # generate pack/unpack methods
    builtin_class_by_name: dict[str, Any] = {"UUID": UUID}
    builtin_class_by_name.update(
        {
            cls.__name__: cls
            for cls in chain(
                NODE_CLASS_BY_TYPE.values(),
                STRUCT_CLASS_BY_TYPE.values(),
                ENUM_CLASS_BY_TYPE.values(),
            )
        }
    )
    for node_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        encoder_name, impl, extra_glbls = _generate_kompakt_object_encoder(node_cls)
        locals_ = {}
        exec_(
            impl,
            {**builtin_class_by_name, **extra_glbls},
            locals_,
            f"{node_cls.__name__}:kompakt",
        )
        encoder_cls = locals_[encoder_name]
        KOMPAKT_OBJECT_ENCODERS[node_cls.__kind__, node_cls.metatype] = encoder_cls()


_generate()
