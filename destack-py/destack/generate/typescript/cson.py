import base64
import textwrap
from typing import TYPE_CHECKING, Any, assert_never, cast

from destack.language import (
    BuiltinObject,
    Encoding,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    Struct,
    StructType,
    Type,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.registry import ENUM_CLASS_BY_TYPE, NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from destack.proto import AnyStructProto
from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


def _upper_first(s: str) -> str:
    """Uppercase the first letter of a string."""
    return s[0].upper() + s[1:]


def generate_object_cson(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject toCson/fromCson method implementations."""

    if cls.__is_abstract__:
        pack_cson = f"throw new Error('cannot pack abstract {cls.__name__}');"
        unpack_cson = f"throw new Error('cannot unpack abstract {cls.__name__}');"
    else:
        pack_cson = _generate_to_cson(cls)
        unpack_cson = _generate_from_cson(cls)

    if cls.__is_frozen__ and not cls.__is_node__:
        to_cson_method = f"""
  toCson(): {{ [key: string]: any }} {{
    if (this._cson === null) {{
      // @ts-expect-error(readonly)
      this._cson = {cls.__name__}.__packCson__(this);
    }}
    return this._cson;
  }}"""
    else:
        to_cson_method = f"""
  toCson(): {{ [key: string]: any }} {{
    return {cls.__name__}.__packCson__(this);
  }}"""

    return f"""{to_cson_method}

  static __packCson__(object: {cls.__name__}): {{ [key: string]: any }} {{
{textwrap.indent(pack_cson, "  ")}
  }}

  static __unpackCson__(
    objectCson: {{ [key: string]: any }},
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): {cls.__name__} {{
{textwrap.indent(unpack_cson, "  ")}
  }}

  static fromCson(
    objectCson: {{ readonly [key: string]: any }},
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): {cls.__name__} {{
    return {cls.__name__}.__unpackCson__(objectCson, _session, _graph, _connection);
  }}"""


def _generate_to_cson(cls: type["BuiltinObject"]) -> str:
    """Generate the toCson method implementation."""
    lines: list[str] = []
    lines.append("const objectCson: { [key: string]: any } = {};")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)

    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            from destack.language.registry import get_builtin_type

            metatype = get_builtin_type(cls)
            lines.append(f'objectCson["{prop.id}"] = {metatype.value};')
            continue

        pack_code = _generate_pack_cson_property(prop)
        if pack_code:
            lines.extend(pack_code)

    lines.append("return objectCson;")
    return "\n".join(lines)


def _generate_from_cson(cls: type["BuiltinObject"]) -> str:
    """Generate the fromCson method implementation."""
    unpack_references: set[NodeType | StructType] = set()
    unpack_assignments: list[str] = []
    unpack_body_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        # initializer
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            unpack_references.add(StructType.NODE_REFERENCE)
        elif prop.scalar_type == ScalarType.STRUCT:
            assert prop.struct_type is not None, f"no struct type for {prop!r}"
            unpack_references.add(prop.struct_type)

        # regular unpacking
        unpack_code = _generate_unpack_cson_property(prop)
        ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        self_name = ts_name
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            ts_name = ts_name + "Ptr"
        if len(unpack_code) == 1:
            assignment = unpack_code[0].split(" = ", 1)[1]
            assignment = assignment.strip().rstrip(";")
            unpack_assignments.append(f"{self_name}: {assignment}")
        else:
            unpack_body_parts.extend(unpack_code)
            unpack_assignments.append(f"{self_name}: unpacked{_upper_first(ts_name)}")
    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append("_cson: objectCson")

    unpack_body_parts.append(f"return new {cls.__name__}({{")
    for assignment in unpack_assignments:
        unpack_body_parts.append(f"  {assignment},")
    if cls.__is_node__:
        unpack_body_parts.append("  _session,")
        unpack_body_parts.append("  _graph,")
        unpack_body_parts.append("  _connection,")
    else:
        unpack_body_parts.append("  _graph,")
    unpack_body_parts.append("});")

    # initializer
    unpack_initializer_parts: list[str] = []
    for ref in sorted(unpack_references):
        if isinstance(ref, NodeType):
            reference_cls = NODE_CLASS_BY_TYPE[ref]
            initializer_str = f"const _{reference_cls.__name__} = NODE_CLASS_BY_TYPE[NodeType.{ref.name}] as typeof {reference_cls.__name__};"
            unpack_initializer_parts.append(initializer_str)
        elif isinstance(ref, StructType):
            reference_cls = STRUCT_CLASS_BY_TYPE[ref]
            initializer_str = f"const _{reference_cls.__name__} = STRUCT_CLASS_BY_TYPE[StructType.{ref.name}] as typeof {reference_cls.__name__};"
            unpack_initializer_parts.append(initializer_str)
        else:
            assert_never(ref)
    initializer_str = "\n".join(unpack_initializer_parts)
    unpack_body_parts.insert(0, initializer_str)

    return "\n".join(unpack_body_parts)


def _generate_pack_cson_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    from .language import _is_property_tracked

    lines: list[str] = []
    prop_ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_ts_name = prop_ts_name + "Ptr"
    obj_cson = f"object._{prop_ts_name}" if _is_property_tracked(prop) else f"object.{prop_ts_name}"
    packed_name = f"packed{_upper_first(prop_ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_pack_cson_scalar(prop, obj_cson)
            lines.append(f'objectCson["{prop.id}"] = {value_expr};')
        else:
            lines.append(f"if ({obj_cson} != null) {{")
            value_expr = _generate_pack_cson_scalar(prop, obj_cson)
            lines.append(f'  objectCson["{prop.id}"] = {value_expr};')
            lines.append("}")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"if ({obj_cson}.length > 0) {{")
        lines.append(f"  const {packed_name}: any[] = [];")
        lines.append(f"  for (const item of {obj_cson}) {{")
        item_expr = _generate_pack_cson_scalar(prop, "item")
        lines.append(f"    {packed_name}.push({item_expr});")
        lines.append("  }")
        lines.append(f'  objectCson["{prop.id}"] = {packed_name};')
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"if (Object.keys({obj_cson}).length > 0) {{")
        lines.append(f"  const {packed_name}: {{ [key: string]: any }} = {{}} as any;")
        lines.append(f"  for (const [key, value] of Object.entries({obj_cson})) {{")
        key_expr = _generate_pack_cson_scalar(prop.key_type, "key")
        value_expr = _generate_pack_cson_scalar(prop, "value")
        lines.append(f"    {packed_name}[String({key_expr})] = {value_expr};")
        lines.append("  }")
        lines.append(f'  objectCson["{prop.id}"] = {packed_name};')
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_cson_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        ts_name = ts_name + "Ptr"
    data_cson = f'objectCson["{prop.id}"]'
    var_name = f"unpacked{_upper_first(ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_cson_scalar(prop, data_cson)
            lines.append(f"const {var_name} = {value_expr};")
        else:
            value_expr = _generate_unpack_cson_scalar(prop, f"{ts_name}Value")
            lines.append(f"const {ts_name}Value = {data_cson};")
            lines.append(f"const {var_name} = {ts_name}Value != undefined ? {value_expr} : null;")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"const {var_name}: any[] = [];")
        lines.append(f"if ({data_cson} != undefined) {{")
        lines.append(f"  for (const item of {data_cson}) {{")
        item_expr = _generate_unpack_cson_scalar(prop, "item")
        lines.append(f"    {var_name}.push({item_expr})")
        lines.append("  }")
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"const {var_name} = {{}} as any;")
        lines.append(f"if ({data_cson} != undefined) {{")
        lines.append(f"  for (const [key, value] of Object.entries({data_cson})) {{")
        key_expr = _generate_unpack_cson_scalar(prop.key_type, "key")
        value_expr = _generate_unpack_cson_scalar(prop, "value as any")
        lines.append(f"    {var_name}[{key_expr}] = {value_expr};")
        lines.append("  }")
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_cson_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64Encode({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"{value_expr}.toString({{ timeZoneName: 'never' }})"
        elif prop.primitive_type in (PrimitiveType.DATE, PrimitiveType.TIME):
            return f"{value_expr}.toString()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedeltaToISOFormat({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return value_expr
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return value_expr
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.toCson()"
    else:
        return value_expr


def _generate_unpack_cson_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"base64Decode({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"Temporal.PlainDate.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"Temporal.PlainTime.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"Temporal.Instant.from({value_expr}).toZonedDateTimeISO('UTC')"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedeltaFromISOFormat({value_expr})"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"Number({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return value_expr
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return f"Number({value_expr})"
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"_{struct_cls.__name__}.fromCson({value_expr}, _session, _graph, _connection)"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"_NodeReference.fromCson({value_expr}, _session, _graph, _connection)"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.fromCson({value_expr}, _session, _graph, _connection)"
    else:
        return value_expr


def generate_cson(type: Type | TypeDeclaration | PropertyDeclaration, value: Any) -> str:
    """Generate a Typescript value literal."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _generate_cson_scalar(type, value)
    elif type.cardinality == TypeCardinality.LIST:
        elements_str = [
            textwrap.indent(_generate_cson_scalar(type, element), "  ") for element in value
        ]
        return f"[\n{',\n'.join(elements_str)}\n]"
    else:
        raise ValueError(f"unsupported value type {type.cardinality!r}: {type!r}")


def _generate_cson_scalar(type: Type | TypeDeclaration | PropertyDeclaration, value: Any) -> str:
    """Generate a Typescript scalar value literal."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        if type.primitive_type == PrimitiveType.BOOLEAN:
            return "true" if value else "false"
        elif type.primitive_type in (
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return str(value)
        elif type.primitive_type == PrimitiveType.STRING:
            return f'"{value}"'
        elif type.primitive_type == PrimitiveType.DATETIME:
            return f"Temporal.Instant.from(\"{value}\").toZonedDateTimeISO('UTC')"
        elif type.primitive_type == PrimitiveType.DATE:
            return f'Temporal.PlainDate.from("{value}")'
        elif type.primitive_type == PrimitiveType.TIME:
            return f'Temporal.PlainTime.from("{value}")'
        elif type.primitive_type == PrimitiveType.DURATION:
            return f'Temporal.Duration.from("{value}")'
        else:
            raise ValueError(f"unsupported primitive type {type.primitive_type!r}: {type!r}")
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum_type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        # return f"{enum_cls.__name__}.{value.name}"
        return f"({int(value)} /* {enum_cls.__name__}.{value.name} */)"
    elif type.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE):
        assert type.struct_type is not None, f"no struct_type for {type!r}"
        assert isinstance(value, Struct), f"value is not a Struct for {type!r}: {value!r}"
        value_bytes = cast(AnyStructProto, value.pack_bytes(Encoding.PROTO)).SerializeToString()
        value_bytes_str = base64.b64encode(value_bytes).decode("ascii")
        return f"{value.__class__.__name__}.fromProtoString({value_bytes_str!r})"
    else:
        raise ValueError(f"unsupported value type {type.scalar_type!r}: {type!r}")
