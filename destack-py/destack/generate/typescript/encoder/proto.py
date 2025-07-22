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
from destack.language.registry import NODE_CLASS_BY_TYPE, STRUCT_CLASS_BY_TYPE, get_builtin_type
from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


def _upper_first(s: str) -> str:
    """Uppercase the first letter of a string."""
    return s[0].upper() + s[1:]


def generate_proto_encoders() -> str:
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
        "import { PROTO_OBJECT_ENCODERS, _ProtoObjectEncoder, getObjectKey } from '@destack/encoder/proto/generate';"
    )
    import_parts.append("import { Temporal } from 'temporal-polyfill';")
    import_parts.append("import { uuid4, uuid7, toNanoId } from '@destack/utils/uuid';")
    import_parts.append(
        "import { packProtoDuration, packProtoTimestamp, packProtoJson, unpackProtoDuration, unpackProtoTimestamp, unpackProtoJson } from '@destack/encoder/proto/wiring';"
    )
    import_parts.append(
        "import { timedeltaToISOFormat, timedeltaFromISOFormat, base64Encode, base64Decode } from '@destack/utils';"
    )
    import_parts.append("import type { AnyNodeProto, AnyStructProto } from '@destack/proto';")
    builtins_names: set[str] = set()
    builtins_names.update(
        cls.__name__ for cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values())
    )
    import_parts.append(f"import type {{ {', '.join(builtins_names)} }} from '@destack/language';")
    proto_names: set[str] = set()
    proto_names.update(
        f"{cls.__name__}Proto"
        for cls in chain(NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values())
    )
    import_parts.append(f"import {{ {', '.join(proto_names)} }} from '@destack/proto';")
    file_parts.append("\n".join(import_parts))

    # body
    file_parts.append("export const PROTO_ENCODERS: { [key: string]: _ProtoObjectEncoder } = {};")

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
        encoder_name, encoder_str = generate_object_proto_encoder(cls)
        body_parts.append(encoder_str)
        body_parts.append(
            f"PROTO_OBJECT_ENCODERS[getObjectKey({cls.__kind__.value}, {cls.metatype.value})] = new {encoder_name}();"
        )
    body_str = textwrap.indent("\n".join(body_parts), "  ")
    file_parts.append(f"""\
export function loadEncoders(): void {{
{textwrap.indent(body_str, "  ")}
}}

loadEncoders();
""")

    return "\n".join(file_parts)


def generate_object_proto_encoder(cls: type["BuiltinObject"]) -> tuple[str, str]:
    """Generate the BuiltinObject Encoder class."""

    if cls.__is_abstract__:
        pack_proto = f"throw new Error('cannot pack abstract {cls.__name__}');"
        unpack_proto = f"throw new Error('cannot unpack abstract {cls.__name__}');"
    else:
        pack_proto = _generate_pack_proto(cls)
        unpack_proto = _generate_unpack_proto(cls)

    encoder_name = f"{cls.__name__}ProtoEncoder"

    return (
        encoder_name,
        f"""
class {encoder_name} implements _ProtoObjectEncoder {{
  packObject(object: {cls.__name__}): {cls.__name__}Proto {{
{textwrap.indent(pack_proto, " " * 4)}
  }}

  unpackObject(options: {{
    value: {cls.__name__}Proto;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }}): {cls.__name__} {{
    const {{ value: objectProto, _session, _graph, _connection }} = options;
{textwrap.indent(unpack_proto, " " * 4)}
  }}

  packObjectBytes(object: {cls.__name__}): Uint8Array {{
    const proto = this.packObject(object);
    return {cls.__name__}Proto.toBinary(proto);
  }}

  unpackObjectBytes(options: {{
    value: Uint8Array;
    _session: Session | null;
    _graph: Graph | null;
    _connection: GraphConnection | null;
  }}): {cls.__name__} {{
    const proto = {cls.__name__}Proto.fromBinary(options.value);
    return this.unpackObject({{
      value: proto,
      _session: options._session,
      _graph: options._graph,
      _connection: options._connection,
    }});
  }}
}}
""",
    )


def _get_indirect_object_cls(type: type[BuiltinObject]) -> str:
    if issubclass(type, Node):
        return f"NODE_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    elif issubclass(type, Struct):
        return f"STRUCT_CLASS_BY_TYPE[{type.metatype.value}] as typeof {type.__name__}"
    else:
        raise ValueError(f"unexpected type {type!r}")


