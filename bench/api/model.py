from django.http import HttpRequest, HttpResponse

from bench.executor.base import Executor
from bench.executor.local import LocalExecutor

executor: Executor = LocalExecutor()


async def predict(request: HttpRequest):
    # model = await sync_to_async(get_object_or_404)(Model, pk=request.GET["model_id"])
    # version = request.GET["version"]
    # prediction = await executor.run_model(
    #     model,
    #     version=version,
    #     record={"text": request.GET["text"]},
    #     prepare_if_needed=True,
    # )
    #
    return HttpResponse()
