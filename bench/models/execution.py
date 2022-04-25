from django.db import models

from bench.models.utils import UUIDModel


class ExecutionManager(models.Manager):
    pass


class Execution(UUIDModel):
    created_at = models.DateTimeField(auto_now_add=True)
    updated_at = models.DateTimeField(auto_now=True)
    started_at = models.DateTimeField()
    state = models.CharField(max_length=256)

    objects = ExecutionManager()


class FunctionExecution(Execution):
    function = models.ForeignKey("Function", on_delete=models.CASCADE)


class FlowExecution(Execution):
    flow = models.ForeignKey("Flow", on_delete=models.CASCADE)
