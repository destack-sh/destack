import textwrap
from collections.abc import Collection
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.language.core import (
    BinaryReader,
    BinaryWriter,
    EncoderOptions,
    Encoding,
    Entity,
    Object,
    ObjectKind,
    ObjectStability,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    Session,
    StructType,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.registry import (
    BUILTIN_CLASS_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
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


class KompaktEncoderGenerator:
    """Generate a KompaktObjectEncoder for an Object."""

    def get_source_property_name(self, prop: PropertyDeclaration) -> str:
        """Get the name of a property."""
        source_name = prop.name
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            source_name += "_ptr"
        return source_name

    def get_encoder_name(self, cls: type["Object"]) -> str:
        """Get the name of the KompaktObjectEncoder for an Object."""
        return f"{cls.__name__}KompaktEncoder"

    def generate_object_encoder(self, cls: type["Object"]) -> tuple[str, str, dict[str, Any]]:
        """Generate the KompaktObjectEncoder class for an Object."""

        is_entity = issubclass(cls, Entity)
        pack_kompakt = self.generate_pack_object(
            cls, is_entity=is_entity, stability=cls.__stability__
        )
        unpack_kompakt = self.generate_unpack_object(
            cls, is_entity=is_entity, stability=cls.__stability__
        )
        encoder_name = self.get_encoder_name(cls)

        impl = f"""
class {encoder_name}(KompaktObjectEncoder):
    
    @override
    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: "{cls.__name__}",
        _writer: BinaryWriter,
        _options: "EncoderOptions",
    ) -> None:
{textwrap.indent(pack_kompakt, " " * 8)}

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _reader: BinaryReader,
        _session: "Session | None",
        _options: "EncoderOptions",
    ) -> "{cls.__name__}":
{textwrap.indent(unpack_kompakt, " " * 8)}
"""
        return (
            encoder_name,
            impl,
            {
                "KompaktObjectEncoder": KompaktObjectEncoder,
                "BinaryReader": BinaryReader,
                "BinaryWriter": BinaryWriter,
                "datetime": datetime,
                "timedelta": timedelta,
                "date": date,
                "time": time,
                "UTC": UTC,
                "UUID": UUID,
                "override": override,
                "Self": cls,
                "cls": cls,
                "Object": Object,
                "Encoding": Encoding,
                "Session": Session,
                "EncoderOptions": EncoderOptions,
            },
        )

    def generate_pack_object(
        self,
        cls: type["Object"],
        *,
        is_entity: bool,
        stability: ObjectStability,
    ) -> str:
        """Generate the pack method for an Object."""
        lines: list[str] = []

        # nocheckin: include object encoded byte size in KompaktEncoder? (so we know when to stop)

        # collect properties
        properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
        properties.sort(key=lambda p: p.id or 0)
        if is_entity:
            set_properties = [p for p in properties if p.is_identity]
            maybe_set_properties = [p for p in properties if not p.is_identity]
        else:
            set_properties = properties
            maybe_set_properties = []

        # pack always set properties
        for prop in set_properties:
            prop_name = self.get_source_property_name(prop)
            pack_code = self.generate_pack_value(
                prop,
                key=f"_{prop.name}",
                source_expr=f"_object.{prop_name}",
            )
            lines.append(pack_code)

        # pack partial properties
        if is_entity and maybe_set_properties:
            maybe_set_lines: list[str] = []
            set_lines: list[str] = []
            for prop in maybe_set_properties:
                prop_name = self.get_source_property_name(prop)
                pack_code = self.generate_pack_value(
                    prop,
                    key=f"_{prop.name}",
                    source_expr=f"_object.{prop_name}",
                )
                set_lines.append(pack_code)
                maybe_set_lines.append(f"""\
if _object.is_set("{prop.name}"):
    _writer.write_bool(True)
{textwrap.indent(pack_code, " " * 4)}
else:
    _writer.write_bool(False)
""")

            maybe_set_code = "\n".join(maybe_set_lines)
            set_code = "\n".join(set_lines)
            lines.append(f"""\
if _object.is_partial:
{textwrap.indent(maybe_set_code, " " * 4)}
else:
{textwrap.indent(set_code, " " * 4)}
""")

        return "\n".join(lines)

    def generate_unpack_object(
        self,
        cls: type["Object"],
        *,
        is_entity: bool,
        stability: ObjectStability,
    ) -> str:
        """Generate the unpack method for an Object."""
        lines: list[str] = []

        # collect properties
        properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
        properties.sort(key=lambda p: p.id or 0)
        if is_entity:
            set_properties = [p for p in properties if p.is_identity]
            maybe_set_properties = [p for p in properties if not p.is_identity]
        else:
            set_properties = properties
            maybe_set_properties = []

        # unpack always set properties
        for prop in set_properties:
            prop_name = self.get_source_property_name(prop)
            unpack_code = self.generate_unpack_value(
                prop,
                key=f"_{prop.name}",
                target_expr=f"_{prop_name}",
            )
            lines.append(unpack_code)

        # unpack partial properties
        if is_entity and maybe_set_properties:
            maybe_set_lines: list[str] = []
            set_lines: list[str] = []
            for prop in maybe_set_properties:
                prop_name = self.get_source_property_name(prop)
                unpack_code = self.generate_unpack_value(
                    prop,
                    key=f"_{prop.name}",
                    target_expr=f"_{prop_name}",
                )
                set_lines.append(unpack_code)
                maybe_set_lines.append(f"""\
if _reader.read_bool():
{textwrap.indent(unpack_code, " " * 4)}
else:
    _{prop_name} = None
""")

            maybe_set_code = "\n".join(maybe_set_lines)
            set_code = "\n".join(set_lines)
            lines.append(f"""\
_is_partial = _reader.read_bool()
if _is_partial:
{textwrap.indent(maybe_set_code, " " * 4)}
else:
{textwrap.indent(set_code, " " * 4)}
""")

        # construct object
        constructor_args = []
        for prop in properties:
            prop_name = self.get_source_property_name(prop)
            constructor_args.append(f"{prop.name}=_{prop_name}")

        lines.append(f"return {cls.__name__}({', '.join(constructor_args)})")
        return "\n".join(lines)

    def generate_pack_value(
        self,
        type: TypeDeclaration,
        key: str,
        source_expr: str,
    ) -> str:
        """Generate code to pack a property to the Kompakt encoding."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if type.is_required:
                return self.generate_pack_scalar_value(type, source_expr)
            else:
                scalar_packed = self.generate_pack_scalar_value(type, source_expr)
                return f"""\
if {source_expr} is not None:
    _writer.write_bool(True)
{textwrap.indent(scalar_packed, " " * 4)}
else:
    _writer.write_bool(False)"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_source_expr = f"{key}_item"
            item_packed = self.generate_pack_scalar_value(
                type.value_type,
                item_source_expr,
            )
            if type.is_required:
                return f"""\
_writer.write_uint32(len({source_expr}))
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 4)}"""
            else:
                return f"""\
if {source_expr} is not None:
    _writer.write_bool(True)
    _writer.write_uint32(len({source_expr}))
    for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 8)}
else:
    _writer.write_bool(False)"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            raise NotImplementedError(f"cannot pack tuple: {type!r}")

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key"
            value_source_expr = f"{key}_value"
            key_packed = self.generate_pack_scalar_value(
                type.key_type,
                key_source_expr,
            )
            value_packed = self.generate_pack_scalar_value(
                type.value_type,
                value_source_expr,
            )
            if type.is_required:
                return f"""\
_writer.write_uint32(len({source_expr}))
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 4)}
{textwrap.indent(value_packed, " " * 4)}"""
            else:
                return f"""\
