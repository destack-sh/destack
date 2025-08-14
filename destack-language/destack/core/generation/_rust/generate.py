from destack.core import (
    VERSION,
    ModuleType,
    PrimitiveType,
    SchemaDefinition,
    StructDefinition,
    Type,
)

from .core import RustAttribute, RustFile, RustGenerationType, to_raw_path

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


def generate_type(type: Type) -> str:
    """Map a Destack type to its Rust declaration."""
    raise NotImplementedError


def generate_type_scalar(type: Type) -> str:
    """Map a Destack scalar type to its Rust declaration."""
    raise NotImplementedError


def generate_struct_definition(struct: StructDefinition) -> str:
    """Map a Destack struct to its Rust declaration."""
    raise NotImplementedError


def generate_files(schema: SchemaDefinition) -> dict[str, RustFile]:
    """Generate the new clean files for a given Destack schema (by raw path)."""
    files: dict[str, RustFile] = {}

    # map object modules to files
    for module in schema.modules:
        if module.type != ModuleType.OBJECT:
            continue  # ignore index modules

        raw_path = to_raw_path(module.path)

        partial_file = RustFile(
            type=RustGenerationType.PARTIAL,
            normalized_path=module.path,
            raw_path=str(raw_path),
            items=[],
            comment=f"//! Generated in {VERSION} from {module.path}",
            attributes=[
                RustAttribute(
                    content=f"#![destack::{RustGenerationType.PARTIAL.value}({module.path}, file)]"
                ),
            ],
        )

        gen_file = RustFile(
            type=RustGenerationType.GENERATED,
            normalized_path=module.path,
            raw_path=str(raw_path) + ".rs",
            items=[],
            comment=f"//! Generated in {VERSION} from {module.path}",
            attributes=[
                RustAttribute(
                    content=f"#![destack::{RustGenerationType.GENERATED.value}({module.path}, file)]"
                ),
            ],
        )

        files[partial_file.raw_path] = partial_file
        files[gen_file.raw_path] = gen_file

    # add files for 'mod.rs' in each module
    # ...

    return files
