import asyncio
from datetime import timedelta
from typing import TYPE_CHECKING, Annotated

from bench.builtin.core import class_to_kit
from bench.language import InterruptionType, Page

if TYPE_CHECKING:
    from bench.runtime import ActionRunner, NonRetryableError, RetryableError

# ruff: noqa: N802,N803


class ICommonKit(ActionRunner if TYPE_CHECKING else object):
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

    # nocheckin
    async def Multiply(self, A: int, B: int) -> Annotated[dict[str, int], {"Result": int}]:
        """
        Multiply two numbers.
        ICON: fas fa-calculator
        """
        await asyncio.sleep(5)
        return {"Result": A * B}


CommonKit = class_to_kit(ICommonKit, "Common")
ActionPage = Page.new("Action", CommonKit)
