import textwrap
from typing import TYPE_CHECKING, assert_never

from destack.language import (
    BuiltinObjectBase,
    IntoType,
    PrimitiveType,
    PropertyDeclaration,
    ScalarType,
    TypeCardinality,
)
from destack.language.registry import STRUCT_CLASS_BY_TYPE
from destack.utils.string import Casing, to_casing

if TYPE_CHECKING:
    pass

# ruff: noqa: FURB113
# pyright: reportIncompatibleVariableOverride=false


def _upper_first(s: str) -> str:
    """Uppercase the first letter of a string."""
    return s[0].upper() + s[1:]


def generate_object_value(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the BuiltinObject toValue/fromValue method implementations."""

    pack_value = textwrap.indent(_generate_to_value(cls), "    ")
    unpack_value = textwrap.indent(_generate_from_value(cls), "    ")

    if cls.__is_frozen__ and not cls.__is_node__:
        to_value_method = f"""
  toValue(): {{ [key: string]: any }} {{
    if (this._value === null) {{
      // @ts-expect-error(readonly)
      this._value = {cls.__name__}.__packValue__(this);
    }}
    return this._value;
  }}"""
    else:
        to_value_method = f"""
  toValue(): {{ [key: string]: any }} {{
    return {cls.__name__}.__packValue__(this);
  }}"""

    return f"""{to_value_method}

  static __packValue__(object: {cls.__name__}): {{ [key: string]: any }} {{
{pack_value}
  }}

  static __unpackValue__(
    objectValue: {{ [key: string]: any }},
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): {cls.__name__} {{
{unpack_value}
  }}

  static fromValue(
    objectValue: {{ [key: string]: any }},
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): {cls.__name__} {{
    return {cls.__name__}.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }}"""


def _generate_to_value(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the toValue method implementation."""
    lines: list[str] = []
    lines.append("const objectValue: { [key: string]: any } = {};")

    wired_properties_in_order = list(cls.__wired_properties__.values())
    wired_properties_in_order.sort(key=lambda p: p.id or 0)

    for prop in wired_properties_in_order:
        if prop.name == "metatype":
            from destack.language.registry import get_builtin_type

            metatype = get_builtin_type(cls)
            lines.append(f'objectValue["{prop.id}"] = {metatype.value};')
            continue

        pack_code = _generate_pack_value_property(prop)
        if pack_code:
            lines.extend(pack_code)

    lines.append("return objectValue;")
    return "\n".join(lines)


def _generate_from_value(cls: type["BuiltinObjectBase"]) -> str:
    """Generate the fromValue method implementation."""
    unpack_assignments: list[str] = []
    unpack_method_parts: list[str] = []

    for prop in cls.__wired_properties__.values():
        if prop.is_computed:
            continue  # set implicitly
        if prop.runtime_prop is not None:
            prop = prop.runtime_prop
        unpack_code = _generate_unpack_value_property(prop)
        ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        if len(unpack_code) == 1:
            assignment = unpack_code[0].split(" = ", 1)[1]
            assignment = assignment.strip().rstrip(";")
            unpack_assignments.append(f"{ts_name}: {assignment}")
        else:
            unpack_method_parts.extend(unpack_code)
            unpack_assignments.append(f"{ts_name}: unpacked{_upper_first(ts_name)}")

    if cls.__is_frozen__ and not cls.__is_node__:
        unpack_assignments.append("_value: objectValue")

    unpack_method_parts.append(f"return new {cls.__name__}({{")
    for assignment in unpack_assignments:
        unpack_method_parts.append(f"  {assignment},")
    if cls.__is_node__:
        unpack_method_parts.append("  _session,")
        unpack_method_parts.append("  _graph,")
        unpack_method_parts.append("  _connection,")
    else:
        unpack_method_parts.append("  _supergraph,")
    unpack_method_parts.append("});")

    return "\n".join(unpack_method_parts)


def _generate_pack_value_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the packing code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    obj_value = f"object.{ts_name}"
    packed_name = f"packed{_upper_first(ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_pack_value_scalar(prop, obj_value)
            lines.append(f'objectValue["{prop.id}"] = {value_expr};')
        else:
            lines.append(f"if ({obj_value} != null) {{")
            value_expr = _generate_pack_value_scalar(prop, obj_value)
            lines.append(f'  objectValue["{prop.id}"] = {value_expr};')
            lines.append("}")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"if ({obj_value}) {{")
        lines.append(f"  const {packed_name}: any[] = [];")
        lines.append(f"  for (const item of {obj_value}) {{")
        item_expr = _generate_pack_value_scalar(prop, "item")
        lines.append(f"    {packed_name}.push({item_expr});")
        lines.append("  }")
        lines.append(f'  objectValue["{prop.id}"] = {packed_name};')
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"if ({obj_value}) {{")
        lines.append(f"  const {packed_name}: {{ [key: string]: any }} = {{}};")
        lines.append(f"  for (const [key, value] of {obj_value}) {{")
        key_expr = _generate_pack_value_scalar(prop.key_type, "key")
        value_expr = _generate_pack_value_scalar(prop, "value")
        lines.append(f"    {packed_name}[String({key_expr})] = {value_expr};")
        lines.append("  }")
        lines.append(f'  objectValue["{prop.id}"] = {packed_name};')
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_unpack_value_property(prop: "PropertyDeclaration") -> list[str]:
    """Generate the unpacking code for a property value."""
    lines: list[str] = []
    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    data_value = f'objectValue["{prop.id}"]'
    var_name = f"unpacked{_upper_first(ts_name)}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            value_expr = _generate_unpack_value_scalar(prop, data_value)
            lines.append(f"const {var_name} = {value_expr};")
        else:
            value_expr = _generate_unpack_value_scalar(prop, f"{ts_name}Value")
            lines.append(f"const {ts_name}Value = {data_value};")
            lines.append(f"const {var_name} = {ts_name}Value != undefined ? {value_expr} : null;")
    elif prop.cardinality == TypeCardinality.LIST:
        lines.append(f"const {var_name}: any[] = [];")
        lines.append(f"if ({data_value} != undefined) {{")
        lines.append(f"  for (const item of {data_value}) {{")
        item_expr = _generate_unpack_value_scalar(prop, "item")
        lines.append(f"    {var_name}.push({item_expr})")
        lines.append("  }")
        lines.append("}")
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key type for {prop!r}"
        lines.append(f"const {var_name} = new Map();")
        lines.append(f"if ({data_value} != undefined) {{")
        lines.append(f"  for (const [key, value] of Object.entries({data_value})) {{")
        key_expr = _generate_unpack_value_scalar(prop.key_type, "key")
        value_expr = _generate_unpack_value_scalar(prop, "value as any")
        lines.append(f"    {var_name}.set({key_expr}, {value_expr});")
        lines.append("  }")
        lines.append("}")
    else:
        assert_never(prop.cardinality)

    return lines


