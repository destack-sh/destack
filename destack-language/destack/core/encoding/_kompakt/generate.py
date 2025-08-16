import textwrap
from collections.abc import Collection
from datetime import UTC, date, datetime, time, timedelta
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never, override

from destack.core import (
    UUID,
    BinaryDecoder,
    BinaryEncoder,
    EncoderFlag,
    Encoding,
    Entity,
    Materialization,
    Object,
    ObjectKind,
    ObjectStability,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    Session,
    Struct,
    StructType,
    TypeCardinality,
    TypeDeclaration,
    execute_arbitrary_code,
)
from destack.registry import (
    BUILTIN_CLASS_BY_NAME,
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)

if TYPE_CHECKING:
    pass

from .core import KompaktObjectEncoder

type_ = type


class KompaktEncoderGenerator:
    """Generate a KompaktObjectEncoder for an Object."""

    def get_source_property_name(self, prop: PropertyDeclaration) -> str:
        """Get the name of a property."""
        source_name = prop.name
        if prop.type.scalar_type == ScalarType.NODE_TEMPORAL:
            source_name += "_ref"
        return source_name

    def get_encoder_name(self, cls: type["Object"]) -> str:
        """Get the name of the KompaktObjectEncoder for an Object."""
        return f"{cls.__name__}KompaktEncoder"

    def generate_object_encoder(self, cls: type["Object"]) -> tuple[str, str, dict[str, Any]]:
        """Generate the KompaktObjectEncoder class for an Object."""

        is_entity = issubclass(cls, Entity)
        pack_kompakt, extra_pack_kompakt = self.generate_pack_object(
            cls, is_entity=is_entity, stability=cls.__declaration__.stability
        )
        unpack_kompakt, extra_unpack_kompakt = self.generate_unpack_object(
            cls, is_entity=is_entity, stability=cls.__declaration__.stability
        )
        encoder_name = self.get_encoder_name(cls)

        impl = f"""
class {encoder_name}(KompaktObjectEncoder):

# hard coded pack/unpack helpers
{textwrap.indent(extra_pack_kompakt, " " * 4)}
{textwrap.indent(extra_unpack_kompakt, " " * 4)}
    
    @override
    def pack_object(
        self,
        _encoder: "KompaktEncoder",
        _object: "{cls.__name__}",
        _binary_encoder: BinaryEncoder,
        _options: "EncoderOptions",
    ) -> None:
{textwrap.indent(pack_kompakt, " " * 8)}

    @override
    def unpack_object(
        self,
        _encoder: "KompaktEncoder",
        _binary_decoder: BinaryDecoder,
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
                "BinaryDecoder": BinaryDecoder,
                "BinaryEncoder": BinaryEncoder,
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
                "EncoderOptions": EncoderFlag,
            },
        )

    def generate_pack_object(
        self,
        cls: type["Object"],
        *,
        is_entity: bool,
        stability: ObjectStability,
    ) -> tuple[str, str]:
        """Generate the pack method for an Object."""
        body_lines: list[str] = []
        extra_lines: list[str] = []

        # collect properties
        properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
        properties.sort(key=lambda p: p.id or 0)

        # determine properties (count, set/null flags)
        body_lines.append("# preamble")
        if stability == ObjectStability.DYNAMIC:
            # if properties may change we need to determine which ones are set
            if is_entity:
                # dynamic entity with a dynamic set of properties
                #  count set properties and null flags
                body_lines.append(f"_is_partial = _object.materialization < {Materialization.FULL}")
                body_lines.append("_num_set_properties = 0")
            else:
                # dynamic event or struct with a fixed set of properties
                body_lines.append(f"_num_set_properties = {len(properties)}")

            body_lines.append("_null_flags = 0")
            for i, prop in enumerate(properties):
                prop_name = self.get_source_property_name(prop)
                body_lines.append(f"# {prop.component.__name__}.{prop.name} ({i}->{prop.id})")
                if is_entity:
                    if not prop.type.is_required:
                        body_lines.append(f"""\
