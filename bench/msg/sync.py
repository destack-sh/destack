"""
Structs for syncing project model contents across servers and clients.
It shouldn't live in models, so we can use it in messages.py, which shouldn't depend on Django.
Maybe a better move would be to make the payload partially opaque and keep this in api.
"""

import enum
from dataclasses import dataclass
from typing import Optional
from uuid import UUID

from bench.language import StatementType


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
    CREATE_STATEMENT_BLANK = "CREATE_STATEMENT_BLANK"
    SOFT_DELETE_STATEMENT = "DELETE_STATEMENT"
    RESTORE_STATEMENT = "RESTORE_STATEMENT"
    UPDATE_STATEMENT_MODIFIER = "UPDATE_STATEMENT_MODIFIER"
    UPDATE_STATEMENT_REFERENCE = "UPDATE_STATEMENT_REFERENCE"
    MORPH_STATEMENT = "MORPH_STATEMENT"
    COMMENT_STATEMENT = "COMMENT_STATEMENT"
    MOVE_STATEMENT = "MOVE_STATEMENT"
    RENAME_STATEMENT = "RENAME_STATEMENT"
    # Statement content ("symbol") mutations
    UPDATE_STATEMENT_TEXT = "UPDATE_STATEMENT_TEXT"  # for comments
    UPDATE_STATEMENT_DESCRIPTION = "UPDATE_STATEMENT_DESCRIPTION"
    UPDATE_STATEMENT_CODE = "UPDATE_STATEMENT_CODE"
    UPDATE_STATEMENT_LANGUAGE = "UPDATE_STATEMENT_LANGUAGE"
    # Relational symbol content mutations
    CREATE_TYPE_NODE = "CREATE_TYPE_NODE"
    UPDATE_TYPE_NODE = "UPDATE_TYPE_NODE"
    MOVE_TYPE_NODE = "MOVE_TYPE_NODE"
    DELETE_TYPE_NODE = "DELETE_TYPE_NODE"
    RESTORE_TYPE_NODE = "RESTORE_TYPE_NODE"
    CREATE_RECORD = "CREATE_RECORD"
    UPDATE_RECORD = "UPDATE_RECORD"
    MOVE_RECORD = "MOVE_RECORD"
    DELETE_RECORD = "DELETE_RECORD"
    RESTORE_RECORD = "RESTORE_RECORD"


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
    ProjectMutationType.CREATE_STATEMENT_BLANK,
    ProjectMutationType.UPDATE_STATEMENT_TEXT,  # for comments
    ProjectMutationType.MOVE_TYPE_NODE,
    ProjectMutationType.MOVE_RECORD,
}
NON_SEMANTIC_STATEMENT_TYPES = {
    StatementType.COMMENT,
    StatementType.BLANK,
}


def is_semantic_statement(statement_type: StatementType) -> bool:
    return statement_type not in NON_SEMANTIC_STATEMENT_TYPES


def is_semantic_mutation(mutation: ProjectMutation) -> bool:
    # trivial filter for definitely non-semantic mutations
    # we could do more here (like filter blank morphs), but not worth it now
    return mutation.type not in NON_SEMANTIC_MUTATION_TYPES
