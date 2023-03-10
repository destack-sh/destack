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

asyncio.run(init_nats(name=worker.worker_id))
asyncio.run(
    worker.run(
        worker_reply_addr=os.environ["ZMQ_WORKER_REPLY_ADDR"],
        worker_pub_addr=os.environ["ZMQ_WORKER_PUB_ADDR"],
        intserver_reply_addr=os.environ["ZMQ_INTSERVER_REPLY_ADDR"],
        intserver_pub_addr=os.environ["ZMQ_INTSERVER_PUB_ADDR"],
    )
)
