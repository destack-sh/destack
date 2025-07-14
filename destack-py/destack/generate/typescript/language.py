import re
import subprocess
import textwrap
from collections import defaultdict
from itertools import chain
from pathlib import Path
from typing import assert_never, cast

from destack.language import (
    EMPTY_DICT,
    UNSET,
    BuiltinObject,
    ConstantDefinition,
    EdgeType,
    Entity,
    EnumDefinition,
    Event,
    Node,
    NodeDefinition,
    NodeType,
    PrimitiveType,
    PropertyDeclaration,
    PropertyDefinition,
    RoleType,
    ScalarType,
    Struct,
    StructDefinition,
    StructFrozen,
    StructType,
    Trait,
    TraitDefinition,
    TraitType,
    Type,
    TypeCardinality,
    TypeDeclaration,
    ValueFactory,
    expand_node_types,
)
from destack.language.registry import (
    CONSTANT_DEFINITIONS,
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.string import Casing, to_casing

from .const import (
    GENERATION_PATH,
    MARKER_CUSTOM_END,
    MARKER_CUSTOM_START,
    MARKER_END,
    MARKER_START,
    Definition,
    Kind,
)
from .core import (
    TypescriptCodeBlock,
    TypescriptDefinition,
    TypescriptDefinitionBlock,
    TypescriptFile,
    TypescriptImport,
    TypescriptImportBlock,
)
from .grpc import generate_object_proto
from .map import TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE
from .value import generate_object_value, generate_value

# ruff: noqa: FURB113


def _generate_multiline_doc(description: str) -> str:
    """Generate a multiline JSDoc comment."""
    if "\n" in description:
        # multiline description - format each line with proper JSDoc comment prefix
        description_lines = description.strip().split("\n")
        formatted_description = "\n".join(
            f" * {line}" if line.strip() else " *" for line in description_lines
        )
    else:
        # single line description
        formatted_description = f" * {description}"
    return f"/**\n{formatted_description}\n */"


def _get_properties(cls: type[BuiltinObject]) -> list[PropertyDeclaration]:
    """Get the properties of a class."""
    properties: list[PropertyDeclaration] = []
    for prop in cls.__wired_properties__.values():
        if prop.id == 1:
            continue
        properties.append(prop)
    properties.sort(key=lambda prop: prop.id or 0)
    return properties


def _is_property_effective_readonly(prop: PropertyDeclaration) -> bool:
    """Check if a property is (effectively) readonly to the user."""
    return (
        prop.is_readonly
        or prop.can_write is None
        or prop.can_write == RoleType.SYSTEM
        or prop.is_managed
    )


def _is_property_tracked(prop: PropertyDeclaration) -> bool:
    """Check if a property is tracked (tracked properties are set on Nodes)."""
    return (
        not prop.is_computed
        and not prop.is_managed
        and not prop.is_readonly
        and (prop.component.__is_node__ or prop.component.__is_trait__)
        and not prop.component.__is_frozen__
    )


def _generate_property_scalar_type(
    prop: PropertyDeclaration | TypeDeclaration, as_ptr: bool = True
) -> str:
    """Generate a scalar property Typescript type annotation."""
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        if as_ptr:
            return "NodeReference"
        else:
            resolved_node_types = expand_node_types(prop.node_types, expand_inheritance=False)
            if not resolved_node_types or len(resolved_node_types) == len(NodeType):
                if isinstance(prop, PropertyDeclaration) and prop.edge_type == EdgeType.PARENT:
                    return "Entity"
                else:
                    return "Node"
            node_classes: list[str] = []
            for node_type in prop.node_types or ():
                if isinstance(node_type, NodeType):
                    node_classes.append(NODE_CLASS_BY_TYPE[node_type].__name__)
                elif isinstance(node_type, TraitType):
                    node_classes.append(f"(Entity & {TRAIT_CLASS_BY_TYPE[node_type].__name__})")
                else:
                    assert_never(node_type)
            return " | ".join(node_classes)
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"no enum_type for {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        return f"{enum_cls.__name__}"
    elif prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive_type for {prop!r}"
        return TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"no struct_type for {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        return f"{struct_cls.__name__}"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise ValueError(f"unsupported scalar_type: {prop!r}")
    else:
        assert_never(prop.scalar_type)


def _generate_property_type(prop: PropertyDeclaration, as_ptr: bool = True) -> str:
    """Generate a property Typescript type annotation."""
    type_str = _generate_property_scalar_type(prop, as_ptr=as_ptr)
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            type_str = f"{type_str} | null"
    elif prop.cardinality == TypeCardinality.LIST:
        type_str = f"readonly {type_str}[]"
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key_type for {prop!r}"
        key_type_str = _generate_property_scalar_type(prop.key_type)
        type_str = f"{{ readonly [key: {key_type_str}]: {type_str} }}"
    else:
        assert_never(prop.cardinality)
    return type_str


def _generate_property(
    prop: PropertyDeclaration,
    *,
    is_effective_readonly: bool,
    is_tracked: bool,
    is_node: bool,
    is_interface: bool,
    is_abstract: bool,
) -> str:
    """
    Generate a Property definition.
    NOTE :Cleanup: typescript SDK generate_property is a bit of a mess
    """

    prop_ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    doc_str = _generate_multiline_doc(
        prop.description or f"{prop.original_component.__name__}.{prop_ts_name}"
    )
    internal_prop_ts_name = f"_{prop_ts_name}" if is_tracked else prop_ts_name

    # add wrapper for node references
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        assert prop.cardinality == TypeCardinality.SCALAR, (
            f"node references must be scalar: {prop!r}"
        )

        # wrap main property
        wrapped_ts_name = prop_ts_name
        prop_ts_name = f"{prop_ts_name}Ptr"
        internal_prop_ts_name = f"{internal_prop_ts_name}Ptr"
        wrapped_node_type_str = _generate_property_scalar_type(prop, as_ptr=False)
        if prop.is_optional:
            wrapped_node_type_str = f"{wrapped_node_type_str} | null"

        # wrap with node getter/setter
        if is_interface:
            # just interface declaration
            wrapped_node_getter_str = f"get {wrapped_ts_name}(): {wrapped_node_type_str} | null"
            if not is_effective_readonly:
                wrapped_node_setter_str = f"set {wrapped_ts_name}(value: {wrapped_node_type_str})"
                wrapped_prefix_str = f"{wrapped_node_getter_str}\n{wrapped_node_setter_str}"
            else:
                wrapped_prefix_str = wrapped_node_getter_str
        elif is_abstract:
            # just abstract declaration
            wrapped_node_getter_str = (
                f"{doc_str}\nabstract get {wrapped_ts_name}(): {wrapped_node_type_str} | null"
            )
            if not is_effective_readonly:
                wrapped_node_setter_str = (
                    f"abstract set {wrapped_ts_name}(value: {wrapped_node_type_str})"
                )
                wrapped_prefix_str = f"{wrapped_node_getter_str}\n{wrapped_node_setter_str}"
            else:
                wrapped_prefix_str = wrapped_node_getter_str
        else:
            # actual getter/setter
            if is_node:
                wrapped_node_getter_str = f"""\
{doc_str}
get {wrapped_ts_name}(): {wrapped_node_type_str} | null {{
    const nodePtr: NodeReference | null = this.{prop_ts_name};
    if (nodePtr !== null) {{
        return this._supergraph.get(nodePtr.id) as {wrapped_node_type_str} | null;
    }}
    return null;
}}"""
                if not is_effective_readonly:
                    if prop.is_optional:
                        wrapped_node_setter_str = f"""\
set {wrapped_ts_name}(node: {wrapped_node_type_str}) {{
    if (node === null) {{
        this.{prop_ts_name} = null;
    }} else {{
        this.{prop_ts_name} = node.toRef();
    }}
}}"""
                    else:
                        wrapped_node_setter_str = f"""\
set {wrapped_ts_name}(node: {wrapped_node_type_str}) {{
    this.{prop_ts_name} = node.toRef();
}}"""
                    wrapped_prefix_str = f"{wrapped_node_getter_str}\n{wrapped_node_setter_str}"
                else:
                    wrapped_prefix_str = wrapped_node_getter_str

            else:
                wrapped_node_getter_str = f"""\
{doc_str}
get {wrapped_ts_name}(): {wrapped_node_type_str} | null {{
    const nodePtr: NodeReference | null = this.{prop_ts_name};
    if (nodePtr !== null) {{
        if (this._supergraph === null) {{
            return null;
        }}
        return this._supergraph.get(nodePtr.id) as {wrapped_node_type_str};
    }}
    return null;
}}"""
                if not is_effective_readonly:
                    if prop.is_optional:
                        wrapped_node_setter_str = f"""\
set {wrapped_ts_name}(value: {wrapped_node_type_str}) {{
    if (value == null) {{
        this.{prop_ts_name} = null;
    }} else {{
        this.{prop_ts_name} = value.toRef();
    }}
}}"""
                    else:
                        wrapped_node_setter_str = f"""\
set {wrapped_ts_name}(value: {wrapped_node_type_str}) {{
    this.{prop_ts_name} = value.toRef();
}}"""
                    wrapped_prefix_str = f"{wrapped_node_getter_str}\n{wrapped_node_setter_str}"
                else:
                    wrapped_prefix_str = wrapped_node_getter_str
        prop_type_str = _generate_property_type(prop, as_ptr=True)
    else:
        # regular property
        wrapped_prefix_str = ""
        prop_type_str = _generate_property_type(prop)

    # main property
    prop_str = f"{internal_prop_ts_name}: {prop_type_str}"
    if is_effective_readonly and not is_tracked:
        prop_str = f"readonly {prop_str}"
    if is_tracked and is_abstract and not is_interface:
        prop_str = f"""\
{doc_str}
abstract get {prop_ts_name}(): {prop_type_str};
abstract set {prop_ts_name}(value: {prop_type_str});
"""
    elif is_tracked and is_interface:
        prop_str = f"""\
{doc_str}
get {prop_ts_name}(): {prop_type_str};
set {prop_ts_name}(value: {prop_type_str});
"""
    elif is_abstract and not is_interface:
        prop_str = f"declare {prop_str}"

    # wrap get/set for tracked Node properties
    if is_tracked and not is_abstract and not is_interface:
        prop_str = f"""\
{doc_str}
get {prop_ts_name}(): {prop_type_str} {{
    return this.{internal_prop_ts_name};
}}
set {prop_ts_name}(value: {prop_type_str}) {{
    const prop = (this.constructor as NodeClass).__properties__["{prop.name}"];
    this._session.updateSetProperty(this, prop, value);
    this.{internal_prop_ts_name} = value;
}}
{prop_str}
"""

    # wrap
    if prop_str.count("\n") < 1:
        prop_str = f"{prop_str};"
    if wrapped_prefix_str:
        prop_str = f"""\
{wrapped_prefix_str}
{prop_str}"""
    else:
        prop_str = f"""\
{doc_str}
{prop_str}"""
    return prop_str


def _generate_init(cls: type[BuiltinObject]) -> str:
    """Generate a Typescript constructor with options-style parameters."""

    def _is_property_required(prop: PropertyDeclaration) -> bool:
        return (
            prop.is_required
            and prop.default is UNSET
            and prop.default_factory is None
            and prop.cardinality == TypeCardinality.SCALAR
            and not prop.is_managed
        )

    parent_cls = cls.__base__ if cls.__base__ and issubclass(cls.__base__, BuiltinObject) else None
    is_parent_concrete = (
        not cls.__is_abstract__ and parent_cls is not None and not parent_cls.__is_abstract__
    )
    properties = {p.name: p for p in _get_properties(cls)}
    header_properties = dict(properties)
    body_properties = dict(properties)
    if issubclass(cls, Node):
        body_properties.pop("id", None)
        body_properties.pop("created_at", None)
        body_properties.pop("created_by", None)
        body_properties.pop("updated_at", None)
        body_properties.pop("updated_by", None)

    # header
    header_parts: list[str] = []
    for prop in header_properties.values():
        if prop.is_computed:
            continue  # computed, can't assign
        ts_name_in = to_casing(prop.name, Casing.LOWER_CAMEL)
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # can be passed either as Node or NodeReference
            node_type_str = _generate_property_scalar_type(prop, as_ptr=False)
            ptr_type_str = _generate_property_scalar_type(prop, as_ptr=True)
            type_str = f"{node_type_str} | {ptr_type_str}"
            if prop.is_optional:
                type_str = f"{type_str} | null"
        else:
            # can be passed as value
            type_str = _generate_property_type(prop)
        if _is_property_required(prop):
            header_parts.append(f"{ts_name_in}: {type_str}")
        else:
            header_parts.append(f"{ts_name_in}?: {type_str}")
    header_parts.extend(("_session?: Session | null", "_supergraph?: Supergraph | null"))
    if issubclass(cls, Node):
        header_parts.extend(("_graph?: Graph | null", "_connection?: QueryConnection | null"))
    elif issubclass(cls, StructFrozen):
        header_parts.extend(
            (
                "_hash?: number | null",
                "_repr?: string | null",
                "_proto?: any | null",
                "_value?: { [key: string]: any } | null",
            )
        )
    header_str = ",\n".join(header_parts)

    # super
    if is_parent_concrete:
        super_str = """\
super(options);
"""
    elif issubclass(cls, Node):
        super_str = """\
super(
    // id
    options.id ?? null,
    // parent
    options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null,
    // session
    options._session ?? null,
    // supergraph
    options._supergraph ?? null,
    // graph
    options._graph ?? null,
    // connection
    options._connection ?? null,
    // is_new
    options.id == null,
);
"""
    else:
        super_str = """\
super(
    // session
    options._session ?? null,
    // supergraph
    options._supergraph ?? null,
);
"""

    # body
    body_parts: list[str] = []
    body_properties_in_order = list(body_properties.values())
    body_properties_in_order.sort(key=lambda p: (p.id is None, p.id, p.name))
    for prop in body_properties_in_order:
        if prop.is_computed:
            continue  # computed, can't assign
        if is_parent_concrete and parent_cls is not None and prop.name in parent_cls.__properties__:
            continue  # parent has this property, don't assign

        ts_name_in = to_casing(prop.name, Casing.LOWER_CAMEL)
        ts_name_self = ts_name_in
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            ts_name_self = ts_name_in + "Ptr"
        if cls.__is_node__ and _is_property_tracked(prop):
            ts_name_self = f"_{ts_name_self}"

        if _is_property_required(prop):
            body_parts.append(f"let _{ts_name_in} = options.{ts_name_in};")
        else:
            body_parts.append(f"let _{ts_name_in} = options.{ts_name_in} ?? null;")

        # convert node to node reference
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            body_parts.append(f"""\
if (_{ts_name_in} != null && _{ts_name_in}.metatype != StructType.NODE_REFERENCE) {{
    _{ts_name_in} = (_{ts_name_in} as Node).toRef();
}}""")
        # init non-scalars if unset
        if prop.cardinality == TypeCardinality.LIST:
            body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    _{ts_name_in} = [];
}}""")
        elif prop.cardinality == TypeCardinality.MAP:
            body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    _{ts_name_in} = {{}};
}}""")

        # init default
        if prop.default is not UNSET and prop.default is not None:
            default_str = generate_value(prop, prop.default)
            body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    _{ts_name_in} = {default_str};
}}""")

        # init default factory
        if prop.default_factory is not None:
            if prop.default_factory == ValueFactory.UUID:
                body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    _{ts_name_in} = uuid7();
}}""")
            elif prop.default_factory == ValueFactory.NOW:
                body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    _{ts_name_in} = Temporal.Now.zonedDateTimeISO("UTC");
}}""")
            elif prop.default_factory == ValueFactory.SELF:
                body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    _{ts_name_in} = this.toRef();
}}""")
            elif prop.default_factory == ValueFactory.SPACE:
                body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    if (this._session === null) {{
        throw new Error(`{cls.__name__} has no Session`);
    }}
    _space = ACTIVE_SPACE.get()
    if (_space === null) {{
        throw new Error(`{cls.__name__} has no Space`);
    }}
    _{ts_name_in} = _space.toRef();
}}""")
            else:
                raise ValueError(
                    f"unsupported default factory for {prop!r}: {prop.default_factory!r}"
                )

        # raise on missing value
        if prop.is_required and prop.cardinality == TypeCardinality.SCALAR:
            body_parts.append(f"""\
