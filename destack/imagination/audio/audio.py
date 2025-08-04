from datetime import timedelta

from destack.basics import File
from destack.core import NodeType, declare_entity, declare_property


@declare_entity(NodeType.AUDIO)
class Audio(File):
    """
    An Audio File.
    """

    duration: timedelta = declare_property(129)
