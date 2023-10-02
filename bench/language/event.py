import enum
from dataclasses import dataclass, field
from uuid import UUID, uuid4


class EventType(enum.StrEnum):
    Edit = "Edit"
    RunStatusChange = "RunStatusChange"
    WorkerStatusChange = "WorkerStatusChange"


@dataclass
class Event:
    id: UUID = field(default_factory=uuid4)
