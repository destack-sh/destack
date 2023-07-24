import json
from dataclasses import asdict, dataclass
from datetime import datetime
from pathlib import Path
from typing import Optional
from uuid import UUID

import structlog

from bench.language.wire import WorkerSetData

#
# Utilities for managing local workers during debugging.
# This is quite hacky but it's only used in local development.
#

LOCAL_WORKERS_CONFIG_PATH = ".local_workers.json"

logger = structlog.get_logger(__name__)


@dataclass
class _LocalWorkers:
    updated_at: Optional[datetime]
    worker_sets: dict[UUID, WorkerSetData]

    @property
    def live_worker_sets(self) -> dict[UUID, WorkerSetData]:
        return {
            project_id: worker_set
            for project_id, worker_set in self.worker_sets.items()
            if worker_set.target_replicas > 0
        }

    def to_dict(self):
        return asdict(self)

    @classmethod
    def from_dict(cls, data):
        return {
            "updated_at": datetime.fromisoformat(data["updated_at"]),
            "worker_sets": {
                UUID(project_id): _LocalWorkers(**data)
                for project_id, data in data["worker_sets"].items()
            },
        }


def _load_local_workers():
    try:
        return _LocalWorkers.from_dict(json.loads(Path(LOCAL_WORKERS_CONFIG_PATH).read_text()))
    except FileNotFoundError:
        return _LocalWorkers(updated_at=None, worker_sets={})


LOCAL_WORKERS = _load_local_workers()


def reload_local_workers():
    global LOCAL_WORKERS
    LOCAL_WORKERS = _load_local_workers()


def update_local_workers(worker_sets: list[WorkerSetData]):
    for worker_set in worker_sets:
        LOCAL_WORKERS.worker_sets[worker_set.id] = worker_set
    LOCAL_WORKERS.updated_at = datetime.now()
    Path(LOCAL_WORKERS_CONFIG_PATH).write_text(json.dumps(LOCAL_WORKERS.to_dict()))
    logger.info("local_workers.updated", worker_sets=worker_sets)
