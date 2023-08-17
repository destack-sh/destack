import asyncio
import typing
from typing import Optional
from uuid import UUID

import structlog

from bench import models
from bench.models import packer
from bench.msg import NMessage
from bench.msg.core import request, subscribe
from bench.msg.messages import (
    NMessageType,
    RepWakeWorkerSetPayload,
    ReqWakeWorkerSetPayload,
    WorkersChangedPayload,
)

logger = structlog.get_logger(__name__)


class WorkerObserver:
    """Observe the state of all worker sets across all modules."""

    def __init__(self):
        self._worker_sets_by_project_id: dict[UUID, models.WorkerSet] = {}
        self._subs = []
        self._until_healthy_waiters: dict[UUID, asyncio.Event] = {}

    @property
    def worker_sets(self) -> typing.Collection[models.WorkerSet]:
        return self._worker_sets_by_project_id.values()

    async def start(self):
        logger.info("worker_observer.start")
        self._subs = [
            await subscribe(
                f"{NMessageType.WORKERS_CHANGED}.>",
                payload_t=WorkersChangedPayload,
                cb=self._on_workers_changed,
            )
        ]
        self._worker_sets_by_project_id = {
            ws.project_id: ws
            async for ws in models.WorkerSet.objects.select_related(
                "project", "project__organization", "project__user"
            ).all()
        }
        logger.info("worker_observer.ready", worker_sets=self.worker_sets)

    async def _on_workers_changed(self, msg: NMessage[WorkersChangedPayload]):
        logger.debug("worker_observer.change", msg=msg)
        for updated_ws in msg.p.worker_sets:
            # upsert properties in local worker set
            updated_ws: models.WorkerSet = packer.unpack_data(updated_ws)
            if updated_ws.project_id not in self._worker_sets_by_project_id:
                ws = await models.WorkerSet.objects.select_related(
                    "project", "project__organization", "project__user"
                ).aget(project_id=updated_ws.project_id)
            else:
                ws = self._worker_sets_by_project_id[updated_ws.project_id]
            for field in models.WorkerSet._meta.fields:
                # skip relational fields
                if field.is_relation:
                    continue
                setattr(ws, field.name, getattr(updated_ws, field.name))

            # trigger 'until healthy' wait events
            if (
                ws.status == models.WorkerSetStatus.HEALTHY
                and ws.project_id in self._until_healthy_waiters
            ):
                self._until_healthy_waiters[ws.project_id].set()

    def is_healthy(self, project_id: UUID) -> bool:
        """Return whether the worker set is healthy."""
        worker_set = self._worker_sets_by_project_id.get(project_id)
        return worker_set and worker_set.status == models.WorkerSetStatus.HEALTHY

    def get(self, project_id: UUID) -> Optional[models.WorkerSet]:
        """Return the worker set if it exists."""
        return self._worker_sets_by_project_id.get(project_id)

    async def wake_until_healthy(self, project_id: UUID, timeout: Optional[int] = None):
        """If not already healthy, wake the worker set and wait until it is healthy."""
        worker_set = self._worker_sets_by_project_id.get(project_id)
        log = logger.bind(project_id=project_id, worker_set=worker_set)
        log.info("worker_observer.wait_until_healthy")
        if worker_set and worker_set.status == models.WorkerSetStatus.HEALTHY:
            return  # already good

        # create waiter
        if project_id not in self._until_healthy_waiters:
            self._until_healthy_waiters[project_id] = asyncio.Event()

        # wake if needed
        if not worker_set or worker_set.sleeping:
            log.info("worker_observer.wait_until_healthy.wake")
            rep: NMessage[RepWakeWorkerSetPayload] = await request(
                NMessageType.WAKE_WORKER_SET,
                ReqWakeWorkerSetPayload(project_id=project_id),
                reply_t=RepWakeWorkerSetPayload,
                retry=3,
            )
            if not rep.p.success:
                raise RuntimeError(f"failed to wake worker set {worker_set}: {rep.p.error}")

        # and wait
        if timeout:
            await asyncio.wait_for(self._until_healthy_waiters[project_id].wait(), timeout)
        else:
            await self._until_healthy_waiters[project_id].wait()
            del self._until_healthy_waiters[project_id]
        log.info("worker_observer.wait_until_healthy.done")

    async def stop(self):
        for sub in self._subs:
            await sub.unsubscribe()
        self._subs.clear()
