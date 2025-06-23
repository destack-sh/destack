import re
import subprocess
import textwrap
from collections import defaultdict
from pathlib import Path
from typing import Any, assert_never, cast

from destack.language import (
    EMPTY_DICT,
    NODE_TYPES,
    UNSET,
    BuiltinObjectBase,
    Enum,
    EnumDefinition,
    IntoType,
    Node,
    NodeBase,
    NodeDefinition,
    NodeType,
    PrimitiveType,
    Property,
    RoleType,
    ScalarType,
    StructDefinition,
    Trait,
    TraitDefinition,
    TraitType,
    TypeCardinality,
    get_node_types,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    ENUM_DEFINITION_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    NODE_DEFINITION_BY_TYPE,
    NODE_TYPES_BY_TRAIT_TYPE,
    STRUCT_CLASS_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
    TRAIT_CLASS_BY_TYPE,
    TRAIT_DEFINITION_BY_TYPE,
    TRAIT_TYPE_BY_CLASS,
)
from destack.utils.string import Casing, to_casing

from .const import GENERATION_PATH, MARKER_CUSTOM_START, MARKER_END, MARKER_START, Definition, Kind
from .core import (
    TypescriptCodeBlock,
    TypescriptDefinition,
    TypescriptDefinitionBlock,
    TypescriptFile,
    TypescriptImport,
    TypescriptImportBlock,
)
from .map import TYPESCRIPT_TYPE_BY_PRIMITIVE_TYPE


def _get_properties(cls: type[BuiltinObjectBase]) -> list[Property]:
    """Get the properties of a class."""
    properties: list[Property] = []
    for prop in cls.__wired_properties__.values():
        if prop.id == 1:
            continue
        if prop.runtime_prop is not None:
            prop = prop.runtime_prop
        properties.append(prop)
    properties.sort(key=lambda prop: prop.id or 0)
    return properties


def _is_property_readonly(prop: Property) -> bool:
    """Check if a property is (effectively) readonly to the user."""
    return prop.can_write is None or prop.can_write == RoleType.SYSTEM or prop.is_managed


def _generate_property_scalar_type(prop: IntoType, as_ptr: bool = True) -> str:
    """Generate a scalar property Typescript type annotation."""
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        if as_ptr:
            return "NodeReference"
        else:
            node_types = get_node_types(prop.node_types)
            node_classes = tuple(NODE_CLASS_BY_TYPE[node_type] for node_type in node_types or ())
            if node_classes and len(node_classes) < len(NODE_TYPES):
                return " | ".join(node_cls.__name__ for node_cls in node_classes)
            else:
                return "Node"
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


def _generate_property_type(prop: Property, as_ptr: bool = True) -> str:
    """Generate a property Typescript type annotation."""
    type_str = _generate_property_scalar_type(prop, as_ptr=as_ptr)
    if prop.cardinality == TypeCardinality.SCALAR:
        if prop.is_optional:
            type_str = f"{type_str} | null"
    elif prop.cardinality == TypeCardinality.LIST:
        type_str = f"Array<{type_str}>"
    elif prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"no key_type for {prop!r}"
        key_type_str = _generate_property_scalar_type(prop.key_type)
        type_str = f"Map<{key_type_str}, {type_str}>"
    else:
        assert_never(prop.cardinality)
    return type_str


def _generate_value(type: IntoType, value: Any) -> str:
    """Generate a Typescript value literal."""
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
        else:
            raise ValueError(f"unsupported primitive type: {type.primitive_type!r}")
    elif type.scalar_type == ScalarType.ENUM:
        if type.cardinality == TypeCardinality.SCALAR:
            assert type.enum_type is not None, f"no enum_type for {type!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[type.enum_type]
            return f"{enum_cls.__name__}.{value.name}"
        else:
            raise ValueError(f"unsupported enum cardinality: {type.cardinality!r}")
    else:
        raise ValueError(f"unsupported value type: {type.scalar_type!r}")


