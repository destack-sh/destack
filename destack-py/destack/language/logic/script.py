from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IsCustomizable,
    IsDeletable,
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
    IsDeletable,
    IsCustomizable,
    Entity,
):
    """A Script."""

    parent: Union[IsScriptable, "Script", None] = builtin_property_parent()
    name: str = builtin_property(101, is_repr=True)

    code: str = builtin_property(110)
    # type, language, code, ...