if not _is_partial or _object.is_set("{prop.name}"):
    _num_set_properties += 1
    if _object.{prop_name} is None:
        _null_flags |= {1 << i} # 1 << {i}""")
                    else:
                        body_lines.append(f"""\
if not _is_partial or _object.is_set("{prop.name}"):
    _num_set_properties += 1""")
                else:
                    if not prop.type.is_required:
                        body_lines.append(f"""\
if _object.{prop_name} is None:
    _null_flags |= {1 << i} # 1 << {i}""")

            # prefix with number of set properties and their null flags
            if is_entity:
                body_lines.append("_binary_encoder.write_uint8(_object.materialization.value)")
            body_lines.append("_binary_encoder.write_uint8(_num_set_properties)")
            body_lines.append("_binary_encoder.write_uint128(_null_flags)")
        elif stability == ObjectStability.STATIC:
            # properties can't change
            assert issubclass(cls, Struct), f"only Structs can be static: {cls!r}"
            if all(p.type.is_required for p in properties):
                # nothing to do, all properties are always set in same order
                pass
            else:
                # if only some properties are required, determine null flags
                body_lines.append("_null_flags = 0")
                for i, prop in enumerate(properties):
                    prop_name = self.get_source_property_name(prop)
                    if not prop.type.is_required:
                        body_lines.append(f"""\
if _object.{prop_name} is None:
    _null_flags |= {1 << i} # 1 << {i}""")
        else:
            assert_never(stability)

        # pack properties
        body_lines.append("\n# properties")
        for i, prop in enumerate(properties):
            body_lines.append(f"# {prop.component.__name__}.{prop.name} ({i}->{prop.id})")
            prop_name = self.get_source_property_name(prop)
            pack_code = self.generate_pack_value(
                prop.type,
                key=f"_{prop.name}",
                source_expr=f"_object.{prop_name}",
            )
            # if it may be null, wrap in an if statement
            if not prop.type.is_required:
                pack_code = self._wrap_pack_maybe(pack_code, f"_object.{prop_name} is not None")
            # if the property order may change, prefix with property id
            # (always write property id so we know which properties are null)
            if stability == ObjectStability.DYNAMIC:
                body_lines.append(f"_binary_encoder.write_uint8({prop.id})")
            body_lines.append(pack_code)

        return "\n".join(body_lines), "\n".join(extra_lines)

    def generate_unpack_object(
        self,
        cls: type["Object"],
        *,
        is_entity: bool,
        stability: ObjectStability,
    ) -> tuple[str, str]:
        """Generate the unpack method for an Object."""
        body_lines: list[str] = []
        extra_lines: list[str] = []

        # collect properties
        properties = [p for p in cls.__properties__.values() if not p.is_runtime_only]
        properties.sort(key=lambda p: p.id or 0)

        # unpack preamble
        body_lines.append("# preamble")
        if stability == ObjectStability.DYNAMIC:
            # dynamic set of properties
            if is_entity:
                body_lines.append(
                    "_materialization = Materialization(_binary_decoder.read_uint8())"
                )
            body_lines.append("_num_set_properties = _binary_decoder.read_uint8()")
            body_lines.append("_null_flags = _binary_decoder.read_uint128()")
        elif stability == ObjectStability.STATIC:
            # properties can't change
            assert issubclass(cls, Struct), f"only Structs can be static: {cls!r}"
            if all(p.type.is_required for p in properties):
                # nothing to do, all properties are always set in same order
                pass
            else:
                # some properties may be null
                body_lines.append("_null_flags = _binary_decoder.read_uint128()")
        else:
            assert_never(stability)

        # unpack properties
        body_lines.append("\n# properties")
        if stability == ObjectStability.DYNAMIC:
            # dynamic set of properties, map them dynamically
            for prop in properties:
                # init to None outside the loop
                body_lines.append(f"_{self.get_source_property_name(prop)} = None")
            # main loop to map them
            prop_map_lines: list[str] = []
            for i, prop in enumerate(properties):
                prop_name = self.get_source_property_name(prop)
                prop_unpacked = self.generate_unpack_value(
                    prop.type,
                    key=f"_{prop_name}",
                    target_expr=f"_{prop_name}",
                    is_not_null_expr="_null_flags & (1 << _i)"
                    if not prop.type.is_required
                    else None,
                )
                prop_map_lines.append(f"""\
# {prop.component.__name__}.{prop.name} ({i}->{prop.id})
{"if" if i == 0 else "elif"} _prop_id == {prop.id}:
{textwrap.indent(prop_unpacked, " " * 4)}""")
            body_lines.append(f"""\
for _i in range(_num_set_properties):
    _prop_id = _binary_decoder.read_uint8()
    # _prop_bytes = _binary_decoder.read_uint32()
{textwrap.indent("\n".join(prop_map_lines), " " * 4)}
    else:
        # unknown property, skip bytes
        raise ValueError(f"unknown property for {cls.__name__}: {{_prop_id}}")""")
        elif stability == ObjectStability.STATIC:
            # static set of properties, just expect them in order
            for i, prop in enumerate(properties):
                prop_name = self.get_source_property_name(prop)
                prop_unpacked = self.generate_unpack_value(
                    prop.type,
                    key=f"_{prop_name}",
                    target_expr=f"_{prop_name}",
                    is_not_null_expr="_null_flags & (1 << _i)"
                    if not prop.type.is_required
                    else None,
                )
                body_lines.append(f"""\
# {prop.component.__name__}.{prop.name} ({i}->{prop.id})
{prop_unpacked}""")
        else:
            assert_never(stability)

        # construct object
        constructor_args = []
        for prop in properties:
            prop_name = self.get_source_property_name(prop)
            constructor_args.append(f"    {prop_name}=_{prop_name}")

        body_lines.append(f"return {cls.__name__}(\n{',\n'.join(constructor_args)}\n)")
        return "\n".join(body_lines), "\n".join(extra_lines)

    def _wrap_pack_maybe(self, code: str, is_not_null_expr: str | None) -> str:
        """Wrap code in an if statement to check if the value is not null."""
        if is_not_null_expr is None:
            return code
        else:
            return f"""\
if {is_not_null_expr}:
{textwrap.indent(code, " " * 4)}"""

    def _wrap_unpack_maybe(self, code: str, target_expr: str, is_not_null_expr: str | None) -> str:
        """Wrap code in an if statement to set the target expression to None if the value is null."""
        if is_not_null_expr is None:
            return code
        else:
            return f"""\
if not {is_not_null_expr}:
{textwrap.indent(code, " " * 4)}"""

    def generate_pack_value(self, type: TypeDeclaration, key: str, source_expr: str) -> str:
        """Generate code to pack a property to the Kompakt encoding."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            scalar_packed = self.generate_pack_scalar_value(type, source_expr)
            return scalar_packed

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            item_source_expr = f"{key}_item"
            item_packed = self.generate_pack_scalar_value(type.value_type, item_source_expr)
            list_packed = f"""\
