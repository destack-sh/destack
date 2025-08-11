import base64
import textwrap
from collections.abc import Collection
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.registry import (
    BUILTIN_CLASS_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)

from ..._utils import execute_arbitrary_code
from ...builtin import (
    UNSET,
    UUID,
    Entity,
    Materialization,
    Object,
    ObjectKind,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    StringCasing,
    StructType,
    TypeCardinality,
    TypeDeclaration,
    to_casing,
)
from ..encoder import EncoderFlag
from ..time import timedelta_from_isoformat, timedelta_to_isoformat

if TYPE_CHECKING:
    pass

from .core import JsonObjectEncoder

type_ = type


class JsonEncoderGenerator:
    """Generate a JsonObjectEncoder for an Object."""

    def get_encoder_name(self, cls: type["Object"]) -> str:
        """Get the name of the JsonObjectEncoder for an Object."""
        return f"{cls.__name__}JsonEncoder"

    def generate_object_encoder(self, cls: type["Object"]) -> tuple[str, str, dict[str, Any]]:
        """Generate the JsonObjectEncoder class for an Object."""

        is_entity = issubclass(cls, Entity)
        pack_json = self.generate_pack_object(cls, is_entity=is_entity)
        unpack_json = self.generate_unpack_object(cls, is_entity=is_entity)
        encoder_name = self.get_encoder_name(cls)

        impl = f"""
class {encoder_name}(JsonObjectEncoder):
    
    @override
    def pack_object(
        self,
        _encoder: "JsonEncoder",
        _object: "{cls.__name__}",
        _options: "EncoderOptions",
    ) -> "dict[str, Any]":
{textwrap.indent(pack_json, " " * 8)}

    @override
    def unpack_object(
        self,
        _encoder: "JsonEncoder",
        _object_json: "dict[str, Any]",
        _session: "Session | None",
        _options: "EncoderOptions",
    ) -> "{cls.__name__}":
{textwrap.indent(unpack_json, " " * 8)}
"""
        return (
            encoder_name,
            impl,
            {
                "JsonObjectEncoder": JsonObjectEncoder,
                "timedelta_from_isoformat": timedelta_from_isoformat,
                "timedelta_to_isoformat": timedelta_to_isoformat,
                "datetime": datetime,
                "timedelta": timedelta,
                "UNSET": UNSET,
                "date": date,
                "time": time,
                "UTC": UTC,
                "base64": base64,
                "UUID": UUID,
                "override": override,
                "dict": dict,
                "Any": Any,
                "Self": cls,
                "cls": cls,
                "Object": Object,
                "EncoderOptions": EncoderFlag,
            },
        )

    def generate_pack_object_type(self, cls: type["Object"]) -> str:
        """Generate the metakind/metatype code for an Object."""
        return f"""\
_object_json['metakind'] = '{cls.metakind.name}'
_object_json['metatype'] = '{cls.metatype.name}'
"""

    def generate_pack_object(
        self,
        cls: type["Object"],
        *,
        is_entity: bool,
    ) -> str:
        """Generate the pack method for an Object."""
        lines: list[str] = [
            "_object_json: dict[str, Any] = {}",
            self.generate_pack_object_type(cls),
        ]

        # collect properties
        properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
        properties.sort(key=lambda p: p.id or 0)

        # pack always set properties
        for prop in properties:
            prop_name = self.get_source_property_name(prop)
            pack_code = self.generate_pack_value(
                prop.type,
                key=f"_{prop_name}",
                source_expr=f"_object.{prop_name}",
                target_expr=f"_object_json['{self.get_target_property_key(prop)}']",
                can_omit_none=True,
            )
            lines.append(pack_code)

        lines.append("return _object_json")
        return "\n".join(lines)

    def generate_unpack_object(self, cls: type["Object"], *, is_entity: bool) -> str:
        """Generate the unpack method for an Object."""
        assignments: list[str] = []
        lines: list[str] = []
        if is_entity:
            materialization_key = self.get_target_property_key(Entity.property("materialization"))
            materialization_expr = self.generate_unpack_enum(
                "Materialization", f"_object_json.get('{materialization_key}')"
            )
            lines.append(f"_is_partial = {materialization_expr} < {Materialization.FULL}")

        # collect properties
        properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
        properties.sort(key=lambda p: p.id or 0)

        # unpack always set properties
        for prop in properties:
            prop_name = self.get_source_property_name(prop)
            unpack_code = self.generate_unpack_value(
                prop.type,
                key=f"_{prop_name}",
                source_expr=f'_object_json.get("{self.get_target_property_key(prop)}")',
                target_expr=f"_unpacked_{prop_name}",
            )
            lines.append(unpack_code)
            assignments.append(f"{prop_name}=_unpacked_{prop_name}")

        lines.append(f"return {cls.__name__}(")
        for assignment in assignments:
            lines.append(f"    {assignment},")
        lines.append("    _session=_session,")
        lines.append(")")

        return "\n".join(lines)

    def get_source_property_name(self, prop: PropertyDeclaration) -> str:
        """Get the name of a property."""
        source_name = prop.name
        if prop.type.scalar_type == ScalarType.NODE_MOMENT:
            source_name += "_ptr"
        return source_name

    def get_target_property_key(self, prop: PropertyDeclaration) -> str:
        """Get the target property key for JSON."""
        target_key = to_casing(prop.name, StringCasing.LOWER_CAMEL)
        if prop.type.scalar_type == ScalarType.NODE_MOMENT:
            target_key += "Ptr"
        return target_key

    def generate_pack_enum(self, enum_name: str, source_expr: str) -> str:
        return f"{source_expr}.name"

    def generate_unpack_enum(self, enum_name: str, source_expr: str) -> str:
        return f"{enum_name}[{source_expr}]"

    def generate_pack_value(
        self,
        type: TypeDeclaration,
        key: str,
        source_expr: str,
        target_expr: str,
        can_omit_none: bool,
    ) -> str:
        """Generate code to pack some value into the JSON encoding."""
        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            value_packed = self.generate_pack_scalar_value(type, source_expr)
            if type.is_required:
                return f"{target_expr} = {value_packed}"
            else:
                return f"""\
if {source_expr} is not None:
    {target_expr} = {value_packed}
{"elif not _options & EncoderOptions.OMIT_NONE:" if can_omit_none else "else:"}
    {target_expr} = None"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_source_expr = f"{key}_item"
            item_target_expr = f"{key}_item_json"
            item_packed = self.generate_pack_value(
                type.value_type,
                key=f"{key}_value",
                source_expr=item_source_expr,
                target_expr=item_target_expr,
                can_omit_none=False,
            )
            if type.is_required:
                return f"""\
{target_expr} = []
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 4)}
    {target_expr}.append({item_target_expr})"""
            else:
                return f"""\
