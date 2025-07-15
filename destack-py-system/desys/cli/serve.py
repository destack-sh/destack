import asyncio
import signal
from collections.abc import Collection, Iterator
from contextlib import contextmanager
from time import time_ns

import structlog
import typer

from destack.cli.utils import async_to_sync
from destack.grpc import GrpcServer, Network, ServiceBase
from destack.language import WORLD_ORACLE
from destack.utils.env import ENV, IS_DEV
from destack.utils.watch import restart_on_file_changes

app = typer.Typer(short_help="run the services")
logger = structlog.get_logger(__name__)


@contextmanager
def _guard_server(
    server: GrpcServer,
    *,
    signals: Collection[int] = (signal.SIGINT, signal.SIGTERM),
) -> Iterator[None]:
    """
    Utility context-manager to help properly shutdown server in response to the OS signals.
    Adapted from grpclib.utils.graceful_exit.
    """
    loop = asyncio.get_event_loop()
    signals = set(signals)
    flag: list[bool] = []

    def _stop(sig_num: "signal.Signals") -> None:
        fail = False
        server.close()
        for service in server._services:
            try:
                service.stop()
            except RuntimeError:
                # probably server wasn't started yet
                fail = True
        if fail:
            # using second stage in case of error will ensure that non-closed
            # server wont start later
            _kill(sig_num)

    def _kill(sig_num: "signal.Signals") -> None:
        raise SystemExit(128 + sig_num)

    def _exit_handler(sig_num: "signal.Signals") -> None:
        if flag:
            _kill(sig_num)
        else:
            _stop(sig_num)
            flag.append(True)

    for sig_num in signals:
        loop.add_signal_handler(sig_num, _exit_handler, sig_num)  # type: ignore
    try:
        yield
    finally:
        for sig_num in signals:
            loop.remove_signal_handler(sig_num)


async def _do_serve(
    handlers: list[ServiceBase], *, network: Network, host: str, port: int, watch: bool
):
    """Serves the given gRPC services."""
    logger.info("serve", handlers=handlers, host=host, port=port, env=ENV)
    start = time_ns()
    server = GrpcServer(handlers=handlers, network=network, oracle=WORLD_ORACLE)
    if IS_DEV and watch:
        _ = asyncio.create_task(restart_on_file_changes())  # noqa: RUF006
    try:
        with _guard_server(server):
            await server.start(host=host, port=port)
            await server.wait_closed()
    finally:
        logger.info("serve.exit", uptime=(time_ns() - start) / 1_000_000)


@app.command()
@async_to_sync
async def runtime(host: str, port: int, *, process_id: int = -1, watch: bool = False):
    """Serve the Runtime."""
    raise NotImplementedError
