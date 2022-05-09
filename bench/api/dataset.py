from asgiref.sync import sync_to_async
from django.http import Http404, HttpRequest, JsonResponse

from bench.executor.base import Executor
from bench.executor.local import LocalExecutor
from bench.models import DatasetVersion

executor: Executor = LocalExecutor()


def _load_dataset_version(name: str, version: str) -> DatasetVersion:
    try:
        return DatasetVersion.objects.select_related("artifact").get(
            artifact__name=name, version=version
        )
    except DatasetVersion.DoesNotExist:
        raise Http404()


async def get(request: HttpRequest) -> JsonResponse:
    dataset_version = await sync_to_async(_load_dataset_version)(
        name=request.GET["name"], version=request.GET["version"]
    )
    await executor.load_artifact(dataset_version)

    return JsonResponse()
