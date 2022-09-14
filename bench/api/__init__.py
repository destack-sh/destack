from __future__ import annotations

from typing import List

from django.urls import path
from rest_framework.routers import BaseRouter

from bench.api.artifact import ArtifactVersionViewSet, ArtifactViewSet
from bench.api.dataset import DatasetRecordViewSet, DatasetVersionViewSet, DatasetViewSet
from bench.api.execution import ExecutionViewSet
from bench.api.flow import (
    FlowArtifactEdgeViewSet,
    FlowNodeEdgeViewSet,
    FlowNodeViewSet,
    FlowVersionViewSet,
    FlowViewSet,
)
from bench.api.meta import list_dataset_handlers, list_function_handlers, list_model_handlers
from bench.api.model import ModelVersionViewSet, ModelViewSet
from bench.api.organization import OrganizationViewSet
from bench.api.project import ProjectViewSet
from bench.api.routing import ExtendedDefaultRouter
from bench.api.tag import TagViewSet
from bench.api.team import TeamViewSet
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
# /organizations/<organization>/teams
teams_router = organizations_router.register_nested("teams", TeamViewSet, lookup="team")

# /organizations/<organization>/projects
projects_router = organizations_router.register_nested("projects", ProjectViewSet, lookup="project")

# /users
users_router = router.register_nested("users", UserViewSet, lookup="user")

# /artifacts/<organization> and /artifacts/<organization>/<artifact>
artifacts_router = router.register_nested(
    "artifacts", ArtifactViewSet, lookup=("organization", "artifact")
)
# /artifacts/<organization>/<artifact>/versions
artifacts_router.register("versions", ArtifactVersionViewSet)

# /datasets/<organization> and /datasets/<organization>/<artifact>
datasets_router = router.register_nested(
    "datasets", DatasetViewSet, lookup=("organization", "artifact")
)
# /datasets/<organization>/<artifact>/records
datasets_router.register("records", DatasetRecordViewSet)
# /datasets/<organization>/<artifact>/versions
datasets_versions_router = datasets_router.register_nested(
    "versions", DatasetVersionViewSet, lookup="version"
)
# /datasets/<organization>/<artifact>/versions/<version>/records
datasets_versions_router.register("records", DatasetRecordViewSet)

# /models/<organization> and /models/<organization>/<model>
models_router = router.register_nested("models", ModelViewSet, lookup=("organization", "artifact"))
# /models/<organization>/<model>/versions
models_router.register("versions", ModelVersionViewSet)

# /flows/<organization>/<flow>
flows_router = router.register_nested("flows", FlowViewSet, lookup=("organization", "flow"))
# /flows/<organization>/<flow>/versions
flows_versions_router = flows_router.register_nested(
    "versions", FlowVersionViewSet, lookup="version"
)
# /flows/<organization>/<flow>/versions/<version>/nodes
flows_versions_router.register("nodes", FlowNodeViewSet)
# /flows/<organization>/<flow>/versions/<version>/node_edges
flows_versions_router.register("node_edges", FlowNodeEdgeViewSet)
# /flows/<organization>/<flow>/versions/<version>/artifact_edges
flows_versions_router.register("artifact_edges", FlowArtifactEdgeViewSet)

api_routers: List[BaseRouter] = [router, *router.descendant_routers]
api_patterns = [
    path("api/meta/models", list_model_handlers),
    path("api/meta/datasets", list_dataset_handlers),
    path("api/meta/functions", list_function_handlers),
]
