import asyncio
import os
import sys
from pathlib import Path
from uuid import UUID

import dotenv
import structlog

from bench.msg.core import init_nats, process_soon_queue
from bench.utils.analytics import init_sentry
from bench.utils.logging import configure_logging
from bench.worker import WorkerNode, debug
from bench.worker.monitoring import WorkerMonitorServer

logger = structlog.get_logger(__name__)

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
configure_logging(apply_logging=True, apply_structlog=True)

init_sentry(django=False)


async def watch_for_changes():
    print("watching for changes...")
    from watchdog.events import FileSystemEventHandler
    from watchdog.observers import Observer

    class Handler(FileSystemEventHandler):
        def on_any_event(self, event):
            if event.is_directory:
                return
            if event.src_path.endswith(".py"):
                print(f"{event.src_path} changed, reloading...")
                os.execv(sys.executable, [sys.executable] + sys.argv)

    observer = Observer()
    observer.schedule(Handler(), ".", recursive=True)
    observer.start()

    try:
        while True:
            await asyncio.sleep(0.1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


async def _run():
    asyncio.create_task(process_soon_queue())

    if os.environ.get("DEBUG") == "1":
        # auto reload on file change if in dev mode
        asyncio.create_task(watch_for_changes())
        await init_nats(name="worker-local")

        await _manage_local_workers_forever()
    else:
        # production mode, one worker per process
        worker_set_id = UUID(os.environ["WORKER_SET_ID"])
        worker_node_id = os.environ["WORKER_NODE_ID"]
        project_id = UUID(os.environ["WORKER_PROJECT_ID"]) if "PROJECT_ID" in os.environ else None
        worker = WorkerNode(
            worker_set_id=worker_set_id, worker_node_id=worker_node_id, project_id=project_id
        )
        await init_nats(name=f"worker-{worker_set_id}-{worker_node_id}")
        logger.info("start_process_worker", worker=worker)
        await WorkerMonitorServer(worker).launch("0.0.0.0", 80)
        await worker.run_forever()


async def _manage_local_workers_forever():
    """Launches/kills local worker nodes inside this process."""
    workers_tasks_by_project_id = {}
    while True:
        debug.reload_local_workers()
        # start new workers
        for worker_set in debug.LOCAL_WORKERS.live_worker_sets.values():
            if worker_set.project_id in workers_tasks_by_project_id:
                continue
            worker = WorkerNode(
                worker_node_id=f"local-{worker_set.id}",
                worker_set_id=worker_set.id,
                project_id=worker_set.project_id,
            )
            logger.info("start_local_worker", worker=worker)
            worker_task = asyncio.create_task(worker.run_forever())
            workers_tasks_by_project_id[worker_set.project_id] = worker_task
        # prune stopped workers
        for project_id, task in list(workers_tasks_by_project_id.items()):
            if project_id not in debug.LOCAL_WORKERS.live_worker_sets:
                task.cancel()
                del workers_tasks_by_project_id[project_id]
        await asyncio.sleep(0.1)


asyncio.run(_run())
