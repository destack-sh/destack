from rest_framework_extensions.routers import ExtendedDefaultRouter

from bench.api.dataset import DatasetVersionViewSet, DatasetViewSet, RecordViewSet


class ExtendedDefaultRouterWithSlash(ExtendedDefaultRouter):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.trailing_slash = r"/?"


router = ExtendedDefaultRouterWithSlash()

datasets_router = router.register("datasets", DatasetViewSet)
datasets_versions_router = datasets_router.register(
    "versions",
    DatasetVersionViewSet,
    "datasets_versions",
    parents_query_lookups=["artifact_name"],
)
datasets_versions_router.register(
    "records",
    RecordViewSet,
    "datasets_versions_records",
    parents_query_lookups=["artifact_name", "version"],
)
