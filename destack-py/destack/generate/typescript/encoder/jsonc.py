import textwrap
from itertools import chain
from typing import TYPE_CHECKING, assert_never

from destack.language import (
    Encoding,
    Node,
    Object,
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


def generate_jsonc_encoders() -> str:
    file_parts: list[str] = []

    # imports
    import_parts: list[str] = []
    import_parts.append(
        "import type { Object, Graph, GraphConnection, Session, Encoder } from '@destack/language';"
    )
    import_parts.append(
        "import { NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE } from '@destack/language/registry';"
    )
    import_parts.append(
        "import { JSONC_OBJECT_ENCODERS, JsoncObjectEncoder, getObjectKey } from '@destack/encoder/jsonc/core';"
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
    file_parts.append("export const JSONC_ENCODERS: { [key: string]: JsoncObjectEncoder } = {};")

    # encoders
    file_parts.append("let loaded = false;")
    body_parts: list[str] = [
        "if (loaded) {",
        "  return;",
        "}",
        "loaded = true;",
    ]
    for cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values()):
        if cls.__declaration__.is_abstract:
            continue
        encoder_name, encoder_str = generate_object_jsonc_encoder(cls)
        body_parts.append(encoder_str)
        body_parts.append(
            f"JSONC_OBJECT_ENCODERS[getObjectKey({cls.__kind__.value}, {cls.metatype.value})] = new {encoder_name}();"
        )
    body_str = textwrap.indent("\n".join(body_parts), "  ")
    file_parts.append(f"""\
export function loadEncoders(): void {{
{textwrap.indent(body_str, "  ")}
}}

loadEncoders();
""")

    return "\n".join(file_parts)


def generate_object_jsonc_encoder(cls: type["Object"]) -> tuple[str, str]:
    """Generate the Object Encoder class."""

    if cls.__declaration__.is_abstract:
        pack_jsonc = f"throw new Error('cannot pack abstract {cls.__name__}');"
        unpack_jsonc = f"throw new Error('cannot unpack abstract {cls.__name__}');"
    else:
        pack_jsonc = _generate_to_jsonc(cls)
        unpack_jsonc = _generate_from_jsonc(cls)

    encoder_name = f"{cls.__name__}JsoncEncoder"

    return (
        encoder_name,
        f"""
class {encoder_name} implements JsoncObjectEncoder {{
  packObject(object: {cls.__name__}): any {{
{textwrap.indent(pack_jsonc, " " * 4)}
  }}

  unpackObject(objectJsonc: any, _session: Session | null): {cls.__name__} {{
{textwrap.indent(unpack_jsonc, " " * 4)}
  }}
}}
""",
    )


def _generate_to_jsonc(cls: type["Object"]) -> str:
    """Generate the packObject method implementation."""
    lines: list[str] = []
    lines.append("const objectJsonc: { [key: string]: any } = {};")

    properties_in_order = list(cls.__properties__.values())
    properties_in_order.sort(key=lambda p: p.id or 0)

    for prop in properties_in_order:
        if prop.name == "metatype":
            from destack.language.registry import get_builtin_type

            metatype = get_builtin_type(cls)
            lines.append(f'objectJsonc["{prop.id}"] = {metatype.value};')
            continue

        pack_code = _generate_pack_jsonc_property(prop)
        if pack_code:
            lines.extend(pack_code)

    lines.append("return objectJsonc;")
    return "\n".join(lines)


def _get_indirect_object_cls(type: type[Object]) -> str:
    if issubclass(type, Node):
        return f"NODE_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    elif issubclass(type, Struct):
        return f"STRUCT_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    else:
        raise ValueError(f"unexpected type {type!r}")


def _generate_from_jsonc(cls: type["Object"]) -> str:
    """Generate the unpackObject method implementation."""
    from ..language import _get_object_references

    object_references = _get_object_references(cls, is_value=True)
    unpack_assignments: list[str] = []
    unpack_body_parts: list[str] = []

    for prop in cls.__properties__.values():
        if prop.is_runtime_only:
            continue  # set implicitly
        # regular unpacking
        unpack_code = _generate_unpack_jsonc_property(prop)
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

    unpack_body_parts.append(f"return new ({_get_indirect_object_cls(cls)})({{")
    for assignment in unpack_assignments:
        unpack_body_parts.append(f"  {assignment},")
    unpack_body_parts.append("  _session,")
    unpack_body_parts.append("});")

    # initializer
    unpack_initializer_parts: list[str] = []
    for ref in sorted(object_references):
        ref_cls = (
            STRUCT_CLASS_BY_TYPE[ref] if isinstance(ref, StructType) else NODE_CLASS_BY_TYPE[ref]
        )
        initializer_str = f"const _{ref_cls.__name__} = {_get_indirect_object_cls(ref_cls)};"
        unpack_initializer_parts.append(initializer_str)
    initializer_str = "\n".join(unpack_initializer_parts)
    unpack_body_parts.insert(0, initializer_str)

    return "\n".join(unpack_body_parts)


def _generate_pack_jsonc_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    from ..language import _is_property_tracked

    lines: list[str] = []
    prop_ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_ts_name = prop_ts_name + "Ptr"
    obj_jsonc = (
        f"object._{prop_ts_name}" if _is_property_tracked(prop) else f"object.{prop_ts_name}"
    )
    packed_name = f"packed{_upper_first(prop_ts_name)}"

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_pack_jsonc_scalar(prop, obj_jsonc)
            lines.append(f'objectJsonc["{prop.id}"] = {value_expr};')
        else:
            lines.append(f"if ({obj_jsonc} != null) {{")
            value_expr = _generate_pack_jsonc_scalar(prop, obj_jsonc)
            lines.append(f'  objectJsonc["{prop.id}"] = {value_expr};')
            lines.append("}")

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_expr = _generate_pack_jsonc_scalar(prop.value_type, "item")
        if prop.is_required:
            lines.append(f"const {packed_name}: any[] = [];")
            lines.append(f"for (const item of {obj_jsonc}) {{")
            lines.append(f"  {packed_name}.push({item_expr});")
            lines.append("}")
            lines.append(f'objectJsonc["{prop.id}"] = {packed_name};')
        else:
            lines.append(f"if ({obj_jsonc} != null) {{")
            lines.append(f"  const {packed_name}: any[] = [];")
            lines.append(f"  for (const item of {obj_jsonc}) {{")
            lines.append(f"    {packed_name}.push({item_expr});")
            lines.append("  }")
            lines.append(f'  objectJsonc["{prop.id}"] = {packed_name};')
            lines.append("}")

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot pack tuple: {prop!r}")

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_expr = _generate_pack_jsonc_scalar(prop.key_type, "key")
        value_expr = _generate_pack_jsonc_scalar(prop.value_type, "value")
        if prop.is_required:
            lines.append(f"const {packed_name}: {{ [key: string]: any }} = {{}} as any;")
            lines.append(f"for (const [key, value] of Object.entries({obj_jsonc})) {{")
            lines.append(f"  {packed_name}[String({key_expr})] = {value_expr};")
            lines.append("}")
            lines.append(f'objectJsonc["{prop.id}"] = {packed_name};')
        else:
            lines.append(f"if ({obj_jsonc} != null) {{")
            lines.append(f"  const {packed_name}: {{ [key: string]: any }} = {{}} as any;")
            lines.append(f"  for (const [key, value] of Object.entries({obj_jsonc})) {{")
            lines.append(f"    {packed_name}[String({key_expr})] = {value_expr};")
            lines.append("  }")
            lines.append(f'  objectJsonc["{prop.id}"] = {packed_name};')
            lines.append("}")

    #
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_jsonc_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        ts_name = ts_name + "Ptr"
    json_key = str(prop.id)
    data_jsonc = f'objectJsonc["{json_key}"]'
    var_name = f"unpacked{_upper_first(ts_name)}"

    # scalar
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_jsonc_scalar(prop, data_jsonc)
            lines.append(f"const {var_name} = {value_expr};")
        else:
            value_expr = _generate_unpack_jsonc_scalar(prop, f"{ts_name}Value")
            lines.append(f"const {ts_name}Value = {data_jsonc};")
            lines.append(
                f"const {var_name} = {ts_name}Value != undefined ? {value_expr} : undefined;"
            )

    # list
    elif prop.cardinality == TypeCardinality.LIST:
        assert prop.value_type is not None, f"no value type for {prop!r}"
        item_expr = _generate_unpack_jsonc_scalar(prop.value_type, "item")
        if prop.is_required:
            lines.append(f"const {var_name}: any[] = [];")
            lines.append(f"for (const item of {data_jsonc}) {{")
            lines.append(f"  {var_name}.push({item_expr})")
            lines.append("}")
        else:
            lines.append(f"let {var_name}: any[] | undefined;")
            lines.append(f"if ({data_jsonc} != undefined) {{")
            lines.append(f"  {var_name} = [];")
            lines.append(f"  for (const item of {data_jsonc}) {{")
            lines.append(f"    {var_name}.push({item_expr})")
            lines.append("  }")
            lines.append("} else {")
            lines.append(f"  {var_name} = undefined;")
            lines.append("}")

    # tuple
    elif prop.cardinality == TypeCardinality.TUPLE:
        raise NotImplementedError(f"cannot unpack tuple: {prop!r}")

    # map
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        assert prop.value_type is not None, f"no value type for {prop!r}"
        key_expr = _generate_unpack_jsonc_scalar(prop.key_type, "key")
        value_expr = _generate_unpack_jsonc_scalar(prop.value_type, "value as any")
        if prop.is_required:
            lines.append(f"const {var_name} = {{}} as any;")
            lines.append(f"for (const [key, value] of Object.entries({data_jsonc})) {{")
            lines.append(f"    {var_name}[{key_expr}] = {value_expr};")
            lines.append("}")
        else:
            lines.append(f"let {var_name}: {{{key_expr}: any}} | undefined;")
            lines.append(f"if ({data_jsonc} != undefined) {{")
            lines.append(f"  {var_name} = {{}} as any;")
            lines.append(f"  for (const [key, value] of Object.entries({data_jsonc})) {{")
            lines.append(f"    {var_name}[{key_expr}] = {value_expr};")
            lines.append("  }")
            lines.append("} else {")
            lines.append(f"  {var_name} = undefined;")
            lines.append("}")

    #
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_jsonc_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""

    assert prop.cardinality == TypeCardinality.SCALAR, f"cannot pack non-scalar: {prop!r}"
    assert prop.scalar_type is not None, f"no scalar type for {prop!r}"

    # primitive
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "null"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
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

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        return value_expr

    # struct
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.pack({Encoding.JSONC.value})"

    #
    else:
        return value_expr


def _generate_unpack_jsonc_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""

    assert prop.cardinality == TypeCardinality.SCALAR, f"cannot unpack non-scalar: {prop!r}"
    assert prop.scalar_type is not None, f"no scalar type for {prop!r}"

    # primitive
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type == PrimitiveType.NONE:
            return "null"
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return f"Boolean({value_expr})"
        elif prop.primitive_type in (
            PrimitiveType.INT8,
            PrimitiveType.INT16,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
            PrimitiveType.INT128,
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

    # enum
    elif prop.scalar_type == ScalarType.ENUM:
        return f"Number({value_expr})"

    # struct
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"_{struct_cls.__name__}.unpack({Encoding.JSONC.value}, {value_expr}, _session) as {struct_cls.__name__}"

    # node reference
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"_NodeReference.unpack({Encoding.JSONC.value}, {value_expr}, _session) as NodeReference"

    # node value
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.unpack({Encoding.JSONC.value}, {value_expr}, _session) as Node"

    #
    else:
        return value_expr
