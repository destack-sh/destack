from rest_framework_nested import routers

from bench.api.dataset import DatasetVersionViewSet, DatasetViewSet, RecordViewSet


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
router.register("datasets", DatasetViewSet)

datasets_router = ExtendedNestedRouter(router, "datasets", lookup="artifact")
datasets_router.register("versions", DatasetVersionViewSet, basename="datasets_versions")

datasets_versions_router = ExtendedNestedRouter(datasets_router, "versions", lookup="version")
datasets_versions_router.register("records", RecordViewSet, basename="datasets_versions_records")

api_routers = [router, datasets_router, datasets_versions_router]
