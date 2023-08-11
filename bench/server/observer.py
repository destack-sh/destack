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


class WorkerSetObserver:
    """Observe the state of all worker sets across all modules."""

    def __init__(self):
        self._worker_sets_by_project_id: dict[UUID, models.WorkerSet] = {}
        self._subs = []
        self._until_healthy_events: dict[UUID, asyncio.Event] = {}

    @property
    def worker_sets(self) -> typing.Collection[models.WorkerSet]:
        return self._worker_sets_by_project_id.values()

    async def start(self):
        logger.info("worker_set_observer.start")
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
        logger.info("worker_set_observer.ready", worker_sets=self.worker_sets)

    async def _on_workers_changed(self, msg: NMessage[WorkersChangedPayload]):
        logger.debug("worker_set_observer.change", msg=msg)
        for updated_ws in msg.p.worker_sets:
            # upsert properties in local worker set
            updated_ws: models.WorkerSet = packer.unpack_data(updated_ws)
            if updated_ws.project_id not in self._worker_sets_by_project_id:
                existing_ws = await models.WorkerSet.objects.select_related(
                    "project", "project__organization", "project__user"
                ).aget(project_id=updated_ws.project_id)
            else:
                existing_ws = self._worker_sets_by_project_id[updated_ws.project_id]
            for field in models.WorkerSet._meta.fields:
                # skip relational fields
                if field.is_relation:
                    continue
                setattr(existing_ws, field.name, getattr(updated_ws, field.name))

            # trigger until_healthy events
            if existing_ws.project_id in self._until_healthy_events:
                self._until_healthy_events[existing_ws.project_id].set()

    def is_healthy(self, project_id: UUID) -> bool:
        """Return whether the worker set is healthy."""
        worker_set = self._worker_sets_by_project_id.get(project_id)
        return worker_set and worker_set.status == models.WorkerSetStatus.HEALTHY

    async def wake_until_healthy(self, project_id: UUID, timeout: Optional[int] = None):
        """If not already healthy, wake the worker set and wait until it is healthy."""
        logger.info("worker_set_observer.wait_until_healthy", project_id=project_id)
        worker_set = self._worker_sets_by_project_id.get(project_id)
        if worker_set and worker_set.status != models.WorkerSetStatus.HEALTHY:
            return
        if project_id not in self._until_healthy_events:
            self._until_healthy_events[project_id] = asyncio.Event()

        if not worker_set or worker_set.sleeping:
            rep: NMessage[RepWakeWorkerSetPayload] = await request(
                NMessageType.WAKE_WORKER_SET,
                ReqWakeWorkerSetPayload(project_id=project_id),
                reply_t=RepWakeWorkerSetPayload,
            )
            if not rep.p.success:
                raise RuntimeError(f"failed to wake worker set {worker_set}: {rep.p.error}")

        if timeout:
            await asyncio.wait_for(self._until_healthy_events[project_id].wait(), timeout)
        else:
            await self._until_healthy_events[project_id].wait()
            del self._until_healthy_events[project_id]

    async def stop(self):
        for sub in self._subs:
            await sub.unsubscribe()
        self._subs.clear()
