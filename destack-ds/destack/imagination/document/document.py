from destack.basics import File
from destack.core import NodeType, declare_entity


@declare_entity(NodeType.DOCUMENT)
class Document(File):
    """
    A Document File.
    """

    pass
