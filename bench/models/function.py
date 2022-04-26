from django.db import models

from bench.models.utils import UUIDModel


class FunctionManager(models.Manager):
    pass


class Function(UUIDModel):
    """
    A function is a curried variant of a registered data function with given arguments,
    including any required "init-time" artifacts like datasets and models.

    A function may be managed by or dependent on an external Controller, who can provide
    additional configuration arguments and control the function execution.
    """

    registered_id = models.CharField(max_length=256)
    arguments = models.JSONField()
    created_at = models.DateTimeField(auto_now_add=True)
    controller = models.ForeignKey(
        "Controller", on_delete=models.RESTRICT, blank=True, null=True
    )

    # TODO @Cleanup: should function arguments not be in Flow?
    functions = models.ManyToManyField("Function", through="FunctionFunctionArgument")
    datasets = models.ManyToManyField("Dataset", through="FunctionDatasetArgument")
    models = models.ManyToManyField("Model", through="FunctionModelArgument")

    objects = FunctionManager()


class FunctionFunctionArgument(UUIDModel):
    caller_function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    callee_function = models.ForeignKey("Function", on_delete=models.RESTRICT)


class FunctionModelArgument(UUIDModel):
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    model = models.ForeignKey("Model", on_delete=models.RESTRICT)
    model_version = models.ForeignKey("ArtifactVersion", on_delete=models.RESTRICT)


class FunctionDatasetArgument(UUIDModel):
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    dataset = models.ForeignKey("Dataset", on_delete=models.RESTRICT)
    dataset_version = models.ForeignKey("ArtifactVersion", on_delete=models.RESTRICT)
