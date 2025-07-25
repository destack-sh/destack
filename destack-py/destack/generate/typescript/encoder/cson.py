import textwrap
from itertools import chain
from typing import TYPE_CHECKING, assert_never

from destack.language import (
    BuiltinObject,
    Encoding,
    Node,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    Struct,
    StructType,
    TypeCardinality,
    TypeDeclaration,
)
from destack.language.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE
from destack.utils.log import get_logger
from destack.utils.string import Casing, to_casing
from destack.utils.telemetry import get_tracer

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113
# ruff: noqa: SIM114
# pyright: reportIncompatibleVariableOverride=false

logger = get_logger(__name__)
tracer = get_tracer(__name__)
type_ = type


def _upper_first(s: str) -> str:
    """Uppercase the first letter of a string."""
    return s[0].upper() + s[1:]


def generate_cson_encoders() -> str:
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
        "import { CSON_OBJECT_ENCODERS, CsonObjectEncoder, getObjectKey } from '@destack/encoder/cson/core';"
    )
    import_parts.append("import { Temporal } from 'temporal-polyfill';")
    import_parts.append("import { uuid4, uuid7, toNanoId } from '@destack/utils/uuid';")
    import_parts.append(
        "import { timedeltaToISOFormat, timedeltaFromISOFormat, base64Encode, base64Decode } from '@destack/utils';"
    )
    builtins_names: set[str] = set()
    builtins_names.update(
        cls.__name__ for cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values())
    )
    import_parts.append(f"import type {{ {', '.join(builtins_names)} }} from '@destack/language';")
    file_parts.append("\n".join(import_parts))

    # body
    file_parts.append("export const CSON_ENCODERS: { [key: string]: CsonObjectEncoder } = {};")

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
        encoder_name, encoder_str = generate_object_cson_encoder(cls)
        body_parts.append(encoder_str)
        body_parts.append(
            f"CSON_OBJECT_ENCODERS[getObjectKey({cls.__kind__.value}, {cls.metatype.value})] = new {encoder_name}();"
        )
    body_str = textwrap.indent("\n".join(body_parts), "  ")
    file_parts.append(f"""\
export function loadEncoders(): void {{
{textwrap.indent(body_str, "  ")}
}}

loadEncoders();
""")

    return "\n".join(file_parts)


def generate_object_cson_encoder(cls: type["BuiltinObject"]) -> tuple[str, str]:
    """Generate the BuiltinObject Encoder class."""

    if cls.__is_abstract__:
        pack_cson = f"throw new Error('cannot pack abstract {cls.__name__}');"
        unpack_cson = f"throw new Error('cannot unpack abstract {cls.__name__}');"
    else:
        pack_cson = _generate_to_cson(cls)
        unpack_cson = _generate_from_cson(cls)

    encoder_name = f"{cls.__name__}CsonEncoder"

    return (
        encoder_name,
        f"""
class {encoder_name} implements CsonObjectEncoder {{
  packObject(object: {cls.__name__}): any {{
{textwrap.indent(pack_cson, " " * 4)}
  }}

  unpackObject(objectCson: any, _session: Session | null): {cls.__name__} {{
{textwrap.indent(unpack_cson, " " * 4)}
  }}
}}
""",
    )


def _generate_to_cson(cls: type["BuiltinObject"]) -> str:
    """Generate the packObject method implementation."""
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


def _get_indirect_object_cls(type: type[BuiltinObject]) -> str:
    if issubclass(type, Node):
        return f"NODE_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    elif issubclass(type, Struct):
        return f"STRUCT_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    else:
        raise ValueError(f"unexpected type {type!r}")


