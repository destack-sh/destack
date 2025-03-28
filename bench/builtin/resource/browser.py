from typing import TYPE_CHECKING

from bench.language import (
    NodeType,
    Page,
)

from .application import IApplication
from .computer import ComputerKit

if TYPE_CHECKING:
    from bench.runtime import Runner


from bench.builtin.core import class_to_kit

# ruff: noqa: N802,N803


class IBrowserApplication(IApplication, Runner if TYPE_CHECKING else object):
    """The common interface for a Browser."""

    pass


BrowserKit = class_to_kit(
    IBrowserApplication, "Browser Kit", icon=NodeType.APPLICATION.icon, template=ComputerKit
)
BrowserPage = Page.new("Browser", BrowserKit)
