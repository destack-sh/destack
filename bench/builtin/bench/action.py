import asyncio
from datetime import timedelta
from typing import TYPE_CHECKING

from bench.builtin.core import class_to_kit
from bench.language import InterruptionType, Page

if TYPE_CHECKING:
    from bench.runtime import ActionRunner, NonRetryableError, RetryableError

# ruff: noqa: N802,N803


class IActionKit(ActionRunner if TYPE_CHECKING else object):
    """Common utility Actions for Flows."""

    async def Pass(self) -> None:
        """
        Do nothing.
        ICON: fas fa-forward
        """
        pass

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


ActionKit = class_to_kit(IActionKit, "Action Kit")
ActionPage = Page.new("Action", ActionKit)
