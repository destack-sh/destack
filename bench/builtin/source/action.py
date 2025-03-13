import asyncio
from datetime import timedelta
from typing import TYPE_CHECKING

from bench.builtin.core import class_to_kit
from bench.language import Action, ActionType, InterruptionType, NodeMode, Page, text

if TYPE_CHECKING:
    from bench.runtime import ActionRunner, NonRetryableError, RetryableError

# ruff: noqa: N802,N803


class IActionKit(ActionRunner if TYPE_CHECKING else object):
    """Common utility Actions for Flows."""

    async def Fail(self, Message: str, Is_Retryable: bool = True) -> None:
        """
        Fail with an error.
        ICON: fas fa-triangle-exclamation
        """
        if Is_Retryable:
            raise RetryableError(Message)
        else:
            raise NonRetryableError(Message)

    async def Wait(self, delay: timedelta) -> None:
        """
        Wait for a given duration.
        ICON: fas fa-clock
        """
        await asyncio.sleep(delay.total_seconds())

    async def Yield(self) -> None:
        """
        Yield control to someone.
        ICON: fas fa-hand
        """
        _ = self._trap_interruption(InterruptionType.YIELD)


ActionKit = class_to_kit(IActionKit, "ActionKit")
# specialized dynamic actions
ActionKit.actions.extend(
    Action.new(
        ActionType.DO,
        name="Think",
        text=text("Reflect on the context"),
        icon="fas fa-brain-circuit",
        mode=NodeMode.TEMPLATE,
    ),
    Action.new(
        ActionType.DO,
        name="Route",
        text=text("Route between Actions"),
        icon="fas fa-split",
        mode=NodeMode.TEMPLATE,
    ),
    Action.new(
        ActionType.DO,
        name="Generate",
        text=text("Generate something new"),
        icon="fas fa-wand-magic-sparkles",
        mode=NodeMode.TEMPLATE,
    ),
    Action.new(
        ActionType.DO,
        name="Transform",
        text=text("Change the form of something"),
        icon="fas fa-arrows-rotate",
        mode=NodeMode.TEMPLATE,
    ),
    Action.new(
        ActionType.DO,
        name="Extract",
        text=text("Extract structured data"),
        icon="fas fa-filter",
        mode=NodeMode.TEMPLATE,
    ),
    Action.new(
        ActionType.DO,
        name="Edit",
        text=text("Edit this Bench"),
        icon="fas fa-pen-to-square",
        mode=NodeMode.TEMPLATE,
    ),
)

ActionPage = Page.new("Action", ActionKit)
