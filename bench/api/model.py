from django.http import HttpRequest, HttpResponse

from bench.model.base import load_model

loaded_model = None


def predict(request: HttpRequest):
    # TODO @Feature: offload model to executor, store model properly, ...
    global loaded_model
    loaded_model = load_model(
        "bench.spacy.bundled",
        storage_uri=None,
        arguments={"model_name": "en_core_web_sm"},
        model_spec=None,
    )
    output = loaded_model.predict({"text": request.GET["text"]})
    return HttpResponse(output)
