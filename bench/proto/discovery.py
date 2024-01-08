import asyncio
import functools
from typing import Collection

from grpclib._typing import IServable
import grpclib.server

from bench.utils.monitoring import Monitored


class ExtendedServiceBase(IServable):
    """gRPC service with some extra stuff for custom loops, metadata, ..."""

    async def start_quick(self) -> None:
        """Start the service. Should be ready for service when returning."""
        raise NotImplementedError

    def close(self) -> None:
        """Close the service.."""
        raise NotImplementedError

    async def wait_closed(self) -> None:
        """Wait for the service to be fully closed."""
        raise NotImplementedError


class MonitoredServiceBase(ExtendedServiceBase, Monitored):
    pass


class ExtendedServer(grpclib.server.Server):
    """gRPC server that knows about our custom service bases."""

    @functools.wraps(grpclib.server.Server.__init__)
    def __init__(self, handlers: Collection["IServable"], **kwargs):
        super().__init__(**kwargs)
        self._our_handlers: tuple[ExtendedServiceBase, ...] = tuple(
            h for h in handlers if isinstance(h, ExtendedServiceBase)
        )

    @functools.wraps(grpclib.server.Server.start)
    async def start(self, **kwargs) -> None:
        await asyncio.gather(*(h.start_quick() for h in self._our_handlers))
        await super().start(**kwargs)

    def close(self) -> None:
        for task in self._our_handlers:
            task.close()
        super().close()

    @functools.wraps(grpclib.server.Server.wait_closed)
    async def wait_closed(self) -> None:
        await super().wait_closed()
        await asyncio.gather(*(h.wait_closed() for h in self._our_handlers))