_binary_encoder.write_uint32(len({source_expr}))
for {item_source_expr} in {source_expr}:
{textwrap.indent(item_packed, " " * 4)}"""
            return list_packed

        # tuple
        elif type.cardinality == TypeCardinality.TUPLE:
            raise NotImplementedError(f"cannot pack tuple: {type!r}")

        # map
        elif type.cardinality == TypeCardinality.MAP:
            assert type.key_type is not None, f"no key type for {type!r}"
            assert type.value_type is not None, f"no value type for {type!r}"
            key_source_expr = f"{key}_key"
            value_source_expr = f"{key}_value"
            key_packed = self.generate_pack_scalar_value(type.key_type, key_source_expr)
            value_packed = self.generate_pack_scalar_value(type.value_type, value_source_expr)
            map_packed = f"""\
_binary_encoder.write_uint32(len({source_expr}))
for {key_source_expr}, {value_source_expr} in {source_expr}.items():
{textwrap.indent(key_packed, " " * 4)}
{textwrap.indent(value_packed, " " * 4)}"""
            return map_packed

        else:
            assert_never(type.cardinality)

    def generate_unpack_value(
        self,
        type: TypeDeclaration,
        key: str,
        target_expr: str,
        is_not_null_expr: str | None,
    ) -> str:
        """Generate code to unpack a property from the Kompakt encoding."""

        # scalar
        if type.cardinality == TypeCardinality.SCALAR:
            value_unpacked = self.generate_unpack_scalar_value(type)
            return self._wrap_unpack_maybe(
                f"{target_expr} = {value_unpacked}", target_expr, is_not_null_expr
            )

        # list
        elif type.cardinality == TypeCardinality.LIST:
            assert type.value_type is not None, f"no value type for {type!r}"
            element_unpacked = self.generate_unpack_scalar_value(type.value_type)
            list_unpacked = f"""\
{key}_length = _binary_decoder.read_uint32()
{target_expr} = []
for _ in range({key}_length):
    {target_expr}.append({element_unpacked})"""
            return self._wrap_unpack_maybe(list_unpacked, target_expr, is_not_null_expr)

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
            map_unpacked = f"""\
{key}_length = _binary_decoder.read_uint32()
{target_expr} = {{}}
for _ in range({key}_length):
    {key_source_expr} = {key_unpacked}
    {value_source_expr} = {value_unpacked}
    {target_expr}[{key_source_expr}] = {value_source_expr}"""
            return self._wrap_unpack_maybe(map_unpacked, target_expr, is_not_null_expr)

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
                return f"_binary_encoder.write_bool({source_expr})"
            elif type.primitive_type == PrimitiveType.INT8:
                return f"_binary_encoder.write_int8({source_expr})"
            elif type.primitive_type == PrimitiveType.INT16:
                return f"_binary_encoder.write_int16({source_expr})"
            elif type.primitive_type == PrimitiveType.INT32:
                return f"_binary_encoder.write_int32({source_expr})"
            elif type.primitive_type == PrimitiveType.INT64:
                return f"_binary_encoder.write_int64({source_expr})"
            elif type.primitive_type == PrimitiveType.INT128:
                return f"_binary_encoder.write_int128({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT8:
                return f"_binary_encoder.write_uint8({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT16:
                return f"_binary_encoder.write_uint16({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT32:
                return f"_binary_encoder.write_uint32({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT64:
                return f"_binary_encoder.write_uint64({source_expr})"
            elif type.primitive_type == PrimitiveType.UINT128:
                return f"_binary_encoder.write_uint128({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return f"_binary_encoder.write_float32({source_expr})"
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return f"_binary_encoder.write_float64({source_expr})"
            elif type.primitive_type == PrimitiveType.DATETIME:
                return f"_binary_encoder.write_datetime({source_expr})"
            elif type.primitive_type == PrimitiveType.DATE:
                return f"_binary_encoder.write_date({source_expr})"
            elif type.primitive_type == PrimitiveType.TIME:
                return f"_binary_encoder.write_time({source_expr})"
            elif type.primitive_type == PrimitiveType.TIMESTAMP:
                return f"_binary_encoder.write_timestamp({source_expr})"
            elif type.primitive_type == PrimitiveType.DURATION:
                return f"_binary_encoder.write_duration({source_expr})"
            elif type.primitive_type == PrimitiveType.STRING:
                return f"_binary_encoder.write_string({source_expr})"
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return f"_binary_encoder.write_character({source_expr})"
            elif type.primitive_type == PrimitiveType.UUID:
                return f"_binary_encoder.write_uuid({source_expr})"
            elif type.primitive_type == PrimitiveType.ORDER:
                return f"_binary_encoder.write_order({source_expr})"
            elif type.primitive_type == PrimitiveType.JSON:
                return f"_binary_encoder.write_json({source_expr})"
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            return f"_binary_encoder.write_uint32({source_expr}.value)"
        # node
        elif type.scalar_type == ScalarType.NODE:
            return f"_encoder.pack_object_binary({source_expr}, _encoder, _options & ~EncoderOptions.OMIT_METATYPE)"
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            return f"_binary_encoder.write_uuid({source_expr})"
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return f"_encoder.pack_object_binary({source_expr}, _encoder, _options | EncoderOptions.OMIT_METATYPE)"
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return f"_encoder.pack_object_binary({source_expr}, _encoder, _options | EncoderOptions.OMIT_METATYPE)"
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return f"_encoder.pack_object_binary({source_expr}, _encoder, _options | EncoderOptions.OMIT_METATYPE)"
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return f"_encoder.pack_object_binary({source_expr}, _encoder, _options | EncoderOptions.OMIT_METATYPE)"
            else:
                return f"_encoder.pack_object_binary({source_expr}, _encoder, _options & ~EncoderOptions.OMIT_METATYPE)"
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot pack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot pack UNION: {type!r}")
        #
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
                return "_binary_decoder.read_bool()"
            elif type.primitive_type == PrimitiveType.INT8:
                return "_binary_decoder.read_int8()"
            elif type.primitive_type == PrimitiveType.INT16:
                return "_binary_decoder.read_int16()"
            elif type.primitive_type == PrimitiveType.INT32:
                return "_binary_decoder.read_int32()"
            elif type.primitive_type == PrimitiveType.INT64:
                return "_binary_decoder.read_int64()"
            elif type.primitive_type == PrimitiveType.INT128:
                return "_binary_decoder.read_int128()"
            elif type.primitive_type == PrimitiveType.UINT8:
                return "_binary_decoder.read_uint8()"
            elif type.primitive_type == PrimitiveType.UINT16:
                return "_binary_decoder.read_uint16()"
            elif type.primitive_type == PrimitiveType.UINT32:
                return "_binary_decoder.read_uint32()"
            elif type.primitive_type == PrimitiveType.UINT64:
                return "_binary_decoder.read_uint64()"
            elif type.primitive_type == PrimitiveType.UINT128:
                return "_binary_decoder.read_uint128()"
            elif type.primitive_type == PrimitiveType.FLOAT32:
                return "_binary_decoder.read_float32()"
            elif type.primitive_type == PrimitiveType.FLOAT64:
                return "_binary_decoder.read_float64()"
            elif type.primitive_type == PrimitiveType.DATETIME:
                return "_binary_decoder.read_datetime()"
            elif type.primitive_type == PrimitiveType.DATE:
                return "_binary_decoder.read_date()"
            elif type.primitive_type == PrimitiveType.TIME:
                return "_binary_decoder.read_time()"
            elif type.primitive_type == PrimitiveType.TIMESTAMP:
                return "_binary_decoder.read_timestamp()"
            elif type.primitive_type == PrimitiveType.DURATION:
                return "_binary_decoder.read_duration()"
            elif type.primitive_type == PrimitiveType.STRING:
                return "_binary_decoder.read_string()"
            elif type.primitive_type == PrimitiveType.CHARACTER:
                return "_binary_decoder.read_character()"
            elif type.primitive_type == PrimitiveType.UUID:
                return "_binary_decoder.read_uuid()"
            elif type.primitive_type == PrimitiveType.ORDER:
                return "_binary_decoder.read_order()"
            elif type.primitive_type == PrimitiveType.JSON:
                return "_binary_decoder.read_json()"
            else:
                assert_never(type.primitive_type)
        # enum
        elif type.scalar_type == ScalarType.ENUM:
            assert type.enum_type is not None, f"no enum type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return f"{enum_cls.__name__}.__options_by_id__[_binary_decoder.read_uint32()]"
        # node
        elif type.scalar_type == ScalarType.NODE:
            return "_encoder.unpack_object_binary(None, None, _binary_decoder, _session, _options | EncoderOptions.OMIT_METATYPE)"
        # node id
        elif type.scalar_type == ScalarType.NODE_RAW:
            return "UUID(_binary_decoder.read_uuid())"
        # node typed id
        elif type.scalar_type == ScalarType.NODE_IDENTITY:
            return f"_encoder.unpack_object_binary({ObjectKind.STRUCT.value}, {StructType.NODE_IDENTITY_REFERENCE.value}, _binary_decoder, _session, _options | EncoderOptions.OMIT_METATYPE)"
        # node location
        elif type.scalar_type == ScalarType.NODE_SPATIAL:
            return f"_encoder.unpack_object_binary({ObjectKind.STRUCT.value}, {StructType.NODE_SPATIAL_REFERENCE.value}, _binary_decoder, _session, _options | EncoderOptions.OMIT_METATYPE)"
        # node reference (moment)
        elif type.scalar_type == ScalarType.NODE_TEMPORAL:
            return f"_encoder.unpack_object_binary({ObjectKind.STRUCT.value}, {StructType.NODE_TEMPORAL_REFERENCE.value}, _binary_decoder, _session, _options | EncoderOptions.OMIT_METATYPE)"
        # struct
        elif type.scalar_type == ScalarType.STRUCT:
            assert type.struct_type is not None, f"no struct type for {type!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
            if struct_cls.__declaration__.is_final:
                return f"_encoder.unpack_object_binary({ObjectKind.STRUCT}, {type.struct_type}, _binary_decoder, _session, _options)"
            else:
                return "_encoder.unpack_object_binary(None, None, _binary_decoder, _session, _options & ~EncoderOptions.OMIT_METATYPE)"
        # handle
        elif type.scalar_type == ScalarType.HANDLE:
            raise NotImplementedError(f"cannot unpack HANDLE: {type!r}")
        # union
        elif type.scalar_type == ScalarType.UNION:
            raise NotImplementedError(f"cannot unpack UNION: {type!r}")
        #
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
            encoders[node_cls.metakind, node_cls.metatype] = encoder_cls()

        return encoders
