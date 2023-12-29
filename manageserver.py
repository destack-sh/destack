import asyncio
import os
import sys
from pathlib import Path

import django
import dotenv
import structlog

from bench.proto.messaging import init_nats
from bench.utils.analytics import init_sentry
from bench.utils.cache import test_redis_connection
from bench.utils.logging import configure_logging
from bench.utils.monitoring import MonitoringServer
from bench.utils.utils import DEBUG, TEST, get_from_env

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

if not (TEST or DEBUG):
    init_sentry(django=False)

django.setup()


async def _run(names: list[str]):
    await init_nats("server")
    await test_redis_connection()

    servers = []
    if "language" in names or "all" in names:
        from bench.server import RuntimeSupervisor

        server = RuntimeSupervisor()
        asyncio.create_task(server.run())
        servers.append(server)
    if "orchestration" in names or "all" in names:
        from bench.server import OrchestrationServer

        server = OrchestrationServer()
        asyncio.create_task(server.run())
        servers.append(server)
    if not servers:
        raise RuntimeError("no servers to run (pass arguments 'all', 'language', 'orchestration')")
    if not DEBUG:
        await MonitoringServer(servers).launch("0.0.0.0", 80, daemon=False)  # keep running forever
    else:
        await asyncio.Event().wait()


asyncio.run(_run(sys.argv[1:]))
