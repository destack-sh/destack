from typing import TYPE_CHECKING, Optional

from destack.language.core import (
    Entity,
    MethodType,
    NodeType,
    RuntimeLanguage,
    RuntimePlatform,
    builtin_entity,
    builtin_property,
)

if TYPE_CHECKING:
    from destack.language import Text

# pyright: reportIncompatibleVariableOverride=false


@builtin_entity(NodeType.FUNCTION, is_abstract=True)
class Function(Entity):
    # meta
    type: MethodType = builtin_property(100)
    text: Optional["Text"] = builtin_property(104)

    platforms: list[RuntimePlatform] | None = builtin_property(
        130,
        description="The platforms this Function is available on (all if empty).",
    )
    languages: list[RuntimeLanguage] | None = builtin_property(
        131,
        description="The languages this Function is available in (all if empty).",
    )
