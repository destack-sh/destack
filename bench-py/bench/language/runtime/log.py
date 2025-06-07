from bench.language.core import (
    HasEnvironment,
    IsInPackage,
    IsLog,
    Node,
    NodeType,
    node_,
)

# pyright: reportIncompatibleMethodOverride=false


@node_(NodeType.LOG)
class Log(HasEnvironment, IsLog, IsInPackage, Node):
    """A Log."""

    pass
