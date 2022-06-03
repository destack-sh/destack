from typing import List

from rest_framework.routers import BaseRouter
from rest_framework_nested import routers

from bench.api.artifact import ArtifactVersionViewSet, ArtifactViewSet
from bench.api.dataset import DatasetVersionViewSet, DatasetViewSet, RecordViewSet
from bench.api.execution import ExecutionViewSet
from bench.api.model import ModelVersionViewSet, ModelViewSet


class ExtendedDefaultRouter(routers.DefaultRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # make trailing slash optional
        self.trailing_slash = r"/?"


class ExtendedNestedRouter(routers.NestedSimpleRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # make trailing slash optional
        self.trailing_slash = r"/?"


router = ExtendedDefaultRouter()
router.register("executions", ExecutionViewSet)

router.register("artifacts", ArtifactViewSet)
artifacts_router = ExtendedNestedRouter(router, "artifacts", lookup="artifact")
artifacts_router.register("versions", ArtifactVersionViewSet, basename="artifacts_versions")

router.register("datasets", DatasetViewSet)
datasets_router = ExtendedNestedRouter(router, "datasets", lookup="artifact")
datasets_router.register("versions", DatasetVersionViewSet, basename="datasets_versions")
datasets_versions_router = ExtendedNestedRouter(datasets_router, "versions", lookup="version")
datasets_versions_router.register("records", RecordViewSet, basename="datasets_versions_records")

router.register("models", ModelViewSet)
models_router = ExtendedNestedRouter(router, "models", lookup="artifact")
models_router.register("versions", ModelVersionViewSet, basename="models_versions")


api_routers: List[BaseRouter] = [
    router,
    artifacts_router,
    datasets_router,
    datasets_versions_router,
    models_router,
]
