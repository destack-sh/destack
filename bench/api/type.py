import enum

import strawberry

from bench.language.edit import MET

EditType = strawberry.enum(MET)
MET = EditType


class ProjectMutationType(enum.StrEnum):
    RENAME_PROJECT = "RENAME_PROJECT"
    MOVE_PROJECT = "MOVE_PROJECT"
    COMMIT_PROJECT = "COMMIT_PROJECT"
    RESTORE_PROJECT = "RESTORE_PROJECT"


ProjectMutationType = strawberry.enum(ProjectMutationType)
PMT = ProjectMutationType
