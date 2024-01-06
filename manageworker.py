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

from bench.runtime import WorkerNode  # noqa: E402
from bench.runtime.host import WorkerHost  # noqa: E402
from bench.utils.analytics import init_sentry  # noqa: E402
from bench.utils.cache import test_redis_connection  # noqa: E402
from bench.utils.monitoring import restart_on_file_changes  # noqa: E402
from bench.utils.utils import DEBUG, LOCAL  # noqa: E402

if not (LOCAL or DEBUG):
    init_sentry(django=False)

if os.environ.get("DEBUG") == "1" and "WORKER_SET_ID" not in os.environ:
    # auto reload on file change if in dev mode
    worker_set_id = None
    worker_id = "local"
    bench_id = None
    module_id = None
    nats_name = "worker-local"
    logger.info("worker.dev_mode")
else:
    # production mode, one worker per process
    worker_set_id = UUID(os.environ["WORKER_SET_ID"])
    worker_id = os.environ["WORKER_ID"].replace(".", "-")
    bench_id = UUID(os.environ["WORKER_BENCH_ID"])
    module_id = UUID(os.environ["WORKER_MODULE_ID"])
    nats_name = f"worker-{worker_set_id}-{worker_id}"
    logger.info(
        "worker.prod_mode",
        worker_set_id=worker_set_id,
        worker_id=worker_id,
        bench_id=bench_id,
    )


async def _run_node():
    await test_redis_connection()
    worker = WorkerNode(
        worker_set_id=worker_set_id,
        worker_id=worker_id,
        bench_id=bench_id,
        module_id=module_id,
    )
    logger.info("start_process_worker", worker=worker)
    await worker.launch_monitoring_server("0.0.0.0", 80)
    await worker.run_forever()


async def _run_host():
    await test_redis_connection()
    host = WorkerHost(
        worker_set_id=worker_set_id,
        worker_id=worker_id,
        bench_id=bench_id,
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
