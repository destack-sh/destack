import asyncio
import os
import uuid

import dotenv

from bench.runtime.worker import RuntimeWorker

dotenv.load_dotenv(verbose=True)

worker = RuntimeWorker(worker_id=os.environ.get("WORKER_ID", uuid.uuid4()))

asyncio.run(
    worker.run(
        worker_rep_addr=os.environ["ZMQ_WORKER_REP_ADDR"],
        worker_pub_addr=os.environ["ZMQ_WORKER_PUB_ADDR"],
        intserver_rep_addr=os.environ["ZMQ_INTSERVER_REP_ADDR"],
        intserver_pub_addr=os.environ["ZMQ_INTSERVER_PUB_ADDR"],
    )
)
