from asgiref.sync import sync_to_async
from django.http import Http404, HttpRequest, JsonResponse

from bench.executor.base import Executor
from bench.executor.local import LocalExecutor
from bench.models import ModelVersion

executor: Executor = LocalExecutor()


def _load_model_version(name: str, version: str) -> ModelVersion:
    try:
        return ModelVersion.objects.select_related("artifact").get(
            artifact__name=name, version=version
        )
    except ModelVersion.DoesNotExist:
        raise Http404()


async def predict(request: HttpRequest):
    model = await sync_to_async(_load_model_version)(
        name=request.GET["name"], version=request.GET["version"]
    )
    prediction = await executor.run_model(
        model,
        record={"text": request.GET["text"]},
        load_if_needed=True,
    )
    return JsonResponse(prediction)
