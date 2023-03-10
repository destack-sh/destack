import asyncio
import os
import uuid
from pathlib import Path

import dotenv

from bench.msg.core import init_nats
from bench.runtime.worker import Worker
from bench.utils.analytics import init_sentry

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)

worker = Worker(worker_id=os.environ.get("WORKER_ID", uuid.uuid4()))

init_sentry(django=False)


async def run():
    await init_nats(name=f"worker-{worker.worker_id}")
    await worker.run_forever()


asyncio.run(run())
