from __future__ import annotations

from typing import List, Optional, Type, Union

from django.urls import path
from rest_framework.routers import BaseRouter
from rest_framework.viewsets import ViewSetMixin
from rest_framework_nested import routers

from bench.api.artifact import ArtifactTagsViewSet, ArtifactVersionViewSet, ArtifactViewSet
from bench.api.dataset import DatasetVersionViewSet, DatasetViewSet, RecordViewSet
from bench.api.execution import ExecutionViewSet
from bench.api.flow import (
    FlowArtifactEdgeViewSet,
    FlowNodeEdgeViewSet,
    FlowNodeViewSet,
    FlowVersionViewSet,
    FlowViewSet,
)
from bench.api.meta import list_model_handlers
from bench.api.model import ModelVersionViewSet, ModelViewSet


class ExtendedDefaultRouter(routers.DefaultRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # make trailing slash optional
        self.trailing_slash = r"/?"
        self.child_routers = []

    def register_nested(
        self, prefix: str, viewset: Type[ViewSetMixin], lookup: str
    ) -> ExtendedNestedRouter:
        self.register(prefix, viewset)
        nested_router = ExtendedNestedRouter(self, parent_prefix=prefix, lookup=lookup)
        self.child_routers.append(nested_router)
        return nested_router

    @property
    def descendant_routers(self):
        def _get_descendants(child_router: Union[ExtendedDefaultRouter, ExtendedNestedRouter]):
            yield from child_router.child_routers
            for grandchild_router in child_router.child_routers:
                yield from _get_descendants(grandchild_router)

        return list(_get_descendants(self))


class ExtendedNestedRouter(routers.NestedSimpleRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # make trailing slash optional
        self.trailing_slash = r"/?"
        self.child_routers = []

    def register(
        self,
        prefix: str,
        viewset: Type[ViewSetMixin],
        basename: Optional[str] = None,
        base_name: Optional[str] = None,
    ) -> None:
        basename = basename or base_name or f"{self.parent_prefix}_{prefix}"
        super().register(prefix, viewset, basename)

    def register_nested(
        self, prefix: str, viewset: Type[ViewSetMixin], lookup: str, basename: str = None
    ) -> ExtendedNestedRouter:
        basename = basename or f"{self.parent_prefix}_{prefix}"
        self.register(prefix, viewset, basename)
        nested_router = ExtendedNestedRouter(self, parent_prefix=prefix, lookup=lookup)
        self.child_routers.append(nested_router)
        return nested_router


router = ExtendedDefaultRouter()
router.register("executions", ExecutionViewSet)

artifacts_router = router.register_nested("artifacts", ArtifactViewSet, lookup="artifact")
artifacts_router.register("versions", ArtifactVersionViewSet)
artifacts_router.register("tags", ArtifactTagsViewSet)

datasets_router = router.register_nested("datasets", DatasetViewSet, lookup="artifact")
datasets_versions_router = datasets_router.register_nested(
    "versions", DatasetVersionViewSet, lookup="version"
)
datasets_versions_router.register("records", RecordViewSet)

models_router = router.register_nested("models", ModelViewSet, lookup="artifact")
models_router.register("versions", ModelVersionViewSet)

flows_router = router.register_nested("flows", FlowViewSet, lookup="flow")
flows_versions_router = flows_router.register_nested(
    "versions", FlowVersionViewSet, lookup="version"
)
flows_versions_router.register("nodes", FlowNodeViewSet)
flows_versions_router.register("node_edges", FlowNodeEdgeViewSet)
flows_versions_router.register("artifact_edges", FlowArtifactEdgeViewSet)

api_routers: List[BaseRouter] = [router, *router.descendant_routers]
api_patterns = [
    path("api/meta/models", list_model_handlers),
]
