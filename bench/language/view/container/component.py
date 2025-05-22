from bench.language.core import NodeType, node_

from .container import IsContainerView


@node_(NodeType.COMPONENT_VIEW)
class ComponentView(IsContainerView):
    """A component View."""

    pass
