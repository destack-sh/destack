import collections
import textwrap

from destack.core import (
    EMPTY_LIST,
    VERSION,
    EnumDefinition,
    ModuleDefinition,
    ModuleType,
    NodeDefinition,
    PrimitiveType,
    SchemaDefinition,
    StructDefinition,
    Type,
)
from destack.registry import (
    ENUM_DEFINITION_BY_TYPE,
    STRUCT_DEFINITION_BY_TYPE,
)

from .core import (
    RustAttribute,
    RustFile,
    RustGenerationType,
    RustImport,
    RustItemScope,
    RustManagedItem,
    RustModDeclaration,
    source_path_to_local_path,
)

INTRINSIC_PRIMITIVE_TYPES: dict[PrimitiveType, str] = {
    PrimitiveType.BOOLEAN: "bool",
    # integer
    PrimitiveType.INT8: "i8",
    PrimitiveType.INT16: "i16",
    PrimitiveType.INT32: "i32",
    PrimitiveType.INT64: "i64",
    PrimitiveType.INT128: "i128",
    PrimitiveType.UINT8: "u8",
    PrimitiveType.UINT16: "u16",
    PrimitiveType.UINT32: "u32",
    PrimitiveType.UINT64: "u64",
    PrimitiveType.UINT128: "u128",
    # float
    PrimitiveType.FLOAT32: "f32",
    PrimitiveType.FLOAT64: "f64",
    # string
    PrimitiveType.STRING: "String",
    PrimitiveType.CHARACTER: "char",
}


def _generate_type(type: Type) -> str:
    """Map a Destack type to its Rust declaration."""
    raise NotImplementedError


def _generate_type_scalar(type: Type) -> str:
    """Map a Destack scalar type to its Rust declaration."""
    raise NotImplementedError


def _generate_doc_comment(object_key: str, doc: str) -> str:
    """Generate a Rust doc comment."""
    if doc:
        return "\n".join(f"/// {line.strip()}" for line in doc.splitlines() if line)
    else:
        return f"/// {object_key}"


def _get_object_key(object: StructDefinition | EnumDefinition | NodeDefinition | ModuleDefinition):
    """Gets the RustManagedItem.object_key for an Object definition."""
    return object.name


def _generate_struct_definition(struct: StructDefinition) -> RustManagedItem:
    """Map a Struct to its Rust definition item."""

    object_key = _get_object_key(struct)

    # inner content
    inner_content_parts: list[str] = []

    inner_content = "\n".join(inner_content_parts)

    # outer content
    outer_content = f"""\
{_generate_doc_comment(object_key, struct.description)}
pub struct {struct.name} {{
{textwrap.indent(inner_content, " " * 4)}
}}
"""

    item = RustManagedItem(
        type=RustGenerationType.GENERATED,
        scope=RustItemScope.BLOCK,
        object_key=object_key,
        inner_key="struct",
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=inner_content,
    )
    return item


def _generate_enum_definition(enum: EnumDefinition) -> RustManagedItem:
    """Map an Enum to its Rust definition item."""

    object_key = _get_object_key(enum)

    # inner content
    inner_content_parts: list[str] = []
    for option in enum.options:
        option_declaration = f"{option.name} = {option.id}"
        if option.description:
            option_declaration = (
                f"{_generate_doc_comment(option.name, option.description)}\n{option_declaration}"
            )
        inner_content_parts.append(option_declaration)
    inner_content = ",\n".join(inner_content_parts)

    # outer content
    outer_content = f"""\
{_generate_doc_comment(object_key, enum.description)}
pub enum {enum.name} {{
{textwrap.indent(inner_content, " " * 4)}
}}
"""

    item = RustManagedItem(
        type=RustGenerationType.GENERATED,
        scope=RustItemScope.BLOCK,
        object_key=_get_object_key(enum),
        inner_key="enum",
        children=EMPTY_LIST,
        outer_content=outer_content,
        inner_content=inner_content,
    )
    return item


