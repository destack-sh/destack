from destack.basics import File
from destack.core import NodeType, declare_entity


@declare_entity(NodeType.MODEL)
class Model(File):
    """
    A Model File.
    """

    pass
