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
    COMMIT = "COMMIT"
    # File mutations
    CREATE_FILE = "CREATE_FILE"
    SOFT_DELETE_FILE = "DELETE_FILE"
    RESTORE_FILE = "RESTORE_FILE"
    RENAME_FILE = "RENAME_FILE"
    MOVE_FILE = "MOVE_FILE"
    # Statement mutations
    CREATE_STATEMENT = "CREATE_STATEMENT"
    SOFT_DELETE_STATEMENT = "DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    UPDATE_STATEMENT_MODIFIER = "UPDATE_STATEMENT_MODIFIER"
    UPDATE_STATEMENT_REFERENCE = "UPDATE_STATEMENT_REFERENCE"
    MORPH_STATEMENT = "MORPH_STATEMENT"
    COMMENT_STATEMENT = "COMMENT_STATEMENT"
    MOVE_STATEMENT = "MOVE_STATEMENT"
    RENAME_STATEMENT = "RENAME_STATEMENT"
    UPDATE_STATEMENT_TEXT = "UPDATE_STATEMENT_TEXT"  # for comments
    # Statement content ("symbol") mutations
    UPDATE_STATEMENT_TYPE_NODE = "UPDATE_STATEMENT_TYPE_NODE"
    UPDATE_STATEMENT_DESCRIPTION = "UPDATE_STATEMENT_DESCRIPTION"
    UPDATE_STATEMENT_CODE = "UPDATE_STATEMENT_CODE"
    UPDATE_STATEMENT_LANGUAGE = "UPDATE_STATEMENT_LANGUAGE"
    UPDATE_STATEMENT_RECORDS = "UPDATE_STATEMENT_RECORDS"


@dataclass
class ProjectMutation:
    type: ProjectMutationType
    project_version_id: UUID
    file_id: Optional[UUID] = None
    statement_id: Optional[UUID] = None

    revision: Optional[int] = None


NON_SEMANTIC_MUTATION_TYPES = {
    ProjectMutationType.COMMIT,
    ProjectMutationType.CREATE_FILE,
    ProjectMutationType.CREATE_STATEMENT,  # statements start as blanks
    ProjectMutationType.UPDATE_STATEMENT_TEXT,  # for comments
}


def is_semantic(mutation: ProjectMutation) -> bool:
    # trivial filter for definitely non-semantic mutations
    # we could do more here (like filter blank morphs), but not worth it now
    return mutation.type not in NON_SEMANTIC_MUTATION_TYPES
