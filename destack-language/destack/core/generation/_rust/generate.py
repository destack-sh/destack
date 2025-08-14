from destack.core import SchemaDefinition

from .core import RustFile


def generate_files(schema: SchemaDefinition) -> list[RustFile]:
    raise NotImplementedError
