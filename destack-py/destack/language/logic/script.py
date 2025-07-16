from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IsCustomizable,
    IsOrdered,
    IsScriptable,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    pass

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCRIPT)
class Script(
    IsOrdered,
    IsCustomizable,
    Entity,
):
    """A Script."""

    parent: Union[IsScriptable, "Script", None] = builtin_property_parent()

    code: str = builtin_property(110)
    # type, language, code, ...