def _generate_from_cson(cls: type["BuiltinObject"]) -> str:
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
        unpack_assignments.append(
            f"_packedCache: [{{ encoding: {Encoding.CSON.value}, isBytes: false, packed: objectCson }}]"
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


def _generate_pack_cson_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    from ..language import _is_property_tracked

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
        item_expr = _generate_pack_cson_scalar(prop, "item")
        if prop.is_required:
            lines.append(f"const {packed_name}: any[] = [];")
            lines.append(f"for (const item of {obj_cson}) {{")
            lines.append(f"  {packed_name}.push({item_expr});")
            lines.append("}")
            lines.append(f'objectCson["{prop.id}"] = {packed_name};')
        else:
            lines.append(f"if ({obj_cson} != null) {{")
            lines.append(f"  const {packed_name}: any[] = [];")
            lines.append(f"  for (const item of {obj_cson}) {{")
            lines.append(f"    {packed_name}.push({item_expr});")
            lines.append("  }")
            lines.append(f'  objectCson["{prop.id}"] = {packed_name};')
            lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        key_expr = _generate_pack_cson_scalar(prop.key_type, "key")
        value_expr = _generate_pack_cson_scalar(prop, "value")
        if prop.is_required:
            lines.append(f"const {packed_name}: {{ [key: string]: any }} = {{}} as any;")
            lines.append(f"for (const [key, value] of Object.entries({obj_cson})) {{")
            lines.append(f"  {packed_name}[String({key_expr})] = {value_expr};")
            lines.append("}")
            lines.append(f'objectCson["{prop.id}"] = {packed_name};')
        else:
            lines.append(f"if ({obj_cson} != null) {{")
            lines.append(f"  const {packed_name}: {{ [key: string]: any }} = {{}} as any;")
            lines.append(f"  for (const [key, value] of Object.entries({obj_cson})) {{")
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
    json_key = str(prop.id)
    data_cson = f'objectCson["{json_key}"]'
    var_name = f"unpacked{_upper_first(ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_cson_scalar(prop, data_cson)
            lines.append(f"const {var_name} = {value_expr};")
        else:
            value_expr = _generate_unpack_cson_scalar(prop, f"{ts_name}Value")
            lines.append(f"const {ts_name}Value = {data_cson};")
            lines.append(
                f"const {var_name} = {ts_name}Value != undefined ? {value_expr} : undefined;"
            )
    elif prop.cardinality == TypeCardinality.LIST:
        item_expr = _generate_unpack_cson_scalar(prop, "item")
        if prop.is_required:
            lines.append(f"const {var_name}: any[] = [];")
            lines.append(f"for (const item of {data_cson}) {{")
            lines.append(f"  {var_name}.push({item_expr})")
            lines.append("}")
        else:
            lines.append(f"let {var_name}: any[] | undefined;")
            lines.append(f"if ({data_cson} != undefined) {{")
            lines.append(f"  {var_name} = [];")
            lines.append(f"  for (const item of {data_cson}) {{")
            lines.append(f"    {var_name}.push({item_expr})")
            lines.append("  }")
            lines.append("} else {")
            lines.append(f"  {var_name} = undefined;")
            lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        key_expr = _generate_unpack_cson_scalar(prop.key_type, "key")
        value_expr = _generate_unpack_cson_scalar(prop, "value as any")
        if prop.is_required:
            lines.append(f"const {var_name} = {{}} as any;")
            lines.append(f"for (const [key, value] of Object.entries({data_cson})) {{")
            lines.append(f"    {var_name}[{key_expr}] = {value_expr};")
            lines.append("}")
        else:
            lines.append(f"let {var_name}: {{{key_expr}: any}} | undefined;")
            lines.append(f"if ({data_cson} != undefined) {{")
            lines.append(f"  {var_name} = {{}} as any;")
            lines.append(f"  for (const [key, value] of Object.entries({data_cson})) {{")
            lines.append(f"    {var_name}[{key_expr}] = {value_expr};")
            lines.append("  }")
            lines.append("} else {")
            lines.append(f"  {var_name} = undefined;")
            lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_cson_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "null"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.SINT8,
            PrimitiveType.SINT16,
            PrimitiveType.SINT32,
            PrimitiveType.SINT64,
            PrimitiveType.SINT128,
        ):
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.UINT8,
            PrimitiveType.UINT16,
            PrimitiveType.UINT32,
            PrimitiveType.UINT64,
            PrimitiveType.UINT128,
        ):
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.FLOAT16,
            PrimitiveType.FLOAT32,
            PrimitiveType.FLOAT64,
        ):
            return value_expr
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
        return value_expr
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack({Encoding.CSON.value})"
    else:
        return value_expr


def _generate_unpack_cson_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "null"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return f"Boolean({value_expr})"
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
        return f"Number({value_expr})"
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"_{struct_cls.__name__}.unpack({Encoding.CSON.value}, {value_expr}, _session) as {struct_cls.__name__}"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return (
            f"_NodeReference.unpack({Encoding.CSON.value}, {value_expr}, _session) as NodeReference"
        )
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack({Encoding.CSON.value}, {value_expr}, _session) as Node"
    else:
        return value_expr