if (_{ts_name_in} === null) {{
    throw new Error(`{cls.__name__}.{ts_name_in} is required`);
}}""")

        body_parts.append(f"this.{ts_name_self} = _{ts_name_in};")

    body_str = "\n".join(body_parts)

    # node identity
    if is_parent_concrete:
        identity_str = """\
// ... (already set in parent)
"""
    elif issubclass(cls, Node):
        if issubclass(cls, Entity):
            identity_str = f"""\
if (options.id == null) {{
  const now = Temporal.Now.zonedDateTimeISO("UTC");
  this.createdAt = now;
  this.createdByPtr = null;
  this.updatedAt = now;
  this.updatedByPtr = null;
}} else {{
  if (options.createdAt == null || options.updatedAt == null) {{
    throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
  }}
  this.createdAt = options.createdAt;
  this.createdByPtr = options.createdBy != null ? (options.createdBy.metatype == StructType.NODE_REFERENCE ? (options.createdBy as NodeReference) : (options.createdBy as Node).toRef()) : null;
  this.updatedAt = options.updatedAt; 
  this.updatedByPtr = options.updatedBy != null ? (options.updatedBy.metatype == StructType.NODE_REFERENCE ? (options.updatedBy as NodeReference) : (options.updatedBy as Node).toRef()) : null;
}}
"""
        elif issubclass(cls, Event):
            identity_str = f"""\
if (options.id == null) {{
  const now = Temporal.Now.zonedDateTimeISO("UTC");
  this.createdAt = now;
  this.createdByPtr = null;
}} else {{
  if (options.createdAt == null) {{
    throw new Error(`{cls.__name__}.createdAt is required for existing Events`);
  }}
  this.createdAt = options.createdAt;
  this.createdByPtr = options.createdBy != null ? (options.createdBy.metatype == StructType.NODE_REFERENCE ? (options.createdBy as NodeReference) : (options.createdBy as Node).toRef()) : null;
}}
"""
        else:
            raise NotImplementedError(f"unexpected node {cls.__name__} extends {cls.__inherits__}")
    else:
        if issubclass(cls, StructFrozen):
            identity_str = """\
