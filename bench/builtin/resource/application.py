from typing import TYPE_CHECKING

from bench.builtin.core import class_to_kit
from bench.language import NodeType

if TYPE_CHECKING:
    from bench.runtime import Runner


class IApplication(Runner if TYPE_CHECKING else object):
    pass


ApplicationKit = class_to_kit(IApplication, "Application Kit", icon=NodeType.APPLICATION.icon)
