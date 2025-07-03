import textwrap
from typing import TYPE_CHECKING, assert_never

from destack.language import (
    BuiltinObject,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
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


def generate_object_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the BuiltinObject toProto/fromProto method implementations."""

    pack_proto = _generate_pack_proto(cls)
    unpack_proto = _generate_unpack_proto(cls)

    if cls.__is_frozen__ and not cls.__is_node__:
        to_proto_method = f"""
  toProto(): {cls.__name__}Proto {{
    if (this._proto === null) {{
      // @ts-expect-error(readonly)
      this._proto = {cls.__name__}.__packProto__(this);
    }}
    return this._proto as {cls.__name__}Proto;
  }}"""
    else:
        to_proto_method = f"""
  toProto(): {cls.__name__}Proto {{
    return {cls.__name__}.__packProto__(this);
  }}"""

    return f"""{to_proto_method}

  static __packProto__(object: {cls.__name__}): {cls.__name__}Proto {{
{textwrap.indent(pack_proto, "  ")}
  }}

  static __unpackProto__(
    objectProto: {cls.__name__}Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): {cls.__name__} {{
{textwrap.indent(unpack_proto, "  ")}
  }}

  static fromProto(
    objectProto: {cls.__name__}Proto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): {cls.__name__} {{
    return {cls.__name__}.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }}

  static fromProtoString(packedProtoString: string): {cls.__name__} {{
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = {cls.__name__}Proto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }}
  """


def _generate_pack_proto(cls: type["BuiltinObject"]) -> str:
    """Generate the toProto method implementation."""
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
    """Generate the fromProto method implementation."""
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
        unpack_assignments.append("_proto: objectProto")

    unpack_body_parts.append(f"return new {cls.__name__}({{")
    for assignment in unpack_assignments:
        unpack_body_parts.append(f"  {assignment},")
    if cls.__is_node__:
        unpack_body_parts.append("  _session,")
        unpack_body_parts.append("  _graph,")
        unpack_body_parts.append("  _connection,")
    else:
        unpack_body_parts.append("  _supergraph,")
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


def _generate_pack_proto_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        ts_name = ts_name + "Ptr"
    obj_value = f"object.{ts_name}"

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
        lines.append(f"  objectProto.{ts_name} = {{}};")
        lines.append(f"  for (const [key, value] of {obj_value}) {{")
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
        lines.append(f"const {var_name} = new Map();")
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
        elif prop.primitive_type == PrimitiveType.JSON:
            return f"packProtoJson({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"packProtoTimestamp({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"packProtoDuration({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        return f"Number({value_expr}) as {prop.enum_type.camel_name}Proto"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"{value_expr}.toProto()"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        return f"{value_expr}.toProto()"
    else:
        return value_expr


def _generate_unpack_proto_scalar(
    prop: "PropertyDeclaration | TypeDeclaration", value_expr: str
) -> str:
    """Generate the unpacking code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return f"unpackProtoJson({value_expr}!)"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"unpackProtoTimestamp({value_expr}!)"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"unpackProtoDuration({value_expr}!)"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"Number({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum type for {prop!r}"
        enum_type_name = prop.enum_type.camel_name
        return f"Number({value_expr}) as {enum_type_name}"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return (
            f"_NodeReference.fromProto(({value_expr})!, _session, _supergraph, _graph, _connection)"
        )
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise RuntimeError(f"node value cannot be wired directly: {prop!r}")
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"_{struct_cls.__name__}.fromProto(({value_expr})!, _session, _supergraph, _graph, _connection)"
    else:
        return value_expr
