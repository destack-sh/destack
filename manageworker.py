import asyncio
import os
import sys
from pathlib import Path
from uuid import UUID

import dotenv
import structlog

from bench.msg.core import init_nats, process_soon_queue
from bench.utils.analytics import init_sentry
from bench.utils.cache import test_redis_connection
from bench.utils.logging import configure_logging
from bench.utils.monitoring import restart_on_file_changes
from bench.worker import WorkerNode
from bench.worker.host import WorkerHost

logger = structlog.get_logger(__name__)

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
configure_logging(apply_logging=True, apply_structlog=True)

init_sentry(django=False)

if os.environ.get("DEBUG") == "1":
    # auto reload on file change if in dev mode
    worker_set_id = None
    worker_node_id = "local"
    project_id = None
    nats_name = "worker-local"
else:
    # production mode, one worker per process
    worker_set_id = UUID(os.environ["WORKER_SET_ID"])
    worker_node_id = os.environ["WORKER_NODE_ID"]
    project_id = UUID(os.environ["WORKER_PROJECT_ID"])
    nats_name = f"worker-{worker_set_id}-{worker_node_id}"


async def _run_node():
    await init_nats(nats_name)
    await test_redis_connection()
    asyncio.create_task(process_soon_queue())
    worker = WorkerNode(
        worker_set_id=worker_set_id, worker_node_id=worker_node_id, project_id=project_id
    )
    logger.info("start_process_worker", worker=worker)
    await worker.launch_monitoring_server("0.0.0.0", 80)
    await worker.run_forever()


async def _run_host():
    await init_nats(nats_name)
    await test_redis_connection()
    asyncio.create_task(process_soon_queue())
    host = WorkerHost(
        worker_set_id=worker_set_id, worker_node_id=worker_node_id, project_id=project_id
    )
    logger.info("start_process_host", host=host)
    if os.environ.get("DEBUG") == "1":
        asyncio.create_task(restart_on_file_changes(on_restart=host.stop_sync))
    await host.run_forever()


# run node if arg1 is 'worker', run host if arg1 is 'host'
if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise RuntimeError("missing argument")
    if sys.argv[1] == "host":
        asyncio.run(_run_host())
    elif sys.argv[1] == "worker":
        asyncio.run(_run_node())
    else:
        raise RuntimeError(f"invalid arguments: {sys.argv}")
