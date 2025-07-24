import base64
import json
import textwrap
from itertools import chain
from typing import TYPE_CHECKING, Any, assert_never

from destack.language import (
    BinaryWriter,
    BuiltinObject,
    Encoding,
    Node,
    NodeReference,
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
from destack.utils.log import get_logger
from destack.utils.string import Casing, to_casing
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113, SIM114
# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def _upper_first(s: str) -> str:
    """Uppercase the first letter of a string."""
    return s[0].upper() + s[1:]


def generate_json_encoders() -> str:
    file_parts: list[str] = []

    # imports
    import_parts: list[str] = []
    import_parts.append(
        "import type { BuiltinObject, Graph, GraphConnection, Session, Encoder } from '@destack/language';"
    )
    import_parts.append(
        "import { NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE } from '@destack/language/registry';"
    )
    import_parts.append(
        "import { JSON_OBJECT_ENCODERS, _JsonObjectEncoder, getObjectKey } from '@destack/encoder/json/core';"
    )
    import_parts.append("import { Temporal } from 'temporal-polyfill';")
    import_parts.append("import { uuid4, uuid7, toNanoId } from '@destack/utils/uuid';")
    import_parts.append(
        "import { timedeltaToISOFormat, timedeltaFromISOFormat, base64Encode, base64Decode } from '@destack/utils';"
    )
    builtins_names: set[str] = set()
    builtins_names.update(
        cls.__name__
        for cls in chain(
            NODE_CLASS_BY_TYPE.values(),
            STRUCT_CLASS_BY_TYPE.values(),
            ENUM_CLASS_BY_TYPE.values(),
        )
    )
    import_parts.append(f"import {{ {', '.join(builtins_names)} }} from '@destack/language';")
    file_parts.append("\n".join(import_parts))

    # body
    file_parts.append("export const JSON_ENCODERS: { [key: string]: _JsonObjectEncoder } = {};")

    # encoders
    file_parts.append("let loaded = false;")
    body_parts: list[str] = [
        "if (loaded) {",
        "  return;",
        "}",
        "loaded = true;",
    ]
    for cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        if cls.__is_abstract__:
            continue
        encoder_name, encoder_str = generate_object_json_encoder(cls)
        body_parts.append(encoder_str)
        body_parts.append(
            f"JSON_OBJECT_ENCODERS[getObjectKey({cls.__kind__.value}, {cls.metatype.value})] = new {encoder_name}();"
        )
    body_str = textwrap.indent("\n".join(body_parts), "  ")
    file_parts.append(f"""\
export function loadEncoders(): void {{
{textwrap.indent(body_str, "  ")}
}}

loadEncoders();
""")

    return "\n".join(file_parts)


def generate_object_json_encoder(cls: type["BuiltinObject"]) -> tuple[str, str]:
    """Generate the BuiltinObject Encoder class."""

    if cls.__is_abstract__:
        pack_json = f"throw new Error('cannot pack abstract {cls.__name__}');"
        unpack_json = f"throw new Error('cannot unpack abstract {cls.__name__}');"
    else:
        pack_json = _generate_to_json(cls)
        unpack_json = _generate_from_json(cls)

    encoder_name = f"{cls.__name__}JsonEncoder"

    return (
        encoder_name,
        f"""
class {encoder_name} implements _JsonObjectEncoder {{
  packObject(object: {cls.__name__}): any {{
{textwrap.indent(pack_json, " " * 4)}
  }}

  unpackObject(objectJson: any, _session: Session | null): {cls.__name__} {{
{textwrap.indent(unpack_json, " " * 4)}
  }}
}}
""",
    )


def _generate_to_json(cls: type["BuiltinObject"]) -> str:
    """Generate the packObject method implementation."""
    lines: list[str] = []
    lines.append("const objectJson: { [key: string]: any } = {};")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)

    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            from destack.language.registry import get_builtin_type

            metatype = get_builtin_type(cls)
            lines.append(f'objectJson["{prop.name}"] = "{metatype.name}";')
            continue

        pack_code = _generate_pack_json_property(prop)
        if pack_code:
            lines.extend(pack_code)

    lines.append("return objectJson;")
    return "\n".join(lines)


def _get_indirect_object_cls(type: type[BuiltinObject]) -> str:
    if issubclass(type, Node):
        return f"NODE_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    elif issubclass(type, Struct):
        return f"STRUCT_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    else:
        raise ValueError(f"unexpected type {type!r}")


