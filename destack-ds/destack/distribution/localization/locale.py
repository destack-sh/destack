from destack.core import Entity, NodeType, declare_entity


@declare_entity(NodeType.LOCALE)
class Locale(Entity):
    """
    A Locale is a language and a region.
    """

    pass
