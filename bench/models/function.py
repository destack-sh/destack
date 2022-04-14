from django.db import models

from bench.models.utils import UUIDModel


class Function(UUIDModel):
    path = models.CharField(max_length=200)
    arguments = models.JSONField()
    datasets = models.ManyToManyField("Dataset", through="FunctionDatasetArgument")
    models = models.ManyToManyField("Model", through="FunctionModelArgument")


class FunctionModelArgument(models.Model):
    arguments = models.JSONField()
    function = models.ForeignKey("Function", on_delete=models.CASCADE)
    model = models.ForeignKey("Model", on_delete=models.CASCADE)


class FunctionDatasetArgument(models.Model):
    arguments = models.JSONField()
    function = models.ForeignKey("Function", on_delete=models.CASCADE)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)
