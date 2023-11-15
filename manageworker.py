import asyncio
import os
import sys
from pathlib import Path
from uuid import UUID

import dotenv
import structlog

# must come first
os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
dotenv.load_dotenv(".env.worker", verbose=True)

from bench.utils.logging import configure_logging  # noqa: E402

logger = structlog.get_logger(__name__)
configure_logging(apply_logging=True, apply_structlog=True)

from bench.msg.core import init_nats, process_soon_queue  # noqa: E402
from bench.utils.analytics import init_sentry  # noqa: E402
from bench.utils.cache import test_redis_connection  # noqa: E402
from bench.utils.monitoring import restart_on_file_changes  # noqa: E402
from bench.utils.utils import DEBUG, LOCAL  # noqa: E402
from bench.worker import WorkerNode  # noqa: E402
from bench.worker.host import WorkerHost  # noqa: E402

if not (LOCAL or DEBUG):
    init_sentry(django=False)

if os.environ.get("DEBUG") == "1" and "WORKER_SET_ID" not in os.environ:
    # auto reload on file change if in dev mode
    worker_set_id = None
    worker_node_id = "local"
    project_id = None
    module_id = None
    nats_name = "worker-local"
    logger.info("worker.dev_mode")
else:
    # production mode, one worker per process
    worker_set_id = UUID(os.environ["WORKER_SET_ID"])
    worker_node_id = os.environ["WORKER_NODE_ID"].replace(".", "-")
    project_id = UUID(os.environ["WORKER_PROJECT_ID"])
    module_id = UUID(os.environ["WORKER_MODULE_ID"])
    nats_name = f"worker-{worker_set_id}-{worker_node_id}"
    logger.info(
        "worker.prod_mode",
        worker_set_id=worker_set_id,
        worker_node_id=worker_node_id,
        project_id=project_id,
    )


async def _run_node():
    await init_nats(nats_name)
    await test_redis_connection()
    asyncio.create_task(process_soon_queue())
    worker = WorkerNode(
        worker_set_id=worker_set_id,
        worker_node_id=worker_node_id,
        project_id=project_id,
        module_id=module_id,
    )
    logger.info("start_process_worker", worker=worker)
    await worker.launch_monitoring_server("0.0.0.0", 80)
    await worker.run_forever()


async def _run_host():
    await init_nats(nats_name)
    await test_redis_connection()
    asyncio.create_task(process_soon_queue())
    host = WorkerHost(
        worker_set_id=worker_set_id,
        worker_node_id=worker_node_id,
        project_id=project_id,
        module_id=module_id,
    )
    logger.info("start_process_host", host=host)
    if os.environ.get("DEBUG") == "1":
        asyncio.create_task(restart_on_file_changes(on_restart=host.stop_sync))
    await host.run_forever()


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise RuntimeError("missing argument")
    if sys.argv[1] == "host":
        asyncio.run(_run_host())
    elif sys.argv[1] == "worker":
        asyncio.run(_run_node())
    else:
        raise RuntimeError(f"invalid arguments: {sys.argv}")