// @ts-expect-error(readonly)
this._hash = options._hash ?? null;
// @ts-expect-error(readonly)
this._repr = options._repr ?? null;
// @ts-expect-error(readonly)
this._proto = options._proto ?? null;
// @ts-expect-error(readonly)
this._value = options._value ?? null;
"""
        else:
            identity_str = """\
// ...
"""

    # assemble constructor
    init_str = f"""\
constructor(options: {{
{textwrap.indent(header_str.strip(), "  ")}
}}) {{
{textwrap.indent(super_str.strip(), "  ")}

  // properties
{textwrap.indent(body_str.strip(), "  ")}

  // identity
{textwrap.indent(identity_str.strip(), "  ")}
}}
"""

    return init_str.strip()


def _generate_repr(cls: type[BuiltinObject]) -> str:
    """Generate BuiltinObject.repr method."""
    repr_properties = [prop for prop in cls.__properties__.values() if prop.is_repr]
    if not repr_properties:
        if cls.__is_node__:
            repr_impl = f"""\
repr(): string {{
    return `<{cls.__name__} "${{this.path}}">`
}}
"""
        else:
            repr_impl = f"""\
repr(): string {{
    return `<{cls.__name__}>`
}}
"""
        return repr_impl

    def _get_scalar_repr(prop: TypeDeclaration, value_expr: str) -> str:
        """Get repr expression for a scalar value."""
        if prop.scalar_type == ScalarType.ENUM:
            assert prop.enum_type is not None, f"no enum type for {prop!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
            return f"{enum_cls.__name__}[{value_expr}]"
        elif prop.scalar_type == ScalarType.STRUCT:
            return f"{value_expr}.repr()"
        elif prop.scalar_type == ScalarType.NODE_REFERENCE:
            return f"{value_expr}?.repr()"
        elif prop.scalar_type in (ScalarType.PRIMITIVE, ScalarType.NODE_VALUE):
            if prop.primitive_type == PrimitiveType.DATETIME:
                return f"{value_expr}.toString({{ timeZoneName: 'never'}})"
            elif prop.primitive_type in (PrimitiveType.DATE, PrimitiveType.TIME):
                return f"{value_expr}.toString()"
            elif prop.primitive_type == PrimitiveType.STRING:
                return f'`"${{{value_expr}}}"`'
            else:
                return value_expr
        else:
            assert_never(prop.scalar_type)

    # property parts
    repr_parts_lines: list[str] = []
    repr_parts_lines.append("const propertyReprs: string[] = [];")
    has_required_repr_props = False

    for prop in repr_properties:
        prop_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        if prop.cardinality == TypeCardinality.SCALAR:
            if prop.is_required:
                scalar_expr = _get_scalar_repr(prop, f"this.{prop_name}")
                repr_parts_lines.append(f"propertyReprs.push(`{prop_name}=${{{scalar_expr}}}`);")
                has_required_repr_props = True
            else:
                scalar_expr = _get_scalar_repr(prop, f"this.{prop_name}")
                repr_parts_lines.append(f"if (this.{prop_name} !== null) {{")
                repr_parts_lines.append(
                    f"    propertyReprs.push(`{prop_name}=${{{scalar_expr}}}`);"
                )
                repr_parts_lines.append("}")
        elif prop.cardinality == TypeCardinality.LIST:
            scalar_repr = _get_scalar_repr(prop, "_item")
            list_expr = f"this.{prop_name}.map(_item => {scalar_repr}).join(', ')"
            repr_parts_lines.append(f"if (this.{prop_name}.length > 0) {{")
            repr_parts_lines.append(f"    propertyReprs.push(`{prop_name}=${{{list_expr}}}`);")
            repr_parts_lines.append("}")
        elif prop.cardinality == TypeCardinality.MAP:
            assert prop.key_type is not None, f"{prop!r} has no key type"
            key_repr = _get_scalar_repr(prop.key_type, "k")
            value_repr = _get_scalar_repr(prop, "v")
            map_expr = f"'{{' + Object.entries(this.{prop_name}).map(([k, v]) => `${{{key_repr}}}: ${{{value_repr}}}`).join(', ') + '}}'"
            repr_parts_lines.append(f"if (Object.keys(this.{prop_name}).length > 0) {{")
            repr_parts_lines.append(f"    propertyReprs.push(`{prop_name}=${{{map_expr}}}`);")
            repr_parts_lines.append("}")
        else:
            assert_never(prop.cardinality)
    # wrap in repr
    repr_parts_str = "\n".join(repr_parts_lines)
    if cls.__is_node__:
        if has_required_repr_props:
            inner_repr_impl = f"""\
{repr_parts_str}
return `<{cls.__name__} "${{this.path}}" ${{propertyReprs.join(' ')}}>`
"""
        else:
            inner_repr_impl = f"""\
{repr_parts_str}
if (propertyReprs.length > 0) {{
    return `<{cls.__name__} "${{this.path}}" ${{propertyReprs.join(' ')}}>`;
}} else {{
    return `<{cls.__name__} "${{this.path}}">`;
}}
"""
    else:
        if has_required_repr_props:
            inner_repr_impl = f"""\
{repr_parts_str}
return `<{cls.__name__} ${{propertyReprs.join(' ')}}>`
"""
        else:
            inner_repr_impl = f"""\
{repr_parts_str}
if (propertyReprs.length > 0) {{
    return `<{cls.__name__} ${{propertyReprs.join(' ')}}>`;
}} else {{
    return `<{cls.__name__}>`;
}}
"""

    if cls.__is_frozen__ and not cls.__is_node__:
        # cache _repr in __repr__ (frozen Struct)
        inner_repr_impl = inner_repr_impl.replace(
            "return ", "// @ts-expect-error(readonly)\nthis._repr = "
        )
        inner_repr_impl = textwrap.indent(inner_repr_impl, "    ")
        inner_repr_impl = f"if (this._repr === null) {{\n{inner_repr_impl}\n}}\nreturn this._repr;"
        inner_repr_impl = textwrap.indent(inner_repr_impl, "    ")
        repr_impl = f"""\
repr(): string {{
{inner_repr_impl}
}}
"""
    else:
        # no cache
        inner_repr_impl = textwrap.indent(inner_repr_impl, "    ")
        repr_impl = f"""\
repr(): string {{
{inner_repr_impl}
}}
"""

    return repr_impl


def _generate_path(cls: type[Node]) -> str:
    """Generate a Typescript path method."""
    # Node._path_key
    if "slug" in cls.__properties__:
        if "name" in cls.__properties__:
            if cls.__properties__["name"].is_required:
                path_key_str = "this.slug ?? this.name"
            else:
                path_key_str = f"this.slug ?? this.name ?? `{cls.__name__}[id=${{this.id}}]`"
        else:
            path_key_str = f"this.slug ?? `{cls.__name__}[id=${{this.id}}]`"
    elif "name" in cls.__properties__:
        if cls.__properties__["name"].is_required:
            path_key_str = "this.name"
        else:
            path_key_str = f"this.name ?? `{cls.__name__}[id=${{this.id}}]`"
    else:
        path_key_str = f"`{cls.__name__}[id=${{this.id}}]`"

    # Node.path
    if cls.__root_type__ is None:
        path_str = f"""\
get _pathKey(): string {{
    return {path_key_str};
}}

get path(): string {{
    return {path_key_str};
}}"""
    else:
        path_str = f"""\
get _pathKey(): string {{
    return {path_key_str};
}}

get path(): string {{
    const pathParts: string[] = [];
    let node: Node | null = this;
    let lastNode: Node | null = this;
    while (node !== null) {{
        pathParts.push(node._pathKey);
        lastNode = node;
        node = node.parent;
    }}
    if (!lastNode.isRoot) {{
        pathParts.push("<detached>");
    }}
    return pathParts.reverse().join("/");
}}
"""

    return path_str.strip()


def _generate_equals(cls: type[BuiltinObject]) -> str:
    """Generate a Typescript equals method."""
    eq_properties = [prop for prop in cls.__properties__.values() if prop.is_eq and prop.is_wired]
    assert eq_properties, f"{cls.__name__} has no properties to compare"

    cmp_strs = []
    for prop in eq_properties:
        cmp_str = _generate_property_cmp_impl(prop)
        cmp_strs.append(cmp_str)

    body_str = "\n".join(cmp_strs)
    body_str = textwrap.indent(body_str, "  ")

    equals_impl = f"""\
equals(other: any): boolean {{
{body_str}
  return true;
}}"""

    return equals_impl.strip()


def _generate_property_cmp_impl(prop: PropertyDeclaration) -> str:
    """Generate equality check code for a single property."""
    prop_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_name = f"{prop_name}Ptr"
    if _is_property_tracked(prop):
        prop_name = f"_{prop_name}"

    scalar_cmps_str, is_simple = _generate_scalar_cmp_impl(prop)
    if prop.cardinality == TypeCardinality.SCALAR:
        # scalar
        if prop.is_required or is_simple:
            # required scalar
            return f"""\
