from strawberry.types import Info
from strawberry_django_plus import gql
from strawberry_django_plus.types import OperationInfo

from bench import models
from bench.api.auth import check_can_read_project


def read_module(info: Info, input: gql.NodeInput) -> Module | OperationInfo:
    module = models.ProjectVersion.objects.get(id=input.id.node_id)
    check_can_read_project(info, module)
    return module
