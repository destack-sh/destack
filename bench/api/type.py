import enum

from strawberry_django_plus import gql

from bench.language.mutate import MMT

# module mutations are special since they're used to sync semantic changes
# so they're defined in the language/runtime
ModuleMutationType = gql.enum(MMT)
MMT = ModuleMutationType


class ProjectMutationType(enum.StrEnum):
    COMMIT = "commit"
    RESTORE = "restore"


ProjectMutationType = gql.enum(ProjectMutationType)
PMT = ProjectMutationType
