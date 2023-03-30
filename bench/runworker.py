import asyncio
import os
import uuid
from pathlib import Path

import dotenv

from bench.msg.core import init_nats
from bench.runtime.worker import Worker
from bench.utils.analytics import init_sentry
from bench.utils.cache import test_redis_connection

os.environ["VERSION"] = Path("version").read_text().strip()
dotenv.load_dotenv(verbose=True)

DEPLOYMENT_ID = os.environ.get("DEPLOYMENT_ID")
if DEPLOYMENT_ID is not None:
    DEPLOYMENT_ID = uuid.UUID(DEPLOYMENT_ID)

worker_id = uuid.UUID(os.environ["WORKER_ID"]) if "WORKER_ID" in os.environ else uuid.uuid4()
worker = Worker(worker_id=worker_id, deployment_id=DEPLOYMENT_ID, project_id=None)

init_sentry(django=False)


async def _run():
    await init_nats(name=f"worker-{worker.worker_id}")
    await worker.run_forever()


asyncio.run(test_redis_connection())  # fail early

asyncio.run(_run())
