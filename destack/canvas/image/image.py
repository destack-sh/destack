from destack.basics import File
from destack.core import NodeType, UInt32, declare_entity, declare_property


@declare_entity(NodeType.IMAGE)
class Image(File):
    """
    An Image File.
    """

    width: UInt32 = declare_property(128)
    height: UInt32 = declare_property(129)