if {source_expr} is not None:
    {target_expr} = []
    for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 8)}
        {target_expr}.append({item_target_expr})
{"elif not _options & EncoderOptions.OMIT_NONE:" if can_omit_none else "else:"}
    {target_expr} = None"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            element_target_exprs = []
            element_packs = []
            for i, element_type in enumerate(type.element_types):
                element_source_expr = f"{key}_element_{i}"
                element_target_expr = f"{key}_element_{i}_json"
                element_target_exprs.append(element_target_expr)
                element_packed = self.generate_pack_value(
                    element_type,
                    key=f"{key}_element_{i}",
                    source_expr=element_source_expr,
                    target_expr=element_target_expr,
                    can_omit_none=False,
                )
                element_packs.append(element_packed)
            element_packed_str = "\n".join(element_packs)
            if type.is_required:
                return f"""\
    {element_packed_str}
    {target_expr} = [{", ".join(element_target_exprs)}]"""
            else:
                return f"""\
if {source_expr} is not None:
{textwrap.indent(element_packed_str, " " * 4)}
    {target_expr} = [{", ".join(element_target_exprs)}]
{"elif not _options & EncoderOptions.OMIT_NONE:" if can_omit_none else "else:"}
    {target_expr} = None"""

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key"
            key_target_expr = f"{key}_key_json"
            value_source_expr = f"{key}_value"
            value_target_expr = f"{key}_value_json"
            key_packed = self.generate_pack_value(
                type.key_type,
                key=f"{key}_key",
                source_expr=key_source_expr,
                target_expr=key_target_expr,
                can_omit_none=False,
            )
            value_packed = self.generate_pack_value(
                type.value_type,
                key=f"{key}_value",
                source_expr=value_source_expr,
                target_expr=value_target_expr,
                can_omit_none=False,
            )
            if type.is_required:
                return f"""\
{target_expr} = {{}}
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 4)}
{textwrap.indent(value_packed, " " * 4)}
    {target_expr}[{key_target_expr}] = {value_target_expr}"""
            else:
                return f"""\
if {source_expr} is not None:
    {target_expr} = {{}}
    for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 8)}
{textwrap.indent(value_packed, " " * 8)}
        {target_expr}[{key_target_expr}] = {value_target_expr}
{"elif not _options & EncoderOptions.OMIT_NONE:" if can_omit_none else "else:"}
    {target_expr} = None"""

        else:
            assert_never(type.cardinality)

    def generate_unpack_value(
        self, type: TypeDeclaration, key: str, source_expr: str, target_expr: str
    ) -> str:
        """Generate code to unpack a property from the JSON encoding."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if type.is_required:
                value_unpacked = self.generate_unpack_scalar_value(type, source_expr)
                return f"{target_expr} = {value_unpacked}"
            else:
                value_unpacked = self.generate_unpack_scalar_value(type, source_expr)
                return f"""\
