import enum

import strawberry

from bench.language.edit import MET

# module mutations are special since they're used to sync semantic changes
# so they're defined in the language/runtime
EditType = strawberry.enum(MET)
MET = EditType


class ProjectMutationType(enum.StrEnum):
    RENAME_PROJECT = "RENAME_PROJECT"
    MOVE_PROJECT = "MOVE_PROJECT"
    COMMIT_PROJECT = "COMMIT_PROJECT"
    RESTORE_PROJECT = "RESTORE_PROJECT"


ProjectMutationType = strawberry.enum(ProjectMutationType)
PMT = ProjectMutationType