def _generate_pack_value_scalar(prop: "PropertyDeclaration | IntoType", value_expr: str) -> str:
    """Generate the packing code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"Buffer.from({value_expr}).toString('base64')"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        elif prop.primitive_type in (
            PrimitiveType.DATE,
            PrimitiveType.TIME,
            PrimitiveType.DATETIME,
        ):
            return f"{value_expr}.toString()"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedeltaToISOFormat({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return value_expr
    elif prop.scalar_type in (ScalarType.STRUCT, ScalarType.NODE_REFERENCE, ScalarType.NODE_VALUE):
        return f"{value_expr}.toValue()"
    else:
        return value_expr


def _generate_unpack_value_scalar(prop: "PropertyDeclaration | IntoType", value_expr: str) -> str:
    """Generate the unpacking code for a scalar value."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type == PrimitiveType.BYTES:
            return f"Buffer.from({value_expr}, 'base64')"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"String({value_expr})"
        elif prop.primitive_type == PrimitiveType.JSON:
            return value_expr
        elif prop.primitive_type == PrimitiveType.DATE:
            return f"Temporal.PlainDate.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.TIME:
            return f"Temporal.PlainTime.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"Temporal.ZonedDateTime.from({value_expr})"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"timedeltaFromISOFormat({value_expr})"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"Number({value_expr})"
        else:
            return value_expr
    elif prop.scalar_type == ScalarType.ENUM:
        return f"Number({value_expr})"
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}.fromValue({value_expr}, _session, _supergraph, _graph, _connection)"
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"NodeReference.fromValue({value_expr}, _session, _supergraph, _graph, _connection)"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        return f"Node.fromValue({value_expr}, _session, _supergraph, _graph, _connection)"
    else:
        return value_expr
