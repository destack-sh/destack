from typing import TYPE_CHECKING

from bench.language import (
    Application,
    ApplicationType,
    NodeMode,
    NodeType,
    Page,
    icon,
)

from .application import IApplication
from .computer import ComputerKit

if TYPE_CHECKING:
    from bench.runtime import Runner


from bench.builtin.core import class_to_kit

# ruff: noqa: N802,N803


class IShellApplication(IApplication, Runner if TYPE_CHECKING else object):
    """The interface to a Shell."""

    pass


ShellKit = class_to_kit(
    IShellApplication, "Shell Kit", icon=NodeType.APPLICATION.icon, template=ComputerKit
)
ShellTemplate = Application(
    name="Shell",
    type=ApplicationType.SHELL,
    mode=NodeMode.TEMPLATE,
    icon=icon("fa-terminal"),
)
ShellPage = Page.new("Shell", ShellTemplate, ShellKit)
