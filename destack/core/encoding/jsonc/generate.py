from typing import TYPE_CHECKING, override

from destack.core import Object, PropertyDeclaration

from ..json.generate import JsonEncoderGenerator

if TYPE_CHECKING:
    pass


type_ = type


class JsoncEncoderGenerator(JsonEncoderGenerator):
    @override
    def get_encoder_name(self, cls: type["Object"]) -> str:
        """Get the name of the JsonObjectEncoder for an Object."""
        return f"{cls.__name__}JsoncEncoder"

    @override
    def generate_pack_object_type(self, cls: type["Object"]) -> str:
        """Generate the metakind/metatype code for an Object."""
        return f"""\
_object_json['metakind'] = '{cls.metakind.name}'
_object_json['metatype'] = '{cls.metatype.name}'
"""

    @override
    def get_target_property_key(self, prop: PropertyDeclaration) -> str:
        return str(prop.id)

    @override
    def generate_pack_enum(self, enum_name: str, source_expr: str) -> str:
        return f"{source_expr}.value"

    @override
    def generate_unpack_enum(self, enum_name: str, source_expr: str) -> str:
        return f"{enum_name}({source_expr})"
