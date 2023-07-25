import asyncio
import subprocess
import sys
import time
from subprocess import Popen
from uuid import UUID

import structlog

from bench.msg import nc_init
from bench.msg.core import NMessage, handle_reply, message_handler
from bench.msg.messages import (
    NMessageType,
    RepDoRestartWorkerNodePayload,
    ReqDoRestartWorkerNodePayload,
)

logger = structlog.get_logger(__name__)


class WorkerHost:
    """
    Manages the lifecycle of the worker node in a separate process. Like an inverted sidecar.
    During development, this can also launch the worker node in the same process.
    """

    def __init__(self, worker_set_id: UUID | None, worker_node_id: UUID, project_id: UUID | None):
        self.worker_set_id = worker_set_id
        self.worker_node_id = worker_node_id
        self.project_id = project_id
        self.worker_process: Popen | None = None
        self.subs = []

    def __str__(self):
        return f"{self.project_id} {self.worker_set_id} {self.worker_node_id}"

    def __repr__(self):
        return f"<WorkerHost {self}>"

    async def run_forever(self):
        await nc_init.wait()
        routing_id = self.project_id or ">"
        self.subs = [
            await handle_reply(
                f"{NMessageType.DO_RESTART_WORKER_NODE}.{routing_id}", self.do_restart_worker_node
            )
        ]
        # launch worker process
        quick_restarts = 0
        while True:
            time_started = time.time()
            self.worker_process = subprocess.Popen(["python", "manageworker.py", "worker"])
            logger.info("host.start", worker_process=self.worker_process)
            while self.worker_process.poll() is None:
                if quick_restarts > 0 and time_started < time.time() - 10:
                    quick_restarts = 0  # success, reset
                await asyncio.sleep(0.1)
            quick_restarts += 1
            if quick_restarts > 5:
                logger.critical("host.too_many_failures", worker_process=self.worker_process)
                sys.exit(1)
            logger.info(
                "host.exit",
                worker_process=self.worker_process,
                returncode=self.worker_process.returncode,
            )

    @message_handler
    async def do_restart_worker_node(self, msg: NMessage[ReqDoRestartWorkerNodePayload]):
        logger.info("host.restart", worker_process=self.worker_process)
        self.worker_process.kill()
        # nocheckin: ensure worker shuts down cleanly (:80 server is not shut down?)
        await msg.reply(
            RepDoRestartWorkerNodePayload(
                worker_set_id=self.worker_set_id,
                worker_node_id=self.worker_node_id,
                success=True,
            )
        )

    def stop_sync(self):
        if self.worker_process:
            self.worker_process.kill()
            self.worker_process.wait(timeout=10)
            logger.info("host.stop", worker_process=self.worker_process)
            self.worker_process = None

    async def stop(self):
        self.stop_sync()
        await asyncio.gather(sub.unsubscribe() for sub in self.subs)
