from asgiref.sync import sync_to_async
from django.http import HttpRequest, HttpResponse
from django.shortcuts import get_object_or_404

from bench.executor.base import Executor
from bench.executor.local import LocalExecutor
from bench.models import Model

executor: Executor = LocalExecutor()


async def predict(request: HttpRequest):
    model = await sync_to_async(get_object_or_404)(Model, pk=request.GET["model_id"])
    version = request.GET.get("version")
    prediction = await executor.run_model(
        model,
        version=version,
        record={"text": request.GET["text"]},
        prepare_if_needed=True,
    )

    return HttpResponse(prediction)
