import asyncio
from datetime import timedelta
from typing import TYPE_CHECKING

from bench.language import InterruptionType

from .reflect import class_to_kit

if TYPE_CHECKING:
    from bench.runtime import ActionRunner, NonRetryableError, RetryableError

# ruff: noqa: N802,N803


class ICommonKit(ActionRunner if TYPE_CHECKING else None):
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


CommonKit = class_to_kit(ICommonKit, "CommonKit")
