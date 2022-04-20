from django.db import models

from bench.models.utils import UUIDModel


class FunctionManager(models.Manager):
    pass


class Function(UUIDModel):
    """
    A function is a curried variant of a registered data function with given arguments,
    including any required datasets and models.
    """

    registered_name = models.CharField(max_length=512)
    arguments = models.JSONField()
    created_at = models.DateTimeField(auto_now_add=True)

    datasets = models.ManyToManyField("Dataset", through="FunctionDatasetArgument")
    models = models.ManyToManyField("Model", through="FunctionModelArgument")

    objects = FunctionManager()


class FunctionModelArgument(UUIDModel):
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    model = models.ForeignKey("Model", on_delete=models.RESTRICT)
    model_version = models.CharField(max_length=256, null=True)
    model_arguments = models.JSONField()


class FunctionDatasetArgument(UUIDModel):
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    dataset = models.ForeignKey("Dataset", on_delete=models.RESTRICT)
    dataset_version = models.CharField(max_length=256, null=True)
    dataset_arguments = models.JSONField()
