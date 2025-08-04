from datetime import timedelta

from destack.basics import File
from destack.core import NodeType, UInt32, declare_entity, declare_property


@declare_entity(NodeType.VIDEO)
class Video(File):
    """
    A Video File.
    """

    width: UInt32 = declare_property(128)
    height: UInt32 = declare_property(129)
    duration: timedelta = declare_property(130)
