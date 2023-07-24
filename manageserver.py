import asyncio
import os
from pathlib import Path

import dotenv
import structlog

from bench.msg.core import process_soon_queue
from bench.settings import RUN_LANGUAGE_SERVER, RUN_ORCHESTRATION_SERVER
from bench.utils.analytics import init_sentry
from bench.utils.logging import configure_logging
from bench.utils.monitoring import MonitoringServer

logger = structlog.get_logger(__name__)

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
configure_logging(apply_logging=True, apply_structlog=True)

init_sentry(django=False)


async def _run():
    asyncio.create_task(process_soon_queue())

    servers = []
    if RUN_LANGUAGE_SERVER:
        from bench.server import LanguageServer

        server = LanguageServer()
        asyncio.create_task(server.run())
        servers.append(server)
    if RUN_ORCHESTRATION_SERVER:
        from bench.server import OrchestrationServer

        server = OrchestrationServer()
        asyncio.create_task(server.run())
        servers.append(server)
    if not servers:
        raise RuntimeError("no servers to run")
    await MonitoringServer(servers).launch("0.0.0.0", 80, daemon=True)


asyncio.run(_run())
