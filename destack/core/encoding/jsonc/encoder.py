from enum import Enum
from typing import Any, cast, override

from destack.core import ObjectKind, PropertyDeclaration, StructType

from ..json.encoder import JsonEncoder, JsonObjectEncoder, JsonValueEncoder


class JsoncEncoder(JsonEncoder):
    """Encoder for our custom constant folded JSON format."""

    @classmethod
    def generate(cls) -> "JsoncEncoder":
        from .generate import JsoncEncoderGenerator

        generator = JsoncEncoderGenerator()
        encoders: dict[tuple[ObjectKind, int], JsonObjectEncoder] = {}
        encoders[ObjectKind.STRUCT, StructType.VALUE] = cast(JsonObjectEncoder, JsonValueEncoder())
        encoders.update(generator.generate(omit=list(encoders.keys())))
        return cls(encoders)

    @override
    def get_target_property_key(self, prop: PropertyDeclaration) -> str:
        return str(prop.id)

    @override
    def pack_scalar_enum(self, enum_cls: type[Enum], value: Any) -> Any:
        return value.value

    @override
    def unpack_scalar_enum(self, enum_cls: type[Enum], value: Any) -> Any:
        return enum_cls(value)
