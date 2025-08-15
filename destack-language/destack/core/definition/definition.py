from ..builtin import (
    Struct,
    StructType,
    declare_property,
    declare_struct,
)


@declare_struct(
    StructType.DEFINITION,
    is_abstract=True,
)
class Definition(Struct):
    """A builtin Definition."""

    name: str = declare_property(101, is_repr=True, tag=None)
    description: str = declare_property(102, tag=None)
