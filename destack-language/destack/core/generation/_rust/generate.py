from destack.core import ObjectDefinition, SchemaDefinition, Type

from .core import RustFile


def generate_type(type: Type) -> str:
    """Map a Destack type to its Rust declaration."""
    raise NotImplementedError


def generate_type_scalar(type: Type) -> str:
    """Map a Destack scalar type to its Rust declaration."""
    raise NotImplementedError


def generate_object(object: ObjectDefinition) -> str:
    """Map a Destack object to its Rust declaration."""
    raise NotImplementedError


def generate_files(schema: SchemaDefinition) -> dict[str, RustFile]:
    """Generate the new clean files for a given Destack schema (by normalized path)."""
    raise NotImplementedError
