import asyncio
import os
import sys
from pathlib import Path

import django
import dotenv
import structlog

from bench.msg.core import process_soon_queue
from bench.utils.analytics import init_sentry
from bench.utils.logging import configure_logging
from bench.utils.monitoring import MonitoringServer
from bench.utils.utils import get_from_env

logger = structlog.get_logger(__name__)

# :Dotenv
LOCAL_ENV = get_from_env("LOCAL_ENV", "local")
if LOCAL_ENV == "prod":
    DOT_ENV_FILES = [".env", ".env.prod"]
else:
    DOT_ENV_FILES = [".env"]
for dot_env_file in DOT_ENV_FILES:
    dotenv.load_dotenv(dot_env_file, verbose=True, override=True)

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "bench.settings")
os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
configure_logging(apply_logging=True, apply_structlog=True)

init_sentry(django=False)

django.setup()


async def _run(names: list[str]):
    asyncio.create_task(process_soon_queue())

    servers = []
    if "language" in names or "all" in names:
        from bench.server import LanguageServer

        server = LanguageServer()
        asyncio.create_task(server.run())
        servers.append(server)
    if "orchestration" in names or "all" in names:
        from bench.server import OrchestrationServer

        server = OrchestrationServer()
        asyncio.create_task(server.run())
        servers.append(server)
    if not servers:
        raise RuntimeError("no servers to run (pass arguments 'all', 'language', 'orchestration')")
    await MonitoringServer(servers).launch("0.0.0.0", 80, daemon=True)


asyncio.run(_run(sys.argv[1:]))
