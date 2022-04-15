from django.db import models

from bench.models.utils import UUIDModel


class FunctionManager(models.Manager):
    pass


class Function(UUIDModel):
    registered_id = models.CharField(max_length=512)
    arguments = models.JSONField()
    datasets = models.ManyToManyField("Dataset", through="FunctionDatasetArgument")
    models = models.ManyToManyField("Model", through="FunctionModelArgument")

    objects = FunctionManager()


class FunctionModelArgument(models.Model):
    arguments = models.JSONField()
    version = models.CharField(max_length=256, null=True)
    function = models.ForeignKey("Function", on_delete=models.CASCADE)
    model = models.ForeignKey("Model", on_delete=models.CASCADE)


class FunctionDatasetArgument(models.Model):
    arguments = models.JSONField()
    version = models.CharField(max_length=256, null=True)
    function = models.ForeignKey("Function", on_delete=models.CASCADE)
    dataset = models.ForeignKey("Dataset", on_delete=models.CASCADE)


class FunctionEdge(UUIDModel):
    input_function = models.ForeignKey(Function, on_delete=models.CASCADE)
    output_function = models.ForeignKey(Function, on_delete=models.CASCADE)