def _generate_from_json(cls: type["BuiltinObject"]) -> str:
    """Generate the unpackObject method implementation."""
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
        unpack_code = _generate_unpack_json_property(prop)
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
        unpack_assignments.append(
            f"_packedCache: [{{ encoding: {Encoding.JSON.value}, isBytes: false, packed: objectJson }}]"
        )

    unpack_body_parts.append(f"return new ({_get_indirect_object_cls(cls)})({{")
    for assignment in unpack_assignments:
        unpack_body_parts.append(f"  {assignment},")
    unpack_body_parts.append("  _session,")
    unpack_body_parts.append("});")

    # initializer
    unpack_initializer_parts: list[str] = []
    for ref in sorted(unpack_references):
        ref_cls = (
            STRUCT_CLASS_BY_TYPE[ref] if isinstance(ref, StructType) else NODE_CLASS_BY_TYPE[ref]
        )
        initializer_str = f"const _{ref_cls.__name__} = {_get_indirect_object_cls(ref_cls)};"
        unpack_initializer_parts.append(initializer_str)
    initializer_str = "\n".join(unpack_initializer_parts)
    unpack_body_parts.insert(0, initializer_str)

    return "\n".join(unpack_body_parts)


def _generate_pack_json_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    from ..language import _is_property_tracked

    lines: list[str] = []
    prop_ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_ts_name = prop_ts_name + "Ptr"
    json_key = to_casing(prop.name, Casing.LOWER_CAMEL)
    obj_json = f"object._{prop_ts_name}" if _is_property_tracked(prop) else f"object.{prop_ts_name}"
    packed_name = f"packed{_upper_first(prop_ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_pack_json_scalar(prop, obj_json)
            lines.append(f'objectJson["{json_key}"] = {value_expr};')
        else:
            lines.append(f"if ({obj_json} != null) {{")
            value_expr = _generate_pack_json_scalar(prop, obj_json)
            lines.append(f'  objectJson["{json_key}"] = {value_expr};')
            lines.append("}")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"if ({obj_json}.length > 0) {{")
        lines.append(f"  const {packed_name}: any[] = [];")
        lines.append(f"  for (const item of {obj_json}) {{")
        item_expr = _generate_pack_json_scalar(prop, "item")
        lines.append(f"    {packed_name}.push({item_expr});")
        lines.append("  }")
        lines.append(f'  objectJson["{json_key}"] = {packed_name};')
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"if (Object.keys({obj_json}).length > 0) {{")
        lines.append(f"  const {packed_name}: {{ [key: string]: any }} = {{}} as any;")
        lines.append(f"  for (const [key, value] of Object.entries({obj_json})) {{")
        key_expr = _generate_pack_json_scalar(prop.key_type, "key")
        value_expr = _generate_pack_json_scalar(prop, "value")
        lines.append(f"    {packed_name}[String({key_expr})] = {value_expr};")
        lines.append("  }")
        lines.append(f'  objectJson["{json_key}"] = {packed_name};')
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_json_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        ts_name = ts_name + "Ptr"
    json_key = to_casing(prop.name, Casing.LOWER_CAMEL)
    data_json = f'objectJson["{json_key}"]'
    var_name = f"unpacked{_upper_first(ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_json_scalar(prop, data_json)
            lines.append(f"const {var_name} = {value_expr};")
        else:
            value_expr = _generate_unpack_json_scalar(prop, f"{ts_name}Value")
            lines.append(f"const {ts_name}Value = {data_json};")
            lines.append(f"const {var_name} = {ts_name}Value != undefined ? {value_expr} : null;")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"const {var_name}: any[] = [];")
        lines.append(f"if ({data_json} != undefined) {{")
        lines.append(f"  for (const item of {data_json}) {{")
        item_expr = _generate_unpack_json_scalar(prop, "item")
        lines.append(f"    {var_name}.push({item_expr})")
        lines.append("  }")
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"const {var_name} = {{}} as any;")
        lines.append(f"if ({data_json} != undefined) {{")
        lines.append(f"  for (const [key, value] of Object.entries({data_json})) {{")
        key_expr = _generate_unpack_json_scalar(prop.key_type, "key")
        value_expr = _generate_unpack_json_scalar(prop, "value as any")
        lines.append(f"    {var_name}[{key_expr}] = {value_expr};")
        lines.append("  }")
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_json_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
        ):
            return f"Number({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return f"Number({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return f"Number({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"{value_expr}.toString({{ timeZoneName: 'never' }})"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"{value_expr}.toString()"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"{value_expr}.toString()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedeltaToISOFormat({value_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return value_expr
        elif prop.primitive_type == PrimitiveType.UUID:
            return value_expr
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64Encode({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        else:
            assert_never(prop.primitive_type)
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}[{value_expr}]"
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack({Encoding.JSON.value})"
    else:
        return value_expr


def _generate_unpack_json_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
        ):
            return f"Number({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return f"Number({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return f"Number({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"Temporal.Instant.from({value_expr}).toZonedDateTimeISO('UTC')"
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"Temporal.PlainDate.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"Temporal.PlainTime.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedeltaFromISOFormat({value_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return value_expr
        elif prop.primitive_type == PrimitiveType.UUID:
            return value_expr
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"base64Decode({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        else:
            assert_never(prop.primitive_type)
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}[{value_expr}] as any"
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"_{struct_cls.__name__}.unpack({Encoding.JSON.value}, {value_expr}, _session) as {struct_cls.__name__}"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return (
            f"_NodeReference.unpack({Encoding.JSON.value}, {value_expr}, _session) as NodeReference"
        )
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack({Encoding.JSON.value}, {value_expr}, _session) as Node"
    else:
        return value_expr


def generate_json_value(type: Type | TypeDeclaration | PropertyDeclaration, value: Any) -> str:
    """Generate a Typescript value literal."""
    if type.cardinality == TypeCardinality.SCALAR:
        return _generate_json_scalar(type, value)
    elif type.cardinality == TypeCardinality.LIST:
        elements_str = [
            textwrap.indent(_generate_json_scalar(type, element), "  ") for element in value
        ]
        return f"[\n{',\n'.join(elements_str)}\n]"
    elif type.cardinality == TypeCardinality.MAP:
        raise ValueError(f"unsupported value type {type.cardinality!r}: {type!r}")
    else:
        assert_never(type.cardinality)


def _generate_json_scalar(type: Type | TypeDeclaration | PropertyDeclaration, value: Any) -> str:
    """Generate a Typescript scalar value literal."""
    if type.scalar_type == ScalarType.PRIMITIVE:
        assert type.primitive_type is not None, f"no primitive_type for {type!r}"
        if type.primitive_type == PrimitiveType.BOOLEAN:
            return "true" if value else "false"
        elif type.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
        ):
            return str(value)
        elif type.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return str(value)
        elif type.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return str(value)
        elif type.primitive_type == PrimitiveType.DATETIME:
            return f"Temporal.Instant.from(\"{value}\").toZonedDateTimeISO('UTC')"
        elif type.primitive_type == PrimitiveType.DATE:
            return f'Temporal.PlainDate.from("{value}")'
        elif type.primitive_type == PrimitiveType.TIME:
            return f'Temporal.PlainTime.from("{value}")'
        elif type.primitive_type == PrimitiveType.DURATION:
            return f'Temporal.Duration.from("{value}")'
        elif type.primitive_type == PrimitiveType.STRING:
            return f'"{value}"'
        elif type.primitive_type == PrimitiveType.UUID:
            return f'"{value}"'
        elif type.primitive_type == PrimitiveType.BYTES:
            return f"base64Decode({base64.b64encode(value).decode()})"
        elif type.primitive_type == PrimitiveType.JSON:
            return json.dumps(value)
        else:
            assert_never(type.primitive_type)
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum_type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        return f"({int(value)} /* {enum_cls.__name__}.{value.name} */)"
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct_type for {type!r}"
        assert isinstance(value, Struct), f"value is not a Struct for {type!r}: {value!r}"
        writer = BinaryWriter()
        value.pack_binary(Encoding.CSON, writer)
        value_bytes = writer.to_bytes()
        value_bytes_str = base64.b64encode(value_bytes).decode("ascii")
        return f"{value.__class__.__name__}.unpackBinaryBase64({Encoding.CSON.value}, {value_bytes_str!r})"
    elif type.scalar_type == ScalarType.NODE_REFERENCE:
        assert isinstance(value, NodeReference), (
            f"value is not a NodeReference for {type!r}: {value!r}"
        )
        writer = BinaryWriter()
        value.pack_binary(Encoding.CSON, writer)
        value_bytes = writer.to_bytes()
        value_bytes_str = base64.b64encode(value_bytes).decode("ascii")
        return f"NodeReference.unpackBinaryBase64({Encoding.CSON.value}, {value_bytes_str!r})"
    elif type.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported value type {type.scalar_type!r}: {type!r}")
    else:
        assert_never(type.scalar_type)
