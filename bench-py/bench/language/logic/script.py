from bench.language.core import (
    HasName,
    IsArchivable,
    IsDeletable,
    IsScriptable,
    Node,
    NodeType,
    node_,
    property_,
    property_parent_,
)

# pyright: reportIncompatibleVariableOverride=false


@node_(NodeType.SCRIPT)
class Script(HasName, IsDeletable, IsArchivable, Node):
    """A Script."""

    parent: IsScriptable | None = property_parent_()
    # type, language, code, ...

    code: str | None = property_(100)
