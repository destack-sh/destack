from typing import TYPE_CHECKING

from bench.builtin.core import class_to_kit
from bench.language import NodeType

from .computer import IComputer

if TYPE_CHECKING:
    from bench.runtime import Runner


class IApplication(Runner if TYPE_CHECKING else object):
    pass


ComputerKit = class_to_kit(IComputer, "Computer Kit", icon=NodeType.APPLICATION.icon)