if (!({scalar_cmps_str.format(self_val=f"this.{prop_name}", other_val=f"other.{prop_name}")})) {{
  return false;
}}"""
        else:
            # optional scalar
            return f"""\
if ((this.{prop_name} == null) !== (other.{prop_name} == null) || (this.{prop_name} != null && !({scalar_cmps_str.format(self_val=f"this.{prop_name}", other_val=f"other.{prop_name}")}))) {{
  return false;
}}"""
    elif prop.cardinality == TypeCardinality.LIST:
        # list (always required)
        return f"""\
if (this.{prop_name}.length !== other.{prop_name}.length) {{
  return false;
}}
for (let i = 0; i < this.{prop_name}.length; i++) {{
  if (!({scalar_cmps_str.format(self_val=f"this.{prop_name}[i]", other_val=f"other.{prop_name}[i]")})) {{
    return false;
  }}
}}"""
    elif prop.cardinality == TypeCardinality.MAP:
        # map (always required)
        if prop.scalar_type in (
            ScalarType.STRUCT,
            ScalarType.NODE_REFERENCE,
            ScalarType.NODE_VALUE,
        ):
            # maps with complex values need key-by-key comparison
            return f"""\
if (Object.keys(this.{prop_name}).length !== Object.keys(other.{prop_name}).length) {{
  return false;
}}
for (const key in this.{prop_name}) {{
  if (!(key in other.{prop_name})) {{
    return false;
  }}
  if (!({scalar_cmps_str.format(self_val=f"this.{prop_name}[key]", other_val=f"other.{prop_name}[key]")})) {{
    return false;
  }}
}}"""
        else:
            # maps with primitive/enum values can use direct comparison
            return f"""\
if (JSON.stringify(this.{prop_name}) !== JSON.stringify(other.{prop_name})) {{
  return false;
}}"""
    else:
        assert_never(prop.cardinality)


def _generate_scalar_cmp_impl(prop: PropertyDeclaration) -> tuple[str, bool]:
    """Generate the core scalar comparison logic. Returns a format string with {self_val} and {other_val} placeholders."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        if prop.primitive_type and prop.primitive_type.is_float:
            return "{self_val} === {other_val} || Math.abs({self_val} - {other_val}) < 1e-10", False
        else:
            return "{self_val} === {other_val}", True
    elif prop.scalar_type == ScalarType.ENUM:
        return "{self_val} === {other_val}", True
    elif prop.scalar_type == ScalarType.NODE_REFERENCE or prop.scalar_type == ScalarType.NODE_VALUE:
        if prop.is_required:
            return "{self_val}.id === {other_val}.id", True
        else:
            return "{self_val}?.id === {other_val}?.id", True
    elif prop.scalar_type == ScalarType.STRUCT:
        return "{self_val}.equals({other_val})", False
    else:
        assert_never(prop.scalar_type)


def _generate_hash(cls: type[BuiltinObject]) -> str:
    """Generate a Typescript hash method."""
    hash_properties = [
        prop for prop in cls.__properties__.values() if prop.is_hash and prop.is_wired
    ]
    assert hash_properties, f"{cls.__name__} has no properties to hash"
    hash_parts: list[str] = ["let h = 1;"]
    for prop in hash_properties:
        prop_hash_impl = _generate_property_hash_impl(prop)
        hash_parts.append(prop_hash_impl)
    hash_parts_str = "\n".join(hash_parts)

    if cls.__is_frozen__ and not cls.__is_node__:
        hash_impl = f"""\
hash(): number {{
  if (this._hash !== null) {{
    return this._hash;
  }}

{textwrap.indent(hash_parts_str, "  ")}

  // @ts-expect-error(readonly)
  this._hash = h;
  return h;
}}
"""
    else:
        hash_impl = f"""\
hash(): number {{
{textwrap.indent(hash_parts_str, "  ")}

  return h;
}}
"""
    return hash_impl.strip()


def _generate_property_hash_impl(prop: PropertyDeclaration) -> str:
    """Generate a Typescript hash method for a single property."""
    prop_name = to_casing(prop.name, Casing.LOWER_CAMEL)
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        prop_name = f"{prop_name}Ptr"
    if _is_property_tracked(prop):
        prop_name = f"_{prop_name}"

    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_required:
            scalar_hash_str = _generate_scalar_hash_impl(prop, f"this.{prop_name}")
            return f"h = ((h * 31) + {scalar_hash_str}) & 0xFFFFFFFF;"
        else:
            scalar_hash_str = _generate_scalar_hash_impl(prop, f"this.{prop_name}")
            return f"""\
if (this.{prop_name} !== null) {{
  h = ((h * 31) + {scalar_hash_str}) & 0xFFFFFFFF;
}}"""
    elif prop.cardinality == TypeCardinality.LIST:
        scalar_hash_str = _generate_scalar_hash_impl(prop, "_item")
        return f"""\
if (this.{prop_name} && this.{prop_name}.length > 0) {{
  for (const _item of this.{prop_name}) {{
    h = ((h * 31) + {scalar_hash_str}) & 0xFFFFFFFF;
  }}
}}"""
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"{prop.name} has no key type"
        key_hash_str = _generate_scalar_hash_impl(prop.key_type, "_key")
        value_hash_str = _generate_scalar_hash_impl(prop, "_value")
        return f"""\
if (this.{prop_name} && Object.keys(this.{prop_name}).length > 0) {{
  for (const [_key, _value] of Object.entries(this.{prop_name})) {{
    h = ((h * 31) + {key_hash_str}) & 0xFFFFFFFF;
    h = ((h * 31) + {value_hash_str}) & 0xFFFFFFFF;
  }}
}}"""
    else:
        assert_never(prop.cardinality)


def _generate_scalar_hash_impl(prop: TypeDeclaration | PropertyDeclaration, value_expr: str) -> str:
    """Generate a Typescript hash method for a single scalar property."""
    if prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"no primitive type for {prop!r}"
        if prop.primitive_type in (PrimitiveType.FLOAT32, PrimitiveType.FLOAT64):
            return f"hashFloat({value_expr})"
        elif prop.primitive_type in (PrimitiveType.INT16, PrimitiveType.INT32, PrimitiveType.INT64):
            return f"hashInt({value_expr})"
        elif prop.primitive_type == PrimitiveType.DECIMAL:
            raise NotImplementedError(f"cannot hash decimal: {prop!r}")
        elif prop.primitive_type == PrimitiveType.BOOLEAN:
            return f"hashBool({value_expr})"
        elif prop.primitive_type == PrimitiveType.STRING:
            return f"hashString({value_expr})"
        elif prop.primitive_type == PrimitiveType.BYTES:
            return f"hashBytes({value_expr})"
        elif prop.primitive_type == PrimitiveType.UUID:
            return f"hashString({value_expr}.toString())"
        elif prop.primitive_type == PrimitiveType.DATETIME:
            return f"hashString({value_expr}.toString({{ timeZoneName: 'never'}}))"
        elif prop.primitive_type in (PrimitiveType.DATE, PrimitiveType.TIME):
            return f"hashString({value_expr}.toString())"
        elif prop.primitive_type == PrimitiveType.DURATION:
            return f"hashFloat({value_expr}.total('seconds'))"
        elif prop.primitive_type == PrimitiveType.JSON:
            return f"hashString(JSON.stringify({value_expr}))"
        else:
            assert_never(prop.primitive_type)
    elif prop.scalar_type == ScalarType.ENUM:
        return value_expr
    elif prop.scalar_type == ScalarType.STRUCT:
        return f"{value_expr}.hash()"
    elif prop.scalar_type == ScalarType.NODE_VALUE:
        raise NotImplementedError(f"cannot hash node value: {prop!r}")
    elif prop.scalar_type == ScalarType.NODE_REFERENCE:
        return f"hashString({value_expr}.id)"
    else:
        assert_never(prop.scalar_type)


def _generate_validate(cls: type[BuiltinObject]) -> str:
    """Generate a Typescript validate method."""
    validate_str = """\
validate(): void {
  throw new Error("not implemented");
}
"""
    return validate_str.strip()


