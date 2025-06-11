from typing import Union

from destack.language.core import (
    Entity,
    HasName,
    IsActionable,
    IsArchivable,
    IsDeletable,
    IsRunnable,
    IsScriptable,
    Node,
    NodeType,
    Spatial,
    node_,
    property_,
    property_parent_,
)
from destack.pb2 import ScriptData

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCRIPT)
class Script(
    Spatial,
    Entity,
    HasName,
    IsDeletable,
    IsArchivable,
    IsRunnable,
    IsActionable,
    Node[ScriptData],
):
    """A Script."""

    parent: Union[IsScriptable, "Script", None] = property_parent_(node_is_customizable=True)
    # type, language, code, ...

    code: str | None = property_(100)
