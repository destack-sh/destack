from typing import TYPE_CHECKING, Union

from destack.language.core import (
    Entity,
    IsCustomizable,
    IsDeletable,
    IsOrdered,
    IsRunnable,
    IsScriptable,
    IsSpatial,
    NodeType,
    builtin_node,
    builtin_property,
    builtin_property_parent,
)

if TYPE_CHECKING:
    from destack.language import Folder

# pyright: reportIncompatibleVariableOverride=false


@builtin_node(NodeType.SCRIPT)
class Script(
    IsSpatial,
    IsOrdered,
    IsDeletable,
    IsRunnable,
    IsCustomizable,
    Entity,
):
    """A Script."""

    parent: Union["Folder", IsScriptable, "Script", None] = builtin_property_parent(
        node_is_extensible=True
    )
    name: str = builtin_property(101, is_repr=True)

    code: str | None = builtin_property(110)
    # type, language, code, ...