def _generate_ref(cls: type["Node"]) -> str:
    """Generate a Typescript toRef method."""

    node_type = cls.metatype
    if node_type == NodeType.SPACE:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
  return new _NodeReference({{
    type: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.id,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    elif node_type == NodeType.SNAPSHOT:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
  return new _NodeReference({{
    type: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.spacePtr?.id ?? null,
    snapshotId: this.id,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    elif TraitType.EXTENSIBLE in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
  return new _NodeReference({{
    type: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.spacePtr?.id ?? null,
    snapshotId: this.snapshotPtr?.id ?? null,
    definitionId: this.definitionPtr?.id ?? null,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    else:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
  return new _NodeReference({{
    type: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.spacePtr?.id ?? null,
    snapshotId: this.snapshotPtr?.id ?? null,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    return ref_impl.strip()


def _generate_enum(definition: EnumDefinition) -> str:
    """Generate a Typescript Enum definition."""
    enum_parts: list[str] = []
    for option in definition.options:
        enum_parts.append(f"{option.name} = {option.id},")
    enum_str = f"""\
{_generate_multiline_doc(definition.description or definition.name)}
export enum {definition.name} {{
{textwrap.indent("\n".join(enum_parts), "  ")}

  {MARKER_CUSTOM_START}
  // ...
  {MARKER_CUSTOM_END}
}}
registerEnumClass(EnumType.{definition.type.name}, {definition.name});
"""
    return enum_str.strip()


def _generate_struct(definition: StructDefinition) -> str:
    """Generate a Typescript Struct definition."""
    struct_cls = STRUCT_CLASS_BY_TYPE[definition.type]
    struct_parts: list[str] = []
    parent_cls = (
        struct_cls.__base__
        if struct_cls.__base__ and issubclass(struct_cls.__base__, Struct)
        else None
    )
    is_parent_concrete = (
        not struct_cls.__is_abstract__ and parent_cls is not None and not parent_cls.__is_abstract__
    )

    # meta
    struct_meta_parts: list[str] = [
        f"static metatype: StructType = StructType.{definition.type.name};",
        f"static __isFrozen__: boolean = {'true' if definition.is_frozen else 'false'};",
    ]
    struct_parts.append("\n".join(struct_meta_parts))

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(struct_cls):
        if is_parent_concrete and parent_cls is not None and prop.name in parent_cls.__properties__:
            continue  # parent has this property, don't declare again
        prop_str = _generate_property(
            prop,
            is_effective_readonly=definition.is_frozen,
            is_tracked=_is_property_tracked(prop),
            is_node=False,
            is_interface=False,
            is_abstract=struct_cls.__is_abstract__,
        )
        prop_parts.append(prop_str)
    struct_parts.append("\n\n".join(prop_parts))

    # body
    if not struct_cls.__is_abstract__:
        init_str = _generate_init(struct_cls)
        struct_parts.append(init_str)
        equals_str = _generate_equals(struct_cls)
        struct_parts.append(equals_str)
        repr_str = _generate_repr(struct_cls)
        struct_parts.append(repr_str)
        hash_str = _generate_hash(struct_cls)
        struct_parts.append(hash_str)
        validate_str = _generate_validate(struct_cls)
        struct_parts.append(validate_str)
        value_str = generate_object_value(struct_cls)
        struct_parts.append(value_str)
        proto_str = generate_object_proto(struct_cls)
        struct_parts.append(proto_str)

    if definition.base_type is None or definition.base_type == StructType.STRUCT:
        base_cls_name = "Struct" if not definition.is_frozen else "StructFrozen"
    else:
        base_cls_name = (
            STRUCT_CLASS_BY_TYPE[definition.base_type].__name__
            if definition.base_type
            else "Struct"
        )
    extends_str = f" extends {base_cls_name}"
    generic_str = " <T extends Node = Node>" if definition.name == "Query" else ""
    struct_str = f"""\
{_generate_multiline_doc(definition.description or definition.name)}
export {"abstract " if struct_cls.__is_abstract__ else ""}class {definition.name}{generic_str}{extends_str} {{
{textwrap.indent("\n\n".join(struct_parts), "  ")}

  {MARKER_CUSTOM_START}
  // ...
  {MARKER_CUSTOM_END}
}}
registerStructClass(StructType.{definition.type.name}, {definition.name});
"""
    return struct_str.strip()


def _generate_trait(definition: TraitDefinition) -> str:
    """Generate a Typescript Trait definition."""

    trait_cls = TRAIT_CLASS_BY_TYPE[definition.type]
    trait_parts: list[str] = []

    # interface properties
    prop_parts: list[str] = []
    for prop in _get_properties(trait_cls):
        if (
            prop.name in ("id", "parent")
            or prop.original_component.__name__ != prop.component.__name__
        ):
            continue  # ignore node base properties for traits
        prop_str = _generate_property(
            prop,
            is_effective_readonly=_is_property_effective_readonly(prop),
            is_tracked=_is_property_tracked(prop),
            is_node=False,
            is_interface=True,
            is_abstract=True,
        )
        prop_parts.append(prop_str)
    trait_parts.append("\n\n".join(prop_parts))

    # interface
    super_trait_classes = [
        super_cls
        for super_cls in trait_cls.__bases__
        if super_cls != trait_cls and issubclass(super_cls, Trait) and super_cls != Trait
    ]
    super_trait_classes.sort(key=lambda cls: TRAIT_TYPE_BY_CLASS[cast(type[Trait], cls)])
    extends_str = (
        " extends " + ", ".join(super_cls.__name__ for super_cls in super_trait_classes)
        if super_trait_classes
        else ""
    )

    # instance
    instance_parts: list[str] = []
    instance_str = "\n".join(instance_parts)

    doc_str = definition.description or definition.name
    trait_str = f"""\
{_generate_multiline_doc(doc_str)}
export interface {definition.alias}{extends_str} {{
{textwrap.indent("\n".join(trait_parts), "  ")}

  {MARKER_CUSTOM_START}
  // ...
  {MARKER_CUSTOM_END}
}}

{_generate_multiline_doc(doc_str)}
class {definition.alias}$Type extends TraitClass<{definition.alias}, TraitType.{definition.type.name}> {{
{textwrap.indent(instance_str, "  ")}
}}

export const {definition.alias} = new {definition.alias}$Type(TraitType.{definition.type.name});
registerTraitClass(TraitType.{definition.type.name}, {definition.alias});
"""
    return trait_str.strip()


def _generate_node(definition: NodeDefinition) -> str:
    """Generate a Typescript Node definition."""

    node_cls = NODE_CLASS_BY_TYPE[definition.type]
    node_parts: list[str] = []
    parent_cls = (
        node_cls.__base__ if node_cls.__base__ and issubclass(node_cls.__base__, Node) else None
    )
    is_parent_concrete = (
        not node_cls.__is_abstract__ and parent_cls is not None and not parent_cls.__is_abstract__
    )

    # meta
    node_meta_parts: list[str] = [
        f"static metatype: NodeType = NodeType.{definition.type.name};",
    ]
    node_parts.append("\n".join(node_meta_parts))

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(node_cls):
        if prop.name == "id":
            continue  # ignore id for nodes (already defined in Node superclass)
        if is_parent_concrete and parent_cls is not None and prop.name in parent_cls.__properties__:
            continue  # parent has this property, don't declare again
        prop_str = _generate_property(
            prop,
            is_effective_readonly=definition.is_frozen or _is_property_effective_readonly(prop),
            is_tracked=_is_property_tracked(prop),
            is_node=True,
            is_interface=False,
            is_abstract=node_cls.__is_abstract__,
        )
        prop_parts.append(prop_str)
    node_parts.append("\n\n".join(prop_parts))

    # body
    if not node_cls.__is_abstract__:
        init_str = _generate_init(node_cls)
        node_parts.append(init_str)
        equals_str = _generate_equals(node_cls)
        node_parts.append(equals_str)
        hash_str = _generate_hash(node_cls)
        node_parts.append(hash_str)
        validate_str = _generate_validate(node_cls)
        node_parts.append(validate_str)
        to_ref_str = _generate_ref(node_cls)
        node_parts.append(to_ref_str)
        path_str = _generate_path(node_cls)
        node_parts.append(path_str)
        repr_str = _generate_repr(node_cls)
        node_parts.append(repr_str)
        value_str = generate_object_value(node_cls)
        node_parts.append(value_str)
        proto_str = generate_object_proto(node_cls)
        node_parts.append(proto_str)

    # class
    super_trait_classes = [
        super_cls
        for super_cls in node_cls.__bases__
        if super_cls != node_cls
        and issubclass(super_cls, Trait)
        and super_cls != Node
        and super_cls.__is_trait__
    ]
    implements_str = (
        " implements " + ", ".join(super_cls.__name__ for super_cls in super_trait_classes)
        if super_trait_classes
        else ""
    )

    base_type = definition.base_type
    base_cls_name = NODE_CLASS_BY_TYPE[base_type].__name__ if base_type else "Node"
    extends_str = f" extends {base_cls_name}"
    generic_str = ""
    node_str = f"""\
{_generate_multiline_doc(definition.description or definition.name)}
export {"abstract " if node_cls.__is_abstract__ else ""}class {definition.name}{generic_str}{extends_str}{implements_str} {{
{textwrap.indent("\n\n".join(node_parts), "  ")}

  {MARKER_CUSTOM_START}
  // ...
  {MARKER_CUSTOM_END}
}}
registerNodeClass(NodeType.{definition.type.name}, {definition.name});
"""
    return node_str.strip()


def _generate_constant(definition: ConstantDefinition) -> str:
    """Generate a Typescript Constant definition."""
    return f"""\
{_generate_multiline_doc(definition.description or definition.name)}
// prettier-ignore
export const {definition.name} = {generate_value(definition.value.type, definition.value.unpack())};
"""


def _get_type_dependencies(
    type: Type | TypeDeclaration | PropertyDeclaration | PropertyDefinition,
    is_abstract: bool,
    is_value: bool = False,
) -> tuple[dict[str, Definition], set[str]]:
    """Get the dependencies of a type."""
    dependencies: dict[str, Definition] = {}
    value_dependencies: set[str] = set()
    if type.scalar_type == ScalarType.NODE_REFERENCE:
        if isinstance(type, (TypeDeclaration, PropertyDeclaration)):
            for node_type in type.node_types or ():
                if isinstance(node_type, NodeType):
                    node_cls = NODE_CLASS_BY_TYPE[node_type]
                    dependencies[node_cls.__name__] = NODE_DEFINITION_BY_TYPE[node_type]
                elif isinstance(node_type, TraitType):
                    trait_cls = TRAIT_CLASS_BY_TYPE[node_type]
                    dependencies[trait_cls.__name__] = TRAIT_DEFINITION_BY_TYPE[node_type]
                else:
                    assert_never(node_type)
        else:
            if type.node_type is not None:
                node_cls = NODE_CLASS_BY_TYPE[type.node_type]
                dependencies[node_cls.__name__] = NODE_DEFINITION_BY_TYPE[type.node_type]
        if is_value:
            value_dependencies.add("NodeReference")
    elif type.scalar_type == ScalarType.STRUCT:
        assert type.struct_type is not None, f"no struct_type for {type!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[type.struct_type]
        dependencies[struct_cls.__name__] = STRUCT_DEFINITION_BY_TYPE[type.struct_type]
        if is_value:
            value_dependencies.add(struct_cls.__name__)
    elif type.scalar_type == ScalarType.ENUM:
        assert type.enum_type is not None, f"no enum_type for {type!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
        dependencies[enum_cls.__name__] = ENUM_DEFINITION_BY_TYPE[type.enum_type]
        if not is_abstract:
            value_dependencies.add(enum_cls.__name__)
    return dependencies, value_dependencies


def _get_builtin_object_dependencies(
    cls: type[BuiltinObject], is_abstract: bool
) -> tuple[dict[str, Definition], set[str]]:
    """Get the dependencies of a definition."""
    dependencies: dict[str, Definition] = {}
    value_dependencies: set[str] = set()

    # base classes
    if issubclass(cls, (Trait, Node)):
        for super_cls in cls.__bases__:
            if (
                super_cls != cls
                and issubclass(super_cls, Trait)
                and super_cls != Trait
                and super_cls != Node
            ):
                super_type = super_cls.metatype
                if isinstance(super_type, TraitType):
                    dependencies[super_cls.__name__] = TRAIT_DEFINITION_BY_TYPE[super_type]
                elif isinstance(super_type, NodeType):
                    dependencies[super_cls.__name__] = NODE_DEFINITION_BY_TYPE[super_type]
                    value_dependencies.add(super_cls.__name__)
                else:
                    assert_never(super_type)

    # properties
    for prop in _get_properties(cls):
        prop_dependencies, prop_value_dependencies = _get_type_dependencies(
            prop, is_abstract=is_abstract
        )
        dependencies.update(prop_dependencies)
        value_dependencies.update(prop_value_dependencies)

    return dependencies, value_dependencies


def _generate_definition(definition: Definition) -> TypescriptDefinition:
    """Generate a Typescript definition."""
    if isinstance(definition, EnumDefinition):
        kind = "ENUM"
        cls = ENUM_CLASS_BY_TYPE[definition.type]
        alias = cls.__name__
        module = cls.__module__
        definition_str = _generate_enum(definition)
        name = definition.name
        dependencies, value_dependencies = EMPTY_DICT, set()
    elif isinstance(definition, StructDefinition):
        kind = "STRUCT"
        cls = STRUCT_CLASS_BY_TYPE[definition.type]
        alias = cls.__name__
        module = cls.__module__
        definition_str = _generate_struct(definition)
        name = definition.name
        dependencies, value_dependencies = _get_builtin_object_dependencies(
            cast(type[BuiltinObject], cls), is_abstract=False
        )
    elif isinstance(definition, TraitDefinition):
        kind = "TRAIT"
        cls = TRAIT_CLASS_BY_TYPE[definition.type]
        alias = cls.__name__
        module = cls.__module__
        definition_str = _generate_trait(definition)
        name = definition.alias
        dependencies, value_dependencies = _get_builtin_object_dependencies(
            cast(type[BuiltinObject], cls), is_abstract=True
        )
    elif isinstance(definition, NodeDefinition):
        kind = "NODE"
        cls = NODE_CLASS_BY_TYPE[definition.type]
        alias = cls.__name__
        module = cls.__module__
        definition_str = _generate_node(definition)
        name = definition.name
        dependencies, value_dependencies = _get_builtin_object_dependencies(
            cast(type[BuiltinObject], cls), is_abstract=cls.__is_abstract__
        )
    elif isinstance(definition, ConstantDefinition):
        kind = "CONSTANT"
        alias = definition.name
        assert definition._declaration is not None, f"missing declaration for {definition.name}"
        module = definition._declaration.module
        definition_str = _generate_constant(definition)
        name = definition.name
        dependencies, value_dependencies = _get_type_dependencies(
            definition.value.type, is_abstract=False, is_value=True
        )
    else:
        assert_never(definition)

    submodule = ".".join(module.split(".")[2:])
    source_definition = TypescriptDefinition(
        name=name,
        alias=alias,
        module=module,
        submodule=submodule,
        kind=kind,
        id=definition.name if isinstance(definition, ConstantDefinition) else definition.id,
        definition=definition,
        definition_str=definition_str,
        dependencies=dependencies,
        value_dependencies=value_dependencies,
    )
    return source_definition


MARKER_START_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_START:([^:]+):([^=]+) ==== \*/")
MARKER_END_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_END:([^:]+):([^=]+) ==== \*/")
MARKER_CUSTOM_START_PATTERN = re.compile(r"/\* ==== DESTACK_CUSTOM_START ==== \*/")
MARKER_CUSTOM_END_PATTERN = re.compile(r"/\* ==== DESTACK_CUSTOM_END ==== \*/")

BUILTIN_NAMES = {
    "Node",
    "Struct",
    "Enum",
    "Trait",
}


def _generate_file(
    file: TypescriptFile, all_definitions_by_name: dict[str, TypescriptDefinition]
) -> str:
    """Generate the contents of a managed TypescriptFile, merging with existing contents if present."""

    # parse existing content into blocks
    existing_content = file.existing_str or ""
    lines = existing_content.split("\n")
    blocks: list[TypescriptDefinitionBlock | TypescriptCodeBlock | TypescriptImportBlock] = []
    char_pos = 0

    # imports
    existing_import_lines: list[str] = []
    existing_imports: list[TypescriptImport] = []

    import_line = 0
    while import_line < len(lines):
        line = lines[import_line]
        stripped = line.strip()

        # skip empty lines
        if not stripped:
            import_line += 1
            continue

        # stop if we hit a non-import line
        if not stripped.startswith("import "):
            break

        # collect all lines for this import (handle multi-line imports)
        import_lines = [line]
        current_import = stripped

        # if the line doesn't end with semicolon and contains opening brace without closing,
        # it's likely a multi-line import
        while (
            not current_import.endswith(";")
            or ("{" in current_import and "}" not in current_import)
        ) and import_line + 1 < len(lines):
            import_line += 1
            next_line = lines[import_line]
            import_lines.append(next_line)
            current_import += " " + next_line.strip()

        # add all lines for this import
        existing_import_lines.extend(import_lines)

        # parse the complete import statement
        is_type = "import type " in current_import

        # extract path
        path_match = re.search(r'from\s+["\']([^"\']+)["\']', current_import)
        if not path_match:
            import_line += 1
            continue
        path = path_match.group(1).replace("/", ".")

        # extract names
        names: list[str] = []
        if "{" in current_import and "}" in current_import:  # named imports
            names_match = re.search(r"\{\s*([^}]+)\s*\}", current_import, re.DOTALL)
            if names_match:
                names_str = names_match.group(1)
                # handle multi-line named imports by splitting on commas and cleaning whitespace
                names = [name.strip() for name in re.split(r",\s*", names_str) if name.strip()]
        elif " * as " in current_import:  # namespace import
            namespace_match = re.search(r"\*\s+as\s+(\w+)", current_import)
            if namespace_match:
                names = [namespace_match.group(1)]
        else:  # default import
            default_match = re.search(r"import\s+(?:type\s+)?(\w+)", current_import)
            if default_match:
                names = [default_match.group(1)]

        import_ = TypescriptImport(
            content="\n".join(import_lines), path=path, is_type=is_type, names=names
        )
        existing_imports.append(import_)
        import_line += 1

    char_pos = sum(len(line) + 1 for line in existing_import_lines)  # +1 for newline

    # add import block
    import_block = TypescriptImportBlock(
        content="\n".join(existing_import_lines), imports=existing_imports
    )
    blocks.append(import_block)

    # regular blocks
    while char_pos < len(existing_content):
        # look for next start marker
        start_match = MARKER_START_PATTERN.search(existing_content, char_pos)
        if start_match is None:
            # no more definition blocks, add remaining content as code block
            if char_pos < len(existing_content):
                remaining = existing_content[char_pos:].strip()
                if remaining:
                    blocks.append(TypescriptCodeBlock(content=remaining))
            break

        # add any content before the start marker as a code block
        if start_match.start() > char_pos:
            content = existing_content[char_pos : start_match.start()].strip()
            if content:
                blocks.append(TypescriptCodeBlock(content=content))

        # parse the definition block
        kind = start_match.group(1)
        assert kind in ("ENUM", "STRUCT", "TRAIT", "NODE", "CONSTANT"), (
            f"invalid kind: {kind} in {file.path}"
        )
        kind = cast(Kind, kind)
        id_str = start_match.group(2).strip()
        id = int(id_str) if kind != "CONSTANT" else id_str

        # find the corresponding end marker
        end_match = MARKER_END_PATTERN.search(existing_content, start_match.end())
        if end_match is None or end_match.group(1) != kind or end_match.group(2).strip() != id_str:
            raise ValueError(f"no matching end marker for {kind}:{id_str}")

        # parse custom content
        custom_content = ""
        custom_match = MARKER_CUSTOM_START_PATTERN.search(existing_content, start_match.end())
        if custom_match is not None and custom_match.start() < end_match.start():
            custom_start = custom_match.end()
            custom_end_match = MARKER_CUSTOM_END_PATTERN.search(existing_content, custom_start)
            if custom_end_match is None or custom_end_match.start() >= end_match.start():
                raise ValueError(
                    f"no matching {MARKER_CUSTOM_END} for {kind}:{id_str} in {file.path}"
                )
            custom_end = custom_end_match.start()
            custom_content = existing_content[custom_start:custom_end].strip()

        # add block
        block = TypescriptDefinitionBlock(kind=kind, id=id, custom_content=custom_content)
        blocks.append(block)
        char_pos = end_match.end()

    # build file by updating existing definition blocks and adding new ones
    file_parts: list[str] = []
    seen_definitions: set[tuple[Kind, int | str]] = set()
    for block in blocks:
        if isinstance(block, TypescriptCodeBlock):
            file_parts.append(block.content)
        elif isinstance(block, TypescriptDefinitionBlock):
            key = (block.kind, block.id)
            if key in seen_definitions:
                continue  # duplicate block (for some reason)
            definition = file.get_definition(block.kind, block.id)
            if definition is None:
                if block.custom_content and block.custom_content.count("\n") > 1:
                    raise RuntimeError(
                        f"stale definition {key} has custom content in {file.path}:\n{block.custom_content[:200]}"
                    )
                continue

            # merge custom content
            if block.custom_content:
                custom_start_match = MARKER_CUSTOM_START_PATTERN.search(definition.definition_str)
                if custom_start_match is None:
                    raise ValueError(
                        f"no {MARKER_CUSTOM_START} found in definition {definition.kind}:{definition.id}"
                    )
                custom_end_match = MARKER_CUSTOM_END_PATTERN.search(
                    definition.definition_str, custom_start_match.end()
                )
                if custom_end_match is None:
                    raise ValueError(
                        f"no {MARKER_CUSTOM_END} found in definition {definition.kind}:{definition.id}"
                    )
                # replace the custom region with block.custom_content
                before_custom = definition.definition_str[: custom_start_match.start()]
                after_custom = definition.definition_str[custom_end_match.end() :]
                if block.custom_content.count("\n") > 1:
                    custom_region = (
                        f"{MARKER_CUSTOM_START}\n\n{block.custom_content}\n\n{MARKER_CUSTOM_END}"
                    )
                else:
                    custom_region = (
                        f"{MARKER_CUSTOM_START}\n{block.custom_content}\n{MARKER_CUSTOM_END}"
                    )
                inner_str = before_custom + custom_region + after_custom
            else:
                inner_str = definition.definition_str

            new_block = f"""\
{MARKER_START.format(kind=definition.kind, id=definition.id)}
{inner_str}
{MARKER_END.format(kind=definition.kind, id=definition.id)}"""
            file_parts.append(new_block)
            seen_definitions.add(key)
        elif isinstance(block, TypescriptImportBlock):
            pass  # re-assembled below
        else:
            assert_never(block)

    # add any new definitions at the end
    for definition in file.definitions.values():
        key = (definition.kind, definition.id)
        if key not in seen_definitions:
            new_block = f"{MARKER_START.format(kind=definition.kind, id=definition.id)}\n"
            new_block += f"{definition.definition_str}\n"
            new_block += f"{MARKER_END.format(kind=definition.kind, id=definition.id)}"
            file_parts.append(new_block)

    # add imports to the top (will be auto-merged by linter)
    language_imports_by_module: dict[str, set[str]] = defaultdict(set)
    value_dependencies: set[str] = set()
    language_imports_by_module["core.runtime.graph"] = {
        "Graph",
        "Supergraph",
        "PolyGraph",
        "SingletonGraph",
    }
    value_dependencies.update(("PolyGraph", "SingletonGraph"))
    language_imports_by_module["core.runtime.session"] = {"Session"}
    language_imports_by_module["core.runtime.connection"] = {"QueryConnection"}
    language_imports_by_module["core.builtin.relation"] = {"NodeReference"}
    language_imports_by_module["core.builtin.common"] = {
        "NodeType",
        "TraitType",
        "StructType",
        "EnumType",
    }
    value_dependencies.update(language_imports_by_module["core.builtin.common"])
    language_imports_by_module["core.builtin.event"] = {
        "Event",
        "CustomEventDefinition",
        "CustomEvent",
    }
    value_dependencies.add("Event")
    language_imports_by_module["core.builtin.entity"] = {
        "Entity",
        "Resource",
        "Metric",
        "CustomEntityDefinition",
        "CustomEntity",
        "CustomTraitDefinition",
    }
    value_dependencies.update(("Entity", "Resource", "Metric"))
    language_imports_by_module["core.builtin.const"] = {
        "ACTIVE_SESSION",
        "activeSession",
        "ACTIVE_SPACE",
        "activeSpace",
    }
    value_dependencies.update(("ACTIVE_SESSION", "activeSession", "ACTIVE_SPACE", "activeSpace"))
    language_imports_by_module["core.builtin.node"] = {"Node", "NodeClass", "isNode", "hasTrait"}
    value_dependencies.update(("Node", "isNode", "hasTrait"))
    language_imports_by_module["core.builtin.trait"] = {"TraitClass"}
    language_imports_by_module["core.builtin.object"] = {"BuiltinObject"}
    value_dependencies.add("BuiltinObject")
    language_imports_by_module["core.builtin.struct"] = {"Struct", "StructFrozen", "isStruct"}
    value_dependencies.update(("Struct", "StructFrozen", "isStruct"))
    language_imports_by_module["registry"] = {
        "registerNodeClass",
        "registerStructClass",
        "registerEnumClass",
        "registerTraitClass",
        "STRUCT_CLASS_BY_TYPE",
        "NODE_CLASS_BY_TYPE",
        "TRAIT_CLASS_BY_TYPE",
        "ENUM_CLASS_BY_TYPE",
    }
    value_dependencies.update(language_imports_by_module["registry"])
    seen_language_imports: set[str] = {*language_imports_by_module["core"]}

    # add any new language imports
    for definition in file.definitions.values():
        for dependency_name in definition.dependencies:
            if dependency_name not in all_definitions_by_name:
                if dependency_name in BUILTIN_NAMES:
                    continue  # manually defined (above)
                raise RuntimeError(f"missing dependency: {dependency_name}")
            dependency_definition = all_definitions_by_name[dependency_name]
            language_imports_by_module[dependency_definition.submodule].add(
                dependency_definition.alias
            )
            seen_language_imports.add(dependency_definition.alias)
            value_dependencies.update(definition.value_dependencies)

    # add any previous language imports
    for import_ in import_block.imports:
        if not import_.path.startswith("@destack/language"):
            continue  # leave be
        for name in import_.names:
            if not import_.is_type and name not in value_dependencies:
                value_dependencies.add(name)
            if name in seen_language_imports:
                continue
            seen_language_imports.add(name)
            definition = all_definitions_by_name.get(name)
            if definition is None:
                language_imports_by_module[""].add(name)
            else:
                language_imports_by_module[definition.submodule].add(name)
    # remove any imports that are already defined in this file
    for _, imports in language_imports_by_module.items():
        imports.difference_update(definition.name for definition in file.definitions.values())

    # special cases
    if file.name.endswith(".trait"):  # defined manually in same file
        language_imports_by_module["core.builtin.trait"].discard("TraitClass")
    if file.name.endswith(".query"):  # needs QueryConnection
        value_dependencies.add("QueryConnection")

    # if we're in the same module, use granular imports, otherwise use top-level imports
    file_root_module = ".".join(file.module.split(".")[2:3])
    combined_language_imports_by_module: dict[str, set[str]] = defaultdict(set)
    for module, imports in language_imports_by_module.items():
        if not imports:
            continue
        root_module = ".".join(module.split(".")[:1])
        if root_module == file_root_module:
            combined_language_imports_by_module[module].update(imports)
        else:
            combined_language_imports_by_module[root_module].update(imports)
    language_imports_by_module = combined_language_imports_by_module

    # generate import statements
    import_parts: list[str] = []
    for module, imports in language_imports_by_module.items():
        if not imports:
            continue
        if module.startswith("core") and not file.module.startswith("destack.language.core"):
            module = "core"  # simplify imports from outside core to just "core"
        import_path = (
            f"@destack/language/{module.replace('.', '/')}" if module else "@destack/language"
        )
        # split into type and non-type imports
        type_imports = sorted(imports - value_dependencies)
        value_imports = sorted(imports & value_dependencies)
        if type_imports:
            import_parts.append(
                f"import type {{ {', '.join(type_imports)} }} from '{import_path}';"
            )
        if value_imports:
            import_parts.append(f"import {{ {', '.join(value_imports)} }} from '{import_path}';")

    # add proto imports for all definitions and dependencies
    proto_names = {
        f"{definition.alias}Proto"
        for definition in chain(file.definitions.values(), file.dependencies.values())
    }
    import_parts.append(f"import {{ {', '.join(sorted(proto_names))} }} from '@destack/proto';")
    import_parts.append(
        "import { packProtoDuration, packProtoTimestamp, packProtoJson, unpackProtoDuration, unpackProtoTimestamp, unpackProtoJson } from '@destack/grpc';"
    )
    import_parts.append(
        "import { timedeltaToISOFormat, timedeltaFromISOFormat, base64Encode, base64Decode } from '@destack/utils';"
    )
    import_parts.append("import type { IMessageType } from '@protobuf-ts/runtime';")
    import_parts.append("import { Temporal } from 'temporal-polyfill';")
    import_parts.append("import { v4 as uuid4 } from 'uuid';")
    import_parts.append("import { uuid7 } from '@destack/utils/uuid';")
    import_parts.append(
        "import { hashString, hashBytes, hashInt, hashFloat, hashBool } from '@destack/utils/hash';"
    )
    for import_ in import_block.imports:
        if not import_.path.startswith("@destack/language"):
            import_parts.append(import_.content)
    file_parts.insert(0, "\n".join(import_parts))

    # assemble and clean up
    new_str = "\n\n".join(file_parts)
    new_str = new_str.replace(" | null | null", " | null")

    return new_str


def _generate_global(definitions_by_name: dict[str, TypescriptDefinition]) -> str:
    """Generate the global mapping file."""

    # mappings
    mapping_str_parts: list[str] = []

    # group imports by submodule
    mapping_import_parts: list[str] = []
    imports_by_module: dict[str, set[str]] = defaultdict(set)
    imports_by_module["core"].add("Node")
    imports_by_module["core"].add("Struct")
    for definition in definitions_by_name.values():
        imports_by_module[definition.submodule].add(definition.alias)

    # mapping imports - add all as type imports
    for module, imports in imports_by_module.items():
        if not imports:
            continue
        import_path = (
            f"@destack/language/{module.replace('.', '/')}" if module else "@destack/language"
        )
        mapping_import_parts.append(
            f"import type {{ {', '.join(sorted(imports))} }} from '{import_path}';"
        )

    mapping_str_parts.append("\n".join(mapping_import_parts))

    # mapping types
    node_map_str_parts: list[str] = ["export type NodeTypeMapping = {"]
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        node_map_str_parts.append(f"  [NodeType.{node_type.name}]: {node_cls.__name__};")
    node_map_str_parts.append("};")
    node_map_str = "\n".join(node_map_str_parts)
    mapping_str_parts.append(node_map_str)

    trait_map_str_parts: list[str] = ["export type TraitTypeMapping = {"]
    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        trait_map_str_parts.append(f"  [TraitType.{trait_type.name}]: {trait_cls.__name__};")
    trait_map_str_parts.append("};")
    trait_map_str = "\n".join(trait_map_str_parts)
    mapping_str_parts.append(trait_map_str)

    struct_map_str_parts: list[str] = ["export type StructTypeMapping = {"]
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        struct_map_str_parts.append(f"  [StructType.{struct_type.name}]: {struct_cls.__name__};")
    struct_map_str_parts.append("};")
    struct_map_str = "\n".join(struct_map_str_parts)
    mapping_str_parts.append(struct_map_str)

    enum_map_str_parts: list[str] = ["export type EnumTypeMapping = {"]
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        enum_map_str_parts.append(f"  [EnumType.{enum_type.name}]: {enum_cls.__name__};")
    enum_map_str_parts.append("};")
    enum_map_str = "\n".join(enum_map_str_parts)
    mapping_str_parts.append(enum_map_str)

    mapping_str = "\n\n".join(mapping_str_parts)

    return mapping_str


def generate():
    """Generate the Typescript language code."""

    # collect definitions
    definitions_by_module: dict[str, list[TypescriptDefinition]] = defaultdict(list)
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        definition = _generate_definition(ENUM_DEFINITION_BY_TYPE[enum_type])
        definitions_by_module[enum_cls.__module__].append(definition)
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        if struct_type == StructType.STRUCT:
            continue  # manually defined
        definition = _generate_definition(STRUCT_DEFINITION_BY_TYPE[struct_type])
        definitions_by_module[struct_cls.__module__].append(definition)
    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        definition = _generate_definition(TRAIT_DEFINITION_BY_TYPE[trait_type])
        definitions_by_module[trait_cls.__module__].append(definition)
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        if node_type == NodeType.NODE:
            continue  # manually defined
        definition = _generate_definition(NODE_DEFINITION_BY_TYPE[node_type])
        definitions_by_module[node_cls.__module__].append(definition)
    for constant_definition in CONSTANT_DEFINITIONS.values():
        definition = _generate_definition(constant_definition)
        assert constant_definition._declaration is not None, (
            f"missing declaration for {constant_definition.name}"
        )
        if constant_definition.is_deferred:
            # put deferred constants in separate top-level file (to avoid circular dependencies)
            definitions_by_module["constants"].append(definition)
        else:
            definitions_by_module[constant_definition._declaration.module].append(definition)
    definitions_by_name: dict[str, TypescriptDefinition] = {}
    for _, definitions in definitions_by_module.items():
        for definition in definitions:
            existing_definition = definitions_by_name.get(definition.name)
            if existing_definition is not None:
                raise RuntimeError(
                    f"duplicate definition name: {definition.name} "
                    f"({existing_definition.module}.{existing_definition.alias} vs "
                    f"{definition.module}.{definition.alias})"
                )
            definitions_by_name[definition.name] = definition

    # generate files
    files_by_module: dict[str, TypescriptFile] = {}
    for module, definitions in definitions_by_module.items():
        # path
        target_path = Path(GENERATION_PATH) / (module.split(".", 2)[-1].replace(".", "/") + ".ts")
        existing_file_str = target_path.read_text() if target_path.exists() else None

        # definitions
        file_definitions_by_name: dict[str, TypescriptDefinition] = {
            definition.definition.name: definition for definition in definitions
        }
        file_dependencies_by_name: dict[str, TypescriptDefinition] = {}
        for definition in definitions:
            for dependency_name in definition.dependencies:
                if dependency_name not in definitions_by_name:
                    if dependency_name in BUILTIN_NAMES:
                        continue  # manually defined
                    raise RuntimeError(f"missing dependency: {dependency_name}")
                file_dependencies_by_name[dependency_name] = definitions_by_name[dependency_name]

        # file
        file = TypescriptFile(
            name=module,
            module=module,
            path=target_path,
            definitions=file_definitions_by_name,
            dependencies=file_dependencies_by_name,
            existing_str=existing_file_str,
        )
        file.new_str = _generate_file(file, definitions_by_name)
        files_by_module[module] = file

    # write files
    for file in files_by_module.values():
        assert file.new_str, f"empty {file.path}"
        file.path.parent.mkdir(parents=True, exist_ok=True)
        file.path.write_text(file.new_str)

    # update mapping files
    mapping_path = Path(GENERATION_PATH) / "mapping.ts"
    mapping_str = _generate_global(definitions_by_name)
    mapping_path.write_text(mapping_str)

    # write index files
    module_paths = list({file.path.parent for file in files_by_module.values()})
    module_paths.append(Path(GENERATION_PATH))
    for module_path in sorted(module_paths):
        # generate index.ts file that re-exports every subfile/subfolder
        index_path = module_path / "index.ts"
        index_path.parent.mkdir(parents=True, exist_ok=True)
        index_lines = []
        # collect all .ts files in this directory (excluding index.ts itself)
        ts_files = [f for f in module_path.glob("*.ts") if f.name != "index.ts"]
        for ts_file in sorted(ts_files):
            relative_path = ts_file.relative_to(GENERATION_PATH).with_suffix("")
            index_lines.append(f"export * from '@destack/language/{relative_path}';")
        # collect all subdirectories that contain .ts files
        for subdir in sorted(module_path.iterdir()):
            if subdir.is_dir() and any(subdir.rglob("*.ts")):
                relative_path = subdir.relative_to(GENERATION_PATH).with_suffix("")
                index_lines.append(f"export * from '@destack/language/{relative_path}';")

        index_content = "\n".join(index_lines) + "\n"
        index_path.write_text(index_content)

    # append finalize call to root index.ts
    root_index_path = Path(GENERATION_PATH) / "index.ts"
    root_index_content = (
        root_index_path.read_text()
        + """
import { finalize } from "@destack/language/finalize";
finalize();
"""
    )
    root_index_path.write_text(root_index_content)

    # format it all
    subprocess.run("cd destack-ts && bun run format-language", shell=True, check=True)
