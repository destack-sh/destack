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


class IBrowserApplication(IApplication, Runner if TYPE_CHECKING else object):
    """The common interface for a Browser."""

    pass


BrowserKit = class_to_kit(
    IBrowserApplication, "Browser Kit", icon=NodeType.APPLICATION.icon, template=ComputerKit
)
ChromeBrowserTemplate = Application(
    name="Chrome Browser",
    type=ApplicationType.CHROME_BROWSER,
    mode=NodeMode.TEMPLATE,
    icon=icon("fab fa-chrome"),
)
BrowserPage = Page.new("Browser", ChromeBrowserTemplate, BrowserKit)
