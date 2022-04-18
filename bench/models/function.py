from django.db import models

from bench.models.utils import UUIDModel


class FunctionManager(models.Manager):
    pass


class Function(UUIDModel):
    registered_name = models.CharField(max_length=512)
    arguments = models.JSONField()
    datasets = models.ManyToManyField("Dataset", through="FunctionDatasetArgument")
    models = models.ManyToManyField("Model", through="FunctionModelArgument")

    objects = FunctionManager()


class FunctionModelArgument(UUIDModel):
    arguments = models.JSONField()
    version = models.CharField(max_length=256, null=True)
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    model = models.ForeignKey("Model", on_delete=models.RESTRICT)


class FunctionDatasetArgument(UUIDModel):
    arguments = models.JSONField()
    version = models.CharField(max_length=256, null=True)
    function = models.ForeignKey("Function", on_delete=models.RESTRICT)
    dataset = models.ForeignKey("Dataset", on_delete=models.RESTRICT)
