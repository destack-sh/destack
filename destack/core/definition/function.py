from ..builtin import (
    RuntimeLanguage,
    RuntimePlatform,
    RuntimeType,
    StructType,
    UInt16,
    declare_property,
    declare_struct,
)
from .definition import Definition


@declare_struct(
    StructType.FUNCTION_DEFINITION,
    is_abstract=True,
)
class FunctionDefinition(Definition):
    """Definition of a builtin Function."""

    # meta
    id: UInt16 = declare_property(2, is_repr=True)
    is_async: bool = declare_property(110)
    is_internal: bool = declare_property(112)

    # content
    # ...

    # availability
    platforms: list[RuntimePlatform] | None = declare_property(
        140,
        description="The platforms this Method is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = declare_property(
        141,
        description="The languages this Method is available in (all if empty).",
    )
    runtimes: list[RuntimeType] | None = declare_property(
        142,
        description="The runtimes this Method is available in (all if empty).",
    )
