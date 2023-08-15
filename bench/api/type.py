import enum

import strawberry

from bench.language.mutate import MMT

# module mutations are special since they're used to sync semantic changes
# so they're defined in the language/runtime
ModuleMutationType = strawberry.enum(MMT)
MMT = ModuleMutationType


class ProjectMutationType(enum.StrEnum):
    RENAME_PROJECT = "RENAME_PROJECT"
    MOVE_PROJECT = "MOVE_PROJECT"
    COMMIT_PROJECT = "COMMIT_PROJECT"
    RESTORE_PROJECT = "RESTORE_PROJECT"


ProjectMutationType = strawberry.enum(ProjectMutationType)
PMT = ProjectMutationType
