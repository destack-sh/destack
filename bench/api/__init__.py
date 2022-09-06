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
from bench.api.meta import list_dataset_handlers, list_function_handlers, list_model_handlers
from bench.api.model import ModelVersionViewSet, ModelViewSet
from bench.api.tag import TagViewSet


def _get_lookup_regex_simple(viewset: Type[ViewSetMixin], lookup_prefix: str = "") -> str:
    if not lookup_prefix:
        raise ValueError("can't use lookup_omit_field when lookup_prefix is not set")
    if lookup_prefix.endswith("_"):
        lookup_prefix = lookup_prefix[:-1]

    # simpler lookup regex that does not include the lookup_field
    # so e.g. instead of /artifacts/{artifact_name}/versions/{version_version}
    # it just becomes /artifacts/{artifact}/versions/{version}
    base_regex = r"(?P<{lookup_prefix}>{lookup_value})"
    lookup_value = getattr(viewset, "lookup_value_regex", "[^/.]+")
    # noinspection StrFormat
    lookup_regex = base_regex.format(lookup_prefix=lookup_prefix, lookup_value=lookup_value)
    return lookup_regex


class ExtendedDefaultRouter(routers.DefaultRouter):
    def __init__(self, *args, lookup_omit_field: bool, **kwargs):
        super().__init__(*args, **kwargs)
        self.lookup_omit_field = lookup_omit_field
        self.trailing_slash = r"/?"
        self.child_routers: List[ExtendedNestedRouter] = []

    def register_nested(
        self, prefix: str, viewset: Type[ViewSetMixin], lookup: str, lookup_omit_field: bool = None
    ) -> ExtendedNestedRouter:
        self.register(prefix, viewset)
        if lookup_omit_field is None:
            lookup_omit_field = self.lookup_omit_field
        nested_router = ExtendedNestedRouter(
            self, parent_prefix=prefix, lookup=lookup, lookup_omit_field=lookup_omit_field
        )
        self.child_routers.append(nested_router)
        return nested_router

    def get_lookup_regex(self, viewset: Type[ViewSetMixin], lookup_prefix: str = "") -> str:
        if self.lookup_omit_field and lookup_prefix:
            return _get_lookup_regex_simple(viewset, lookup_prefix)
        else:
            return super().get_lookup_regex(viewset, lookup_prefix)

    @property
    def descendant_routers(self):
        def _get_descendants(child_router: Union[ExtendedDefaultRouter, ExtendedNestedRouter]):
            yield from child_router.child_routers
            for grandchild_router in child_router.child_routers:
                yield from _get_descendants(grandchild_router)

        return list(_get_descendants(self))


class ExtendedNestedRouter(routers.NestedSimpleRouter):
    def __init__(self, *args, lookup_omit_field: bool = True, **kwargs):
        super().__init__(*args, **kwargs)
        self.lookup_omit_field = lookup_omit_field
        self.trailing_slash = r"/?"
        self.child_routers: List[ExtendedNestedRouter] = []

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
        self,
        prefix: str,
        viewset: Type[ViewSetMixin],
        lookup: str,
        basename: str = None,
        lookup_omit_field: bool = None,
    ) -> ExtendedNestedRouter:
        basename = basename or f"{self.parent_prefix}_{prefix}"
        self.register(prefix, viewset, basename)
        if lookup_omit_field is None:
            lookup_omit_field = self.lookup_omit_field
        nested_router = ExtendedNestedRouter(
            self, parent_prefix=prefix, lookup=lookup, lookup_omit_field=lookup_omit_field
        )
        self.child_routers.append(nested_router)
        return nested_router

    def get_lookup_regex(self, viewset: Type[ViewSetMixin], lookup_prefix: str = "") -> str:
        if self.lookup_omit_field and lookup_prefix:
            return _get_lookup_regex_simple(viewset, lookup_prefix)
        else:
            return super().get_lookup_regex(viewset, lookup_prefix)


router = ExtendedDefaultRouter(lookup_omit_field=True)
router.register("executions", ExecutionViewSet)
router.register("tags", TagViewSet)

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
    path("api/meta/datasets", list_dataset_handlers),
    path("api/meta/functions", list_function_handlers),
]
