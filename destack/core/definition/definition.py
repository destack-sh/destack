from ..builtin import (
    StructFrozen,
    StructType,
    declare_property,
    declare_struct,
)


@declare_struct(
    StructType.DEFINITION,
    frozen=True,
    is_abstract=True,
)
class Definition(StructFrozen):
    """A builtin Definition."""

    name: str = declare_property(101, is_repr=True)
    description: str | None = declare_property(102, is_repr=True)
