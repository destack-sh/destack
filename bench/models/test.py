from django.db import models

from bench.models import Function
from bench.models.execution import Execution, FunctionExecution
from bench.models.utils import MAX_DESCRIPTION_LENGTH, MAX_NAME_LENGTH


class Test(Function):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    class Meta:
        proxy = True


class TestExecution(FunctionExecution):
    test_suite_run = models.ForeignKey(
        "TestExecution", on_delete=models.CASCADE, blank=True, null=True
    )


class TestSuite(Function):
    name = models.CharField(max_length=MAX_NAME_LENGTH)
    description = models.CharField(max_length=MAX_DESCRIPTION_LENGTH)

    class Meta:
        proxy = True


class TestSuiteExecution(Execution):
    test_suite = models.ForeignKey("TestSuite", on_delete=models.CASCADE)