def _generate_property(
    prop: Property, *, is_readonly: bool, is_node: bool, is_interface: bool
) -> str:
    """Generate a Property definition."""

    ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)

    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        # special case node references: custom getters/setters
        assert prop.cardinality == TypeCardinality.SCALAR, (
            f"node references must be scalar: {prop!r}"
        )
        ptr_type_str = _generate_property_type(prop, as_ptr=True)
        ptr_prop_name = f"{ts_name}Ptr"
        node_type_str = _generate_property_scalar_type(prop, as_ptr=False)
        if prop.is_optional:
            node_type_str = f"{node_type_str} | null"

        # node getter/setter
        if is_interface:
            # interface declaration
            node_getter_str = f"get {ts_name}(): {node_type_str}"
            if not is_readonly:
                node_setter_str = f"set {ts_name}(value: {node_type_str})"
                node_prop_str = f"{node_getter_str}\n{node_setter_str}"
            else:
                node_prop_str = node_getter_str
        else:
            # actual getter/setter
            if is_node:
                node_getter_str = f"""\
get {ts_name}(): {node_type_str} | null {{
    const nodePtr: NodeReference | null = this.{ptr_prop_name};
    if (nodePtr !== null) {{
        return this._supergraph.get(nodePtr.id) as {node_type_str} | null;
    }}
    return null;
}}
"""
                if not is_readonly:
                    if prop.is_optional:
                        node_setter_str = f"""\
set {ts_name}(node: {node_type_str}) {{
    if (node === null) {{
        this.{ptr_prop_name} = null;
    }} else {{
        this.{ptr_prop_name} = node.toRef();
    }}
}}
"""
                    else:
                        node_setter_str = f"""\
set {ts_name}(node: {node_type_str}) {{
    this.{ptr_prop_name} = node.toRef();
}}
"""
                    node_prop_str = f"{node_getter_str}\n{node_setter_str}"
                else:
                    node_prop_str = node_getter_str

            else:
                node_getter_str = f"""\
get {ts_name}(): {node_type_str} | null {{
    const nodePtr: NodeReference | null = this.{ptr_prop_name};
    if (nodePtr !== null) {{
        if (this._supergraph === null) {{
            return null;
        }}
        return this._supergraph.get(nodePtr.id) as {node_type_str};
    }}
    return null;
}}
"""
                if not is_readonly:
                    node_setter_str = f"""\
set {ts_name}(value: {node_type_str}) {{
    if (value == null) {{
        this.{ptr_prop_name} = null;
    }} else {{
        this.{ptr_prop_name} = value.toRef();
    }}
}}
"""
                    node_prop_str = f"{node_getter_str}\n{node_setter_str}"
                else:
                    node_prop_str = node_getter_str

        ptr_prop_str = f"{ptr_prop_name}: {ptr_type_str}"
        if is_readonly:
            ptr_prop_str = f"readonly {ptr_prop_str}"
        return f"{node_prop_str};\n{ptr_prop_str}"

    else:
        type_str = _generate_property_type(prop)
        node_prop_str = f"{ts_name}: {type_str}"
        if is_readonly:
            node_prop_str = f"readonly {node_prop_str}"
        if node_prop_str.count("\n") < 1:
            node_prop_str = f"{node_prop_str};"
        return node_prop_str