if {source_expr} is not None:
    {target_expr} = {value_unpacked}
else:
    {target_expr} = None"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_source_expr = f"{key}_item_json"
            item_target_expr = f"{key}_item"
            item_unpacked = self.generate_unpack_value(
                type.value_type,
                key=f"{key}_value",
                source_expr=item_source_expr,
                target_expr=item_target_expr,
            )
            if type.is_required:
                return f"""\
{target_expr} = []
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_unpacked, " " * 4)}
    {target_expr}.append({item_target_expr})"""
            else:
                return f"""\
if {source_expr} is not None:
    {target_expr} = []
    for {item_source_expr} in {source_expr}:
{textwrap.indent(item_unpacked, " " * 8)}
        {target_expr}.append({item_target_expr})
else:
    {target_expr} = None"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            assert type.element_types is not None, f"no element types for {type!r}"
            element_target_exprs = []
            element_unpacks = []
            for i, element_type in enumerate(type.element_types):
                element_source_expr = f"{key}_element_{i}_json"
                element_target_expr = f"{key}_element_{i}"
                element_target_exprs.append(element_target_expr)
                element_unpacked = self.generate_unpack_value(
                    element_type,
                    key=f"{key}_element_{i}",
                    source_expr=element_source_expr,
                    target_expr=element_target_expr,
                )
                element_unpacks.append(element_unpacked)
            element_unpacked_str = "\n".join(element_unpacks)
            if type.is_required:
                return f"""\
    {element_unpacked_str}
    {target_expr} = [{", ".join(element_target_exprs)}]"""
            else:
                return f"""\
if {source_expr} is not None:
    {textwrap.indent(element_unpacked_str, " " * 4)}
    {target_expr} = ({", ".join(element_target_exprs)})
else:
    {target_expr} = None"""

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key_json"
            key_target_expr = f"{key}_key"
            value_source_expr = f"{key}_value_json"
            value_target_expr = f"{key}_value"
            key_unpacked = self.generate_unpack_value(
                type.key_type,
                key=f"{key}_key",
                source_expr=key_source_expr,
                target_expr=key_target_expr,
            )
            value_unpacked = self.generate_unpack_value(
                type.value_type,
                key=f"{key}_value",
                source_expr=value_source_expr,
                target_expr=value_target_expr,
            )
            if type.is_required:
                return f"""\
{target_expr} = {{}}
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_unpacked, " " * 4)}
{textwrap.indent(value_unpacked, " " * 4)}
    {target_expr}[{key_target_expr}] = {value_target_expr}"""
            else:
                return f"""\
if {source_expr} is not None:
    {target_expr} = {{}}
    for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_unpacked, " " * 8)}
{textwrap.indent(value_unpacked, " " * 8)}
        {target_expr}[{key_target_expr}] = {value_target_expr}
else:
    {target_expr} = None"""

        else:
            assert_never(type.cardinality)

    def generate_pack_scalar_value(self, type: "TypeDeclaration", source_expr: str) -> str:
        """Generate the packing code for a scalar value in the JSON encoding."""

        assert type.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return "None"
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return source_expr
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
                return source_expr
            elif type.primitive_type in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            ):
                return source_expr
            elif type.primitive_type == PrimitiveType.DATETIME:
                return f"{source_expr}.astimezone(UTC).isoformat()"
            elif type.primitive_type == PrimitiveType.DATE:
                return f"{source_expr}.isoformat()"
            elif type.primitive_type == PrimitiveType.TIME:
                return f"{source_expr}.astimezone(UTC).replace(tzinfo=None).isoformat()"
            elif type.primitive_type == PrimitiveType.DURATION:
                return f"timedelta_to_isoformat({source_expr})"
            elif type.primitive_type == PrimitiveType.STRING:
                return source_expr
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return f"str({source_expr})"
            elif type.primitive_type == PrimitiveType.UUID:
                return f"str({source_expr})"
            elif type.primitive_type == PrimitiveType.BYTES:
                return f"base64.b64encode({source_expr}).decode()"
            elif type.primitive_type == PrimitiveType.JSON:
                return source_expr
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return self.generate_pack_enum(enum_cls.__name__, source_expr)
        # node
        elif type.scalar_type == ScalarType.NODE:
            return f"""\
