import asyncio
import os
import sys
import uuid
from pathlib import Path

import dotenv

# nocheckin print all files  at working directory
print(os.listdir(os.getcwd() + "/bench/bench"))

from bench.msg.core import init_nats, process_soon_queue
from bench.runtime.worker import SandboxedWorker
from bench.utils.analytics import init_sentry
from bench.utils.logging import configure_logging

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)
configure_logging(apply_logging=True, apply_structlog=True)

DEPLOYMENT_ID = os.environ.get("DEPLOYMENT_ID")
if DEPLOYMENT_ID is not None:
    DEPLOYMENT_ID = uuid.UUID(DEPLOYMENT_ID)

worker_id = uuid.UUID(os.environ["WORKER_ID"]) if "WORKER_ID" in os.environ else uuid.uuid4()
project_id = uuid.UUID(os.environ["PROJECT_ID"]) if "PROJECT_ID" in os.environ else None
worker = SandboxedWorker(worker_id=worker_id, project_id=project_id)

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
            await asyncio.sleep(1)
    except KeyboardInterrupt:
        observer.stop()
    observer.join()


async def _run():
    if os.environ.get("DEBUG") == "1":
        asyncio.create_task(watch_for_changes())

    asyncio.create_task(process_soon_queue())
    await init_nats(name=f"worker-{worker.worker_id}")
    await worker.run_forever()


asyncio.run(_run())

# auto reload on file change if in dev mode
