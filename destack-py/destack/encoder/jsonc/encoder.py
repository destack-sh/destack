from typing import Any, ClassVar, cast, override

from destack.language.core import (
    EncoderOptions,
    Encoding,
    NodeType,
    ObjectKind,
    ScalarType,
    Session,
    StructType,
    Type,
)
from destack.language.core.builtin.const import METATYPE_PROPERTY_KEY
from destack.language.registry import ENUM_CLASS_BY_TYPE

from ..json.encoder import JsonEncoder, JsonObjectEncoder, JsonValueEncoder


class JsoncEncoder(JsonEncoder):
    """Encoder for our custom constant folded JSON format."""

    encoding: ClassVar[Encoding] = Encoding.JSONC

    @classmethod
    def generate(cls) -> "JsoncEncoder":
        from .generate import JsoncEncoderGenerator

        generator = JsoncEncoderGenerator()
        encoders: dict[tuple[ObjectKind, int], JsonObjectEncoder] = {}
        encoders[ObjectKind.STRUCT, StructType.VALUE] = cast(JsonObjectEncoder, JsonValueEncoder())
        encoders.update(generator.generate(omit=list(encoders.keys())))
        return cls(encoders)

    @override
    def pack_scalar_value(
        self,
        type: Type,
        value: Any,
        options: EncoderOptions,
    ) -> str:
        if type.scalar_type == ScalarType.ENUM:
            return value.value
        else:
            return super().pack_scalar_value(type, value, options)

    @override
    def unpack_scalar_value(
        self,
        type: Type,
        value: Any,
        session: Session | None,
        options: EncoderOptions,
    ) -> Any:
        # enum
        if type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return enum_cls(value)

        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            node_type = NodeType[value[METATYPE_PROPERTY_KEY]]
            return self.unpack_object(ObjectKind.NODE, node_type, value, session, options)

        # default
        else:
            return super().unpack_scalar_value(type, value, session, options)
