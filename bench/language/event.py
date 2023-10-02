import enum
from dataclasses import dataclass
from uuid import UUID


class EventType(enum.StrEnum):
    Edit = "Edit"
    RunStatusChange = "RunStatusChange"
    WorkerStatusChange = "WorkerStatusChange"


@dataclass
class Event:
    id: UUID
    type: EventType
    # edit: Optional[Edit] = None
    # run: Optional[Run] = None
