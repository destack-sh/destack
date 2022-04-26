from django.db import models

from bench.models.utils import MAX_NAME_LENGTH, UUIDModel


class Controller(UUIDModel):
    """
    A controller is an external entity that manages some part of the ML lifecycle,
    for instance by creating, hosting, serving or gating certain artifacts.

    Depending on the controller we may just use its artefacts, synchronize/mirror it,
    or even subsume it by importing its artifacts/functions/flows/etc. Generally,
    if no controller is set on an instance, we control that instance directly.

    Examples: Mlflow, Comet, HuggingFace hub, TF hub, OpenAI, AI21 Labs.
    """

    registered_id = models.CharField(max_length=256)
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    arguments = models.JSONField()
    created_at = models.DateTimeField(auto_now_add=True)