def _generate_object_module(
    module: ModuleDefinition,
) -> tuple[list[RustManagedItem], list[RustManagedItem]]:
    """Generate the partial and generated Rust items for an object module."""

    partial_items: list[RustManagedItem] = []
    gen_items: list[RustManagedItem] = []

    # generate structs
    for struct_type in module.struct_types:
        struct_def = STRUCT_DEFINITION_BY_TYPE[struct_type]
        struct_item = _generate_struct_definition(struct_def)
        partial_items.append(struct_item)

    # generate enums
    for enum_type in module.enum_types:
        enum_def = ENUM_DEFINITION_BY_TYPE[enum_type]
        enum_item = _generate_enum_definition(enum_def)
        partial_items.append(enum_item)

    return partial_items, gen_items


def generate_files(schema: SchemaDefinition) -> dict[str, RustFile]:
    """Generate the new clean files for a given Destack schema (by local path)."""
    files: dict[str, RustFile] = {}

    # map object modules to files
    for module in schema.modules:
        if module.type != ModuleType.OBJECT:
            continue  # ignore index modules

        # map
        partial_items, gen_items = _generate_object_module(module)

        # partial file (always exists)
        partial_file = RustFile(
            type=RustGenerationType.PARTIAL,
            source_path=module.path,
            local_path=source_path_to_local_path(module.path, is_gen=False),
            items=partial_items,
            comment=f"//! {module.path}@{VERSION}",
            attributes=[
                RustAttribute(
                    content=f"#![destack::{RustGenerationType.PARTIAL.value}({module.path}, file)]"
                ),
            ],
        )
        files[partial_file.local_path] = partial_file

        # generated file (only if there are generated items)
        if gen_items:
            gen_file = RustFile(
                type=RustGenerationType.GENERATED,
                source_path=module.path,
                local_path=source_path_to_local_path(module.path, is_gen=True),
                items=gen_items,
                comment=f"//! {module.path}@{VERSION}",
                attributes=[
                    RustAttribute(
                        content=f"#![destack::{RustGenerationType.GENERATED.value}({module.path}, file)]"
                    ),
                ],
            )
            files[gen_file.local_path] = gen_file

    # add files for 'mod.rs' in each module

    # group files by directory
    mods_by_directory: dict[str, set[str]] = collections.defaultdict(set)
    for file in files.values():
        directory = file.local_path.rsplit("/", 1)[0]
        inner_name = file.source_path.split(".")[-1]
        mods_by_directory[directory].add(inner_name)
    # group directories "recursively"
    # (e.g. for basics/access/membership we also want basics->access and access->membership)
    for directory in list(mods_by_directory.keys()):
        directory_parts = directory.split("/")
        for i in range(len(directory_parts) - 1):
            mods_by_directory[directory_parts[i]].add(directory_parts[i + 1])
    # generate mod.rs files
    for directory, directory_files in mods_by_directory.items():
        # generate items
        mods: list[RustModDeclaration] = []
        imports: list[RustImport] = []
        for inner_name in directory_files:
            # mod
            mod = RustModDeclaration(name=inner_name, is_public=False)
            mods.append(mod)
            # import
            imp = RustImport(
                rust_path=inner_name,
                imports=EMPTY_LIST,
                is_internal=True,
                is_public=True,
                is_glob=True,
            )
            imports.append(imp)

        # generate mod file
        source_path = directory + ".mod"
        local_path = source_path_to_local_path(source_path, is_gen=False)
        mod_file = RustFile(
            type=RustGenerationType.PARTIAL,
            source_path=source_path,
            local_path=local_path,
            imports=imports,
            mods=mods,
            items=(),
            is_mod_rs=True,
            comment=f"//! {directory}@{VERSION}",
            attributes=[
                RustAttribute(
                    content=f"#![destack::{RustGenerationType.PARTIAL.value}({directory}, file)]"
                ),
            ],
        )
        files[mod_file.local_path] = mod_file

    return files
