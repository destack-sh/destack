from typing import TYPE_CHECKING, Optional

from destack.core import (
    Entity,
    MethodType,
    NodeType,
    RuntimeLanguage,
    RuntimePlatform,
    declare_entity,
    declare_property,
)

if TYPE_CHECKING:
    from destack import Text


@declare_entity(NodeType.FUNCTION, is_abstract=True)
class Function(Entity):
    # meta
    type: MethodType = declare_property(
        100,
        tag=None,
    )
    text: Optional["Text"] = declare_property(
        104,
        tag=None,
    )

    platforms: list[RuntimePlatform] | None = declare_property(
        130,
        description="The platforms this Function is available on (all if empty).",
        tag=None,
    )
    languages: list[RuntimeLanguage] | None = declare_property(
        131,
        description="The languages this Function is available in (all if empty).",
        tag=None,
    )