def _generate_pack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the packObject method implementation."""
    lines: list[str] = []

    metatype = get_builtin_type(cls)
    lines.append(
        f"const objectProto: Partial<{cls.__name__}Proto> = {{ metatype: {metatype.value} }};"
    )

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)

    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            continue  # already set

        pack_code = _generate_pack_proto_property(prop)
        if pack_code:
            lines.extend(pack_code)

    lines.append(f"return objectProto as {cls.__name__}Proto;")
    return "\n".join(lines)


def _generate_unpack_proto(cls: type["BuiltinObject"]) -> str:
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
        ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        unpack_code = _generate_unpack_proto_property(prop)
        if len(unpack_code) == 1:
            assignment = unpack_code[0].split(" = ", 1)[1]
            assignment = assignment.strip().rstrip(";")
            unpack_assignments.append(f"{ts_name}: {assignment}")
        else:
            unpack_body_parts.extend(unpack_code)
            unpack_assignments.append(f"{ts_name}: unpacked{_upper_first(ts_name)}")

    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append(
            f"_packedCache: [{{ encoding: {Encoding.PROTO.value}, isBytes: false, packed: objectProto }}]"
        )

    unpack_body_parts.append(f"return new ({_get_indirect_object_cls(cls)})({{")
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
            initializer_str = f"const _{reference_cls.__name__} = NODE_CLASS_BY_TYPE[{ref.value}] as typeof {reference_cls.__name__};"
            unpack_initializer_parts.append(initializer_str)
        elif isinstance(ref, StructType):
            reference_cls = STRUCT_CLASS_BY_TYPE[ref]
            initializer_str = f"const _{reference_cls.__name__} = STRUCT_CLASS_BY_TYPE[{ref.value}] as typeof {reference_cls.__name__};"
            unpack_initializer_parts.append(initializer_str)
        else:
            assert_never(ref)
    initializer_str = "\n".join(unpack_initializer_parts)
    unpack_body_parts.insert(0, initializer_str)

    return "\n".join(unpack_body_parts)


def _generate_pack_proto_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    from ..language import _is_property_tracked

    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        ts_name = ts_name + "Ptr"
    obj_value = f"object._{ts_name}" if _is_property_tracked(prop) else f"object.{ts_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            lines.append(f"if ({obj_value} != null) {{")
            scalar_expr = _generate_pack_proto_scalar(prop, obj_value)
            lines.append(f"  objectProto.{ts_name} = {scalar_expr};")
            lines.append("}")
        else:
            scalar_expr = _generate_pack_proto_scalar(prop, obj_value)
            lines.append(f"objectProto.{ts_name} = {scalar_expr};")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"if ({obj_value}) {{")
        item_expr = _generate_pack_proto_scalar(prop, "item")
        lines.append(f"  const packed{_upper_first(ts_name)}: any[] = [];")
        lines.append(f"  for (const item of {obj_value}) {{")
        lines.append(f"    packed{_upper_first(ts_name)}.push({item_expr});")
        lines.append("  }")
        lines.append(f"  objectProto.{ts_name} = packed{_upper_first(ts_name)};")
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"if ({obj_value}) {{")
        lines.append(f"  objectProto.{ts_name} = {{}} as any;")
        lines.append(f"  for (const [key, value] of Object.entries({obj_value}) ) {{")
        key_expr = _generate_pack_proto_scalar(prop.key_type, "key")
        value_expr = _generate_pack_proto_scalar(prop, "value")
        lines.append(f"    objectProto.{ts_name}![{key_expr}] = {value_expr};")
        lines.append("  }")
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_proto_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        ts_name = ts_name + "Ptr"
    proto_value = f"objectProto.{ts_name}"
    var_name = f"unpacked{_upper_first(ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            scalar_expr = _generate_unpack_proto_scalar(prop, proto_value)
            lines.append(f"const {var_name} = {proto_value} != undefined ? {scalar_expr} : null;")
        else:
            scalar_expr = _generate_unpack_proto_scalar(prop, proto_value)
            lines.append(f"const {var_name} = {scalar_expr};")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"const {var_name}: any[] = [];")
        lines.append(f"if ({proto_value}) {{")
        lines.append(f"  for (const item of {proto_value}) {{")
        item_expr = _generate_unpack_proto_scalar(prop, "item")
        lines.append(f"    {var_name}.push({item_expr});")
        lines.append("  }")
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"const {var_name} = {{}} as any;")
        lines.append(f"if ({proto_value}) {{")
        lines.append(f"  for (const [key, value] of Object.entries({proto_value})) {{")
        key_expr = _generate_unpack_proto_scalar(prop.key_type, "key")
        value_expr = _generate_unpack_proto_scalar(prop, "(value as any)")
        lines.append(f"    {var_name}.set({key_expr}, {value_expr});")
        lines.append("  }")
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_proto_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the packing code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"packProtoTimestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"packProtoDuration({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return f"packProtoJson({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        return f"Number({value_expr}) as any"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"{value_expr}.pack({Encoding.PROTO.value})"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        return f"{value_expr}.pack({Encoding.PROTO.value})"
    else:
        return value_expr


def _generate_unpack_proto_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"unpackProtoTimestamp({value_expr}!)"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"unpackProtoDuration({value_expr}!)"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"Number({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON or prop.primitive_type == PrimitiveType.CSON:
            return f"unpackProtoJson({value_expr}!)"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        return f"Number({value_expr}) as any"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"_NodeReference.unpack({{ encoding: {Encoding.PROTO.value}, value: {value_expr}, _session, _graph, _connection }}) as NodeReference"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"_{struct_cls.__name__}.unpack({{ encoding: {Encoding.PROTO.value}, value: {value_expr}, _session, _graph, _connection }}) as {struct_cls.__name__}"
    else:
        return value_expr