def _generate_init(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript constructor with options-style parameters."""

    properties = {p.name: p for p in _get_properties(cls)}

    # constructor header parts
    constructor_header_parts: list[str] = []
    constructor_body_parts: list[str] = []
    constructor_assignment_parts: list[str] = []

    # constructor main properties
    for prop in properties.values():
        if prop.is_computed:
            continue  # computed, can't assign

        ts_name = to_casing(prop.name, Casing.LOWER_CAMEL)
        is_required = (
            prop.is_required
            and prop.default is UNSET
            and prop.default_factory is None
            and prop.cardinality == TypeCardinality.SCALAR
        )

        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # can be passed either as Node or NodeReference
            node_type_str = _generate_property_scalar_type(prop, as_ptr=False)
            ptr_type_str = _generate_property_scalar_type(prop, as_ptr=True)
            type_str = f"{node_type_str} | {ptr_type_str}"
            if prop.is_optional:
                type_str = f"{type_str} | null"

            # use ptr_prop for assignment
            if prop.ptr_prop is not None:
                ptr_prop_name = to_casing(prop.ptr_prop.name, Casing.LOWER_CAMEL)
                constructor_assignment_parts.append(
                    f"this.{ptr_prop_name} = options.{ts_name} != null ? (options.{ts_name}.metatype == StructType.NODE_REFERENCE ? (options.{ts_name} as NodeReference) : (options.{ts_name} as Node).toRef()) : null;"
                )
        else:
            # can be passed as value
            type_str = _generate_property_type(prop)
            if prop.default is not UNSET and prop.default is not None:
                constructor_assignment_parts.append(
                    f"this.{ts_name} = options.{ts_name} ?? {_generate_value(prop, prop.default)};"
                )
            elif prop.default_factory is not None:
                constructor_assignment_parts.append(
                    f"this.{ts_name} = options.{ts_name};"
                )  # nocheckin: Typescript default factory
            elif prop.cardinality == TypeCardinality.LIST:
                constructor_assignment_parts.append(f"this.{ts_name} = options.{ts_name} ?? [];")
            elif prop.cardinality == TypeCardinality.MAP:
                constructor_assignment_parts.append(
                    f"this.{ts_name} = options.{ts_name} ?? new Map();"
                )
            elif not is_required:
                constructor_assignment_parts.append(f"this.{ts_name} = options.{ts_name} ?? null;")
            else:
                constructor_assignment_parts.append(f"this.{ts_name} = options.{ts_name};")

        if is_required:
            constructor_header_parts.append(f"{ts_name}: {type_str}")
        else:
            constructor_header_parts.append(f"{ts_name}?: {type_str}")

    # constructor super call
    if issubclass(cls, Node):
        constructor_body_parts.append("""\
super(
    // id
    options.id,
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
    // is_attached
    options.id != null,
);
""")
    else:
        constructor_body_parts.append("""\
super(
    // session
    options._session ?? null,
    // supergraph
    options._supergraph ?? null,
);
""")

    constructor_header_parts.append("_session?: Session | null")
    constructor_header_parts.append("_supergraph?: Supergraph | null")
    if issubclass(cls, Node):
        constructor_header_parts.append("_graph?: Graph | null")
        constructor_header_parts.append("_connection?: QueryConnection | null")

    # assemble constructor
    constructor_header_str = ",\n".join(constructor_header_parts)
    constructor_body_str = "\n".join(constructor_body_parts)
    constructor_assignment_str = "\n".join(constructor_assignment_parts)
    constructor_str = f"""\
constructor(options: {{
{textwrap.indent(constructor_header_str, "  ")}
}}) {{
{textwrap.indent(constructor_body_str, "  ")}
{textwrap.indent(constructor_assignment_str, "  ")}
}}
"""

    return constructor_str.strip()


def _generate_equals(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript equals method."""
    equals_str = """\
equals(other: any): boolean {
  throw new Error("not implemented");
}
"""
    return equals_str.strip()


def _generate_hash(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript hash method."""
    hash_str = """\
hash(): number {
  throw new Error("not implemented");
}
"""
    return hash_str.strip()


def _generate_validate(cls: type[BuiltinObjectBase]) -> str:
    """Generate a Typescript validate method."""
    validate_str = """\
validate(): void {
  throw new Error("not implemented");
}
"""
    return validate_str.strip()


def _generate_to_ref(cls: type["Node"]) -> str:
    """Generate a Typescript toRef method."""

    node_type = cls.metatype
    if node_type == NodeType.SPACE:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference({{
    nodeType: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.id,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    elif TraitType.CUSTOM_NODE in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference({{
    nodeType: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.spacePtr?.id ?? null,
    definitionId: this.definitionPtr?.id ?? null,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    elif TraitType.SPATIAL in cls.__traits__:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference({{
    nodeType: NodeType.{node_type.name},
    id: this.id,
    spaceId: this.spacePtr?.id ?? null,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    else:
        ref_impl = f"""\
__toRef__(): NodeReference {{
  return new NodeReference({{
    nodeType: NodeType.{node_type.name},
    id: this.id,
    _session: this._session,
    _supergraph: this._supergraph,
  }});
}}
"""
    return ref_impl.strip()


def _generate_path(cls: type[Node]) -> str:
    """Generate a Typescript path method."""
    # Node._path_key
    if "slug" in cls.__properties__:
        if "name" in cls.__properties__:
            path_key_str = "this.slug ?? this.name"
        else:
            path_key_str = f'this.slug ?? "{cls.__name__}[id={{this.id}}]"'
    elif "name" in cls.__properties__:
        path_key_str = "this.name"
    elif "title" in cls.__properties__:
        path_key_str = "this.title"
    else:
        path_key_str = f'"{cls.__name__}[id={{this.id}}]"'

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
    while (node !== null) {{
        pathParts.push(node._pathKey);
        node = node.parent;
    }}
    if (!this._isAttached) {{
        pathParts.push("<detached>");
    }}
    return pathParts.reverse().join("/");
}}
"""

    return path_str.strip()


def _generate_enum(definition: EnumDefinition) -> str:
    """Generate a Typescript Enum definition."""
    enum_parts: list[str] = []
    for option in definition.options:
        enum_parts.append(f"{option.name} = {option.id},")
    enum_str = f"""\
export enum {definition.name} {{
{textwrap.indent("\n".join(enum_parts), "  ")}
}}
"""
    return enum_str.strip()


def _generate_struct(definition: StructDefinition) -> str:
    """Generate a Typescript Struct definition."""
    struct_cls = STRUCT_CLASS_BY_TYPE[definition.type]
    struct_parts: list[str] = []

    # meta
    struct_meta_parts: list[str] = [
        f"static metatype: StructType = StructType.{definition.type.name};",
        f"static __isFrozen__: boolean = {'true' if definition.is_frozen else 'false'};",
    ]
    struct_parts.append("\n".join(struct_meta_parts))

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(struct_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=definition.is_frozen,
            is_node=False,
            is_interface=False,
        )
        prop_parts.append(prop_str)
    struct_parts.append("\n".join(prop_parts))

    # body
    init_str = _generate_init(struct_cls)
    struct_parts.append(init_str)
    equals_str = _generate_equals(struct_cls)
    struct_parts.append(equals_str)
    hash_str = _generate_hash(struct_cls)
    struct_parts.append(hash_str)
    validate_str = _generate_validate(struct_cls)
    struct_parts.append(validate_str)

    struct_str = f"""\
export class {definition.name} extends {"StructFrozen" if definition.is_frozen else "Struct"} {{
{textwrap.indent("\n\n".join(struct_parts), "  ")}
}}"""
    return struct_str.strip()


def _generate_trait(definition: TraitDefinition) -> str:
    """Generate a Typescript Trait definition."""

    trait_cls = TRAIT_CLASS_BY_TYPE[definition.type]
    trait_parts: list[str] = []

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(trait_cls):
        if prop.name == "parent":
            continue  # ignore parent for traits
        prop_str = _generate_property(
            prop,
            is_readonly=_is_property_readonly(prop),
            is_node=False,
            is_interface=True,
        )
        prop_parts.append(prop_str)
    trait_parts.append("\n".join(prop_parts))

    # interface
    trait_classes: list[type[NodeBase]] = [
        TRAIT_CLASS_BY_TYPE[trait_type] for trait_type in definition.traits
    ]
    trait_classes.sort(key=lambda cls: TRAIT_TYPE_BY_CLASS[cast(type[Trait], cls)])
    implements_str = (
        " implements " + ", ".join(trait_cls.__name__ for trait_cls in trait_classes)
        if trait_classes
        else ""
    )
    trait_str = f"""\
export interface {definition.alias}{implements_str} {{
{textwrap.indent("\n".join(trait_parts), "  ")}
}}"""
    return trait_str.strip()


def _generate_node(definition: NodeDefinition) -> str:
    """Generate a Typescript Node definition."""

    node_cls = NODE_CLASS_BY_TYPE[definition.type]
    node_parts: list[str] = []

    # meta
    node_meta_parts: list[str] = [
        f"static metatype: NodeType = NodeType.{definition.type.name};",
        f"static __traits__: TraitType[] = [{', '.join(f'TraitType.{trait_type.name}' for trait_type in definition.traits)}];",
        f"static __rootType__: NodeType | null = {f'NodeType.{definition.root_type.name}' if definition.root_type else 'null'};",
        f"static __parentTypes__: NodeType[] = [{', '.join(f'NodeType.{parent_type.name}' for parent_type in definition.parent_types)}];",
        f"static __childTypes__: NodeType[] = [{', '.join(f'NodeType.{child_type.name}' for child_type in definition.child_types)}];",
        f"static __ancestorTypes__: NodeType[] = [{', '.join(f'NodeType.{ancestor_type.name}' for ancestor_type in definition.ancestor_types)}];",
        f"static __descendantTypes__: NodeType[] = [{', '.join(f'NodeType.{descendant_type.name}' for descendant_type in definition.descendant_types)}];",
    ]
    node_parts.append("\n".join(node_meta_parts))

    # properties
    prop_parts: list[str] = []
    for prop in _get_properties(node_cls):
        prop_str = _generate_property(
            prop,
            is_readonly=_is_property_readonly(prop),
            is_node=True,
            is_interface=False,
        )
        prop_parts.extend(prop_str.splitlines())
    node_parts.append("\n".join(prop_parts))

    # body
    init_str = _generate_init(node_cls)
    node_parts.append(init_str)
    equals_str = _generate_equals(node_cls)
    node_parts.append(equals_str)
    hash_str = _generate_hash(node_cls)
    node_parts.append(hash_str)
    validate_str = _generate_validate(node_cls)
    node_parts.append(validate_str)
    to_ref_str = _generate_to_ref(node_cls)
    node_parts.append(to_ref_str)
    path_str = _generate_path(node_cls)
    node_parts.append(path_str)

    # class
    trait_classes: list[type[NodeBase]] = [
        TRAIT_CLASS_BY_TYPE[trait_type] for trait_type in definition.traits
    ]
    trait_classes.sort(key=lambda cls: TRAIT_TYPE_BY_CLASS[cast(type[Trait], cls)])
    implements_str = (
        " implements " + ", ".join(trait_cls.__name__ for trait_cls in trait_classes)
        if trait_classes
        else ""
    )
    node_str = f"""\
export class {definition.name} extends Node{implements_str} {{
{textwrap.indent("\n\n".join(node_parts), "  ")}
}}
"""
    return node_str.strip()


def _get_definition_dependencies(cls: type[BuiltinObjectBase]) -> dict[str, Definition]:
    """Get the dependencies of a definition."""
    dependencies: dict[str, Definition] = {}

    # base classes
    if issubclass(cls, (Trait, NodeBase)):
        for trait_type in cls.__traits__:
            trait_cls = TRAIT_CLASS_BY_TYPE[trait_type]
            dependencies[trait_cls.__name__] = TRAIT_DEFINITION_BY_TYPE[trait_type]

    # properties
    for prop in _get_properties(cls):
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            node_types = get_node_types(prop.node_types)
            for node_type in node_types or ():
                node_cls = NODE_CLASS_BY_TYPE[node_type]
                dependencies[node_cls.__name__] = NODE_DEFINITION_BY_TYPE[node_type]
        elif prop.scalar_type == ScalarType.STRUCT:
            assert prop.struct_type is not None, f"no struct_type for {prop!r}"
            struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
            dependencies[struct_cls.__name__] = STRUCT_DEFINITION_BY_TYPE[prop.struct_type]
        elif prop.scalar_type == ScalarType.ENUM:
            assert prop.enum_type is not None, f"no enum_type for {prop!r}"
            enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
            dependencies[enum_cls.__name__] = ENUM_DEFINITION_BY_TYPE[prop.enum_type]

    return dependencies


def _generate_definition(definition: Definition) -> TypescriptDefinition:
    """Generate a Typescript definition."""
    if isinstance(definition, EnumDefinition):
        kind = "ENUM"
        cls = ENUM_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_enum(definition)
        name = definition.name
        dependencies = EMPTY_DICT
    elif isinstance(definition, StructDefinition):
        kind = "STRUCT"
        cls = STRUCT_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_struct(definition)
        name = definition.name
        dependencies = _get_definition_dependencies(cast(type[BuiltinObjectBase], cls))
    elif isinstance(definition, TraitDefinition):
        kind = "TRAIT"
        cls = TRAIT_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_trait(definition)
        name = definition.alias
        dependencies = _get_definition_dependencies(cast(type[BuiltinObjectBase], cls))
    elif isinstance(definition, NodeDefinition):
        kind = "NODE"
        cls = NODE_CLASS_BY_TYPE[definition.type]
        definition_str = _generate_node(definition)
        name = definition.name
        dependencies = _get_definition_dependencies(cast(type[BuiltinObjectBase], cls))
    else:
        assert_never(definition)

    source_definition = TypescriptDefinition(
        name=name,
        cls=cls,
        module=cls.__module__,
        kind=kind,
        id=definition.id,
        definition=definition,
        definition_str=definition_str,
        dependencies=dependencies,
    )
    return source_definition


MARKER_START_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_START:([^:]+):([^=]+) ==== \*/")
MARKER_CUSTOM_START_PATTERN = re.compile(r"/\* ==== DESTACK_CUSTOM_START ==== \*/")
MARKER_END_PATTERN = re.compile(r"/\* ==== DESTACK_GENERATED_END:([^:]+):([^=]+) ==== \*/")


def _generate_file(file: TypescriptFile) -> str:
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
        path = path_match.group(1)

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
        assert kind in ("ENUM", "STRUCT", "TRAIT", "NODE"), f"invalid kind: {kind} in {file.path}"
        kind = cast(Kind, kind)
        id_str = start_match.group(2).strip()
        id = int(id_str)

        # find the corresponding end marker
        end_match = MARKER_END_PATTERN.search(existing_content, start_match.end())
        if end_match is None or end_match.group(1) != kind or end_match.group(2).strip() != id_str:
            raise ValueError(f"no matching end marker for {kind}:{id_str}")

        # look for custom content within the definition block
        custom_content = ""
        custom_match = MARKER_CUSTOM_START_PATTERN.search(existing_content, start_match.end())
        if custom_match is not None and custom_match.start() < end_match.start():
            custom_start = custom_match.end()
            custom_end = end_match.start()
            custom_content = existing_content[custom_start:custom_end].strip()

        # add block
        block = TypescriptDefinitionBlock(kind=kind, id=id, custom_content=custom_content)
        blocks.append(block)
        char_pos = end_match.end()

    # build file by updating existing definition blocks and adding new ones
    file_parts: list[str] = []
    seen_definitions: set[tuple[Kind, int]] = set()
    for block in blocks:
        if isinstance(block, TypescriptCodeBlock):
            file_parts.append(block.content)
        elif isinstance(block, TypescriptDefinitionBlock):
            key = (block.kind, block.id)
            if key in seen_definitions:
                continue  # duplicate block (for some reason)
            definition = file.get_definition(block.kind, block.id)
            if definition is None:
                raise RuntimeError(
                    f"unknown definition: {key} in {file.path} (defines: {list(file.definitions.keys())})"
                )
            new_block = f"{MARKER_START.format(kind=definition.kind, id=definition.id)}\n"
            new_block += definition.definition_str
            if block.custom_content:
                # trim last } if it exists (it's also in custom content)
                new_block = new_block.strip().rstrip("}")
                new_block += f"\n{MARKER_CUSTOM_START}\n\n{block.custom_content}\n"
            new_block += f"\n{MARKER_END.format(kind=definition.kind, id=definition.id)}"
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
    language_imports = {
        "NodeType",
        "TraitType",
        "StructType",
        "EnumType",
        "BuiltinObject",
        "Struct",
        "StructFrozen",
        "Node",
        "NodeReference",
        "Graph",
        "Supergraph",
        "Session",
        "QueryConnection",
        "ACTIVE_SESSION",
        "activeSession",
        *(d.name for d in file.dependencies.values() if d.module != file.module),
    }
    # add any previous language imports
    for import_ in import_block.imports:
        if import_.path.startswith("@/language"):
            language_imports.update(import_.names)
    # remove any imports that are already defined in this file
    language_imports.difference_update(definition.name for definition in file.definitions.values())
    import_parts: list[str] = [
        f"import {{ {', '.join(sorted(language_imports))} }} from '@/language';"
    ]
    if not any("Temporal" in import_.content for import_ in import_block.imports):
        import_parts.append("import { Temporal } from 'temporal-polyfill';")
    for import_ in import_block.imports:
        if not import_.path.startswith("@/language"):
            import_parts.append(import_.content)
    file_parts.insert(0, "\n".join(import_parts))

    new_str = "\n\n".join(file_parts)
    return new_str


def generate():
    """Generate the Typescript language code."""

    # collect definitions
    definition_by_cls: dict[type[BuiltinObjectBase] | type[Enum], TypescriptDefinition] = {}
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        definition = _generate_definition(ENUM_DEFINITION_BY_TYPE[enum_type])
        definition_by_cls[enum_cls] = definition
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        definition = _generate_definition(STRUCT_DEFINITION_BY_TYPE[struct_type])
        definition_by_cls[struct_cls] = definition
    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        definition = _generate_definition(TRAIT_DEFINITION_BY_TYPE[trait_type])
        definition_by_cls[trait_cls] = definition
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        definition = _generate_definition(NODE_DEFINITION_BY_TYPE[node_type])
        definition_by_cls[node_cls] = definition

    # organize definitions into files
    definitions_by_name: dict[str, TypescriptDefinition] = {}
    definitions_by_module: dict[str, list[TypescriptDefinition]] = defaultdict(list)
    for cls, definition in definition_by_cls.items():
        module = cls.__module__
        definitions_by_module[module].append(definition)
        existing_definition = definitions_by_name.get(definition.name)
        if existing_definition is not None:
            raise RuntimeError(
                f"duplicate definition name: {definition.name} "
                f"({existing_definition.module}.{existing_definition.cls.__name__} vs "
                f"{definition.module}.{definition.cls.__name__})"
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
        file.new_str = _generate_file(file)
        files_by_module[module] = file

    # write files
    for file in files_by_module.values():
        assert file.new_str, f"empty {file.path}"
        file.path.parent.mkdir(parents=True, exist_ok=True)
        file.path.write_text(file.new_str)

    # update registry file
    registry_path = Path(GENERATION_PATH) / "registry.ts"
    registry_str_parts: list[str] = [
        "import { Node, Struct } from '@/language';",
        f"import {{ {', '.join(definition.cls.__name__ for definition in definition_by_cls.values())} }} from '@/language';",
    ]
    # node maps
    node_map_str_parts: list[str] = ["export type NodeTypeMapping = {"]
    node_cls_by_type_str_parts: list[str] = ["export const NODE_CLASS_BY_TYPE = {"]
    for node_type, node_cls in NODE_CLASS_BY_TYPE.items():
        node_map_str_parts.append(f"  [NodeType.{node_type.name}]: {node_cls.__name__},")
        node_cls_by_type_str_parts.append(f"  [NodeType.{node_type.name}]: {node_cls.__name__},")
    node_map_str_parts.append("};")
    node_cls_by_type_str_parts.append("};")
    node_map_str = "\n".join(node_map_str_parts)
    node_cls_by_type_str = "\n".join(node_cls_by_type_str_parts)
    registry_str_parts.extend((node_map_str, node_cls_by_type_str))
    # trait maps
    trait_map_str_parts: list[str] = ["export type TraitTypeMapping = {"]
    node_type_by_trait_str_parts: list[str] = [
        "export const NODE_TYPES_BY_TRAIT_TYPE: Record<TraitType, NodeType[]> = {"
    ]
    for trait_type, trait_cls in TRAIT_CLASS_BY_TYPE.items():
        trait_map_str_parts.append(f"  [TraitType.{trait_type.name}]: {trait_cls.__name__},")
        node_types = NODE_TYPES_BY_TRAIT_TYPE.get(trait_type, ())
        node_type_by_trait_str_parts.append(
            f"  [TraitType.{trait_type.name}]: [{', '.join(f'NodeType.{node_type.name}' for node_type in node_types)}], "
        )
    node_type_by_trait_str_parts.append("};")
    trait_map_str_parts.append("};")
    trait_map_str = "\n".join(trait_map_str_parts)
    node_type_by_trait_str = "\n".join(node_type_by_trait_str_parts)
    registry_str_parts.extend((trait_map_str, node_type_by_trait_str))
    # struct maps
    struct_map_str_parts: list[str] = ["export type StructTypeMapping = {"]
    struct_cls_by_type_str_parts: list[str] = ["export const STRUCT_CLASS_BY_TYPE = {"]
    for struct_type, struct_cls in STRUCT_CLASS_BY_TYPE.items():
        struct_map_str_parts.append(f"  [StructType.{struct_type.name}]: {struct_cls.__name__},")
        struct_cls_by_type_str_parts.append(
            f"  [StructType.{struct_type.name}]: {struct_cls.__name__},"
        )
    struct_map_str_parts.append("};")
    struct_cls_by_type_str_parts.append("};")
    struct_map_str = "\n".join(struct_map_str_parts)
    struct_cls_by_type_str = "\n".join(struct_cls_by_type_str_parts)
    registry_str_parts.extend((struct_map_str, struct_cls_by_type_str))
    # enum maps
    enum_map_str_parts: list[str] = ["export type EnumTypeMapping = {"]
    enum_cls_by_type_str_parts: list[str] = ["export const ENUM_CLASS_BY_TYPE = {"]
    for enum_type, enum_cls in ENUM_CLASS_BY_TYPE.items():
        enum_map_str_parts.append(f"  [EnumType.{enum_type.name}]: {enum_cls.__name__},")
        enum_cls_by_type_str_parts.append(f"  [EnumType.{enum_type.name}]: {enum_cls.__name__},")
    enum_map_str_parts.append("};")
    enum_cls_by_type_str_parts.append("};")
    enum_map_str = "\n".join(enum_map_str_parts)
    enum_cls_by_type_str = "\n".join(enum_cls_by_type_str_parts)
    registry_str_parts.extend((enum_map_str, enum_cls_by_type_str))
    registry_str = "\n\n".join(registry_str_parts)
    registry_path.write_text(registry_str)

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
            module_name = ts_file.stem
            index_lines.append(f"export * from './{module_name}';")
        # collect all subdirectories that contain .ts files
        for subdir in sorted(module_path.iterdir()):
            if subdir.is_dir() and any(subdir.rglob("*.ts")):
                subdir_name = subdir.name
                index_lines.append(f"export * from './{subdir_name}';")

        index_content = "\n".join(index_lines) + "\n"
        index_path.write_text(index_content)

    # format it all
    subprocess.run("cd destack-ts && bun run format-language", shell=True, check=True)
