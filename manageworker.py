import asyncio
import os
from pathlib import Path
from uuid import UUID

import dotenv
import structlog

from bench.msg.core import init_nats, process_soon_queue
from bench.utils.analytics import init_sentry
from bench.utils.logging import configure_logging
from bench.utils.monitoring import restart_on_file_changes
from bench.worker import WorkerNode

logger = structlog.get_logger(__name__)

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
configure_logging(apply_logging=True, apply_structlog=True)

init_sentry(django=False)


async def _run():
    asyncio.create_task(process_soon_queue())

    if os.environ.get("DEBUG") == "1":
        # auto reload on file change if in dev mode
        asyncio.create_task(restart_on_file_changes())
        await init_nats(name="worker-local")
        worker_set_id = None
        worker_node_id = "local"
        project_id = None
    else:
        # production mode, one worker per process
        worker_set_id = UUID(os.environ["WORKER_SET_ID"])
        worker_node_id = os.environ["WORKER_NODE_ID"]
        project_id = UUID(os.environ["WORKER_PROJECT_ID"]) if "PROJECT_ID" in os.environ else None
        await init_nats(name=f"worker-{worker_set_id}-{worker_node_id}")
    worker = WorkerNode(
        worker_set_id=worker_set_id, worker_node_id=worker_node_id, project_id=project_id
    )
    logger.info("start_process_worker", worker=worker)
    await worker.launch_monitoring_server("0.0.0.0", 80)
    await worker.run_forever()


asyncio.run(_run())
