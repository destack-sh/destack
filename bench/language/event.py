import enum
from dataclasses import dataclass
from uuid import UUID


class EventKind(enum.StrEnum):
    Edit = "Edit"
    RunChange = "RunChange"
    WorkerChange = "WorkerChange"


@dataclass
class Event:
    id: UUID
    kind: EventKind
    # edit: Optional[Edit] = None
    # run: Optional[Run] = None
