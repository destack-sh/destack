from bench.language.core import NodeType, node_

from .container import ContainerViewBase


@node_(NodeType.COMPONENT_VIEW)
class ComponentView(ContainerViewBase):
    """A component View."""

    pass