_encoder.pack_object({source_expr}, _options & ~EncoderOptions.OMIT_METATYPE)"""
        # node id (untyped)
        elif type.scalar_type == ScalarType.NODE_UNTYPED_IDENTITY:
            return f"str({source_expr})"
        # node id (typed)
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return f"_encoder.pack_object({source_expr}, _options)"
        # node location
        elif type.scalar_type == ScalarType.NODE_LOCATION:
            return f"_encoder.pack_object({source_expr}, _options)"
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_MOMENT:
            return f"""\
_encoder.pack_object({source_expr}, _options)"""
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return f"""\
_encoder.pack_object({source_expr}, _options)"""
            else:
                return f"""\
_encoder.pack_object({source_expr}, _options & ~EncoderOptions.OMIT_METATYPE)"""
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot pack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot pack UNION: {type!r}")
        else:
            assert_never(type.scalar_type)

    def generate_unpack_scalar_value(self, type: "TypeDeclaration", source_expr: str) -> str:
        """Generate the unpacking code for a scalar value in the JSON encoding."""

        assert type.cardinality == TypeCardinality.SCALAR, f"cannot unpack non-scalar: {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                return "None"
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return source_expr
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
                return f"int({source_expr})"
            elif type.primitive_type in (
                PrimitiveType.FLOAT16,
                PrimitiveType.FLOAT32,
                PrimitiveType.FLOAT64,
            ):
                return f"float({source_expr})"
            elif type.primitive_type == PrimitiveType.DATETIME:
                return f"datetime.fromisoformat({source_expr})"
            elif type.primitive_type == PrimitiveType.DATE:
                return f"date.fromisoformat({source_expr})"
            elif type.primitive_type == PrimitiveType.TIME:
                return f"time.fromisoformat({source_expr})"
            elif type.primitive_type == PrimitiveType.DURATION:
                return f"timedelta_from_isoformat({source_expr})"
            elif type.primitive_type == PrimitiveType.STRING:
                return source_expr
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return source_expr
            elif type.primitive_type == PrimitiveType.UUID:
                return f"UUID({source_expr})"
            elif type.primitive_type == PrimitiveType.BYTES:
                return f"base64.b64decode({source_expr})"
            elif type.primitive_type == PrimitiveType.JSON:
                return source_expr
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return self.generate_unpack_enum(enum_cls.__name__, source_expr)
        # node
        elif type.scalar_type == ScalarType.NODE:
            return f"""\
_encoder.unpack_object(None, None, {source_expr}, _session, _options & ~EncoderOptions.OMIT_METATYPE)"""
        # node id
        elif type.scalar_type == ScalarType.NODE_UNTYPED_IDENTITY:
            return f"UUID({source_expr})"
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return f"_encoder.unpack_object({ObjectKind.STRUCT.value}, {StructType.NODE_IDENTITY.value}, {source_expr}, _session, _options)"
        # node location
        elif type.scalar_type == ScalarType.NODE_LOCATION:
            return f"_encoder.unpack_object({ObjectKind.STRUCT.value}, {StructType.NODE_LOCATION.value}, {source_expr}, _session, _options)"
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_MOMENT:
            return f"""\
_encoder.unpack_object({ObjectKind.STRUCT.value}, {StructType.NODE_MOMENT.value}, {source_expr}, _session, _options)"""
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return f"""\
_encoder.unpack_object({ObjectKind.STRUCT.value}, {type.struct_type.value}, {source_expr}, _session, _options)"""
            else:
                return f"""\
_encoder.unpack_object(None, None, {source_expr}, _session, _options & ~EncoderOptions.OMIT_METATYPE)"""
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot unpack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot unpack UNION: {type!r}")
        else:
            assert_never(type.scalar_type)

    def generate(
        self, *, omit: Collection[tuple[ObjectKind, int]] = ()
    ) -> dict[tuple[ObjectKind, int], JsonObjectEncoder]:
        # generate pack/unpack methods
        encoders = {}
        for node_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
            if (
                node_cls.__declaration__.is_abstract
                or (node_cls.metakind, node_cls.metatype.value) in omit
            ):
                continue
            encoder_name, impl, extra_glbls = self.generate_object_encoder(node_cls)
            locals_ = {}
            execute_arbitrary_code(
                impl,
                {**BUILTIN_CLASS_BY_NAME, **extra_glbls},
                locals_,
                encoder_name,
            )
            encoder_cls = locals_[encoder_name]
            encoders[node_cls.metakind, node_cls.metatype.value] = encoder_cls()

        return encoders
