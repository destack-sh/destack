from __future__ import annotations

from typing import List

from rest_framework.routers import BaseRouter

from bench.api.dataset import DatasetRecordViewSet, DatasetVersionViewSet, DatasetViewSet
from bench.api.execution import ExecutionViewSet
from bench.api.flow import FlowInstructionViewSet, FlowVersionViewSet, FlowViewSet
from bench.api.model import ModelViewSet
from bench.api.organization import OrganizationViewSet
from bench.api.project import ProjectViewSet
from bench.api.routing import ExtendedDefaultRouter
from bench.api.tag import TagViewSet
from bench.api.user import UserViewSet

router = ExtendedDefaultRouter(lookup_omit_field=True)

# /organizations
organizations_router = router.register_nested(
    "organizations", OrganizationViewSet, lookup="organization"
)
# /organizations/<organization>/tags
organizations_router.register("tags", TagViewSet)
# /organizations/<organization>/executions
organizations_router.register("executions", ExecutionViewSet)

# /organizations/<organization>/projects
projects_router = organizations_router.register_nested("projects", ProjectViewSet, lookup="project")

# /users
users_router = router.register_nested("users", UserViewSet, lookup="user")

# /datasets/<organization> and /datasets/<organization>/<dataset>
datasets_router = router.register_nested(
    "datasets", DatasetViewSet, lookup=("organization", "artifact")
)
# /datasets/<organization>/<dataset>/records
datasets_router.register("records", DatasetRecordViewSet)
# /datasets/<organization>/<dataset>/versions
datasets_versions_router = datasets_router.register_nested(
    "versions", DatasetVersionViewSet, lookup="version"
)
# /datasets/<organization>/<dataset>/versions/<version>/records
datasets_versions_router.register("records", DatasetRecordViewSet)

# /models/<organization> and /models/<organization>/<model>
models_router = router.register_nested("models", ModelViewSet, lookup=("organization", "artifact"))

# /flows/<organization>/<flow>
flows_router = router.register_nested("flows", FlowViewSet, lookup=("organization", "flow"))
# /flows/<organization>/<flow>/versions
flows_versions_router = flows_router.register_nested(
    "versions", FlowVersionViewSet, lookup="version"
)

routers: List[BaseRouter] = [router, *router.descendant_routers]
