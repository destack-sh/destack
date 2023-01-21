"""
Structs for syncing project model contents across servers and clients.
Does not strictly relate to zmq, but is used here and I couldn't think of a better place.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""

import enum
from dataclasses import dataclass
from typing import Optional
from uuid import UUID


class ProjectMutationType(enum.Enum):
    CREATE_FILE = "CREATE_FILE"
    SOFT_DELETE_FILE = "DELETE_FILE"
    RESTORE_FILE = "RESTORE_FILE"
    RENAME_FILE = "RENAME_FILE"
    MOVE_FILE = "MOVE_FILE"
    CREATE_STATEMENT = "CREATE_STATEMENT"
    SOFT_DELETE_STATEMENT = "DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    COMMIT = "COMMIT"


@dataclass
class ProjectMutation:
    type: ProjectMutationType
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None
    revision: Optional[int] = None
