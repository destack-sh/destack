import asyncio
import os
import signal
import subprocess
import sys
import time
from subprocess import Popen
from uuid import UUID

import structlog

from bench.proto.messaging import nc_init

logger = structlog.get_logger(__name__)


class WorkerHost:
    """
    Manages the lifecycle of the worker node's worker processes in a main sidecar process.
    During local development, this may also launch the worker node in the same process.
    """

    def __init__(
        self,
        worker_set_id: UUID | None,
        worker_node_id: str,
        project_id: UUID | None,
        module_id: UUID | None,
    ):
        self.worker_set_id = worker_set_id
        self.worker_node_id = worker_node_id
        self.project_id = project_id
        self.module_id = module_id
        self.worker_process: Popen | None = None
        self._stopped = False

    def __str__(self):
        return f"{self.project_id} {self.worker_set_id} {self.worker_node_id}"

    def __repr__(self):
        return f"<WorkerHost {self}>"

    async def run_forever(self):
        await nc_init.wait()
        logger.info("host.start", worker_process=self.worker_process, host=self)
        if self.project_id:
            routing_ids = {
                f"{self.project_id}.{self.worker_set_id or 'all'}",
                f"{self.project_id}.all",
            }
        else:
            routing_ids = (">",)
        # launch worker process
        suspiciously_rapid_restarts = 0
        while not self._stopped:
            time_started = time.time()
            self.worker_process = subprocess.Popen(
                ["python", "manageworker.py", "worker"], preexec_fn=os.setsid
            )
            logger.info("host.start", worker_process=self.worker_process)
            while self.worker_process.poll() is None:
                if suspiciously_rapid_restarts > 0 and time_started < time.time() - 2:
                    suspiciously_rapid_restarts = 0  # success, reset
                await asyncio.sleep(0.1)
            suspiciously_rapid_restarts += 1
            if suspiciously_rapid_restarts > 10:
                logger.critical("host.too_many_failures", worker_process=self.worker_process)
                sys.exit(1)
            logger.info(
                "host.exit",
                worker_process=self.worker_process,
                returncode=self.worker_process.returncode,
            )
        logger.info("host.stopped", worker_process=self.worker_process, host=self)

    def _terminate_worker(self):
        # see https://stackoverflow.com/questions/4789837/how-to-terminate-a-python-subprocess-launched-with-shell-true/4791612#4791612
        os.killpg(os.getpgid(self.worker_process.pid), signal.SIGTERM)

    @message_handler
    async def do_restart_worker_node(self, msg: NMessage[ReqDoRestartWorkerNodePayload]):
        logger.info("host.restart", worker_process=self.worker_process)
        self._terminate_worker()
        await msg.reply(
            RepDoRestartWorkerNodePayload(
                worker_set_id=self.worker_set_id,
                worker_node_id=self.worker_node_id,
                worker_process_id=str(self.worker_process.pid),
                success=True,
            )
        )

    def stop_sync(self):
        self._stopped = True
        if self.worker_process:
            self._terminate_worker()
            self.worker_process.wait(timeout=10)
            logger.info("host.stop", worker_process=self.worker_process)
            self.worker_process = None

    async def stop(self):
        self.stop_sync()