if {source_expr} is not None:
    _writer.write_bool(True)
    _writer.write_uint32(len({source_expr}))
    for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 8)}
{textwrap.indent(value_packed, " " * 8)}
else:
    _writer.write_bool(False)"""

        else:
            assert_never(type.cardinality)

    def generate_unpack_value(
        self,
        type: TypeDeclaration,
        key: str,
        target_expr: str,
    ) -> str:
        """Generate code to unpack a property from the Kompakt encoding."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            if type.is_required:
                value_unpacked = self.generate_unpack_scalar_value(type)
                return f"{target_expr} = {value_unpacked}"
            else:
                value_unpacked = self.generate_unpack_scalar_value(type)
                return f"""\
if _reader.read_bool():
    {target_expr} = {value_unpacked}
else:
    {target_expr} = None"""

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            element_unpacked = self.generate_unpack_scalar_value(type.value_type)
            if type.is_required:
                return f"""\
{key}_length = _reader.read_uint32()
{target_expr} = []
for _ in range({key}_length):
    {target_expr}.append({element_unpacked})"""
            else:
                return f"""\
if _reader.read_bool():
    {key}_length = _reader.read_uint32()
    {target_expr} = []
    for _ in range({key}_length):
        {target_expr}.append({element_unpacked})
else:
    {target_expr} = None"""

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            raise NotImplementedError(f"cannot unpack tuple: {type!r}")

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key"
            value_source_expr = f"{key}_value"
            key_unpacked = self.generate_unpack_scalar_value(type.key_type)
            value_unpacked = self.generate_unpack_scalar_value(type.value_type)
            if type.is_required:
                return f"""\
{key}_length = _reader.read_uint32()
{target_expr} = {{}}
for _ in range({key}_length):
    {key_source_expr} = {key_unpacked}
    {value_source_expr} = {value_unpacked}
    {target_expr}[{key_source_expr}] = {value_source_expr}"""
            else:
                return f"""\
if _reader.read_bool():
    {key}_length = _reader.read_uint32()
    {target_expr} = {{}}
    for _ in range({key}_length):
        {key_source_expr} = {key_unpacked}
        {value_source_expr} = {value_unpacked}
        {target_expr}[{key_source_expr}] = {value_source_expr}
else:
    {target_expr} = None"""

        else:
            assert_never(type.cardinality)

    def generate_pack_scalar_value(self, type: TypeDeclaration, source_expr: str) -> str:
        """Generate the packing code for a scalar value in the Kompakt encoding."""

        assert type.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                raise NotImplementedError(f"cannot pack none: {type!r}")
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return f"_writer.write_bool({source_expr})"
            elif type.primitive_type == PrimitiveType.INT8:
                return f"_writer.write_int8({source_expr})"
            elif type.primitive_type == PrimitiveType.INT16:
                return f"_writer.write_int16({source_expr})"
            elif type.primitive_type == PrimitiveType.INT32:
                return f"_writer.write_int32({source_expr})"
            elif type.primitive_type == PrimitiveType.INT64:
                return f"_writer.write_int64({source_expr})"
            elif type.primitive_type == PrimitiveType.INT128:
                return f"_writer.write_int128({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT8:
                return f"_writer.write_uint8({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT16:
                return f"_writer.write_uint16({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT32:
                return f"_writer.write_uint32({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT64:
                return f"_writer.write_uint64({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT128:
                return f"_writer.write_uint128({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT16:
                return f"_writer.write_float16({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return f"_writer.write_float32({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return f"_writer.write_float64({source_expr})"
            elif type.primitive_type == PrimitiveType.DATETIME:
                return f"_writer.write_datetime({source_expr})"
            elif type.primitive_type == PrimitiveType.DATE:
                return f"_writer.write_date({source_expr})"
            elif type.primitive_type == PrimitiveType.TIME:
                return f"_writer.write_time({source_expr})"
            elif type.primitive_type == PrimitiveType.DURATION:
                return f"_writer.write_duration({source_expr})"
            elif type.primitive_type == PrimitiveType.STRING:
                return f"_writer.write_string({source_expr})"
            elif type.primitive_type == PrimitiveType.UUID:
                return f"_writer.write_uuid({source_expr})"
            elif type.primitive_type == PrimitiveType.BYTES:
                return f"_writer.write_bytes({source_expr})"
            elif type.primitive_type == PrimitiveType.JSON:
                return f"_writer.write_json({source_expr})"
            else:
                assert_never(type.primitive_type)

        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return f"_writer.write_uint32({source_expr}.value)"

        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            return f"_encoder.pack_object_binary({ObjectKind.STRUCT}, {type.struct_type}, {source_expr}, _writer, _options)"

        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return f"_encoder.pack_object_binary({ObjectKind.STRUCT}, {StructType.NODE_REFERENCE}, {source_expr}, _writer, _options)"

        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return f"""\
_writer.write_uint32({source_expr}.metatype.value)
_encoder.pack_object_binary({ObjectKind.NODE}, {source_expr}.metatype, {source_expr}, _writer, _options)"""

        else:
            assert_never(type.scalar_type)

    def generate_unpack_scalar_value(self, type: TypeDeclaration) -> str:
        """Generate the unpacking code for a scalar value in the Kompakt encoding."""

        assert type.cardinality == TypeCardinality.SCALAR, f"cannot unpack non-scalar: {type!r}"
        assert type.scalar_type is not None, f"no scalar type for {type!r}"

        # primitive
        if type.scalar_type == ScalarType.PRIMITIVE:
            assert type.primitive_type is not None, f"no primitive type for {type!r}"
            if type.primitive_type == PrimitiveType.NONE:
                raise NotImplementedError(f"cannot unpack none: {type!r}")
            elif type.primitive_type == PrimitiveType.BOOLEAN:
                return "_reader.read_bool()"
            elif type.primitive_type == PrimitiveType.INT8:
                return "_reader.read_int8()"
            elif type.primitive_type == PrimitiveType.INT16:
                return "_reader.read_int16()"
            elif type.primitive_type == PrimitiveType.INT32:
                return "_reader.read_int32()"
            elif type.primitive_type == PrimitiveType.INT64:
                return "_reader.read_int64()"
            elif type.primitive_type == PrimitiveType.INT128:
                return "_reader.read_int128()"
            elif type.primitive_type == PrimitiveType.UINT8:
                return "_reader.read_uint8()"
            elif type.primitive_type == PrimitiveType.UINT16:
                return "_reader.read_uint16()"
            elif type.primitive_type == PrimitiveType.UINT32:
                return "_reader.read_uint32()"
            elif type.primitive_type == PrimitiveType.UINT64:
                return "_reader.read_uint64()"
            elif type.primitive_type == PrimitiveType.UINT128:
                return "_reader.read_uint128()"
            elif type.primitive_type == PrimitiveType.FLOAT16:
                return "_reader.read_float16()"
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return "_reader.read_float32()"
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return "_reader.read_float64()"
            elif type.primitive_type == PrimitiveType.DATETIME:
                return "_reader.read_datetime()"
            elif type.primitive_type == PrimitiveType.DATE:
                return "_reader.read_date()"
            elif type.primitive_type == PrimitiveType.TIME:
                return "_reader.read_time()"
            elif type.primitive_type == PrimitiveType.DURATION:
                return "_reader.read_duration()"
            elif type.primitive_type == PrimitiveType.STRING:
                return "_reader.read_string()"
            elif type.primitive_type == PrimitiveType.UUID:
                return "_reader.read_uuid()"
            elif type.primitive_type == PrimitiveType.BYTES:
                return "_reader.read_bytes()"
            elif type.primitive_type == PrimitiveType.JSON:
                return "_reader.read_json()"
            else:
                assert_never(type.primitive_type)

        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return f"{enum_cls.__name__}(_reader.read_uint32())"

        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            return (
                f"{struct_cls.__name__}.unpack_binary({Encoding.KOMPAKT.value}, _reader, _session)"
            )

        # node reference
        elif type.scalar_type == ScalarType.NODE_REFERENCE:
            return f"_encoder.unpack_object_binary({ObjectKind.STRUCT}, {StructType.NODE_REFERENCE}, _reader, _session, _options)"

        # node value
        elif type.scalar_type == ScalarType.NODE_VALUE:
            return f"_encoder.unpack_object_binary({ObjectKind.NODE}, _reader.read_uint32(), _reader, _session, _options)"

        else:
            assert_never(type.scalar_type)

    def generate(
        self,
        *,
        omit: Collection[tuple[ObjectKind, int]] = (),
    ) -> dict[tuple[ObjectKind, int], KompaktObjectEncoder]:
        # generate pack/unpack methods
        encoders = {}
        for node_cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
            if (
                node_cls.__declaration__.is_abstract
                or (node_cls.__kind__, node_cls.metatype.value) in omit
            ):
                continue
            encoder_name, impl, extra_glbls = self.generate_object_encoder(node_cls)
            locals_ = {}
            exec_(
                impl,
                {**BUILTIN_CLASS_BY_NAME, **extra_glbls},
                locals_,
                encoder_name,
            )
            encoder_cls = locals_[encoder_name]
            encoders[node_cls.__kind__, node_cls.metatype] = encoder_cls()

        return encoders
